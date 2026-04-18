use crate::models::cluster::ClusterProfileDto;
use crate::models::correlation::CorrelationRuleDto;
use crate::models::error::{AppError, AppResult};
use crate::models::message::{MessageRefDto, TimeRangeDto};
use crate::models::trace::{
    RunTraceQueryRequest, TraceEventDto, TraceQueryResultDto, TraceQuerySummaryDto,
};
use crate::repositories::sqlite;
use crate::services::kafka_config::apply_kafka_read_consumer_config;
use chrono::{Local, NaiveDateTime, TimeZone};
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::{Headers, Message};
use rdkafka::topic_partition_list::{Offset, TopicPartitionList};
use rdkafka::ClientConfig;
use serde_json::Value;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

const MAX_TRACE_RESULTS: usize = 200;
const MAX_TRACE_TOPICS: usize = 12;

pub struct TraceService<'a> {
    pool: &'a SqlitePool,
}

struct PartitionBounds {
    start: i64,
    end_exclusive: i64,
}

impl<'a> TraceService<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn run_trace_query(
        &self,
        request: RunTraceQueryRequest,
    ) -> AppResult<TraceQueryResultDto> {
        validate_trace_request(&request)?;

        let profile = sqlite::get_cluster_profile(self.pool, &request.cluster_profile_id).await?;
        let enabled_rules = sqlite::list_correlation_rules(self.pool)
            .await?
            .into_iter()
            .filter(|rule| rule.is_enabled && rule.cluster_profile_id == request.cluster_profile_id)
            .collect::<Vec<_>>();
        let topics = resolve_trace_topics(&request, &enabled_rules)?;
        let confidence_notes = build_confidence_notes(&request, &enabled_rules, &topics);
        let request_for_worker = request.clone();
        let topics_for_worker = topics.clone();

        let mut events = tokio::task::spawn_blocking(move || {
            run_trace_scan(&profile, &request_for_worker, &topics_for_worker)
        })
        .await
        .map_err(|error| {
            AppError::Internal(format!("failed to join trace query task: {error}"))
        })??;

        events.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));

        Ok(TraceQueryResultDto {
            query_summary: TraceQuerySummaryDto {
                key_type: request.key_type.clone(),
                key_value: request.key_value.clone(),
                scanned_topics: topics,
                matched_count: events.len(),
                result_mode: request
                    .result_mode
                    .unwrap_or_else(|| "timeline".to_string()),
            },
            timeline: events.clone(),
            events,
            confidence_notes: Some(confidence_notes),
        })
    }
}

fn validate_trace_request(request: &RunTraceQueryRequest) -> AppResult<()> {
    if request.cluster_profile_id.trim().is_empty() {
        return Err(AppError::Validation(
            "cluster profile id is required".to_string(),
        ));
    }
    if request.key_type.trim().is_empty() {
        return Err(AppError::Validation("key type is required".to_string()));
    }
    if request.key_value.trim().is_empty() {
        return Err(AppError::Validation("key value is required".to_string()));
    }
    if request.time_range.start.trim().is_empty() || request.time_range.end.trim().is_empty() {
        return Err(AppError::Validation(
            "bounded trace requires both start and end time".to_string(),
        ));
    }
    if let Some(result_mode) = request.result_mode.as_deref() {
        if result_mode != "timeline" && result_mode != "table" {
            return Err(AppError::Validation(
                "resultMode must be timeline or table".to_string(),
            ));
        }
    }
    Ok(())
}

fn resolve_trace_topics(
    request: &RunTraceQueryRequest,
    enabled_rules: &[CorrelationRuleDto],
) -> AppResult<Vec<String>> {
    let explicit_topics = request
        .topic_scope
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|topic| topic.trim().to_string())
        .filter(|topic| !topic.is_empty())
        .collect::<Vec<_>>();

    let matching_rules = enabled_rules
        .iter()
        .filter(|rule| rule_matches_key_type(rule, &request.key_type))
        .collect::<Vec<_>>();

    let derived_topics = matching_rules
        .iter()
        .flat_map(|rule| extract_topics_from_scope(&rule.scope_json))
        .collect::<Vec<_>>();

    let mut topics = if explicit_topics.is_empty() {
        derived_topics
    } else {
        explicit_topics
    };
    topics.sort();
    topics.dedup();

    if topics.is_empty() {
        return Err(AppError::Validation(
            "trace query requires at least one topic in scope or an enabled correlation rule with topic scope".to_string(),
        ));
    }

    if topics.len() > MAX_TRACE_TOPICS {
        return Err(AppError::Validation(format!(
            "trace query may scan at most {MAX_TRACE_TOPICS} topics"
        )));
    }

    Ok(topics)
}

