use crate::models::cluster::ClusterProfileDto;
use crate::models::error::{AppError, AppResult};
use crate::models::group::{
    GetGroupDetailRequest, GroupCoordinatorInfoDto, GroupDetailResponseDto, GroupPartitionLagDto,
    GroupSummaryDto, GroupTopicLagDto, ListGroupsRequest,
};
use crate::repositories::sqlite;
use crate::services::kafka_config::apply_kafka_read_consumer_config;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::topic_partition_list::{Offset, TopicPartitionList};
use rdkafka::ClientConfig;
use sqlx::SqlitePool;
use std::collections::BTreeMap;
use std::time::Duration;

pub struct GroupService<'a> {
    pool: &'a SqlitePool,
}

struct GroupLagSnapshot {
    total_lag: i64,
    topic_count: usize,
    partition_count: usize,
    topic_breakdown: Vec<GroupTopicLagDto>,
    partition_breakdown: Vec<GroupPartitionLagDto>,
}

impl<'a> GroupService<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_groups(&self, request: ListGroupsRequest) -> AppResult<Vec<GroupSummaryDto>> {
        validate_list_groups_request(&request)?;

        let profile = sqlite::get_cluster_profile(self.pool, &request.cluster_profile_id).await?;
        let query = request
            .query
            .clone()
            .map(|value| value.trim().to_lowercase());
        let topic_filter = request
            .topic_filter
            .clone()
            .map(|value| value.trim().to_lowercase());
        let lagging_only = request.lagging_only.unwrap_or(false);
        let limit = request.limit.unwrap_or(200).min(500);

        tokio::task::spawn_blocking(move || {
            let inspector = build_group_consumer(&profile, None)?;
            let group_list = inspector
                .fetch_group_list(None, Duration::from_secs(5))
                .map_err(|error| {
                    AppError::Network(format!("failed to load consumer groups: {error}"))
                })?;

            let mut rows = Vec::new();

            for group in group_list.groups() {
                let snapshot = build_group_lag_snapshot(&profile, group.name())?;

                if lagging_only && snapshot.total_lag <= 0 {
                    continue;
                }

                if let Some(query) = query.as_ref() {
                    if !group.name().to_lowercase().contains(query) {
                        continue;
                    }
                }

                if let Some(topic_filter) = topic_filter.as_ref() {
                    if !snapshot
                        .topic_breakdown
                        .iter()
                        .any(|item| item.topic.to_lowercase().contains(topic_filter))
                    {
                        continue;
                    }
                }

                rows.push(GroupSummaryDto {
                    name: group.name().to_string(),
                    state: group.state().to_string(),
                    total_lag: snapshot.total_lag,
                    topic_count: snapshot.topic_count,
                    partition_count: snapshot.partition_count,
                    last_seen_at: None,
                });
            }

            rows.sort_by(|left, right| {
                right
                    .total_lag
                    .cmp(&left.total_lag)
                    .then_with(|| left.name.cmp(&right.name))
            });
            rows.truncate(limit);

            Ok::<Vec<GroupSummaryDto>, AppError>(rows)
        })
        .await
        .map_err(|error| AppError::Internal(format!("failed to join groups task: {error}")))?
    }

    pub async fn get_group_detail(
        &self,
        request: GetGroupDetailRequest,
    ) -> AppResult<GroupDetailResponseDto> {
        validate_group_detail_request(&request)?;

        let profile = sqlite::get_cluster_profile(self.pool, &request.cluster_profile_id).await?;
        let group_name = request.group_name.clone();

        tokio::task::spawn_blocking(move || {
            let inspector = build_group_consumer(&profile, None)?;
            let group_list = inspector
                .fetch_group_list(Some(&group_name), Duration::from_secs(5))
                .map_err(|error| {
                    AppError::Network(format!("failed to load consumer group detail: {error}"))
                })?;

            let group = group_list
                .groups()
                .iter()
                .find(|item| item.name() == group_name)
                .ok_or_else(|| {
                    AppError::NotFound(format!("group '{}' was not found", group_name))
                })?;

            let snapshot = build_group_lag_snapshot(&profile, group.name())?;

            Ok::<GroupDetailResponseDto, AppError>(GroupDetailResponseDto {
                group: GroupSummaryDto {
                    name: group.name().to_string(),
                    state: group.state().to_string(),
                    total_lag: snapshot.total_lag,
                    topic_count: snapshot.topic_count,
                    partition_count: snapshot.partition_count,
                    last_seen_at: None,
                },
                topic_lag_breakdown: snapshot.topic_breakdown,
                partition_lag_breakdown: snapshot.partition_breakdown,
                coordinator_info: Some(GroupCoordinatorInfoDto {
                    broker_id: group.members().first().map(|member| member.client_id().to_string()),
                    host: group.members().first().map(|member| member.client_host().to_string()),
                }),
            })
        })
        .await
        .map_err(|error| AppError::Internal(format!("failed to join group detail task: {error}")))?
    }
}

