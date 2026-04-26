use crate::models::cluster::ClusterProfileDto;
use crate::models::error::{AppError, AppResult};
use crate::models::topic::{
    ExpandTopicPartitionsRequest, ExpandTopicPartitionsResponseDto, GetTopicDetailRequest,
    GetTopicOperationsOverviewRequest, ListTopicsRequest, TopicConfigEntryDto,
    TopicDetailResponseDto, TopicOperationConfigEntryDto, TopicOperationsOverviewResponseDto,
    TopicPartitionDto, TopicRelatedGroupDto, TopicSummaryDto, UpdateTopicConfigRequest,
    UpdateTopicConfigResponseDto,
};
use crate::models::replay::AuditEventRecord;
use crate::models::validation::{summarize_validation_status, ValidationStageDto};
use crate::repositories::sqlite;
use crate::services::kafka_config::{apply_kafka_read_consumer_config, build_kafka_admin_client};
use chrono::Utc;
use rdkafka::admin::{AdminOptions, ConfigEntry, ConfigResource, ConfigSource, NewPartitions, ResourceSpecifier};
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::metadata::MetadataTopic;
use rdkafka::topic_partition_list::{Offset, TopicPartitionList};
use rdkafka::ClientConfig;
use rdkafka_sys as rdsys;
use serde_json::json;
use sqlx::SqlitePool;
use std::ffi::{CStr, CString};
use std::collections::HashMap;
use std::os::raw::{c_char, c_int};
use std::time::Duration;
use uuid::Uuid;

const TOPIC_OPERATION_CONFIG_KEYS: [&str; 3] =
    ["cleanup.policy", "retention.ms", "max.message.bytes"];

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

struct TopicGroupSnapshotDiscovery {
    snapshots: Vec<TopicGroupSnapshot>,
    skipped_group_count: usize,
}

struct KafkaQueue(*mut rdsys::rd_kafka_queue_t);

impl Drop for KafkaQueue {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                rdsys::rd_kafka_queue_destroy(self.0);
            }
        }
    }
}

struct KafkaAdminOptions(*mut rdsys::rd_kafka_AdminOptions_t);

impl Drop for KafkaAdminOptions {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                rdsys::rd_kafka_AdminOptions_destroy(self.0);
            }
        }
    }
}

struct KafkaConfigResource(*mut rdsys::rd_kafka_ConfigResource_t);

impl Drop for KafkaConfigResource {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                rdsys::rd_kafka_ConfigResource_destroy(self.0);
            }
        }
    }
}

struct KafkaEvent(*mut rdsys::rd_kafka_event_t);

impl Drop for KafkaEvent {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                rdsys::rd_kafka_event_destroy(self.0);
            }
        }
    }
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
        let cluster_profile_id = request.cluster_profile_id.clone();
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

        let tag_map = sqlite::get_topic_tags_map(self.pool, &cluster_profile_id).await?;
        Ok(topics
            .into_iter()
            .map(|mut topic| {
                topic.tags = tag_map.get(&topic.name).cloned().unwrap_or_default();
                topic
            })
            .collect())
    }

    pub async fn get_topic_detail(
        &self,
        request: GetTopicDetailRequest,
    ) -> AppResult<TopicDetailResponseDto> {
        validate_get_topic_detail_request(&request)?;

        let profile = sqlite::get_cluster_profile(self.pool, &request.cluster_profile_id).await?;
        let cluster_profile_id = request.cluster_profile_id.clone();
        let topic_name = request.topic_name.clone();

        let mut response = tokio::task::spawn_blocking(move || {
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
            let group_snapshot_discovery =
                fetch_topic_group_snapshots(&profile, topic.name(), &partition_ids)?;
            let partition_group_summary =
                build_partition_group_summary(group_snapshot_discovery.snapshots.as_slice());

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
                related_groups: map_related_groups(group_snapshot_discovery.snapshots.as_slice()),
                advanced_config,
            })
        })
        .await
        .map_err(|error| AppError::Internal(format!("failed to join topic detail task: {error}")))??;

        let tag_map = sqlite::get_topic_tags_map(self.pool, &cluster_profile_id).await?;
        response.topic.tags = tag_map.get(&response.topic.name).cloned().unwrap_or_default();

        Ok(response)
    }

    pub async fn get_topic_operations_overview(
        &self,
        request: GetTopicOperationsOverviewRequest,
    ) -> AppResult<TopicOperationsOverviewResponseDto> {
        validate_get_topic_operations_overview_request(&request)?;

        let profile = sqlite::get_cluster_profile(self.pool, &request.cluster_profile_id).await?;
        let topic_name = request.topic_name.clone();
        let metadata_profile = profile.clone();
        let metadata_topic_name = topic_name.clone();

        let partition_ids = tokio::task::spawn_blocking(move || {
            let consumer = build_metadata_consumer(&metadata_profile)?;
            let metadata = consumer
                .fetch_metadata(Some(&metadata_topic_name), Duration::from_secs(5))
                .map_err(|error| {
                    AppError::Network(format!("failed to load topic operations metadata: {error}"))
                })?;

            let topic = metadata
                .topics()
                .iter()
                .find(|topic| topic.name() == metadata_topic_name)
                .ok_or_else(|| {
                    AppError::NotFound(format!("topic '{}' was not found", metadata_topic_name))
                })?;

            Ok::<Vec<i32>, AppError>(
                topic
                    .partitions()
                    .iter()
                    .map(|partition| partition.id())
                    .collect(),
            )
        })
        .await
        .map_err(|error| {
            AppError::Internal(format!(
                "failed to join topic operations overview task: {error}"
            ))
        })??;

        let partition_count = partition_ids.len();

        let (config_inspection_stage, config_entries) =
            inspect_topic_operation_configs(&profile, &topic_name).await;
        let config_update_stage =
            inspect_topic_config_update_precheck(&profile, &topic_name, &config_entries).await;
        let tag_management_stage =
            inspect_topic_tag_management_precheck(&profile, &topic_name).await;
        let partition_expansion_stage =
            inspect_topic_partition_expansion_precheck(&profile, &topic_name, partition_count)
                .await;
        let offset_reset_stage =
            inspect_topic_offset_reset_precheck(&profile, &topic_name, partition_ids.as_slice())
                .await;

        let stages = vec![
            ValidationStageDto::passed(
                "metadata-client",
                "元数据客户端",
                "Kafka 元数据客户端已成功建立。",
            )
            .with_detail("当前连接可用于 Topic 存在性检查与后续运维能力探测。"),
            ValidationStageDto::passed(
                "topic-exists",
                "Topic 存在性",
                format!("Topic '{}' 已存在。", topic_name),
            )
            .with_detail("基础 Topic 元数据已成功读取。"),
            config_inspection_stage,
            config_update_stage,
            tag_management_stage,
            partition_expansion_stage,
            offset_reset_stage,
        ];

        Ok(build_topic_operations_overview_response(stages, config_entries))
    }

    pub async fn update_topic_config(
        &self,
        request: UpdateTopicConfigRequest,
    ) -> AppResult<UpdateTopicConfigResponseDto> {
        validate_update_topic_config_request(&request)?;

        let profile = sqlite::get_cluster_profile(self.pool, &request.cluster_profile_id).await?;
        let topic_name = request.topic_name.trim().to_string();
        let config_key = request.config_key.trim().to_string();
        let requested_value = normalize_required_update_value(
            request.requested_value.as_deref(),
            "requested value",
            &config_key,
        )?;
        let expected_current_value = normalize_required_update_value(
            request.expected_current_value.as_deref(),
            "expected current value",
            &config_key,
        )?;

        let config_entries = describe_topic_operation_configs(&profile, &topic_name).await?;
        let current_entry = config_entries
            .iter()
            .find(|entry| entry.key == config_key)
            .ok_or_else(|| {
                AppError::Unsupported(format!(
                    "topic '{}' does not expose supported config key '{}'",
                    topic_name, config_key
                ))
            })?;

        if !current_entry.is_supported {
            return Err(AppError::Unsupported(format!(
                "topic '{}' config key '{}' is not supported by this cluster",
                topic_name, config_key
            )));
        }

        if current_entry.is_read_only == Some(true) {
            return Err(AppError::Unsupported(format!(
                "topic '{}' config key '{}' is read only on this cluster",
                topic_name, config_key
            )));
        }

        let previous_value = current_entry.value.clone();
        validate_topic_config_expected_current_value(
            &topic_name,
            &config_key,
            previous_value.as_deref(),
            &expected_current_value,
        )?;

        validate_topic_config_requested_change(
            &topic_name,
            &config_key,
            previous_value.as_deref(),
            &requested_value,
        )?;

        let audit_created_at = Utc::now().to_rfc3339();
        let audit_id = Uuid::new_v4().to_string();
        tokio::task::spawn_blocking({
            let profile = profile.clone();
            let topic_name = topic_name.clone();
            let config_key = config_key.clone();
            let requested_value_for_write = requested_value.clone();
            move || {
                perform_topic_config_update(
                    &profile,
                    &topic_name,
                    &config_key,
                    requested_value_for_write,
                )
            }
        })
        .await
        .map_err(|error| {
            AppError::Internal(format!("failed to join topic config update task: {error}"))
        })??;

        let mut warning_messages = Vec::new();
        let resulting_value = match describe_topic_operation_configs(&profile, &topic_name).await {
            Ok(updated_config_entries) => updated_config_entries
                .iter()
                .find(|entry| entry.key == config_key)
                .map(|entry| entry.value.clone())
                .unwrap_or_else(|| {
                    warning_messages.push(format!(
                        "Kafka 已应用配置修改，但后续校验未返回 '{}' 的最新值。",
                        config_key
                    ));
                    Some(requested_value.clone())
                }),
            Err(error) => {
                warning_messages.push(format!(
                    "Kafka 已应用配置修改，但后续校验失败：{}",
                    error
                ));
                Some(requested_value.clone())
            }
        };

        let audit_record = AuditEventRecord {
            id: audit_id.clone(),
            event_type: "topic_config_updated".to_string(),
            target_type: "topic_config".to_string(),
            target_ref: Some(format!("{}::{}", topic_name, config_key)),
            actor_profile: Some(request.cluster_profile_id.clone()),
            cluster_profile_id: Some(request.cluster_profile_id.clone()),
            outcome: if warning_messages.is_empty() {
                "success".to_string()
            } else {
                "warning".to_string()
            },
            summary: format!(
                "Updated topic config '{}' on topic '{}'",
                config_key, topic_name
            ),
            details_json: Some(
                json!({
                    "topicName": topic_name,
                    "configKey": config_key,
                    "previousValue": previous_value,
                    "requestedValue": request.requested_value,
                    "resultingValue": resulting_value,
                    "expectedCurrentValue": request.expected_current_value,
                    "riskAcknowledged": request.risk_acknowledged,
                    "warningMessages": warning_messages.clone(),
                })
                .to_string(),
            ),
            created_at: audit_created_at,
        };

        let audit_ref = match sqlite::insert_audit_event(self.pool, &audit_record).await {
            Ok(()) => Some(audit_id),
            Err(error) => {
                warning_messages.push(format!(
                    "Kafka 已应用配置修改，但审计记录写入失败：{}",
                    error
                ));
                None
            }
        };

        Ok(UpdateTopicConfigResponseDto {
            topic_name,
            config_key,
            previous_value,
            requested_value: Some(requested_value),
            resulting_value,
            audit_ref,
            warning: (!warning_messages.is_empty()).then(|| warning_messages.join(" ")),
        })
    }

    pub async fn expand_topic_partitions(
        &self,
        request: ExpandTopicPartitionsRequest,
    ) -> AppResult<ExpandTopicPartitionsResponseDto> {
        validate_expand_topic_partitions_request(&request)?;

        let profile = sqlite::get_cluster_profile(self.pool, &request.cluster_profile_id).await?;
        let topic_name = request.topic_name.trim().to_string();
        let requested_partition_count = request.requested_partition_count;
        let previous_partition_count = fetch_topic_partition_count(&profile, &topic_name).await?;

        validate_topic_partition_expected_count(
            &topic_name,
            previous_partition_count,
            request.expected_current_partition_count,
        )?;
        validate_topic_partition_expansion_change(
            &topic_name,
            previous_partition_count,
            requested_partition_count,
        )?;

        let audit_created_at = Utc::now().to_rfc3339();
        let audit_id = Uuid::new_v4().to_string();

        perform_topic_partition_expansion(&profile, &topic_name, requested_partition_count).await?;

        let mut warning_messages = Vec::new();
        let resulting_partition_count = match fetch_topic_partition_count(&profile, &topic_name).await {
            Ok(count) => {
                if count < requested_partition_count {
                    warning_messages.push(format!(
                        "Kafka 已接受 Topic 分区扩容请求，但后续校验只读取到 {} 个分区，少于请求的 {} 个。",
                        count, requested_partition_count
                    ));
                }
                count
            }
            Err(error) => {
                warning_messages.push(format!(
                    "Kafka 已接受 Topic 分区扩容请求，但后续元数据校验失败：{}",
                    error
                ));
                requested_partition_count
            }
        };

        let audit_record = AuditEventRecord {
            id: audit_id.clone(),
            event_type: "topic_partitions_expanded".to_string(),
            target_type: "topic".to_string(),
            target_ref: Some(topic_name.clone()),
            actor_profile: Some(request.cluster_profile_id.clone()),
            cluster_profile_id: Some(request.cluster_profile_id.clone()),
            outcome: if warning_messages.is_empty() {
                "success".to_string()
            } else {
                "warning".to_string()
            },
            summary: format!(
                "Expanded topic '{}' partitions from {} to {}",
                topic_name, previous_partition_count, requested_partition_count
            ),
            details_json: Some(
                json!({
                    "topicName": topic_name,
                    "previousPartitionCount": previous_partition_count,
                    "requestedPartitionCount": requested_partition_count,
                    "resultingPartitionCount": resulting_partition_count,
                    "expectedCurrentPartitionCount": request.expected_current_partition_count,
                    "riskAcknowledged": request.risk_acknowledged,
                    "warningMessages": warning_messages.clone(),
                })
                .to_string(),
            ),
            created_at: audit_created_at,
        };

        let audit_ref = match sqlite::insert_audit_event(self.pool, &audit_record).await {
            Ok(()) => Some(audit_id),
            Err(error) => {
                warning_messages.push(format!(
                    "Kafka 已接受 Topic 分区扩容请求，但审计记录写入失败：{}",
                    error
                ));
                None
            }
        };

        Ok(ExpandTopicPartitionsResponseDto {
            topic_name,
            previous_partition_count,
            requested_partition_count,
            resulting_partition_count,
            audit_ref,
            warning: (!warning_messages.is_empty()).then(|| warning_messages.join(" ")),
        })
    }

    pub async fn update_topic_tags(
        &self,
        request: crate::models::topic::UpdateTopicTagsRequest,
    ) -> AppResult<TopicSummaryDto> {
        validate_update_topic_tags_request(&request)?;
        let profile = sqlite::get_cluster_profile(self.pool, &request.cluster_profile_id).await?;
        let topic_name = request.topic_name.trim().to_string();
        let normalized_tags = normalize_tags(&request.tags);

        let partition_count = fetch_topic_partition_count(&profile, &topic_name).await?;
        sqlite::upsert_topic_tags(
            self.pool,
            &request.cluster_profile_id,
            &topic_name,
            &normalized_tags,
            &Utc::now().to_rfc3339(),
        )
        .await?;

        Ok(TopicSummaryDto {
            name: topic_name,
            partition_count,
            replication_factor: None,
            schema_type: None,
            retention_summary: None,
            activity_hint: Some(if partition_count > 32 {
                "高分区规模".to_string()
            } else if partition_count > 8 {
                "中等分区规模".to_string()
            } else {
                "常规分区规模".to_string()
            }),
            is_favorite: false,
            tags: normalized_tags,
        })
    }
}

