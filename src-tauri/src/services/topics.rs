use crate::models::cluster::ClusterProfileDto;
use crate::models::error::{AppError, AppResult};
use crate::models::topic::{
    GetTopicDetailRequest, ListTopicsRequest, TopicConfigEntryDto, TopicDetailResponseDto,
    TopicPartitionDto, TopicRelatedGroupDto, TopicSummaryDto,
};
use crate::repositories::sqlite;
use crate::services::kafka_config::apply_kafka_read_consumer_config;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::metadata::MetadataTopic;
use rdkafka::topic_partition_list::{Offset, TopicPartitionList};
use rdkafka::ClientConfig;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::time::Duration;

pub struct TopicService<'a> {
    pool: &'a SqlitePool,
}

struct TopicGroupSnapshot {
    name: String,
    state: String,
    total_lag: i64,
    partitions_impacted: usize,
    partition_lags: HashMap<i32, i64>,
}

impl<'a> TopicService<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_topics(&self, request: ListTopicsRequest) -> AppResult<Vec<TopicSummaryDto>> {
        validate_list_topics_request(&request)?;

        if request.favorites_only.unwrap_or(false) {
            return Ok(Vec::new());
        }

        let profile = sqlite::get_cluster_profile(self.pool, &request.cluster_profile_id).await?;
        let query = request
            .query
            .clone()
            .map(|value| value.trim().to_lowercase());
        let include_internal = request.include_internal.unwrap_or(false);
        let limit = request.limit.unwrap_or(500).min(1000);

        let topics = tokio::task::spawn_blocking(move || {
            let consumer = build_metadata_consumer(&profile)?;

            let metadata = consumer
                .fetch_metadata(None, Duration::from_secs(5))
                .map_err(|error| {
                    AppError::Network(format!("failed to load Kafka metadata: {error}"))
                })?;

            let mut topics = metadata
                .topics()
                .iter()
                .filter(|topic| include_internal || !topic.name().starts_with("__"))
                .filter(|topic| {
                    query
                        .as_ref()
                        .map(|needle| topic.name().to_lowercase().contains(needle))
                        .unwrap_or(true)
                })
                .map(map_topic_summary)
                .collect::<Vec<_>>();

            topics.sort_by(|left, right| left.name.cmp(&right.name));
            topics.truncate(limit);

            Ok::<Vec<TopicSummaryDto>, AppError>(topics)
        })
        .await
        .map_err(|error| {
            AppError::Internal(format!("failed to join Kafka metadata task: {error}"))
        })??;

        Ok(topics)
    }

    pub async fn get_topic_detail(
        &self,
        request: GetTopicDetailRequest,
    ) -> AppResult<TopicDetailResponseDto> {
        validate_get_topic_detail_request(&request)?;

        let profile = sqlite::get_cluster_profile(self.pool, &request.cluster_profile_id).await?;
        let topic_name = request.topic_name.clone();

        tokio::task::spawn_blocking(move || {
            let consumer = build_metadata_consumer(&profile)?;
            let metadata = consumer
                .fetch_metadata(Some(&topic_name), Duration::from_secs(5))
                .map_err(|error| {
                    AppError::Network(format!("failed to load topic metadata: {error}"))
                })?;

            let topic = metadata
                .topics()
                .iter()
                .find(|topic| topic.name() == topic_name)
                .ok_or_else(|| {
                    AppError::NotFound(format!("topic '{}' was not found", topic_name))
                })?;

            let partition_ids = topic
                .partitions()
                .iter()
                .map(|partition| partition.id())
                .collect::<Vec<_>>();
            let group_snapshots = fetch_topic_group_snapshots(&profile, topic.name(), &partition_ids)?;
            let partition_group_summary = build_partition_group_summary(&group_snapshots);

            let partitions = topic
                .partitions()
                .iter()
                .map(|partition| {
                    let (earliest, high) = consumer
                        .fetch_watermarks(topic.name(), partition.id(), Duration::from_secs(1))
                        .map_err(|error| {
                            AppError::Network(format!(
                                "failed to load watermarks for topic '{}' partition {}: {error}",
                                topic.name(),
                                partition.id()
                            ))
                        })?;

                    let latest_existing = if high > earliest { high - 1 } else { high };

                    Ok::<TopicPartitionDto, AppError>(TopicPartitionDto {
                        partition_id: partition.id(),
                        earliest_offset: Some(earliest.to_string()),
                        latest_offset: Some(latest_existing.to_string()),
                        leader: Some(partition.leader().to_string()),
                        replica_status: Some(format!(
                            "副本 {} / ISR {}",
                            partition.replicas().len(),
                            partition.isr().len()
                        )),
                        consumer_group_summary: partition_group_summary
                            .get(&partition.id())
                            .cloned(),
                    })
                })
                .collect::<AppResult<Vec<_>>>()?;

            let advanced_config = Some(vec![TopicConfigEntryDto {
                key: "brokerBootstrap".to_string(),
                value: profile.bootstrap_servers.clone(),
            }]);

            Ok::<TopicDetailResponseDto, AppError>(TopicDetailResponseDto {
                topic: map_topic_summary(topic),
                partitions,
                related_groups: map_related_groups(&group_snapshots),
                advanced_config,
            })
        })
        .await
        .map_err(|error| AppError::Internal(format!("failed to join topic detail task: {error}")))?
    }
}