fn build_confidence_notes(
    request: &RunTraceQueryRequest,
    rules: &[CorrelationRuleDto],
    topics: &[String],
) -> Vec<String> {
    let mut notes = vec![format!(
        "当前追踪仅在显式时间范围内扫描 {} 个主题。",
        topics.len()
    )];

    let matching_rules = rules
        .iter()
        .filter(|rule| rule_matches_key_type(rule, &request.key_type))
        .count();
    if request.key_type.starts_with("header:") {
        if matching_rules == 0 {
            notes.push(
                "当前 Header 键没有匹配的已启用关联规则；只会使用显式主题范围执行有界追踪，不会自动补齐跨主题链路。"
                    .to_string(),
            );
        } else {
            notes.push(format!(
                "Header 追踪基于 {} 条已启用规则做范围限定。",
                matching_rules
            ));
        }
    } else {
        notes.push("当前为基础 trace-by-key 能力，尚未包含图谱推断和跨主题因果分析。".to_string());
    }

    notes
}

fn rule_matches_key_type(rule: &CorrelationRuleDto, key_type: &str) -> bool {
    if key_type == "message-key" {
        return rule.match_strategy == "key-match" || rule.match_strategy == "ordered-multi-topic";
    }

    let Some(header_key) = key_type.strip_prefix("header:") else {
        return false;
    };

    if rule.match_strategy != "header-match" {
        return false;
    }

    serde_json::from_str::<Value>(&rule.rule_json)
        .ok()
        .and_then(|value| {
            value
                .get("matchKey")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .map(|match_key| match_key == header_key)
        .unwrap_or(false)
}

fn extract_topics_from_scope(scope_json: &str) -> Vec<String> {
    serde_json::from_str::<Value>(scope_json)
        .ok()
        .and_then(|value| value.get("topics").and_then(Value::as_array).cloned())
        .map(|items| {
            items
                .into_iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn run_trace_scan(
    profile: &ClusterProfileDto,
    request: &RunTraceQueryRequest,
    topics: &[String],
) -> AppResult<Vec<TraceEventDto>> {
    let consumer = build_query_consumer(profile)?;
    let mut bounds_map = HashMap::new();
    let mut assignment = TopicPartitionList::new();

    for topic_name in topics {
        let metadata = consumer
            .fetch_metadata(Some(topic_name), Duration::from_secs(5))
            .map_err(|error| {
                AppError::Network(format!(
                    "failed to load topic metadata for '{}': {error}",
                    topic_name
                ))
            })?;

        let topic = metadata
            .topics()
            .iter()
            .find(|topic| topic.name() == topic_name)
            .ok_or_else(|| AppError::NotFound(format!("topic '{}' was not found", topic_name)))?;

        for partition in topic.partitions().iter().map(|partition| partition.id()) {
            let bounds =
                resolve_partition_bounds(&consumer, topic_name, partition, &request.time_range)?;
            if bounds.start >= bounds.end_exclusive {
                continue;
            }

            assignment
                .add_partition_offset(topic_name, partition, Offset::Offset(bounds.start))
                .map_err(|error| {
                    AppError::Internal(format!(
                        "failed to assign partition '{}': {error}",
                        partition
                    ))
                })?;
            bounds_map.insert((topic_name.clone(), partition), bounds);
        }
    }

    if bounds_map.is_empty() {
        return Ok(Vec::new());
    }

    consumer.assign(&assignment).map_err(|error| {
        AppError::Network(format!("failed to assign trace partitions: {error}"))
    })?;

    let mut finished_partitions = HashSet::new();
    let mut results = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut idle_polls = 0;

    while Instant::now() < deadline
        && results.len() < MAX_TRACE_RESULTS
        && finished_partitions.len() < bounds_map.len()
    {
        match consumer.poll(Duration::from_millis(200)) {
            Some(Ok(message)) => {
                idle_polls = 0;
                let key = (message.topic().to_string(), message.partition());
                let Some(bounds) = bounds_map.get(&key) else {
                    continue;
                };

                if message.offset() >= bounds.end_exclusive {
                    finished_partitions.insert(key);
                    continue;
                }

                if message.offset() < bounds.start {
                    continue;
                }

                let Some(matched_by) =
                    match_trace_key(&message, &request.key_type, &request.key_value)
                else {
                    continue;
                };

                results.push(map_trace_event(
                    &request.cluster_profile_id,
                    &message,
                    matched_by,
                ));
            }
            Some(Err(_)) => idle_polls += 1,
            None => idle_polls += 1,
        }

        if idle_polls >= 10 {
            break;
        }
    }

    Ok(results)
}

fn build_query_consumer(profile: &ClusterProfileDto) -> AppResult<BaseConsumer> {
    let mut config = ClientConfig::new();
    apply_kafka_read_consumer_config(&mut config, profile)?;

    config.create().map_err(|error| {
        AppError::Network(format!("failed to create trace query consumer: {error}"))
    })
}

fn resolve_partition_bounds(
    consumer: &BaseConsumer,
    topic: &str,
    partition: i32,
    time_range: &TimeRangeDto,
) -> AppResult<PartitionBounds> {
    let (low, high) = consumer
        .fetch_watermarks(topic, partition, Duration::from_secs(1))
        .map_err(|error| {
            AppError::Network(format!(
                "failed to load watermarks for topic '{}' partition {}: {error}",
                topic, partition
            ))
        })?;

    let start_ms = parse_local_datetime_millis(&time_range.start)?;
    let end_ms = parse_local_datetime_millis(&time_range.end)?;
    if start_ms > end_ms {
        return Err(AppError::Validation(
            "trace start time must be earlier than end time".to_string(),
        ));
    }

    let start = resolve_offset_for_time(consumer, topic, partition, start_ms)?
        .unwrap_or(high)
        .clamp(low, high);
    let end_exclusive = resolve_offset_for_time(consumer, topic, partition, end_ms)?
        .unwrap_or(high)
        .clamp(low, high);

    Ok(PartitionBounds {
        start,
        end_exclusive,
    })
}

fn resolve_offset_for_time(
    consumer: &BaseConsumer,
    topic: &str,
    partition: i32,
    timestamp_ms: i64,
) -> AppResult<Option<i64>> {
    let mut timestamps = TopicPartitionList::new();
    timestamps
        .add_partition_offset(topic, partition, Offset::Offset(timestamp_ms))
        .map_err(|error| {
            AppError::Internal(format!(
                "failed to build trace timestamp lookup partition list: {error}"
            ))
        })?;

    let offsets = consumer
        .offsets_for_times(timestamps, Duration::from_secs(2))
        .map_err(|error| {
            AppError::Network(format!("failed to resolve trace timestamp bounds: {error}"))
        })?;

    let Some(element) = offsets.find_partition(topic, partition) else {
        return Ok(None);
    };

    Ok(match element.offset() {
        Offset::Offset(value) => Some(value),
        _ => None,
    })
}

fn parse_local_datetime_millis(value: &str) -> AppResult<i64> {
    let naive = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M").map_err(|_| {
        AppError::Validation(format!(
            "invalid datetime-local value '{}': expected YYYY-MM-DDTHH:MM",
            value
        ))
    })?;

    Local
        .from_local_datetime(&naive)
        .single()
        .map(|datetime| datetime.timestamp_millis())
        .ok_or_else(|| {
            AppError::Validation(format!(
                "invalid local datetime '{}': ambiguous or nonexistent",
                value
            ))
        })
}

fn match_trace_key<M: Message>(message: &M, key_type: &str, key_value: &str) -> Option<String> {
    if key_type == "message-key" {
        let key = message
            .key()
            .map(|value| String::from_utf8_lossy(value).to_string())?;
        return if key == key_value {
            Some("message-key".to_string())
        } else {
            None
        };
    }

    let header_key = key_type.strip_prefix("header:")?;
    let headers = message.headers()?;
    headers.iter().find_map(|header| {
        if header.key != header_key {
            return None;
        }

        let value = header
            .value
            .map(|bytes| String::from_utf8_lossy(bytes).to_string())
            .unwrap_or_default();
        if value == key_value {
            Some(format!("header:{header_key}"))
        } else {
            None
        }
    })
}

fn map_trace_event(
    cluster_profile_id: &str,
    message: &impl Message,
    matched_by: String,
) -> TraceEventDto {
    let payload_preview = message
        .payload()
        .map(|payload| truncate_preview(&String::from_utf8_lossy(payload), 160));
    let key_preview = message
        .key()
        .map(|key| truncate_preview(&String::from_utf8_lossy(key), 80));

    TraceEventDto {
        message_ref: MessageRefDto {
            cluster_profile_id: cluster_profile_id.to_string(),
            topic: message.topic().to_string(),
            partition: message.partition(),
            offset: message.offset().to_string(),
        },
        timestamp: message
            .timestamp()
            .to_millis()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "0".to_string()),
        topic: message.topic().to_string(),
        partition: message.partition(),
        offset: message.offset().to_string(),
        key_preview,
        payload_preview,
        matched_by,
    }
}

fn truncate_preview(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let preview = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{preview}…")
    } else {
        preview
    }
}

#[cfg(test)]
mod tests {
    use super::build_query_consumer;
    use crate::models::cluster::ClusterProfileDto;
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    const TEST_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----\nMIICsDCCAZgCCQD4vKEpwC7YOjANBgkqhkiG9w0BAQsFADAaMRgwFgYDVQQDDA90\ncmFjZWZvcmdlLXRlc3QwHhcNMjYwNDE4MDczMTU0WhcNMjcwNDE4MDczMTU0WjAa\nMRgwFgYDVQQDDA90cmFjZWZvcmdlLXRlc3QwggEiMA0GCSqGSIb3DQEBAQUAA4IB\nDwAwggEKAoIBAQDB3uz/2t6m3/qJaENMNk1KW6L0nE6Sr6a16C9alMoNk53q8Glx\nQA7shqJLUj8AHpbMMMddlc/RXz4cYXKLMq5XH2YVJ+QKxjcXR0Q6Swu3cqoMO7n8\nfiOiA9biPR3/78KSEPd9zdqOs3DwMHr65v3oEI9tc1rWvam9Xz+4MrkBFbmF4wZR\nYZ4ac+lgB9ie1FL4S4cRHYqYzvumNdMDtFv8hDCmyPdKUlfEowxtueuQ5WGB82yP\nGd0LlYOvxt79MiR0sIQsxVnSY5F0nLcNW9Z+bCuGgEWchakiUb9Nj2BcJ88IAGxs\npitqC3v+noeQA4SWkc+7Byd4dZS4rLGYv8FhAgMBAAEwDQYJKoZIhvcNAQELBQAD\nggEBACk9WDt7D7thZkoT8VJkyukWx4uPGXczOfp0+hu2eP1TODurSQziwVj3xF3O\noSjN8HrWg3U0vGqZGgqIPxPknbmwk5fjVorwWelRlX2X7DMElsFeRMZSY9leLC10\ntqdEu8mIJsGzR/Aua56fo3dywhIglYG/8O0tcZYjdp6YczXWW64lPz2vVv+9ZVVj\nnVrKYbU118mkVhd7jmV9QR5KdBY1th6qVEzI340S7CQ2PdweT0kemFwBTCp5gvJ5\na3Xi8pKrQKJk/L2O6oxhXOCCGvWhdEvZ8mel2Qp/whg6MupIciDKozdf68yECrUW\nEhr3a4kltLXboZZ+DJx+KZCTRv0=\n-----END CERTIFICATE-----\n";
    const TEST_PRIVATE_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQDB3uz/2t6m3/qJ\naENMNk1KW6L0nE6Sr6a16C9alMoNk53q8GlxQA7shqJLUj8AHpbMMMddlc/RXz4c\nYXKLMq5XH2YVJ+QKxjcXR0Q6Swu3cqoMO7n8fiOiA9biPR3/78KSEPd9zdqOs3Dw\nMHr65v3oEI9tc1rWvam9Xz+4MrkBFbmF4wZRYZ4ac+lgB9ie1FL4S4cRHYqYzvum\nNdMDtFv8hDCmyPdKUlfEowxtueuQ5WGB82yPGd0LlYOvxt79MiR0sIQsxVnSY5F0\nnLcNW9Z+bCuGgEWchakiUb9Nj2BcJ88IAGxspitqC3v+noeQA4SWkc+7Byd4dZS4\nrLGYv8FhAgMBAAECggEAU/ogZuODtn0mpQaIwCZ1bFQtTg+26Us0x27/tBjnPOJI\ncVAaHHhG/qWC/2Vs7LxTTbeDZEJUdrjuypRbbXhSlGKBcYCKAzDBBFBWeXmwZZJB\nnnLwqTJqdO980RrwN8C/Y03+JnxYw59uuFwHqU8NhMxHlH13R7V4JRNxInS2NoVV\nXVfgcTjax8pdbdzKKwIn3AUk27SwSJwBlYuMKgDq741/L8PvyjOmolvzwM+aF2FO\n1gqb3xZKM1867psYo4Z09qdc8GyG+joPEbJW9rQW2nORUy1mqApXmO8qprt+K3yY\nhWQUjYFpngx7OKOv0RhRSwzm9swK03QbLKK5i5/IwQKBgQDokMmZxVKbnpTlI8Fl\n6H2pQAIPi1HPdTTMapxBlP5CsLgtkBiYu60LYevmAcSdkzbpVr7uqWxy7+Z0upao\n7kheHaqovcO8xm1n1BDOnEgnmFJ9wLFDi8qG9EQd4dusvJWb6u3dvvWVWQFh4Pz4\nZPKxGbfa7VHeFSk9wizXuCGQpQKBgQDVZ/nhoCOIzwF9bxR9Gko6fdeLf6ZTK0ht\nMJZ8kbDlYoSjRWjX6zXdoLL7mQ2y7avQ8mEYVyOcBCb5xGa43cF6eKy6W59lXQqI\nB1Z64gaKAsFgqbhpJRo4sfEsft9pip/oI49Y9B6vP9do9UbfBg4ZZm5fZdS5+MnY\nWE70VTB1DQKBgQCzQh7SkuD4qIRWFnhUp55sXbT47Ecz5EC9K5OjjUdqejKMlBwR\nZd+c/W5KDKTTXIyf0Mg8x4SbF0UIRmYocfp/6NgJVrPQBxZ/SFtoFdgcBPHYkjVQ\nPijuWstCSTv86iNbWfrcx/sdkcxZ+ISkpZLXZV5sti47Qw5V1xyfbgMZLQKBgQCq\nI92LTvtFpZSQhrEVFJK9k3r3kuvuPwHdW/F+m0EngKYy7bGrA7HMYsSP5vSPBQII\n8lUK7N5NEtpoI3eqR9JrbC553XZ1f/pXfVIrYmzIN24pPObznUsMjIG1cel44baf\ng0pUJz0Xh5Sb74FzagZvpcS1diBlrL5wJ+e60PhzOQKBgQCy9K28nvKeC+JO40ST\ngp5YqlQnRNaWfXHTwbeeYygYfgiY+lw7hmtInnLcu7s3uVepoWpJJPGeVG7MiW+1\nfN3BpyGmBc+fA8UbUl2RHZKvKaLTWGwujYqwbfSOtI56FpNBHfjy4HyAvzAHwB7a\nlHc4mf+dH3zMIvjxLn2jsGjjFw==\n-----END PRIVATE KEY-----\n";

    fn create_temp_file(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("traceforge-{name}-{}.pem", Uuid::new_v4()));
        fs::write(&path, contents).expect("temp cert material should write");
        path
    }

    fn sample_mtls_profile(cert_path: &PathBuf, key_path: &PathBuf, ca_path: &PathBuf) -> ClusterProfileDto {
        ClusterProfileDto {
            id: "cluster-1".to_string(),
            name: "Cluster One".to_string(),
            environment: "dev".to_string(),
            bootstrap_servers: "localhost:9092".to_string(),
            auth_mode: "mtls".to_string(),
            auth_credential_ref: None,
            tls_mode: "tls-required".to_string(),
            tls_ca_cert_path: Some(ca_path.to_string_lossy().into_owned()),
            tls_client_cert_path: Some(cert_path.to_string_lossy().into_owned()),
            tls_client_key_path: Some(key_path.to_string_lossy().into_owned()),
            schema_registry_profile_id: None,
            notes: None,
            tags: vec![],
            is_favorite: false,
            created_at: "2026-04-18T00:00:00Z".to_string(),
            updated_at: "2026-04-18T00:00:00Z".to_string(),
            last_connected_at: None,
            is_archived: false,
        }
    }

    #[test]
    fn build_query_consumer_supports_mtls_cluster_profile() {
        let ca_path = create_temp_file("trace-ca", TEST_CERT_PEM);
        let cert_path = create_temp_file("trace-cert", TEST_CERT_PEM);
        let key_path = create_temp_file("trace-key", TEST_PRIVATE_KEY_PEM);
        let profile = sample_mtls_profile(&cert_path, &key_path, &ca_path);

        let consumer = build_query_consumer(&profile)
            .expect("trace query consumer should build for mTLS profile");
        drop(consumer);

        let _ = fs::remove_file(ca_path);
        let _ = fs::remove_file(cert_path);
        let _ = fs::remove_file(key_path);
    }
}