async fn perform_topic_partition_expansion(
    profile: &ClusterProfileDto,
    topic_name: &str,
    requested_partition_count: usize,
) -> AppResult<()> {
    let admin_client = build_kafka_admin_client(profile)?;
    let options = AdminOptions::new()
        .operation_timeout(Some(Duration::from_secs(30)))
        .validate_only(false);
    let partitions = [NewPartitions::new(topic_name, requested_partition_count)];
    let results = admin_client
        .create_partitions(partitions.iter(), &options)
        .await
        .map_err(|error| {
            AppError::Network(format!(
                "failed to request partition expansion for topic '{}': {error}",
                topic_name
            ))
        })?;

    for result in results {
        match result {
            Ok(returned_topic) if returned_topic == topic_name => return Ok(()),
            Ok(returned_topic) => {
                return Err(AppError::Internal(format!(
                    "Kafka returned partition expansion result for unexpected topic '{}' while expanding '{}'",
                    returned_topic, topic_name
                )));
            }
            Err((returned_topic, error_code)) => {
                return Err(AppError::Unsupported(format!(
                    "Kafka rejected partition expansion for topic '{}': {:?}",
                    returned_topic, error_code
                )));
            }
        }
    }

    Err(AppError::Internal(format!(
        "Kafka returned no partition expansion result for topic '{}'",
        topic_name
    )))
}

fn perform_topic_config_update(
    profile: &ClusterProfileDto,
    topic_name: &str,
    config_key: &str,
    requested_value: String,
) -> AppResult<()> {
    let admin_client = build_kafka_admin_client(profile)?;
    let native_client = admin_client.inner().native_ptr();

    let queue = KafkaQueue(unsafe { rdsys::rd_kafka_queue_new(native_client) });
    if queue.0.is_null() {
        return Err(AppError::Internal(format!(
            "failed to create Kafka admin queue for topic '{}' config '{}'",
            topic_name, config_key
        )));
    }

    let options = KafkaAdminOptions(unsafe {
        rdsys::rd_kafka_AdminOptions_new(
            native_client,
            rdsys::RDKafkaAdminOp::RD_KAFKA_ADMIN_OP_INCREMENTALALTERCONFIGS,
        )
    });
    if options.0.is_null() {
        return Err(AppError::Internal(format!(
            "failed to create Kafka admin options for topic '{}' config '{}'",
            topic_name, config_key
        )));
    }

    let topic_c = cstring(topic_name, "topic name")?;
    let key_c = cstring(config_key, "config key")?;
    let resource = KafkaConfigResource(unsafe {
        rdsys::rd_kafka_ConfigResource_new(
            rdsys::RDKafkaResourceType::RD_KAFKA_RESOURCE_TOPIC,
            topic_c.as_ptr(),
        )
    });
    if resource.0.is_null() {
        return Err(AppError::Internal(format!(
            "failed to create Kafka config resource for topic '{}'",
            topic_name
        )));
    }

    let value_c = cstring(&requested_value, &format!("config value for '{}'", config_key))?;
    let op_type = rdsys::rd_kafka_AlterConfigOpType_t::RD_KAFKA_ALTER_CONFIG_OP_TYPE_SET;

    let error = unsafe {
        rdsys::rd_kafka_ConfigResource_add_incremental_config(
            resource.0,
            key_c.as_ptr(),
            op_type,
            value_c.as_ptr(),
        )
    };
    if !error.is_null() {
        let message = unsafe {
            CStr::from_ptr(rdsys::rd_kafka_error_string(error))
                .to_string_lossy()
                .into_owned()
        };
        unsafe {
            rdsys::rd_kafka_error_destroy(error);
        }
        return Err(AppError::Validation(format!(
            "invalid incremental config request for topic '{}' key '{}': {}",
            topic_name, config_key, message
        )));
    }

    unsafe {
        let mut resources = [resource.0];
        rdsys::rd_kafka_IncrementalAlterConfigs(
            native_client,
            resources.as_mut_ptr(),
            resources.len(),
            options.0,
            queue.0,
        );
    }

    let event_ptr = unsafe { rdsys::rd_kafka_queue_poll(queue.0, TOPIC_CONFIG_UPDATE_POLL_TIMEOUT_MS) };
    if event_ptr.is_null() {
        return Err(AppError::Network(format!(
            "timed out waiting for Kafka to apply topic '{}' config '{}'",
            topic_name, config_key
        )));
    }

    let event = KafkaEvent(event_ptr);
    let event_error = unsafe { rdsys::rd_kafka_event_error(event.0) };
    if event_error != rdsys::RDKafkaRespErr::RD_KAFKA_RESP_ERR_NO_ERROR {
        let message = unsafe { cstr_or_empty(rdsys::rd_kafka_event_error_string(event.0)) };
        return Err(AppError::Unsupported(format!(
            "Kafka rejected incremental config update for topic '{}' key '{}': {}",
            topic_name, config_key, message
        )));
    }

    let result = unsafe { rdsys::rd_kafka_event_IncrementalAlterConfigs_result(event.0) };
    if result.is_null() {
        return Err(AppError::Internal(format!(
            "Kafka returned an unexpected response for topic '{}' config '{}'",
            topic_name, config_key
        )));
    }

    let mut resource_count = 0;
    let resources = unsafe {
        rdsys::rd_kafka_IncrementalAlterConfigs_result_resources(result, &mut resource_count)
    };
    if resources.is_null() || resource_count == 0 {
        return Err(AppError::Internal(format!(
            "Kafka returned no resource result for topic '{}' config '{}'",
            topic_name, config_key
        )));
    }

    for index in 0..resource_count {
        let resource = unsafe { *resources.add(index) };
        let resource_error = unsafe { rdsys::rd_kafka_ConfigResource_error(resource) };
        if resource_error != rdsys::RDKafkaRespErr::RD_KAFKA_RESP_ERR_NO_ERROR {
            let message = unsafe {
                cstr_or_empty(rdsys::rd_kafka_ConfigResource_error_string(resource))
            };
            return Err(AppError::Unsupported(format!(
                "Kafka rejected incremental config update for topic '{}' key '{}': {}",
                topic_name, config_key, message
            )));
        }
    }

    Ok(())
}