fn build_group_consumer(
    profile: &ClusterProfileDto,
    group_id: Option<&str>,
) -> AppResult<BaseConsumer> {
    let mut config = ClientConfig::new();
    apply_kafka_read_consumer_config(&mut config, profile)?;

    if let Some(group_id) = group_id {
        config.set("group.id", group_id);
    }

    config.create().map_err(|error| {
        AppError::Network(format!("failed to create group inspector client: {error}"))
    })
}

fn build_group_lag_snapshot(
    profile: &ClusterProfileDto,
    group_name: &str,
) -> AppResult<GroupLagSnapshot> {
    let consumer = build_group_consumer(profile, Some(group_name))?;
    let metadata = consumer
        .fetch_metadata(None, Duration::from_secs(5))
        .map_err(|error| {
            AppError::Network(format!(
                "failed to load metadata for group '{}': {error}",
                group_name
            ))
        })?;

    let mut tpl = TopicPartitionList::new();
    for topic in metadata.topics() {
        if topic.name().starts_with("__") {
            continue;
        }

        for partition in topic.partitions() {
            tpl.add_partition(topic.name(), partition.id());
        }
    }

    let committed = consumer
        .committed_offsets(tpl, Duration::from_secs(5))
        .map_err(|error| {
            AppError::Network(format!(
                "failed to load committed offsets for group '{}': {error}",
                group_name
            ))
        })?;

    let mut partitions = Vec::new();
    let mut topic_totals: BTreeMap<String, (i64, usize)> = BTreeMap::new();

    for element in committed.elements() {
        let committed_offset = match element.offset() {
            Offset::Offset(value) => value,
            _ => continue,
        };

        let (low, high) = consumer
            .fetch_watermarks(element.topic(), element.partition(), Duration::from_secs(1))
            .map_err(|error| {
                AppError::Network(format!(
                    "failed to load watermarks for group '{}' topic '{}' partition {}: {error}",
                    group_name,
                    element.topic(),
                    element.partition()
                ))
            })?;

        let effective_committed = committed_offset.max(low).min(high);
        let lag = (high - effective_committed).max(0);

        partitions.push(GroupPartitionLagDto {
            topic: element.topic().to_string(),
            partition: element.partition(),
            committed_offset: Some(committed_offset.to_string()),
            log_end_offset: Some(high.to_string()),
            lag,
        });

        let entry = topic_totals
            .entry(element.topic().to_string())
            .or_insert((0, 0));
        entry.0 += lag;
        entry.1 += 1;
    }

    partitions.sort_by(|left, right| {
        right
            .lag
            .cmp(&left.lag)
            .then_with(|| left.topic.cmp(&right.topic))
    });

    let topic_breakdown = topic_totals
        .into_iter()
        .map(
            |(topic, (total_lag, partitions_impacted))| GroupTopicLagDto {
                topic,
                total_lag,
                partitions_impacted,
            },
        )
        .collect::<Vec<_>>();

    let total_lag = partitions.iter().map(|item| item.lag).sum();
    let topic_count = topic_breakdown.len();
    let partition_count = partitions.len();

    Ok(GroupLagSnapshot {
        total_lag,
        topic_count,
        partition_count,
        topic_breakdown,
        partition_breakdown: partitions,
    })
}

fn validate_list_groups_request(request: &ListGroupsRequest) -> AppResult<()> {
    if request.cluster_profile_id.trim().is_empty() {
        return Err(AppError::Validation(
            "cluster profile id is required".to_string(),
        ));
    }

    Ok(())
}

fn validate_group_detail_request(request: &GetGroupDetailRequest) -> AppResult<()> {
    if request.cluster_profile_id.trim().is_empty() {
        return Err(AppError::Validation(
            "cluster profile id is required".to_string(),
        ));
    }

    if request.group_name.trim().is_empty() {
        return Err(AppError::Validation("group name is required".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::build_group_consumer;
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
    fn build_group_consumer_supports_mtls_cluster_profile() {
        let ca_path = create_temp_file("groups-ca", TEST_CERT_PEM);
        let cert_path = create_temp_file("groups-cert", TEST_CERT_PEM);
        let key_path = create_temp_file("groups-key", TEST_PRIVATE_KEY_PEM);
        let profile = sample_mtls_profile(&cert_path, &key_path, &ca_path);

        let consumer = build_group_consumer(&profile, Some("orders-consumer"))
            .expect("group consumer should build for mTLS profile");
        drop(consumer);

        let _ = fs::remove_file(ca_path);
        let _ = fs::remove_file(cert_path);
        let _ = fs::remove_file(key_path);
    }
}