fn build_metadata_consumer(profile: &ClusterProfileDto) -> AppResult<BaseConsumer> {
    let mut config = ClientConfig::new();
    apply_kafka_read_consumer_config(&mut config, profile)?;

    config.create().map_err(|error| {
        AppError::Network(format!("failed to create Kafka metadata client: {error}"))
    })
}

fn build_group_consumer(profile: &ClusterProfileDto, group_id: Option<&str>) -> AppResult<BaseConsumer> {
    let mut config = ClientConfig::new();
    apply_kafka_read_consumer_config(&mut config, profile)?;

    if let Some(group_id) = group_id {
        config.set("group.id", group_id);
    }

    config.create().map_err(|error| {
        AppError::Network(format!("failed to create topic-group inspection client: {error}"))
    })
}

fn fetch_topic_group_snapshots(
    profile: &ClusterProfileDto,
    topic_name: &str,
    partition_ids: &[i32],
) -> AppResult<Vec<TopicGroupSnapshot>> {
    let inspector = build_group_consumer(profile, None)?;
    let groups = inspector.fetch_group_list(None, Duration::from_secs(5)).map_err(|error| {
        AppError::Network(format!(
            "failed to load consumer groups for topic '{}': {error}",
            topic_name
        ))
    })?;

    let mut snapshots = Vec::new();
    for group in groups.groups() {
        if let Some(snapshot) = fetch_topic_group_snapshot(profile, topic_name, partition_ids, group.name(), group.state())? {
            snapshots.push(snapshot);
        }
    }

    snapshots.sort_by(|left, right| {
        right
            .total_lag
            .cmp(&left.total_lag)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(snapshots)
}

fn fetch_topic_group_snapshot(
    profile: &ClusterProfileDto,
    topic_name: &str,
    partition_ids: &[i32],
    group_name: &str,
    group_state: &str,
) -> AppResult<Option<TopicGroupSnapshot>> {
    let consumer = build_group_consumer(profile, Some(group_name))?;
    let mut tpl = TopicPartitionList::new();
    for partition_id in partition_ids {
        tpl.add_partition(topic_name, *partition_id);
    }

    let committed = consumer
        .committed_offsets(tpl, Duration::from_secs(5))
        .map_err(|error| {
            AppError::Network(format!(
                "failed to load committed offsets for group '{}' on topic '{}': {error}",
                group_name, topic_name
            ))
        })?;

    let mut total_lag = 0;
    let mut partition_lags = HashMap::new();
    for element in committed.elements() {
        let committed_offset = match element.offset() {
            Offset::Offset(value) => value,
            _ => continue,
        };

        let (_, high) = consumer
            .fetch_watermarks(topic_name, element.partition(), Duration::from_secs(1))
            .map_err(|error| {
                AppError::Network(format!(
                    "failed to load watermarks for group '{}' topic '{}' partition {}: {error}",
                    group_name,
                    topic_name,
                    element.partition()
                ))
            })?;
        let lag = (high - committed_offset).max(0);
        total_lag += lag;
        partition_lags.insert(element.partition(), lag);
    }

    if partition_lags.is_empty() {
        return Ok(None);
    }

    Ok(Some(TopicGroupSnapshot {
        name: group_name.to_string(),
        state: group_state.to_string(),
        total_lag,
        partitions_impacted: partition_lags.len(),
        partition_lags,
    }))
}

fn build_partition_group_summary(group_snapshots: &[TopicGroupSnapshot]) -> HashMap<i32, String> {
    let mut grouped: HashMap<i32, Vec<(String, i64)>> = HashMap::new();
    for snapshot in group_snapshots {
        for (partition_id, lag) in &snapshot.partition_lags {
            grouped
                .entry(*partition_id)
                .or_default()
                .push((snapshot.name.clone(), *lag));
        }
    }

    grouped
        .into_iter()
        .map(|(partition_id, mut groups)| {
            groups.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
            let summary = groups
                .iter()
                .take(2)
                .map(|(name, lag)| format!("{} (lag {})", name, lag))
                .collect::<Vec<_>>()
                .join(" · ");
            let summary = if groups.len() > 2 {
                format!("{} · +{} 组", summary, groups.len() - 2)
            } else {
                summary
            };
            (partition_id, summary)
        })
        .collect()
}

fn map_related_groups(group_snapshots: &[TopicGroupSnapshot]) -> Vec<TopicRelatedGroupDto> {
    if group_snapshots.is_empty() {
        return Vec::new();
    }

    group_snapshots
        .iter()
        .map(|snapshot| TopicRelatedGroupDto {
            name: snapshot.name.clone(),
            total_lag: snapshot.total_lag,
            state: format!("{} · 影响 {} 个分区", snapshot.state, snapshot.partitions_impacted),
        })
        .collect()
}

fn map_topic_summary(topic: &MetadataTopic) -> TopicSummaryDto {
    let partition_count = topic.partitions().len();
    let replication_factor = topic
        .partitions()
        .first()
        .map(|partition| partition.replicas().len() as i32);

    TopicSummaryDto {
        name: topic.name().to_string(),
        partition_count,
        replication_factor,
        schema_type: None,
        retention_summary: None,
        activity_hint: Some(if partition_count > 32 {
            "高分区".to_string()
        } else if partition_count > 8 {
            "中等分区".to_string()
        } else {
            "常规".to_string()
        }),
        is_favorite: false,
    }
}

fn validate_list_topics_request(request: &ListTopicsRequest) -> AppResult<()> {
    if request.cluster_profile_id.trim().is_empty() {
        return Err(AppError::Validation(
            "cluster profile id is required".to_string(),
        ));
    }

    Ok(())
}

fn validate_get_topic_detail_request(request: &GetTopicDetailRequest) -> AppResult<()> {
    if request.cluster_profile_id.trim().is_empty() {
        return Err(AppError::Validation(
            "cluster profile id is required".to_string(),
        ));
    }

    if request.topic_name.trim().is_empty() {
        return Err(AppError::Validation("topic name is required".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::build_metadata_consumer;
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
    fn build_metadata_consumer_supports_mtls_cluster_profile() {
        let ca_path = create_temp_file("topics-ca", TEST_CERT_PEM);
        let cert_path = create_temp_file("topics-cert", TEST_CERT_PEM);
        let key_path = create_temp_file("topics-key", TEST_PRIVATE_KEY_PEM);
        let profile = sample_mtls_profile(&cert_path, &key_path, &ca_path);

        let consumer = build_metadata_consumer(&profile)
            .expect("topic metadata consumer should build for mTLS profile");
        drop(consumer);

        let _ = fs::remove_file(ca_path);
        let _ = fs::remove_file(cert_path);
        let _ = fs::remove_file(key_path);
    }
}