const TOPIC_CONFIG_UPDATE_POLL_TIMEOUT_MS: c_int = 15_000;

fn validate_expand_topic_partitions_request(
    request: &ExpandTopicPartitionsRequest,
) -> AppResult<()> {
    if request.cluster_profile_id.trim().is_empty() {
        return Err(AppError::Validation(
            "cluster profile id is required".to_string(),
        ));
    }

    if request.topic_name.trim().is_empty() {
        return Err(AppError::Validation("topic name is required".to_string()));
    }

    if !request.risk_acknowledged {
        return Err(AppError::Validation(
            "topic partition expansion risk acknowledgement is required".to_string(),
        ));
    }

    if request.expected_current_partition_count == 0 {
        return Err(AppError::Validation(
            "expected current partition count must be greater than zero".to_string(),
        ));
    }

    if request.requested_partition_count == 0 {
        return Err(AppError::Validation(
            "requested partition count must be greater than zero".to_string(),
        ));
    }

    Ok(())
}

fn validate_topic_partition_expected_count(
    topic_name: &str,
    current_partition_count: usize,
    expected_current_partition_count: usize,
) -> AppResult<()> {
    if current_partition_count != expected_current_partition_count {
        return Err(AppError::Validation(format!(
            "topic '{}' partition count changed before expansion; expected {}, found {}",
            topic_name, expected_current_partition_count, current_partition_count
        )));
    }

    Ok(())
}

fn validate_topic_partition_expansion_change(
    topic_name: &str,
    current_partition_count: usize,
    requested_partition_count: usize,
) -> AppResult<()> {
    if requested_partition_count <= current_partition_count {
        return Err(AppError::Validation(format!(
            "topic '{}' partition expansion requires a count greater than current count {}; requested {}",
            topic_name, current_partition_count, requested_partition_count
        )));
    }

    Ok(())
}

fn validate_update_topic_config_request(request: &UpdateTopicConfigRequest) -> AppResult<()> {
    if request.cluster_profile_id.trim().is_empty() {
        return Err(AppError::Validation(
            "cluster profile id is required".to_string(),
        ));
    }

    if request.topic_name.trim().is_empty() {
        return Err(AppError::Validation("topic name is required".to_string()));
    }

    let config_key = request.config_key.trim();
    if config_key.is_empty() {
        return Err(AppError::Validation("config key is required".to_string()));
    }

    if !request.risk_acknowledged {
        return Err(AppError::Validation(
            "topic config update risk acknowledgement is required".to_string(),
        ));
    }

    if !TOPIC_OPERATION_CONFIG_KEYS.contains(&config_key) {
        return Err(AppError::Unsupported(format!(
            "topic config key '{}' is not supported by this workflow",
            config_key
        )));
    }

    if request.requested_value.is_none() {
        return Err(AppError::Validation(format!(
            "requested value for '{}' is required",
            config_key
        )));
    }

    validate_topic_config_update_value(config_key, request.requested_value.as_deref())?;

    let expected_current_value = request.expected_current_value.as_deref().ok_or_else(|| {
        AppError::Validation(format!(
            "expected current value for '{}' is required",
            config_key
        ))
    })?;

    if expected_current_value.trim().is_empty() {
        return Err(AppError::Validation(format!(
            "expected current value for '{}' must not be empty",
            config_key
        )));
    }

    Ok(())
}

fn validate_topic_config_update_value(
    config_key: &str,
    requested_value: Option<&str>,
) -> AppResult<()> {
    let Some(value) = requested_value else {
        return Ok(());
    };

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(format!(
            "requested value for '{}' must not be empty",
            config_key
        )));
    }

    match config_key {
        "cleanup.policy" => {
            let mut seen = std::collections::BTreeSet::new();
            for policy in trimmed.split(',').map(str::trim) {
                if policy.is_empty() {
                    return Err(AppError::Validation(
                        "cleanup.policy must list one or more values separated by commas"
                            .to_string(),
                    ));
                }

                match policy {
                    "compact" | "delete" => {}
                    other => {
                        return Err(AppError::Validation(format!(
                            "cleanup.policy does not accept '{}'",
                            other
                        )));
                    }
                }

                if !seen.insert(policy.to_string()) {
                    return Err(AppError::Validation(
                        "cleanup.policy must not contain duplicate values".to_string(),
                    ));
                }
            }
        }
        "retention.ms" | "max.message.bytes" => {
            trimmed.parse::<u64>().map_err(|_| {
                AppError::Validation(format!(
                    "{} must be a non-negative integer",
                    config_key
                ))
            })?;
        }
        other => {
            return Err(AppError::Unsupported(format!(
                "topic config key '{}' is not supported by this workflow",
                other
            )));
        }
    }

    Ok(())
}

fn normalize_required_update_value(
    value: Option<&str>,
    field_label: &str,
    config_key: &str,
) -> AppResult<String> {
    let Some(value) = value else {
        return Err(AppError::Validation(format!(
            "{} for '{}' is required",
            field_label, config_key
        )));
    };

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(format!(
            "{} for '{}' must not be empty",
            field_label, config_key
        )));
    }

    Ok(trimmed.to_string())
}

fn validate_topic_config_expected_current_value(
    topic_name: &str,
    config_key: &str,
    previous_value: Option<&str>,
    expected_current_value: &str,
) -> AppResult<()> {
    if previous_value != Some(expected_current_value) {
        return Err(AppError::Validation(format!(
            "topic '{}' config key '{}' changed before update; expected current value {:?}, found {:?}",
            topic_name, config_key, expected_current_value, previous_value
        )));
    }

    Ok(())
}

fn validate_topic_config_requested_change(
    topic_name: &str,
    config_key: &str,
    previous_value: Option<&str>,
    requested_value: &str,
) -> AppResult<()> {
    if previous_value == Some(requested_value) {
        return Err(AppError::Validation(format!(
            "topic '{}' config key '{}' already matches the requested value",
            topic_name, config_key
        )));
    }

    Ok(())
}

fn cstring(value: &str, label: &str) -> AppResult<CString> {
    CString::new(value)
        .map_err(|_| AppError::Validation(format!("{} must not contain NUL bytes", label)))
}

unsafe fn cstr_or_empty(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

fn build_topic_operations_overview_response(
    stages: Vec<ValidationStageDto>,
    config_entries: Vec<TopicOperationConfigEntryDto>,
) -> TopicOperationsOverviewResponseDto {
    let status = summarize_validation_status(&stages);
    let message = match status {
        crate::models::validation::ValidationStatusDto::Passed => {
            "Topic 运维概览已完成只读配置探测、配置修改预检查、标签管理预检查、分区扩容预检查与 offset reset 预检查。".to_string()
        }
        crate::models::validation::ValidationStatusDto::Warning => {
            "Topic 运维概览已建立，但当前集群对只读运维预检查仍存在限制或缺失。".to_string()
        }
        crate::models::validation::ValidationStatusDto::Failed => {
            "Topic 运维能力当前不可用。".to_string()
        }
        crate::models::validation::ValidationStatusDto::Skipped => {
            "Topic 运维能力当前未执行。".to_string()
        }
    };

    TopicOperationsOverviewResponseDto {
        status,
        message,
        stages,
        config_entries,
    }
}

async fn inspect_topic_operation_configs(
    profile: &ClusterProfileDto,
    topic_name: &str,
) -> (ValidationStageDto, Vec<TopicOperationConfigEntryDto>) {
    match describe_topic_operation_configs(profile, topic_name).await {
        Ok(config_entries) => {
            let supported_count = config_entries
                .iter()
                .filter(|entry| entry.is_supported)
                .count();
            let missing_count = config_entries.len().saturating_sub(supported_count);

            let stage = if missing_count == 0 {
                ValidationStageDto::passed(
                    "topic-config-inspection",
                    "Topic 配置探测",
                    format!(
                        "已完成 Topic '{}' 的只读配置探测，共返回 {} 个受支持配置项。",
                        topic_name, supported_count
                    ),
                )
                .with_detail("当前版本会如实返回配置值、来源、默认态与只读/敏感标记。")
            } else {
                ValidationStageDto::warning(
                    "topic-config-inspection",
                    "Topic 配置探测",
                    format!(
                        "已完成 Topic '{}' 的只读配置探测，但仍有 {} 个配置项未被 broker 返回。",
                        topic_name, missing_count
                    ),
                )
                .with_detail(
                    "这通常意味着当前集群未暴露对应配置，或 broker/权限侧尚不支持完整配置探测。",
                )
            };

            (stage, config_entries)
        }
        Err(error) => {
            let detail = format!(
                "Kafka admin/config 探测未成功完成：{}。这通常与 broker 版本、权限或集群策略限制有关。",
                error
            );

            (
                ValidationStageDto::warning(
                    "topic-config-inspection",
                    "Topic 配置探测",
                    format!("Topic '{}' 当前无法返回只读配置探测结果。", topic_name),
                )
                .with_detail(detail.clone()),
                build_unsupported_topic_operation_entries(&detail),
            )
        }
    }
}

async fn inspect_topic_partition_expansion_precheck(
    profile: &ClusterProfileDto,
    topic_name: &str,
    current_partition_count: usize,
) -> ValidationStageDto {
    match build_kafka_admin_client(profile) {
        Ok(_) => build_topic_partition_expansion_stage(topic_name, current_partition_count, None),
        Err(error) => build_topic_partition_expansion_stage(
            topic_name,
            current_partition_count,
            Some(&error.to_string()),
        ),
    }
}

async fn inspect_topic_config_update_precheck(
    profile: &ClusterProfileDto,
    topic_name: &str,
    config_entries: &[TopicOperationConfigEntryDto],
) -> ValidationStageDto {
    let admin_client_error = build_kafka_admin_client(profile)
        .err()
        .map(|error| error.to_string());

    build_topic_config_update_stage(topic_name, config_entries, admin_client_error.as_deref())
}

async fn inspect_topic_tag_management_precheck(
    profile: &ClusterProfileDto,
    topic_name: &str,
) -> ValidationStageDto {
    match build_kafka_admin_client(profile) {
        Ok(_) => build_topic_tag_management_stage(topic_name, None),
        Err(error) => build_topic_tag_management_stage(topic_name, Some(&error.to_string())),
    }
}

fn build_topic_tag_management_stage(
    topic_name: &str,
    admin_client_error: Option<&str>,
) -> ValidationStageDto {
    match admin_client_error {
        Some(error) => ValidationStageDto::warning(
            "topic-tag-management",
            "Topic 标签管理前置预检查",
            format!(
                "已启动 Topic '{}' 的标签管理前置预检查，但当前运行时尚不足以进入后续标签管理工作流。",
                topic_name
            ),
        )
        .with_detail(format!(
            "当前未执行任何标签写入。Kafka admin 客户端预检查失败：{}。更重要的是，当前 backend 还没有 topic 标签持久化或读取路径，因此无法如实声明 Topic 标签管理已被支持。",
            error
        )),
        None => ValidationStageDto::warning(
            "topic-tag-management",
            "Topic 标签管理前置预检查",
            format!(
                "已完成 Topic '{}' 的标签管理前置预检查，但当前 backend 仍未提供 topic 标签持久化或读取路径。",
                topic_name
            ),
        )
        .with_detail(
            "当前未执行任何标签写入。topic 级标签仅停留在计划阶段，backend 还没有可验证的 topic-tag persistence/read support，因此这一步只能作为能力占位与风险提示，不能被解释为标签管理已可用。",
        ),
    }
}

fn build_topic_config_update_stage(
    topic_name: &str,
    config_entries: &[TopicOperationConfigEntryDto],
    admin_client_error: Option<&str>,
) -> ValidationStageDto {
    let supported_entries = config_entries
        .iter()
        .filter(|entry| entry.is_supported)
        .collect::<Vec<_>>();
    let writable_entries = supported_entries
        .iter()
        .filter(|entry| entry.is_read_only == Some(false))
        .map(|entry| entry.key.as_str())
        .collect::<Vec<_>>();

    let baseline_detail = format!(
        "当前 allowlist 共返回 {} 个受支持配置项，其中 {} 个未被 broker 标记为只读。本阶段未执行任何配置写入。",
        supported_entries.len(),
        writable_entries.len()
    );

    if let Some(error) = admin_client_error {
        return ValidationStageDto::warning(
            "topic-config-update",
            "Topic 配置修改只读预检查",
            format!(
                "已完成 Topic '{}' 的配置修改只读预检查，但当前运行时尚不足以进入后续配置修改工作流。",
                topic_name
            ),
        )
        .with_detail(format!(
            "{} 当前无法完成 Kafka admin 客户端预检查：{}。这意味着后续配置修改仍需先确认本地凭据、证书与运行时连通性。",
            baseline_detail, error
        ));
    }

    if writable_entries.is_empty() {
        if supported_entries.is_empty() {
            return ValidationStageDto::warning(
                "topic-config-update",
                "Topic 配置修改只读预检查",
                format!(
                    "已完成 Topic '{}' 的配置修改只读预检查，但当前缺少足够的可修改配置证据。",
                    topic_name
                ),
            )
            .with_detail(format!(
                "{} 当前仅能确认配置探测链路存在限制或缺失，因此不能如实判断后续配置修改候选项。",
                baseline_detail
            ));
        }

        return ValidationStageDto::warning(
            "topic-config-update",
            "Topic 配置修改只读预检查",
            format!(
                "已完成 Topic '{}' 的配置修改只读预检查，但当前受支持配置项均被标记为只读。",
                topic_name
            ),
        )
        .with_detail(format!(
            "{} 这意味着当前 allowlist 尚未提供适合进入后续配置修改工作流的候选项；即使后续开放写路径，也仍需再次确认 broker 策略、权限和审计要求。",
            baseline_detail
        ));
    }

    ValidationStageDto::passed(
        "topic-config-update",
        "Topic 配置修改只读预检查",
        format!(
            "已完成 Topic '{}' 的配置修改只读预检查，发现 {} 个候选配置项，但这并不代表已具备实际写入执行条件。",
            topic_name,
            writable_entries.len()
        ),
    )
    .with_detail(format!(
        "{} 当前候选项包括：{}。当前仅确认这些配置项在本次探测结果中未被 broker 标记为只读，且本地运行时可以组装 Kafka admin 客户端；这并不代表 broker 可达、具备 AlterConfigs/IncrementalAlterConfigs 支持，或已经满足实际写入所需的权限、确认环节与审计要求。",
        baseline_detail,
        writable_entries.join("、")
    ))
}

async fn inspect_topic_offset_reset_precheck(
    profile: &ClusterProfileDto,
    topic_name: &str,
    partition_ids: &[i32],
) -> ValidationStageDto {
    let profile = profile.clone();
    let topic_name = topic_name.to_string();
    let topic_name_for_fetch = topic_name.clone();
    let partition_ids = partition_ids.to_vec();

    let snapshot_result = tokio::task::spawn_blocking(move || {
        fetch_topic_group_snapshots(&profile, &topic_name_for_fetch, partition_ids.as_slice())
    })
    .await
    .map_err(|error| {
        AppError::Internal(format!(
            "failed to join topic offset-reset precheck task: {error}"
        ))
    });

    match snapshot_result {
        Ok(Ok(discovery)) => build_topic_offset_reset_stage(
            topic_name.as_str(),
            discovery.snapshots.as_slice(),
            discovery.skipped_group_count,
        ),
        Ok(Err(error)) => ValidationStageDto::warning(
            "topic-offset-reset",
            "Consumer Offset Reset 只读预检查",
            format!(
                "已启动 Topic '{}' 的 offset reset 只读预检查，但当前无法完整读取候选消费组位点信息。",
                topic_name
            ),
        )
        .with_detail(format!(
            "当前未执行任何 offset 写入。只读预检查失败原因：{}。后续真正进入 offset reset 工作流前，仍需确认消费组状态、目标位点策略、broker/ACL 权限以及审计要求。",
            error
        )),
        Err(error) => ValidationStageDto::warning(
            "topic-offset-reset",
            "Consumer Offset Reset 只读预检查",
            format!(
                "已启动 Topic '{}' 的 offset reset 只读预检查，但当前运行时无法完成候选消费组发现。",
                topic_name
            ),
        )
        .with_detail(format!(
            "当前未执行任何 offset 写入。运行时任务未能完成：{}。后续真正进入 offset reset 工作流前，仍需确认消费组状态、目标位点策略、broker/ACL 权限以及审计要求。",
            error
        )),
    }
}

fn build_topic_offset_reset_stage(
    topic_name: &str,
    group_snapshots: &[TopicGroupSnapshot],
    skipped_group_count: usize,
) -> ValidationStageDto {
    if group_snapshots.is_empty() {
        let detail = if skipped_group_count > 0 {
            format!(
                "当前未执行任何 offset 写入。只读预检查没有发现 topic-scoped committed offsets，因此不能如实判断后续 offset reset 候选范围；同时有 {} 个消费组因位点或水位信息不可读而被跳过。这并不等同于 reset 不受支持。",
                skipped_group_count
            )
        } else {
            "当前未执行任何 offset 写入。只读预检查没有发现 topic-scoped committed offsets，因此不能如实判断后续 offset reset 候选范围；这并不等同于 reset 不受支持。".to_string()
        };

        return ValidationStageDto::warning(
            "topic-offset-reset",
            "Consumer Offset Reset 只读预检查",
            format!(
                "已完成 Topic '{}' 的 offset reset 只读预检查，但当前未发现具备已提交位点的相关消费组。",
                topic_name
            ),
        )
        .with_detail(detail);
    }

    if skipped_group_count > 0 {
        let lagging_group_count = group_snapshots
            .iter()
            .filter(|snapshot| snapshot.total_lag > 0)
            .count();
        let total_impacted_partitions = group_snapshots
            .iter()
            .map(|snapshot| snapshot.partitions_impacted)
            .sum::<usize>();
        let candidate_groups = group_snapshots
            .iter()
            .take(3)
            .map(|snapshot| format!("{} ({})", snapshot.name, snapshot.state))
            .collect::<Vec<_>>()
            .join(" · ");

        return ValidationStageDto::warning(
            "topic-offset-reset",
            "Consumer Offset Reset 只读预检查",
            format!(
                "已完成 Topic '{}' 的 offset reset 只读预检查，发现 {} 个候选消费组，但有 {} 个消费组因位点或水位信息不可读而被跳过。",
                topic_name,
                group_snapshots.len(),
                skipped_group_count
            ),
        )
        .with_detail(format!(
            "当前未执行任何 offset 写入。已识别 {} 个存在 topic-scoped committed offsets 的消费组，其中 {} 个当前存在 lag，累计影响 {} 个分区。候选消费组示例：{}。另有 {} 个消费组因位点或水位信息不可读而被跳过，导致本次只读预检查只能覆盖部分候选范围；实际 reset 前仍需在正式工作流中确认消费组状态、目标位点策略、broker/ACL 权限、活跃成员影响以及审计要求。",
            group_snapshots.len(),
            lagging_group_count,
            total_impacted_partitions,
            candidate_groups,
            skipped_group_count
        ));
    }

    let lagging_group_count = group_snapshots
        .iter()
        .filter(|snapshot| snapshot.total_lag > 0)
        .count();
    let total_impacted_partitions = group_snapshots
        .iter()
        .map(|snapshot| snapshot.partitions_impacted)
        .sum::<usize>();
    let candidate_groups = group_snapshots
        .iter()
        .take(3)
        .map(|snapshot| format!("{} ({})", snapshot.name, snapshot.state))
        .collect::<Vec<_>>()
        .join(" · ");

    let partial_visibility_note = if skipped_group_count > 0 {
        format!(
            "另有 {} 个消费组因位点或水位信息不可读而被跳过。",
            skipped_group_count
        )
    } else {
        String::new()
    };

    ValidationStageDto::passed(
        "topic-offset-reset",
        "Consumer Offset Reset 只读预检查",
        format!(
            "已完成 Topic '{}' 的 offset reset 只读预检查，发现 {} 个候选消费组，但这并不代表已具备实际写入执行条件。",
            topic_name,
            group_snapshots.len()
        ),
    )
    .with_detail(format!(
        "当前未执行任何 offset 写入。已识别 {} 个存在 topic-scoped committed offsets 的消费组，其中 {} 个当前存在 lag，累计影响 {} 个分区。候选消费组示例：{}。{} 实际 reset 前仍需在正式工作流中确认消费组状态、目标位点策略、broker/ACL 权限、活跃成员影响以及审计要求。",
        group_snapshots.len(),
        lagging_group_count,
        total_impacted_partitions,
        candidate_groups,
        partial_visibility_note
    ))
}

fn compute_topic_group_partition_lag(low: i64, high: i64, committed_offset: i64) -> i64 {
    let effective_committed = committed_offset.max(low).min(high);
    (high - effective_committed).max(0)
}

fn build_topic_partition_expansion_stage(
    topic_name: &str,
    current_partition_count: usize,
    admin_client_error: Option<&str>,
) -> ValidationStageDto {
    let baseline_detail = format!(
        "当前 Topic 基线分区数为 {}。Kafka 分区扩容只允许增加分区，本阶段未执行任何写操作。",
        current_partition_count
    );

    match admin_client_error {
        None => ValidationStageDto::passed(
            "topic-partition-expansion",
            "Topic 分区扩容只读预检查",
            format!(
                "已完成 Topic '{}' 的分区扩容只读预检查，当前分区数为 {}，但这并不代表已具备实际写入执行条件。",
                topic_name, current_partition_count
            ),
        )
        .with_detail(format!(
            "{} 当前仅确认运行时可组装 Kafka admin 客户端；broker 版本、ACL 与实际 CreatePartitions 写权限仍需在正式工作流中再次确认。",
            baseline_detail
        )),
        Some(error) => ValidationStageDto::warning(
            "topic-partition-expansion",
            "Topic 分区扩容只读预检查",
            format!(
                "已读取 Topic '{}' 的分区基线，但分区扩容前置检查仍不完整。",
                topic_name
            ),
        )
        .with_detail(format!(
            "{} 当前无法完成 Kafka admin 客户端预检查：{}。这通常意味着本地运行时配置、证书或凭据尚不足以进入后续扩容工作流。",
            baseline_detail, error
        )),
    }
}

async fn describe_topic_operation_configs(
    profile: &ClusterProfileDto,
    topic_name: &str,
) -> AppResult<Vec<TopicOperationConfigEntryDto>> {
    let admin_client = build_kafka_admin_client(profile)?;
    let resource = ResourceSpecifier::Topic(topic_name);
    let options = AdminOptions::new().request_timeout(Some(Duration::from_secs(5)));
    let mut results = admin_client
        .describe_configs([&resource], &options)
        .await
        .map_err(|error| {
            AppError::Network(format!(
                "failed to describe topic configs for '{}': {error}",
                topic_name
            ))
        })?;

    let config_resource = results
        .pop()
        .ok_or_else(|| {
            AppError::Internal("Kafka did not return a config resource result".to_string())
        })?
        .map_err(|error| {
            AppError::Network(format!(
                "Kafka rejected topic config inspection for '{}': {error}",
                topic_name
            ))
        })?;

    Ok(map_topic_operation_config_entries(&config_resource))
}

fn map_topic_operation_config_entries(
    config_resource: &ConfigResource,
) -> Vec<TopicOperationConfigEntryDto> {
    let config_map = config_resource.entry_map();

    TOPIC_OPERATION_CONFIG_KEYS
        .iter()
        .map(|key| match config_map.get(key) {
            Some(entry) => map_topic_operation_config_entry(entry),
            None => unsupported_topic_operation_entry(
                key,
                "当前 broker 未返回该配置项，可能是版本、权限或发行版能力限制。",
            ),
        })
        .collect()
}

fn map_topic_operation_config_entry(entry: &ConfigEntry) -> TopicOperationConfigEntryDto {
    let mut notes = Vec::new();

    if entry.is_sensitive && entry.value.is_none() {
        notes.push("该配置被标记为敏感字段，Kafka 不会返回明文值。".to_string());
    } else if entry.value.is_none() {
        notes.push("Kafka 返回了该配置项，但未提供具体值。".to_string());
    }

    if matches!(entry.source, ConfigSource::Unknown) {
        notes.push("当前 broker 未可靠提供配置来源元数据，常见于较旧 Kafka 版本。".to_string());
    }

    TopicOperationConfigEntryDto {
        key: entry.name.clone(),
        value: entry.value.clone(),
        is_supported: true,
        is_read_only: Some(entry.is_read_only),
        is_default: Some(entry.is_default),
        is_sensitive: Some(entry.is_sensitive),
        source: Some(map_config_source(&entry.source).to_string()),
        note: (!notes.is_empty()).then(|| notes.join(" ")),
    }
}

fn build_unsupported_topic_operation_entries(note: &str) -> Vec<TopicOperationConfigEntryDto> {
    TOPIC_OPERATION_CONFIG_KEYS
        .iter()
        .map(|key| unsupported_topic_operation_entry(key, note))
        .collect()
}

fn unsupported_topic_operation_entry(key: &str, note: &str) -> TopicOperationConfigEntryDto {
    TopicOperationConfigEntryDto {
        key: key.to_string(),
        value: None,
        is_supported: false,
        is_read_only: None,
        is_default: None,
        is_sensitive: None,
        source: None,
        note: Some(note.to_string()),
    }
}

fn map_config_source(source: &ConfigSource) -> &'static str {
    match source {
        ConfigSource::Unknown => "unknown",
        ConfigSource::DynamicTopic => "dynamic-topic",
        ConfigSource::DynamicBroker => "dynamic-broker",
        ConfigSource::DynamicDefaultBroker => "dynamic-default-broker",
        ConfigSource::StaticBroker => "static-broker",
        ConfigSource::Default => "default",
    }
}

fn build_metadata_consumer(profile: &ClusterProfileDto) -> AppResult<BaseConsumer> {
    let mut config = ClientConfig::new();
    apply_kafka_read_consumer_config(&mut config, profile)?;

    config.create().map_err(|error| {
        AppError::Network(format!("failed to create Kafka metadata client: {error}"))
    })
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
        AppError::Network(format!(
            "failed to create topic-group inspection client: {error}"
        ))
    })
}

fn fetch_topic_group_snapshots(
    profile: &ClusterProfileDto,
    topic_name: &str,
    partition_ids: &[i32],
) -> AppResult<TopicGroupSnapshotDiscovery> {
    let inspector = build_group_consumer(profile, None)?;
    let groups = inspector
        .fetch_group_list(None, Duration::from_secs(5))
        .map_err(|error| {
            AppError::Network(format!(
                "failed to load consumer groups for topic '{}': {error}",
                topic_name
            ))
        })?;

    let mut snapshots = Vec::new();
    let mut skipped_group_count = 0;
    for group in groups.groups() {
        match fetch_topic_group_snapshot(
            profile,
            topic_name,
            partition_ids,
            group.name(),
            group.state(),
        ) {
            Ok(Some(snapshot)) => snapshots.push(snapshot),
            Ok(None) => {}
            Err(_) => {
                skipped_group_count += 1;
            }
        }
    }

    snapshots.sort_by(|left, right| {
        right
            .total_lag
            .cmp(&left.total_lag)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(TopicGroupSnapshotDiscovery {
        snapshots,
        skipped_group_count,
    })
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

        let (low, high) = consumer
            .fetch_watermarks(topic_name, element.partition(), Duration::from_secs(1))
            .map_err(|error| {
                AppError::Network(format!(
                    "failed to load watermarks for group '{}' topic '{}' partition {}: {error}",
                    group_name,
                    topic_name,
                    element.partition()
                ))
            })?;
        let lag = compute_topic_group_partition_lag(low, high, committed_offset);
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
            state: format!(
                "{} · 影响 {} 个分区",
                snapshot.state, snapshot.partitions_impacted
            ),
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
            "高分区规模".to_string()
        } else if partition_count > 8 {
            "中等分区规模".to_string()
        } else {
            "常规分区规模".to_string()
        }),
        is_favorite: false,
        tags: Vec::new(),
    }
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for tag in tags {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !normalized.iter().any(|existing| existing == trimmed) {
            normalized.push(trimmed.to_string());
        }
    }
    normalized
}

fn validate_update_topic_tags_request(
    request: &crate::models::topic::UpdateTopicTagsRequest,
) -> AppResult<()> {
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

async fn fetch_topic_partition_count(
    profile: &ClusterProfileDto,
    topic_name: &str,
) -> AppResult<usize> {
    let profile = profile.clone();
    let topic_name = topic_name.to_string();

    tokio::task::spawn_blocking(move || {
        let consumer = build_metadata_consumer(&profile)?;
        let metadata = consumer
            .fetch_metadata(Some(&topic_name), Duration::from_secs(5))
            .map_err(|error| {
                AppError::Network(format!(
                    "failed to load topic '{}' metadata for partition expansion: {error}",
                    topic_name
                ))
            })?;

        let topic = metadata
            .topics()
            .iter()
            .find(|topic| topic.name() == topic_name)
            .ok_or_else(|| AppError::NotFound(format!("topic '{}' was not found", topic_name)))?;

        Ok::<usize, AppError>(topic.partitions().len())
    })
    .await
    .map_err(|error| {
        AppError::Internal(format!(
            "failed to join topic partition-count task: {error}"
        ))
    })?
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

fn validate_get_topic_operations_overview_request(
    request: &GetTopicOperationsOverviewRequest,
) -> AppResult<()> {
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
    use super::{
        build_metadata_consumer, build_topic_config_update_stage, build_topic_offset_reset_stage,
        build_topic_operations_overview_response, build_topic_partition_expansion_stage,
        build_topic_tag_management_stage, compute_topic_group_partition_lag,
        map_topic_operation_config_entries, validate_get_topic_operations_overview_request,
        validate_topic_config_expected_current_value, validate_topic_config_requested_change,
        validate_topic_config_update_value,
        validate_expand_topic_partitions_request, validate_topic_partition_expected_count,
        validate_topic_partition_expansion_change,
        validate_update_topic_config_request,
        TopicGroupSnapshot, ValidationStageDto,
    };
    use crate::models::cluster::ClusterProfileDto;
    use crate::models::topic::{
        ExpandTopicPartitionsRequest,
        GetTopicOperationsOverviewRequest, TopicOperationConfigEntryDto,
        UpdateTopicConfigRequest,
    };
    use crate::models::validation::ValidationStatusDto;
    use rdkafka::admin::{ConfigEntry, ConfigResource, ConfigSource, OwnedResourceSpecifier};
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    const TEST_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----\nMIICsDCCAZgCCQD4vKEpwC7YOjANBgkqhkiG9w0BAQsFADAaMRgwFgYDVQQDDA90\ncmFjZWZvcmdlLXRlc3QwHhcNMjYwNDE4MDczMTU0WhcNMjcwNDE4MDczMTU0WjAa\nMRgwFgYDVQQDDA90cmFjZWZvcmdlLXRlc3QwggEiMA0GCSqGSIb3DQEBAQUAA4IB\nDwAwggEKAoIBAQDB3uz/2t6m3/qJaENMNk1KW6L0nE6Sr6a16C9alMoNk53q8Glx\nQA7shqJLUj8AHpbMMMddlc/RXz4cYXKLMq5XH2YVJ+QKxjcXR0Q6Swu3cqoMO7n8\nfiOiA9biPR3/78KSEPd9zdqOs3DwMHr65v3oEI9tc1rWvam9Xz+4MrkBFbmF4wZR\nYZ4ac+lgB9ie1FL4S4cRHYqYzvumNdMDtFv8hDCmyPdKUlfEowxtueuQ5WGB82yP\nGd0LlYOvxt79MiR0sIQsxVnSY5F0nLcNW9Z+bCuGgEWchakiUb9Nj2BcJ88IAGxs\npitqC3v+noeQA4SWkc+7Byd4dZS4rLGYv8FhAgMBAAEwDQYJKoZIhvcNAQELBQAD\nggEBACk9WDt7D7thZkoT8VJkyukWx4uPGXczOfp0+hu2eP1TODurSQziwVj3xF3O\noSjN8HrWg3U0vGqZGgqIPxPknbmwk5fjVorwWelRlX2X7DMElsFeRMZSY9leLC10\ntqdEu8mIJsGzR/Aua56fo3dywhIglYG/8O0tcZYjdp6YczXWW64lPz2vVv+9ZVVj\nnVrKYbU118mkVhd7jmV9QR5KdBY1th6qVEzI340S7CQ2PdweT0kemFwBTCp5gvJ5\na3Xi8pKrQKJk/L2O6oxhXOCCGvWhdEvZ8mel2Qp/whg6MupIciDKozdf68yECrUW\nEhr3a4kltLXboZZ+DJx+KZCTRv0=\n-----END CERTIFICATE-----\n";
    const TEST_PRIVATE_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQDB3uz/2t6m3/qJ\naENMNk1KW6L0nE6Sr6a16C9alMoNk53q8GlxQA7shqJLUj8AHpbMMMddlc/RXz4c\nYXKLMq5XH2YVJ+QKxjcXR0Q6Swu3cqoMO7n8fiOiA9biPR3/78KSEPd9zdqOs3Dw\nMHr65v3oEI9tc1rWvam9Xz+4MrkBFbmF4wZRYZ4ac+lgB9ie1FL4S4cRHYqYzvum\nNdMDtFv8hDCmyPdKUlfEowxtueuQ5WGB82yPGd0LlYOvxt79MiR0sIQsxVnSY5F0\nnLcNW9Z+bCuGgEWchakiUb9Nj2BcJ88IAGxspitqC3v+noeQA4SWkc+7Byd4dZS4\nrLGYv8FhAgMBAAECggEAU/ogZuODtn0mpQaIwCZ1bFQtTg+26Us0x27/tBjnPOJI\ncVAaHHhG/qWC/2Vs7LxTTbeDZEJUdrjuypRbbXhSlGKBcYCKAzDBBFBWeXmwZZJB\nnnLwqTJqdO980RrwN8C/Y03+JnxYw59uuFwHqU8NhMxHlH13R7V4JRNxInS2NoVV\nXVfgcTjax8pdbdzKKwIn3AUk27SwSJwBlYuMKgDq741/L8PvyjOmolvzwM+aF2FO\n1gqb3xZKM1867psYo4Z09qdc8GyG+joPEbJW9rQW2nORUy1mqApXmO8qprt+K3yY\nhWQUjYFpngx7OKOv0RhRSwzm9swK03QbLKK5i5/IwQKBgQDokMmZxVKbnpTlI8Fl\n6H2pQAIPi1HPdTTMapxBlP5CsLgtkBiYu60LYevmAcSdkzbpVr7uqWxy7+Z0upao\n7kheHaqovcO8xm1n1BDOnEgnmFJ9wLFDi8qG9EQd4dusvJWb6u3dvvWVWQFh4Pz4\nZPKxGbfa7VHeFSk9wizXuCGQpQKBgQDVZ/nhoCOIzwF9bxR9Gko6fdeLf6ZTK0ht\nMJZ8kbDlYoSjRWjX6zXdoLL7mQ2y7avQ8mEYVyOcBCb5xGa43cF6eKy6W59lXQqI\nB1Z64gaKAsFgqbhpJRo4sfEsft9pip/oI49Y9B6vP9do9UbfBg4ZZm5fZdS5+MnY\nWE70VTB1DQKBgQCzQh7SkuD4qIRWFnhUp55sXbT47Ecz5EC9K5OjjUdqejKMlBwR\nZd+c/W5KDKTTXIyf0Mg8x4SbF0UIRmYocfp/6NgJVrPQBxZ/SFtoFdgcBPHYkjVQ\nPijuWstCSTv86iNbWfrcx/sdkcxZ+ISkpZLXZV5sti47Qw5V1xyfbgMZLQKBgQCq\nI92LTvtFpZSQhrEVFJK9k3r3kuvuPwHdW/F+m0EngKYy7bGrA7HMYsSP5vSPBQII\n8lUK7N5NEtpoI3eqR9JrbC553XZ1f/pXfVIrYmzIN24pPObznUsMjIG1cel44baf\ng0pUJz0Xh5Sb74FzagZvpcS1diBlrL5wJ+e60PhzOQKBgQCy9K28nvKeC+JO40ST\ngp5YqlQnRNaWfXHTwbeeYygYfgiY+lw7hmtInnLcu7s3uVepoWpJJPGeVG7MiW+1\nfN3BpyGmBc+fA8UbUl2RHZKvKaLTWGwujYqwbfSOtI56FpNBHfjy4HyAvzAHwB7a\nlHc4mf+dH3zMIvjxLn2jsGjjFw==\n-----END PRIVATE KEY-----\n";

    fn create_temp_file(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("traceforge-{name}-{}.pem", Uuid::new_v4()));
        fs::write(&path, contents).expect("temp cert material should write");
        path
    }

    fn sample_mtls_profile(
        cert_path: &PathBuf,
        key_path: &PathBuf,
        ca_path: &PathBuf,
    ) -> ClusterProfileDto {
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

    fn sample_operation_config_entry(
        key: &str,
        is_supported: bool,
        is_read_only: Option<bool>,
    ) -> TopicOperationConfigEntryDto {
        TopicOperationConfigEntryDto {
            key: key.to_string(),
            value: Some("sample".to_string()),
            is_supported,
            is_read_only,
            is_default: is_supported.then_some(false),
            is_sensitive: is_supported.then_some(false),
            source: is_supported.then_some("dynamic-topic".to_string()),
            note: None,
        }
    }

    fn sample_topic_group_snapshot(
        name: &str,
        state: &str,
        total_lag: i64,
        partitions_impacted: usize,
    ) -> TopicGroupSnapshot {
        TopicGroupSnapshot {
            name: name.to_string(),
            state: state.to_string(),
            total_lag,
            partitions_impacted,
            partition_lags: (0..partitions_impacted as i32)
                .map(|partition_id| (partition_id, total_lag.max(0)))
                .collect(),
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

    #[test]
    fn topic_operations_overview_requires_topic_name() {
        let error =
            validate_get_topic_operations_overview_request(&GetTopicOperationsOverviewRequest {
                cluster_profile_id: "cluster-1".to_string(),
                topic_name: "   ".to_string(),
            })
            .expect_err("empty topic name should fail validation");

        assert_eq!(
            error.to_string(),
            "validation error: topic name is required"
        );
    }

    #[test]
    fn maps_topic_operation_config_entries_with_source_metadata() {
        let config_resource = ConfigResource {
            specifier: OwnedResourceSpecifier::Topic("orders".to_string()),
            entries: vec![
                ConfigEntry {
                    name: "cleanup.policy".to_string(),
                    value: Some("delete".to_string()),
                    source: ConfigSource::DynamicTopic,
                    is_read_only: false,
                    is_default: false,
                    is_sensitive: false,
                },
                ConfigEntry {
                    name: "retention.ms".to_string(),
                    value: Some("604800000".to_string()),
                    source: ConfigSource::Default,
                    is_read_only: true,
                    is_default: true,
                    is_sensitive: false,
                },
                ConfigEntry {
                    name: "max.message.bytes".to_string(),
                    value: None,
                    source: ConfigSource::Unknown,
                    is_read_only: true,
                    is_default: false,
                    is_sensitive: true,
                },
            ],
        };

        let entries = map_topic_operation_config_entries(&config_resource);

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].key, "cleanup.policy");
        assert_eq!(entries[0].value.as_deref(), Some("delete"));
        assert!(entries[0].is_supported);
        assert_eq!(entries[0].is_read_only, Some(false));
        assert_eq!(entries[0].source.as_deref(), Some("dynamic-topic"));
        assert!(entries[0].note.is_none());

        assert_eq!(entries[1].key, "retention.ms");
        assert_eq!(entries[1].is_default, Some(true));
        assert_eq!(entries[1].source.as_deref(), Some("default"));

        assert_eq!(entries[2].key, "max.message.bytes");
        assert_eq!(entries[2].is_sensitive, Some(true));
        assert_eq!(entries[2].source.as_deref(), Some("unknown"));
        assert!(entries[2]
            .note
            .as_deref()
            .expect("sensitive unknown entry should have note")
            .contains("敏感字段"));
        assert!(entries[2]
            .note
            .as_deref()
            .expect("sensitive unknown entry should have note")
            .contains("较旧 Kafka 版本"));
    }

    #[test]
    fn fills_missing_topic_operation_entries_with_truthful_note() {
        let config_resource = ConfigResource {
            specifier: OwnedResourceSpecifier::Topic("orders".to_string()),
            entries: vec![ConfigEntry {
                name: "cleanup.policy".to_string(),
                value: Some("compact".to_string()),
                source: ConfigSource::DynamicTopic,
                is_read_only: false,
                is_default: false,
                is_sensitive: false,
            }],
        };

        let entries = map_topic_operation_config_entries(&config_resource);
        let missing_entry = entries
            .iter()
            .find(|entry| entry.key == "retention.ms")
            .expect("retention entry should be present in allowlist response");

        assert!(!missing_entry.is_supported);
        assert!(missing_entry.value.is_none());
        assert!(missing_entry.is_read_only.is_none());
        assert!(missing_entry.is_default.is_none());
        assert!(missing_entry.is_sensitive.is_none());
        assert!(missing_entry.source.is_none());
        assert!(missing_entry
            .note
            .as_deref()
            .expect("missing entry should include note")
            .contains("broker 未返回该配置项"));
    }

    #[test]
    fn omits_unknown_metadata_fields_for_unsupported_topic_operation_entries() {
        let config_resource = ConfigResource {
            specifier: OwnedResourceSpecifier::Topic("orders".to_string()),
            entries: vec![ConfigEntry {
                name: "cleanup.policy".to_string(),
                value: Some("compact".to_string()),
                source: ConfigSource::DynamicTopic,
                is_read_only: false,
                is_default: false,
                is_sensitive: false,
            }],
        };

        let entry = map_topic_operation_config_entries(&config_resource)
            .into_iter()
            .find(|entry| entry.key == "retention.ms")
            .expect("retention entry should be present in allowlist response");

        let value = serde_json::to_value(&entry).expect("entry should serialize to json value");
        let object = value
            .as_object()
            .expect("serialized entry should be represented as object");

        assert!(!object.contains_key("isReadOnly"));
        assert!(!object.contains_key("isDefault"));
        assert!(!object.contains_key("isSensitive"));
        assert!(!object.contains_key("source"));
        assert_eq!(
            object.get("isSupported"),
            Some(&serde_json::Value::Bool(false))
        );
    }

    #[test]
    fn topic_config_update_request_requires_acknowledgement_and_known_key() {
        let error = validate_update_topic_config_request(&UpdateTopicConfigRequest {
            cluster_profile_id: "cluster-1".to_string(),
            topic_name: "orders".to_string(),
            config_key: "cleanup.policy".to_string(),
            requested_value: Some("delete".to_string()),
            expected_current_value: Some("compact".to_string()),
            risk_acknowledged: false,
        })
        .expect_err("missing acknowledgement should fail validation");

        assert_eq!(
            error.to_string(),
            "validation error: topic config update risk acknowledgement is required"
        );
    }

    #[test]
    fn topic_config_update_request_requires_requested_and_expected_values() {
        let missing_requested_value = validate_update_topic_config_request(&UpdateTopicConfigRequest {
            cluster_profile_id: "cluster-1".to_string(),
            topic_name: "orders".to_string(),
            config_key: "retention.ms".to_string(),
            requested_value: None,
            expected_current_value: Some("604800000".to_string()),
            risk_acknowledged: true,
        })
        .expect_err("missing requested value should fail validation");

        assert!(missing_requested_value
            .to_string()
            .contains("requested value for 'retention.ms' is required"));

        let missing_expected_value = validate_update_topic_config_request(&UpdateTopicConfigRequest {
            cluster_profile_id: "cluster-1".to_string(),
            topic_name: "orders".to_string(),
            config_key: "retention.ms".to_string(),
            requested_value: Some("86400000".to_string()),
            expected_current_value: None,
            risk_acknowledged: true,
        })
        .expect_err("missing expected value should fail validation");

        assert!(missing_expected_value
            .to_string()
            .contains("expected current value for 'retention.ms' is required"));
    }

    #[test]
    fn topic_config_update_value_validation_accepts_known_shapes() {
        validate_topic_config_update_value("cleanup.policy", Some("compact,delete"))
            .expect("cleanup policy should accept known values");
        validate_topic_config_update_value("retention.ms", Some("604800000"))
            .expect("retention.ms should accept integers");
        validate_topic_config_update_value("max.message.bytes", Some("1048576"))
            .expect("max.message.bytes should accept integers");
    }

    #[test]
    fn topic_config_update_value_validation_rejects_bad_shapes() {
        let cleanup_error =
            validate_topic_config_update_value("cleanup.policy", Some("compact,foo"))
                .expect_err("unknown cleanup policy value should fail");
        assert!(cleanup_error
            .to_string()
            .contains("cleanup.policy does not accept 'foo'"));

        let retention_error = validate_topic_config_update_value("retention.ms", Some("abc"))
            .expect_err("non-numeric retention.ms should fail");
        assert!(retention_error
            .to_string()
            .contains("retention.ms must be a non-negative integer"));
    }

    #[test]
    fn topic_config_update_expected_value_validation_rejects_stale_values() {
        let error = validate_topic_config_expected_current_value(
            "orders",
            "cleanup.policy",
            Some("delete"),
            "compact",
        )
        .expect_err("stale expected value should fail");

        assert!(error
            .to_string()
            .contains("expected current value \"compact\", found Some(\"delete\")"));
    }

    #[test]
    fn topic_config_update_rejects_no_op_requested_values() {
        let error = validate_topic_config_requested_change(
            "orders",
            "retention.ms",
            Some("86400000"),
            "86400000",
        )
        .expect_err("no-op update should be rejected");

        assert!(error
            .to_string()
            .contains("already matches the requested value"));
    }

    #[test]
    fn topic_partition_expansion_requires_acknowledgement_and_positive_counts() {
        let missing_acknowledgement = validate_expand_topic_partitions_request(
            &ExpandTopicPartitionsRequest {
                cluster_profile_id: "cluster-1".to_string(),
                topic_name: "orders".to_string(),
                requested_partition_count: 8,
                expected_current_partition_count: 6,
                risk_acknowledged: false,
            },
        )
        .expect_err("missing acknowledgement should fail validation");

        assert_eq!(
            missing_acknowledgement.to_string(),
            "validation error: topic partition expansion risk acknowledgement is required"
        );

        let zero_expected = validate_expand_topic_partitions_request(
            &ExpandTopicPartitionsRequest {
                cluster_profile_id: "cluster-1".to_string(),
                topic_name: "orders".to_string(),
                requested_partition_count: 8,
                expected_current_partition_count: 0,
                risk_acknowledged: true,
            },
        )
        .expect_err("zero expected partition count should fail validation");

        assert!(zero_expected
            .to_string()
            .contains("expected current partition count must be greater than zero"));
    }

    #[test]
    fn topic_partition_expansion_rejects_stale_no_op_and_shrink_counts() {
        let stale_error = validate_topic_partition_expected_count("orders", 8, 6)
            .expect_err("stale expected partition count should fail");
        assert!(stale_error
            .to_string()
            .contains("expected 6, found 8"));

        let no_op_error = validate_topic_partition_expansion_change("orders", 6, 6)
            .expect_err("same partition count should fail");
        assert!(no_op_error
            .to_string()
            .contains("requires a count greater than current count 6"));

        let shrink_error = validate_topic_partition_expansion_change("orders", 6, 4)
            .expect_err("lower partition count should fail");
        assert!(shrink_error
            .to_string()
            .contains("requested 4"));
    }

    #[test]
    fn topic_partition_expansion_stage_reports_read_only_precheck_success() {
        let stage = build_topic_partition_expansion_stage("orders", 12, None);

        assert_eq!(stage.key, "topic-partition-expansion");
        assert_eq!(stage.label, "Topic 分区扩容只读预检查");
        assert_eq!(stage.status, ValidationStatusDto::Passed);
        assert!(stage.message.contains("只读预检查"));
        assert!(stage.message.contains("12"));
        assert!(stage.message.contains("不代表已具备实际写入执行条件"));
        assert!(stage
            .detail
            .as_deref()
            .expect("success stage should include detail")
            .contains("未执行任何写操作"));
        assert!(stage
            .detail
            .as_deref()
            .expect("success stage should include detail")
            .contains("CreatePartitions"));
    }

    #[test]
    fn topic_partition_expansion_stage_warns_when_admin_runtime_is_not_ready() {
        let stage = build_topic_partition_expansion_stage(
            "orders",
            12,
            Some("failed to create Kafka admin client: missing key material"),
        );

        assert_eq!(stage.key, "topic-partition-expansion");
        assert_eq!(stage.label, "Topic 分区扩容只读预检查");
        assert_eq!(stage.status, ValidationStatusDto::Warning);
        assert!(stage.message.contains("前置检查仍不完整"));
        assert!(stage
            .detail
            .as_deref()
            .expect("warning stage should include detail")
            .contains("基线分区数为 12"));
        assert!(stage
            .detail
            .as_deref()
            .expect("warning stage should include detail")
            .contains("missing key material"));
    }

    #[test]
    fn topic_config_update_stage_reports_candidate_entries_without_claiming_write_ready() {
        let config_entries = vec![
            sample_operation_config_entry("cleanup.policy", true, Some(false)),
            sample_operation_config_entry("retention.ms", true, Some(true)),
        ];

        let stage = build_topic_config_update_stage("orders", &config_entries, None);

        assert_eq!(stage.key, "topic-config-update");
        assert_eq!(stage.label, "Topic 配置修改只读预检查");
        assert_eq!(stage.status, ValidationStatusDto::Passed);
        assert!(stage.message.contains("1 个候选配置项"));
        assert!(stage.message.contains("不代表已具备实际写入执行条件"));
        assert!(stage
            .detail
            .as_deref()
            .expect("success stage should include detail")
            .contains("cleanup.policy"));
        assert!(stage
            .detail
            .as_deref()
            .expect("success stage should include detail")
            .contains("未执行任何配置写入"));
    }

    #[test]
    fn topic_config_update_stage_warns_when_all_supported_entries_are_read_only() {
        let config_entries = vec![
            sample_operation_config_entry("cleanup.policy", true, Some(true)),
            sample_operation_config_entry("retention.ms", true, Some(true)),
        ];

        let stage = build_topic_config_update_stage("orders", &config_entries, None);

        assert_eq!(stage.key, "topic-config-update");
        assert_eq!(stage.label, "Topic 配置修改只读预检查");
        assert_eq!(stage.status, ValidationStatusDto::Warning);
        assert!(stage.message.contains("均被标记为只读"));
        assert!(stage
            .detail
            .as_deref()
            .expect("warning stage should include detail")
            .contains("2 个受支持配置项"));
    }

    #[test]
    fn topic_config_update_stage_warns_when_admin_runtime_is_not_ready() {
        let config_entries = vec![sample_operation_config_entry(
            "cleanup.policy",
            true,
            Some(false),
        )];

        let stage = build_topic_config_update_stage(
            "orders",
            &config_entries,
            Some("failed to create Kafka admin client: missing key material"),
        );

        assert_eq!(stage.key, "topic-config-update");
        assert_eq!(stage.label, "Topic 配置修改只读预检查");
        assert_eq!(stage.status, ValidationStatusDto::Warning);
        assert!(stage
            .message
            .contains("运行时尚不足以进入后续配置修改工作流"));
        assert!(stage
            .detail
            .as_deref()
            .expect("warning stage should include detail")
            .contains("missing key material"));
    }

    #[test]
    fn topic_config_update_stage_warns_when_no_supported_entries_are_available() {
        let config_entries = vec![sample_operation_config_entry("cleanup.policy", false, None)];

        let stage = build_topic_config_update_stage("orders", &config_entries, None);

        assert_eq!(stage.key, "topic-config-update");
        assert_eq!(stage.label, "Topic 配置修改只读预检查");
        assert_eq!(stage.status, ValidationStatusDto::Warning);
        assert!(stage.message.contains("缺少足够的可修改配置证据"));
        assert!(stage
            .detail
            .as_deref()
            .expect("warning stage should include detail")
            .contains("0 个受支持配置项"));
    }

    #[test]
    fn topic_tag_management_stage_warns_even_when_runtime_is_ready() {
        let stage = build_topic_tag_management_stage("orders", None);

        assert_eq!(stage.key, "topic-tag-management");
        assert_eq!(stage.label, "Topic 标签管理前置预检查");
        assert_eq!(stage.status, ValidationStatusDto::Warning);
        assert!(stage.message.contains("topic 标签持久化或读取路径"));
        assert!(stage
            .detail
            .as_deref()
            .expect("warning stage should include detail")
            .contains("topic-tag persistence/read support"));
    }

    #[test]
    fn topic_tag_management_stage_warns_when_runtime_is_not_ready() {
        let stage = build_topic_tag_management_stage(
            "orders",
            Some("failed to create Kafka admin client: missing key material"),
        );

        assert_eq!(stage.key, "topic-tag-management");
        assert_eq!(stage.label, "Topic 标签管理前置预检查");
        assert_eq!(stage.status, ValidationStatusDto::Warning);
        assert!(stage
            .message
            .contains("运行时尚不足以进入后续标签管理工作流"));
        assert!(stage
            .detail
            .as_deref()
            .expect("warning stage should include detail")
            .contains("missing key material"));
    }

    #[test]
    fn topic_operations_overview_summary_stays_warning_when_placeholder_tag_stage_exists() {
        let stages = vec![
            ValidationStageDto::passed("metadata-client", "元数据客户端", "ok"),
            ValidationStageDto::warning(
                "topic-tag-management",
                "Topic 标签管理前置预检查",
                "placeholder",
            ),
        ];

        let response = build_topic_operations_overview_response(stages, Vec::new());

        assert_eq!(response.status, ValidationStatusDto::Warning);
        assert!(response
            .message
            .contains("当前集群对只读运维预检查仍存在限制或缺失"));
        assert_eq!(response.config_entries.len(), 0);
    }

    #[test]
    fn topic_offset_reset_stage_reports_candidate_groups_without_claiming_write_ready() {
        let group_snapshots = vec![
            sample_topic_group_snapshot("orders-consumer-a", "Stable", 42, 2),
            sample_topic_group_snapshot("orders-consumer-b", "Empty", 0, 1),
        ];

        let stage = build_topic_offset_reset_stage("orders", &group_snapshots, 0);

        assert_eq!(stage.key, "topic-offset-reset");
        assert_eq!(stage.label, "Consumer Offset Reset 只读预检查");
        assert_eq!(stage.status, ValidationStatusDto::Passed);
        assert!(stage.message.contains("2 个候选消费组"));
        assert!(stage.message.contains("不代表已具备实际写入执行条件"));
        assert!(stage
            .detail
            .as_deref()
            .expect("success stage should include detail")
            .contains("未执行任何 offset 写入"));
        assert!(stage
            .detail
            .as_deref()
            .expect("success stage should include detail")
            .contains("orders-consumer-a (Stable)"));
    }

    #[test]
    fn topic_offset_reset_stage_warns_when_no_candidate_groups_are_found() {
        let stage = build_topic_offset_reset_stage("orders", &[], 0);

        assert_eq!(stage.key, "topic-offset-reset");
        assert_eq!(stage.label, "Consumer Offset Reset 只读预检查");
        assert_eq!(stage.status, ValidationStatusDto::Warning);
        assert!(stage.message.contains("未发现具备已提交位点的相关消费组"));
        assert!(stage
            .detail
            .as_deref()
            .expect("warning stage should include detail")
            .contains("不能如实判断后续 offset reset 候选范围"));
    }

    #[test]
    fn topic_offset_reset_stage_reports_partial_visibility_when_some_groups_are_skipped() {
        let group_snapshots = vec![sample_topic_group_snapshot(
            "orders-consumer-a",
            "Stable",
            42,
            2,
        )];

        let stage = build_topic_offset_reset_stage("orders", &group_snapshots, 2);

        assert_eq!(stage.status, ValidationStatusDto::Warning);
        assert!(stage
            .message
            .contains("2 个消费组因位点或水位信息不可读而被跳过"));
        assert!(stage
            .detail
            .as_deref()
            .expect("warning stage should include detail")
            .contains("2 个消费组因位点或水位信息不可读而被跳过"));
    }

    #[test]
    fn topic_offset_reset_stage_warns_when_all_groups_are_unreadable() {
        let stage = build_topic_offset_reset_stage("orders", &[], 3);

        assert_eq!(stage.key, "topic-offset-reset");
        assert_eq!(stage.label, "Consumer Offset Reset 只读预检查");
        assert_eq!(stage.status, ValidationStatusDto::Warning);
        assert!(stage.message.contains("未发现具备已提交位点的相关消费组"));
        assert!(stage
            .detail
            .as_deref()
            .expect("warning stage should include detail")
            .contains("3 个消费组因位点或水位信息不可读而被跳过"));
    }

    #[test]
    fn topic_group_partition_lag_clamps_committed_offset_to_low_watermark() {
        assert_eq!(compute_topic_group_partition_lag(100, 150, 90), 50);
        assert_eq!(compute_topic_group_partition_lag(100, 150, 120), 30);
        assert_eq!(compute_topic_group_partition_lag(100, 150, 180), 0);
    }
}
