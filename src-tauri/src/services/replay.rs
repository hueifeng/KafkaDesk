use crate::models::error::{AppError, AppResult};
use crate::models::message::MessageHeaderDto;
use crate::models::replay::{
    AuditEventRecord, CreateReplayJobRequest, ReplayJobDetailResponseDto, ReplayJobEventDto,
    ReplayJobRecord, ReplayJobSummaryDto,
};
use crate::models::replay_policy::ReplayPolicyDto;
use crate::repositories::sqlite;
use crate::services::kafka_config::{
    apply_kafka_read_consumer_config, apply_kafka_security_config,
};
use crate::services::replay_policy::ReplayPolicyService;
use chrono::Utc;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::{Header, Headers, Message, OwnedHeaders};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::topic_partition_list::{Offset, TopicPartitionList};
use rdkafka::ClientConfig;
use serde_json::json;
use sqlx::SqlitePool;
use std::{future::Future, pin::Pin, time::Duration};
use tracing::{info, warn};
use uuid::Uuid;

type ReplayExecutionFuture<'a> =
    Pin<Box<dyn Future<Output = Result<DeliveryEvidence, String>> + Send + 'a>>;
type ReplayExecutor = for<'a> fn(
    &'a crate::models::cluster::ClusterProfileDto,
    &'a ReplayPolicyDto,
    &'a CreateReplayJobRequest,
    i64,
) -> ReplayExecutionFuture<'a>;

pub struct ReplayService<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ReplayService<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn recover_stale_publishing_jobs(&self) -> AppResult<usize> {
        let candidates = sqlite::list_replay_jobs_by_status(self.pool, "publishing").await?;
        if candidates.is_empty() {
            info!("startup replay recovery found no stale publishing jobs");
            return Ok(0);
        }

        warn!(
            stale_publishing_jobs = candidates.len(),
            "startup replay recovery detected stale publishing jobs"
        );

        let recovered_at = Utc::now().to_rfc3339();
        for candidate in &candidates {
            let result_summary = json!({
                "mode": "broker-delivery",
                "executionStage": "delivery_unknown",
                "deliveryConfirmed": false,
                "note": "job remained in publishing when app started; broker delivery outcome is unknown"
            })
            .to_string();
            let error_message =
                "replay was interrupted during broker delivery; outcome is unknown".to_string();

            sqlite::update_replay_job_execution(
                self.pool,
                &candidate.id,
                "delivery_unknown",
                candidate.started_at.as_deref(),
                Some(&recovered_at),
                Some(&error_message),
                Some(&result_summary),
            )
            .await?;

            let event_payload = json!({
                "executionStage": "delivery_unknown",
                "deliveryConfirmed": false,
                "recoveredAt": recovered_at,
                "note": "status repaired during startup recovery"
            })
            .to_string();
            sqlite::insert_replay_job_event(
                self.pool,
                &Uuid::new_v4().to_string(),
                &candidate.id,
                "delivery_unknown_recovered",
                Some(&event_payload),
                &recovered_at,
            )
            .await?;

            sqlite::insert_audit_event(
                self.pool,
                &AuditEventRecord {
                    id: Uuid::new_v4().to_string(),
                    event_type: "replay_delivery_unknown_recovered".to_string(),
                    target_type: "replay_job".to_string(),
                    target_ref: Some(candidate.id.clone()),
                    actor_profile: Some(candidate.cluster_profile_id.clone()),
                    cluster_profile_id: Some(candidate.cluster_profile_id.clone()),
                    outcome: "delivery_unknown".to_string(),
                    summary: format!(
                        "Replay delivery outcome is unknown after restart for topic '{}'",
                        candidate.target_topic
                    ),
                    details_json: Some(
                        json!({
                            "replayJobId": candidate.id,
                            "executionStage": "delivery_unknown",
                            "deliveryConfirmed": false,
                            "recoveredAt": recovered_at,
                        })
                        .to_string(),
                    ),
                    created_at: recovered_at.clone(),
                },
            )
            .await?;
        }

        info!(
            recovered_delivery_unknown_jobs = candidates.len(),
            "startup replay recovery marked stale publishing jobs as delivery_unknown"
        );

        Ok(candidates.len())
    }

    pub async fn create_replay_job(
        &self,
        request: CreateReplayJobRequest,
    ) -> AppResult<ReplayJobDetailResponseDto> {
        self.create_replay_job_with_executor(request, execute_live_replay_boxed)
            .await
    }

    async fn create_replay_job_with_executor(
        &self,
        request: CreateReplayJobRequest,
        execute_live_replay_fn: ReplayExecutor,
    ) -> AppResult<ReplayJobDetailResponseDto> {
        validate_create_replay_job_request(&request)?;
        let replay_policy = ReplayPolicyService::new(self.pool)
            .get_replay_policy()
            .await?;
        validate_replay_job_against_policy(&request, &replay_policy)?;

        let source_offset = request
            .source_message_ref
            .offset
            .parse::<i64>()
            .map_err(|_| {
                AppError::Validation("source message offset must be a valid integer".to_string())
            })?;

        let cluster_profile =
            sqlite::get_cluster_profile(self.pool, &request.cluster_profile_id).await?;

        if !request.dry_run {
            validate_live_replay_profile(&cluster_profile)?;
        }

        let now = Utc::now().to_rfc3339();
        let job_id = Uuid::new_v4().to_string();
        let risk_level = if request.dry_run { "low" } else { "high" }.to_string();
        let initial_status = if request.dry_run {
            "validated"
        } else {
            "accepted"
        }
        .to_string();
        let mode = if request.dry_run {
            "dry-run"
        } else {
            "broker-delivery"
        }
        .to_string();
        let result_summary_json = if request.dry_run {
            Some(
                json!({
                    "mode": "dry-run",
                    "executionStage": "completed_local_validation",
                    "deliveryConfirmed": false,
                    "note": "validated and stored locally; no broker write executed"
                })
                .to_string(),
            )
        } else {
            Some(
                json!({
                    "mode": "broker-delivery",
                    "executionStage": "accepted",
                    "deliveryConfirmed": false,
                    "note": "replay request accepted; broker delivery has not started yet"
                })
                .to_string(),
            )
        };

        let record = ReplayJobRecord {
            id: job_id.clone(),
            cluster_profile_id: request.cluster_profile_id.clone(),
            source_topic: request.source_message_ref.topic.clone(),
            source_partition: request.source_message_ref.partition,
            source_offset: request
                .source_message_ref
                .offset
                .parse::<i64>()
                .map_err(|_| {
                    AppError::Validation(
                        "source message offset must be a valid integer".to_string(),
                    )
                })?,
            source_timestamp: request.source_timestamp.clone(),
            target_topic: request.target_topic.clone(),
            status: initial_status.clone(),
            mode,
            payload_edit_json: request
                .edited_payload
                .clone()
                .map(|value| json!({ "payload": value }).to_string()),
            headers_edit_json: request
                .edited_headers
                .clone()
                .map(|value| serde_json::to_string(&value).unwrap_or_default()),
            key_edit_json: request
                .edited_key
                .clone()
                .map(|value| json!({ "key": value }).to_string()),
            dry_run: request.dry_run,
            requested_by_profile: Some(request.cluster_profile_id.clone()),
            risk_level: risk_level.clone(),
            created_at: now.clone(),
            started_at: None,
            completed_at: if request.dry_run {
                Some(now.clone())
            } else {
                None
            },
            error_message: None,
            result_summary_json: result_summary_json.clone(),
        };

        sqlite::insert_replay_job(self.pool, &record).await?;

        let validated_event = ReplayJobEventDto {
            id: Uuid::new_v4().to_string(),
            event_type: "validated".to_string(),
            event_payload_json: Some(
                json!({
                    "targetTopic": request.target_topic,
                    "dryRun": request.dry_run,
                    "riskAcknowledged": request.risk_acknowledged,
                    "sourceTimestamp": request.source_timestamp,
                    "executionStage": if request.dry_run { "completed_local_validation" } else { "accepted" }
                })
                .to_string(),
            ),
            created_at: now.clone(),
        };
        sqlite::insert_replay_job_event(
            self.pool,
            &validated_event.id,
            &job_id,
            &validated_event.event_type,
            validated_event.event_payload_json.as_deref(),
            &validated_event.created_at,
        )
        .await?;

        let lifecycle_event = ReplayJobEventDto {
            id: Uuid::new_v4().to_string(),
            event_type: if request.dry_run {
                "local_validation_completed"
            } else {
                "accepted"
            }
            .to_string(),
            event_payload_json: result_summary_json.clone(),
            created_at: now.clone(),
        };
        sqlite::insert_replay_job_event(
            self.pool,
            &lifecycle_event.id,
            &job_id,
            &lifecycle_event.event_type,
            lifecycle_event.event_payload_json.as_deref(),
            &lifecycle_event.created_at,
        )
        .await?;

        let audit_id = Uuid::new_v4().to_string();
        sqlite::insert_audit_event(
            self.pool,
            &AuditEventRecord {
                id: audit_id.clone(),
                event_type: "replay_job_created".to_string(),
                target_type: "replay_job".to_string(),
                target_ref: Some(job_id.clone()),
                actor_profile: Some(request.cluster_profile_id.clone()),
                cluster_profile_id: Some(request.cluster_profile_id.clone()),
                outcome: if request.dry_run { "validated" } else { "accepted" }.to_string(),
                summary: if request.dry_run {
                    format!("Dry run replay validated locally for target topic '{}'", record.target_topic)
                } else {
                    format!("Replay request accepted for broker delivery to topic '{}'", record.target_topic)
                },
                details_json: Some(
                    json!({
                        "replayJobId": job_id,
                        "source": {
                            "topic": record.source_topic,
                            "partition": record.source_partition,
                            "offset": record.source_offset
                        },
                        "targetTopic": record.target_topic,
                        "dryRun": request.dry_run,
                        "executionStage": if request.dry_run { "completed_local_validation" } else { "accepted" },
                        "deliveryConfirmed": false
                    })
                    .to_string(),
                ),
                created_at: now.clone(),
            },
        )
        .await?;

        if !request.dry_run {
            let started_at = Utc::now().to_rfc3339();
            sqlite::update_replay_job_execution(
                self.pool,
                &job_id,
                "publishing",
                Some(&started_at),
                None,
                None,
                Some(
                    &json!({
                        "mode": "broker-delivery",
                        "executionStage": "publishing",
                        "deliveryConfirmed": false,
                        "note": "broker delivery is in progress"
                    })
                    .to_string(),
                ),
            )
            .await?;

            let publishing_event_payload = json!({
                "executionStage": "publishing",
                "startedAt": started_at,
                "targetTopic": request.target_topic
            })
            .to_string();
            sqlite::insert_replay_job_event(
                self.pool,
                &Uuid::new_v4().to_string(),
                &job_id,
                "publishing",
                Some(&publishing_event_payload),
                &started_at,
            )
            .await?;

            let execution =
                execute_live_replay_fn(&cluster_profile, &replay_policy, &request, source_offset)
                    .await;
            let completed_at = Utc::now().to_rfc3339();

            match execution {
                Ok(delivery) => {
                    let result_summary = json!({
                        "mode": "broker-delivery",
                        "executionStage": "delivered",
                        "deliveryConfirmed": true,
                        "delivery": {
                            "partition": delivery.partition,
                            "offset": delivery.offset,
                            "timestamp": delivery.timestamp,
                        },
                        "note": "broker acknowledged replay delivery"
                    })
                    .to_string();

                    sqlite::update_replay_job_execution(
                        self.pool,
                        &job_id,
                        "delivered",
                        Some(&started_at),
                        Some(&completed_at),
                        None,
                        Some(&result_summary),
                    )
                    .await?;

                    let delivered_event_payload = json!({
                        "executionStage": "delivered",
                        "deliveryConfirmed": true,
                        "delivery": {
                            "partition": delivery.partition,
                            "offset": delivery.offset,
                            "timestamp": delivery.timestamp,
                        }
                    })
                    .to_string();
                    sqlite::insert_replay_job_event(
                        self.pool,
                        &Uuid::new_v4().to_string(),
                        &job_id,
                        "delivery_confirmed",
                        Some(&delivered_event_payload),
                        &completed_at,
                    )
                    .await?;

                    sqlite::insert_audit_event(
                        self.pool,
                        &AuditEventRecord {
                            id: Uuid::new_v4().to_string(),
                            event_type: "replay_delivery_confirmed".to_string(),
                            target_type: "replay_job".to_string(),
                            target_ref: Some(job_id.clone()),
                            actor_profile: Some(request.cluster_profile_id.clone()),
                            cluster_profile_id: Some(request.cluster_profile_id.clone()),
                            outcome: "delivered".to_string(),
                            summary: format!(
                                "Replay delivered to '{}' with broker ack at partition {} offset {}",
                                request.target_topic, delivery.partition, delivery.offset
                            ),
                            details_json: Some(
                                json!({
                                    "replayJobId": job_id,
                                    "executionStage": "delivered",
                                    "deliveryConfirmed": true,
                                    "delivery": {
                                        "partition": delivery.partition,
                                        "offset": delivery.offset,
                                        "timestamp": delivery.timestamp,
                                    }
                                })
                                .to_string(),
                            ),
                            created_at: completed_at.clone(),
                        },
                    )
                    .await?;
                }
                Err(error_message) => {
                    let result_summary = json!({
                        "mode": "broker-delivery",
                        "executionStage": "failed",
                        "deliveryConfirmed": false,
                        "error": error_message,
                        "note": "broker delivery failed"
                    })
                    .to_string();

                    sqlite::update_replay_job_execution(
                        self.pool,
                        &job_id,
                        "failed",
                        Some(&started_at),
                        Some(&completed_at),
                        Some(&error_message),
                        Some(&result_summary),
                    )
                    .await?;

                    let failed_event_payload = json!({
                        "executionStage": "failed",
                        "deliveryConfirmed": false,
                        "error": error_message,
                    })
                    .to_string();
                    sqlite::insert_replay_job_event(
                        self.pool,
                        &Uuid::new_v4().to_string(),
                        &job_id,
                        "delivery_failed",
                        Some(&failed_event_payload),
                        &completed_at,
                    )
                    .await?;

                    sqlite::insert_audit_event(
                        self.pool,
                        &AuditEventRecord {
                            id: Uuid::new_v4().to_string(),
                            event_type: "replay_delivery_failed".to_string(),
                            target_type: "replay_job".to_string(),
                            target_ref: Some(job_id.clone()),
                            actor_profile: Some(request.cluster_profile_id.clone()),
                            cluster_profile_id: Some(request.cluster_profile_id.clone()),
                            outcome: "failed".to_string(),
                            summary: format!(
                                "Replay delivery failed for topic '{}': {}",
                                request.target_topic, error_message
                            ),
                            details_json: Some(
                                json!({
                                    "replayJobId": job_id,
                                    "executionStage": "failed",
                                    "deliveryConfirmed": false,
                                    "error": error_message,
                                })
                                .to_string(),
                            ),
                            created_at: completed_at.clone(),
                        },
                    )
                    .await?;
                }
            }
        }

        let job = sqlite::get_replay_job(self.pool, &job_id).await?;
        let event_history = sqlite::list_replay_job_events(self.pool, &job_id).await?;
        let audit_ref =
            sqlite::find_latest_audit_ref_for_target(self.pool, "replay_job", &job_id).await?;

        Ok(ReplayJobDetailResponseDto {
            job,
            event_history,
            audit_ref,
        })
    }

    pub async fn list_replay_jobs(
        &self,
        cluster_profile_id: &str,
    ) -> AppResult<Vec<ReplayJobSummaryDto>> {
        if cluster_profile_id.trim().is_empty() {
            return Err(AppError::Validation(
                "cluster profile id is required".to_string(),
            ));
        }

        sqlite::list_replay_jobs(self.pool, cluster_profile_id).await
    }

    pub async fn get_replay_job(&self, id: &str) -> AppResult<ReplayJobDetailResponseDto> {
        if id.trim().is_empty() {
            return Err(AppError::Validation(
                "replay job id is required".to_string(),
            ));
        }

        let job = sqlite::get_replay_job(self.pool, id).await?;
        let event_history = sqlite::list_replay_job_events(self.pool, id).await?;
        let audit_ref =
            sqlite::find_latest_audit_ref_for_target(self.pool, "replay_job", id).await?;

        Ok(ReplayJobDetailResponseDto {
            job,
            event_history,
            audit_ref,
        })
    }
}

struct ReplaySourceMessage {
    timestamp: String,
    key: Option<Vec<u8>>,
    payload: Option<Vec<u8>>,
    headers: Vec<MessageHeaderDto>,
}

struct DeliveryEvidence {
    partition: i32,
    offset: i64,
    timestamp: Option<i64>,
}

async fn execute_live_replay(
    profile: &crate::models::cluster::ClusterProfileDto,
    policy: &ReplayPolicyDto,
    request: &CreateReplayJobRequest,
    source_offset: i64,
) -> Result<DeliveryEvidence, String> {
    let source = load_source_message_for_replay(
        profile,
        &request.source_message_ref.topic,
        request.source_message_ref.partition,
        source_offset,
    )
    .map_err(|error| format!("failed to load replay source message: {error}"))?;

    let effective_payload = request
        .edited_payload
        .as_ref()
        .map(|value| value.as_bytes().to_vec())
        .or(source.payload);
    let effective_key = request
        .edited_key
        .as_ref()
        .map(|value| value.as_bytes().to_vec())
        .or(source.key);
    let effective_headers = request.edited_headers.clone().unwrap_or(source.headers);

    let producer = build_replay_producer(profile, policy.delivery_timeout_seconds)
        .map_err(|error| format!("failed to build replay producer: {error}"))?;

    let retry_attempts = policy.max_retry_attempts.max(1);
    let mut last_error = None;
    for attempt in 1..=retry_attempts {
        let send_record = rebuild_record(
            &request.target_topic,
            effective_payload.as_deref(),
            effective_key.as_deref(),
            &effective_headers,
        );
        match producer.send(send_record, Duration::from_secs(0)).await {
            Ok(delivery) => {
                return Ok(DeliveryEvidence {
                    partition: delivery.partition,
                    offset: delivery.offset,
                    timestamp: match delivery.timestamp {
                        rdkafka::message::Timestamp::NotAvailable => None,
                        rdkafka::message::Timestamp::CreateTime(value) => Some(value),
                        rdkafka::message::Timestamp::LogAppendTime(value) => Some(value),
                    },
                })
            }
            Err((error, _owned_message)) => {
                last_error = Some(format!(
                    "attempt {attempt}/{retry_attempts} broker delivery failed: {error}; sourceTimestamp={}",
                    source.timestamp
                ));
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "broker delivery failed for an unknown reason".to_string()))
}

fn execute_live_replay_boxed<'a>(
    profile: &'a crate::models::cluster::ClusterProfileDto,
    policy: &'a ReplayPolicyDto,
    request: &'a CreateReplayJobRequest,
    source_offset: i64,
) -> ReplayExecutionFuture<'a> {
    Box::pin(execute_live_replay(profile, policy, request, source_offset))
}

fn build_replay_producer(
    profile: &crate::models::cluster::ClusterProfileDto,
    delivery_timeout_seconds: u64,
) -> AppResult<FutureProducer> {
    let delivery_timeout_ms = delivery_timeout_seconds.saturating_mul(1000).to_string();
    let mut config = ClientConfig::new();
    config
        .set("bootstrap.servers", &profile.bootstrap_servers)
        .set("message.timeout.ms", &delivery_timeout_ms)
        .set("delivery.timeout.ms", &delivery_timeout_ms)
        .set("acks", "all");

    apply_kafka_security_config(&mut config, profile)?;

    config
        .create()
        .map_err(|error| AppError::Network(format!("failed to create replay producer: {error}")))
}

fn rebuild_record<'a>(
    target_topic: &'a str,
    payload: Option<&'a [u8]>,
    key: Option<&'a [u8]>,
    headers: &'a [MessageHeaderDto],
) -> FutureRecord<'a, [u8], [u8]> {
    let mut record = FutureRecord::to(target_topic);
    if let Some(payload) = payload {
        record = record.payload(payload);
    }
    if let Some(key) = key {
        record = record.key(key);
    }
    if !headers.is_empty() {
        let mut owned_headers = OwnedHeaders::new();
        for header in headers {
            owned_headers = owned_headers.insert(Header {
                key: &header.key,
                value: Some(header.value.as_str()),
            });
        }
        record = record.headers(owned_headers);
    }
    record
}

fn load_source_message_for_replay(
    profile: &crate::models::cluster::ClusterProfileDto,
    topic: &str,
    partition: i32,
    target_offset: i64,
) -> AppResult<ReplaySourceMessage> {
    let mut config = ClientConfig::new();
    apply_kafka_read_consumer_config(&mut config, profile)?;

    let consumer: BaseConsumer = config.create().map_err(|error| {
        AppError::Network(format!("failed to create replay source consumer: {error}"))
    })?;

    let mut assignment = TopicPartitionList::new();
    assignment
        .add_partition_offset(topic, partition, Offset::Offset(target_offset))
        .map_err(|error| {
            AppError::Internal(format!("failed to assign replay source partition: {error}"))
        })?;

    consumer.assign(&assignment).map_err(|error| {
        AppError::Network(format!("failed to assign replay source consumer: {error}"))
    })?;

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        match consumer.poll(Duration::from_millis(200)) {
            Some(Ok(message)) => {
                if message.topic() != topic || message.partition() != partition {
                    continue;
                }
                if message.offset() < target_offset {
                    continue;
                }
                if message.offset() > target_offset {
                    return Err(AppError::NotFound(format!(
                        "source message '{} / {} / {}' was not found",
                        topic, partition, target_offset
                    )));
                }

                let timestamp = match message.timestamp() {
                    rdkafka::message::Timestamp::NotAvailable => String::new(),
                    rdkafka::message::Timestamp::CreateTime(value)
                    | rdkafka::message::Timestamp::LogAppendTime(value) => value.to_string(),
                };

                let mut headers = Vec::new();
                if let Some(message_headers) = message.headers() {
                    for index in 0..message_headers.count() {
                        let header = message_headers.get(index);
                        headers.push(MessageHeaderDto {
                            key: header.key.to_string(),
                            value: header
                                .value
                                .map(|value| String::from_utf8_lossy(value).to_string())
                                .unwrap_or_default(),
                        });
                    }
                }

                return Ok(ReplaySourceMessage {
                    timestamp,
                    key: message.key().map(|value| value.to_vec()),
                    payload: message.payload().map(|value| value.to_vec()),
                    headers,
                });
            }
            Some(Err(_)) | None => {}
        }
    }

    Err(AppError::NotFound(format!(
        "source message '{} / {} / {}' was not found within replay source window",
        topic, partition, target_offset
    )))
}

fn validate_live_replay_profile(
    profile: &crate::models::cluster::ClusterProfileDto,
) -> AppResult<()> {
    let mut config = ClientConfig::new();
    apply_kafka_security_config(&mut config, profile)?;
    Ok(())
}

fn validate_create_replay_job_request(request: &CreateReplayJobRequest) -> AppResult<()> {
    if request.cluster_profile_id.trim().is_empty() {
        return Err(AppError::Validation(
            "cluster profile id is required".to_string(),
        ));
    }
    if request.source_message_ref.topic.trim().is_empty() {
        return Err(AppError::Validation("source topic is required".to_string()));
    }
    if request.source_message_ref.offset.trim().is_empty() {
        return Err(AppError::Validation(
            "source offset is required".to_string(),
        ));
    }
    if request.target_topic.trim().is_empty() {
        return Err(AppError::Validation(
            "target topic must be explicit".to_string(),
        ));
    }
    if !request.dry_run && !request.risk_acknowledged {
        return Err(AppError::Validation(
            "risk acknowledgement required for broker delivery replay".to_string(),
        ));
    }

    Ok(())
}

fn validate_replay_job_against_policy(
    request: &CreateReplayJobRequest,
    policy: &ReplayPolicyDto,
) -> AppResult<()> {
    if !request.dry_run && !policy.allow_live_replay {
        return Err(AppError::Validation(
            "current replay policy blocks broker delivery replay beyond dry run".to_string(),
        ));
    }

    if !request.dry_run && policy.require_risk_acknowledgement && !request.risk_acknowledged {
        return Err(AppError::Validation(
            "current replay policy requires explicit risk acknowledgement".to_string(),
        ));
    }

    if !request.dry_run
        && policy.sandbox_only
        && !request
            .target_topic
            .starts_with(&policy.sandbox_topic_prefix)
    {
        return Err(AppError::Validation(format!(
            "current replay policy only allows broker delivery replay to topics starting with '{}'",
            policy.sandbox_topic_prefix
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        build_replay_producer, validate_live_replay_profile, DeliveryEvidence,
        ReplayExecutionFuture, ReplayService,
    };
    use crate::models::audit::ListAuditEventsRequest;
    use crate::models::cluster::ClusterProfileDto;
    use crate::models::replay::CreateReplayJobRequest;
    use crate::models::replay::ReplayJobRecord;
    use crate::models::replay_policy::ReplayPolicyDto;
    use crate::repositories::sqlite;
    use sqlx::SqlitePool;
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    const TEST_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----\nMIICsDCCAZgCCQD4vKEpwC7YOjANBgkqhkiG9w0BAQsFADAaMRgwFgYDVQQDDA90\ncmFjZWZvcmdlLXRlc3QwHhcNMjYwNDE4MDczMTU0WhcNMjcwNDE4MDczMTU0WjAa\nMRgwFgYDVQQDDA90cmFjZWZvcmdlLXRlc3QwggEiMA0GCSqGSIb3DQEBAQUAA4IB\nDwAwggEKAoIBAQDB3uz/2t6m3/qJaENMNk1KW6L0nE6Sr6a16C9alMoNk53q8Glx\nQA7shqJLUj8AHpbMMMddlc/RXz4cYXKLMq5XH2YVJ+QKxjcXR0Q6Swu3cqoMO7n8\nfiOiA9biPR3/78KSEPd9zdqOs3DwMHr65v3oEI9tc1rWvam9Xz+4MrkBFbmF4wZR\nYZ4ac+lgB9ie1FL4S4cRHYqYzvumNdMDtFv8hDCmyPdKUlfEowxtueuQ5WGB82yP\nGd0LlYOvxt79MiR0sIQsxVnSY5F0nLcNW9Z+bCuGgEWchakiUb9Nj2BcJ88IAGxs\npitqC3v+noeQA4SWkc+7Byd4dZS4rLGYv8FhAgMBAAEwDQYJKoZIhvcNAQELBQAD\nggEBACk9WDt7D7thZkoT8VJkyukWx4uPGXczOfp0+hu2eP1TODurSQziwVj3xF3O\noSjN8HrWg3U0vGqZGgqIPxPknbmwk5fjVorwWelRlX2X7DMElsFeRMZSY9leLC10\ntqdEu8mIJsGzR/Aua56fo3dywhIglYG/8O0tcZYjdp6YczXWW64lPz2vVv+9ZVVj\nnVrKYbU118mkVhd7jmV9QR5KdBY1th6qVEzI340S7CQ2PdweT0kemFwBTCp5gvJ5\na3Xi8pKrQKJk/L2O6oxhXOCCGvWhdEvZ8mel2Qp/whg6MupIciDKozdf68yECrUW\nEhr3a4kltLXboZZ+DJx+KZCTRv0=\n-----END CERTIFICATE-----\n";
    const TEST_PRIVATE_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQDB3uz/2t6m3/qJ\naENMNk1KW6L0nE6Sr6a16C9alMoNk53q8GlxQA7shqJLUj8AHpbMMMddlc/RXz4c\nYXKLMq5XH2YVJ+QKxjcXR0Q6Swu3cqoMO7n8fiOiA9biPR3/78KSEPd9zdqOs3Dw\nMHr65v3oEI9tc1rWvam9Xz+4MrkBFbmF4wZRYZ4ac+lgB9ie1FL4S4cRHYqYzvum\nNdMDtFv8hDCmyPdKUlfEowxtueuQ5WGB82yPGd0LlYOvxt79MiR0sIQsxVnSY5F0\nnLcNW9Z+bCuGgEWchakiUb9Nj2BcJ88IAGxspitqC3v+noeQA4SWkc+7Byd4dZS4\nrLGYv8FhAgMBAAECggEAU/ogZuODtn0mpQaIwCZ1bFQtTg+26Us0x27/tBjnPOJI\ncVAaHHhG/qWC/2Vs7LxTTbeDZEJUdrjuypRbbXhSlGKBcYCKAzDBBFBWeXmwZZJB\nnnLwqTJqdO980RrwN8C/Y03+JnxYw59uuFwHqU8NhMxHlH13R7V4JRNxInS2NoVV\nXVfgcTjax8pdbdzKKwIn3AUk27SwSJwBlYuMKgDq741/L8PvyjOmolvzwM+aF2FO\n1gqb3xZKM1867psYo4Z09qdc8GyG+joPEbJW9rQW2nORUy1mqApXmO8qprt+K3yY\nhWQUjYFpngx7OKOv0RhRSwzm9swK03QbLKK5i5/IwQKBgQDokMmZxVKbnpTlI8Fl\n6H2pQAIPi1HPdTTMapxBlP5CsLgtkBiYu60LYevmAcSdkzbpVr7uqWxy7+Z0upao\n7kheHaqovcO8xm1n1BDOnEgnmFJ9wLFDi8qG9EQd4dusvJWb6u3dvvWVWQFh4Pz4\nZPKxGbfa7VHeFSk9wizXuCGQpQKBgQDVZ/nhoCOIzwF9bxR9Gko6fdeLf6ZTK0ht\nMJZ8kbDlYoSjRWjX6zXdoLL7mQ2y7avQ8mEYVyOcBCb5xGa43cF6eKy6W59lXQqI\nB1Z64gaKAsFgqbhpJRo4sfEsft9pip/oI49Y9B6vP9do9UbfBg4ZZm5fZdS5+MnY\nWE70VTB1DQKBgQCzQh7SkuD4qIRWFnhUp55sXbT47Ecz5EC9K5OjjUdqejKMlBwR\nZd+c/W5KDKTTXIyf0Mg8x4SbF0UIRmYocfp/6NgJVrPQBxZ/SFtoFdgcBPHYkjVQ\nPijuWstCSTv86iNbWfrcx/sdkcxZ+ISkpZLXZV5sti47Qw5V1xyfbgMZLQKBgQCq\nI92LTvtFpZSQhrEVFJK9k3r3kuvuPwHdW/F+m0EngKYy7bGrA7HMYsSP5vSPBQII\n8lUK7N5NEtpoI3eqR9JrbC553XZ1f/pXfVIrYmzIN24pPObznUsMjIG1cel44baf\ng0pUJz0Xh5Sb74FzagZvpcS1diBlrL5wJ+e60PhzOQKBgQCy9K28nvKeC+JO40ST\ngp5YqlQnRNaWfXHTwbeeYygYfgiY+lw7hmtInnLcu7s3uVepoWpJJPGeVG7MiW+1\nfN3BpyGmBc+fA8UbUl2RHZKvKaLTWGwujYqwbfSOtI56FpNBHfjy4HyAvzAHwB7a\nlHc4mf+dH3zMIvjxLn2jsGjjFw==\n-----END PRIVATE KEY-----\n";

    async fn create_test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite should be available");

        sqlx::query(
            r#"
            CREATE TABLE cluster_profiles (
              id TEXT PRIMARY KEY NOT NULL,
              name TEXT NOT NULL,
              environment TEXT NOT NULL,
              bootstrap_servers TEXT NOT NULL,
              auth_mode TEXT NOT NULL,
              auth_credential_ref TEXT NULL,
              tls_mode TEXT NOT NULL,
              tls_ca_cert_path TEXT NULL,
              tls_client_cert_path TEXT NULL,
              tls_client_key_path TEXT NULL,
              schema_registry_profile_id TEXT NULL,
              notes TEXT NULL,
              tags_json TEXT NOT NULL DEFAULT '[]',
              is_favorite INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              last_connected_at TEXT NULL,
              is_archived INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("cluster_profiles table should be created");

        sqlx::query(
            r#"
            CREATE TABLE app_preferences (
              key TEXT PRIMARY KEY NOT NULL,
              value_json TEXT NOT NULL,
              updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("app_preferences table should be created");

        sqlx::query(
            r#"
            CREATE TABLE replay_jobs (
              id TEXT PRIMARY KEY NOT NULL,
              cluster_profile_id TEXT NOT NULL,
              source_topic TEXT NOT NULL,
              source_partition INTEGER NOT NULL,
              source_offset INTEGER NOT NULL,
              source_timestamp TEXT NULL,
              target_topic TEXT NOT NULL,
              status TEXT NOT NULL,
              mode TEXT NOT NULL,
              payload_edit_json TEXT NULL,
              headers_edit_json TEXT NULL,
              key_edit_json TEXT NULL,
              dry_run INTEGER NOT NULL DEFAULT 0,
              requested_by_profile TEXT NULL,
              risk_level TEXT NOT NULL,
              created_at TEXT NOT NULL,
              started_at TEXT NULL,
              completed_at TEXT NULL,
              error_message TEXT NULL,
              result_summary_json TEXT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("replay_jobs table should be created");

        sqlx::query(
            r#"
            CREATE TABLE replay_job_events (
              id TEXT PRIMARY KEY NOT NULL,
              replay_job_id TEXT NOT NULL,
              event_type TEXT NOT NULL,
              event_payload_json TEXT NULL,
              created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("replay_job_events table should be created");

        sqlx::query(
            r#"
            CREATE TABLE audit_events (
              id TEXT PRIMARY KEY NOT NULL,
              event_type TEXT NOT NULL,
              target_type TEXT NOT NULL,
              target_ref TEXT NULL,
              actor_profile TEXT NULL,
              cluster_profile_id TEXT NULL,
              outcome TEXT NOT NULL,
              summary TEXT NOT NULL,
              details_json TEXT NULL,
              created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("audit_events table should be created");

        pool
    }

    async fn seed_default_replay_policy(pool: &SqlitePool) {
        sqlite::seed_default_preferences(pool, "2026-04-12T00:00:00Z")
            .await
            .expect("default preferences should seed");
    }

    async fn seed_cluster_profile(
        pool: &SqlitePool,
        auth_mode: &str,
        cert_paths: Option<(&str, &str, &str)>,
    ) {
        let (tls_mode, tls_ca_cert_path, tls_client_cert_path, tls_client_key_path) =
            match (auth_mode, cert_paths) {
                ("mtls", Some((ca, cert, key))) => (
                    "tls-required".to_string(),
                    Some(ca.to_string()),
                    Some(cert.to_string()),
                    Some(key.to_string()),
                ),
                _ => ("system-default".to_string(), None, None, None),
            };

        sqlite::insert_cluster_profile(
            pool,
            &ClusterProfileDto {
                id: "cluster-1".to_string(),
                name: "Cluster One".to_string(),
                environment: "dev".to_string(),
                bootstrap_servers: "localhost:9092".to_string(),
                auth_mode: auth_mode.to_string(),
                auth_credential_ref: None,
                tls_mode,
                tls_ca_cert_path,
                tls_client_cert_path,
                tls_client_key_path,
                schema_registry_profile_id: None,
                notes: None,
                tags: vec![],
                is_favorite: false,
                created_at: "2026-04-12T00:00:00Z".to_string(),
                updated_at: "2026-04-12T00:00:00Z".to_string(),
                last_connected_at: None,
                is_archived: false,
            },
        )
        .await
        .expect("cluster profile should seed");
    }

    fn sample_request(target_topic: &str) -> CreateReplayJobRequest {
        CreateReplayJobRequest {
            cluster_profile_id: "cluster-1".to_string(),
            source_message_ref: crate::models::message::MessageRefDto {
                cluster_profile_id: "cluster-1".to_string(),
                topic: "orders.events".to_string(),
                partition: 0,
                offset: "42".to_string(),
            },
            source_timestamp: Some("2026-04-12T00:00:00Z".to_string()),
            target_topic: target_topic.to_string(),
            edited_key: Some("order-42".to_string()),
            edited_headers: Some(vec![crate::models::message::MessageHeaderDto {
                key: "x-trace-id".to_string(),
                value: "trace-42".to_string(),
            }]),
            edited_payload: Some("{\"status\":\"replayed\"}".to_string()),
            dry_run: false,
            risk_acknowledged: true,
        }
    }

    fn success_executor<'a>(
        _profile: &'a ClusterProfileDto,
        _policy: &'a ReplayPolicyDto,
        _request: &'a CreateReplayJobRequest,
        _source_offset: i64,
    ) -> ReplayExecutionFuture<'a> {
        Box::pin(async {
            Ok(DeliveryEvidence {
                partition: 2,
                offset: 108,
                timestamp: Some(1_744_412_800_000),
            })
        })
    }

    fn failure_executor<'a>(
        _profile: &'a ClusterProfileDto,
        _policy: &'a ReplayPolicyDto,
        _request: &'a CreateReplayJobRequest,
        _source_offset: i64,
    ) -> ReplayExecutionFuture<'a> {
        Box::pin(async {
            Err("attempt 1/1 broker delivery failed: broker unavailable".to_string())
        })
    }

    fn missing_source_executor<'a>(
        _profile: &'a ClusterProfileDto,
        _policy: &'a ReplayPolicyDto,
        _request: &'a CreateReplayJobRequest,
        _source_offset: i64,
    ) -> ReplayExecutionFuture<'a> {
        Box::pin(async {
            Err("failed to load replay source message: not found: source message 'orders.events / 0 / 42' was not found".to_string())
        })
    }

    fn create_temp_file(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("traceforge-{name}-{}.pem", Uuid::new_v4()));
        fs::write(&path, contents).expect("temp cert material should write");
        path
    }

    #[test]
    fn validate_live_replay_profile_accepts_existing_mtls_material() {
        let ca_path = create_temp_file("replay-helper-ca", TEST_CERT_PEM);
        let cert_path = create_temp_file("replay-helper-cert", TEST_CERT_PEM);
        let key_path = create_temp_file("replay-helper-key", TEST_PRIVATE_KEY_PEM);

        let profile = ClusterProfileDto {
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
        };

        validate_live_replay_profile(&profile)
            .expect("replay profile validation should use shared helper");

        let _ = fs::remove_file(ca_path);
        let _ = fs::remove_file(cert_path);
        let _ = fs::remove_file(key_path);
    }

    #[test]
    fn build_replay_producer_supports_mtls_cluster_profile() {
        let ca_path = create_temp_file("replay-producer-ca", TEST_CERT_PEM);
        let cert_path = create_temp_file("replay-producer-cert", TEST_CERT_PEM);
        let key_path = create_temp_file("replay-producer-key", TEST_PRIVATE_KEY_PEM);

        let profile = ClusterProfileDto {
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
        };

        let producer = build_replay_producer(&profile, 10)
            .expect("replay producer should build for mTLS profile");
        drop(producer);

        let _ = fs::remove_file(ca_path);
        let _ = fs::remove_file(cert_path);
        let _ = fs::remove_file(key_path);
    }

    #[tokio::test]
    async fn recovers_stale_publishing_jobs_to_delivery_unknown() {
        let pool = create_test_pool().await;

        sqlite::insert_replay_job(
            &pool,
            &ReplayJobRecord {
                id: "job-1".to_string(),
                cluster_profile_id: "cluster-1".to_string(),
                source_topic: "source-topic".to_string(),
                source_partition: 0,
                source_offset: 12,
                source_timestamp: Some("2026-04-12T00:00:00Z".to_string()),
                target_topic: "target-topic".to_string(),
                status: "publishing".to_string(),
                mode: "broker-delivery".to_string(),
                payload_edit_json: None,
                headers_edit_json: None,
                key_edit_json: None,
                dry_run: false,
                requested_by_profile: Some("cluster-1".to_string()),
                risk_level: "high".to_string(),
                created_at: "2026-04-12T00:00:00Z".to_string(),
                started_at: Some("2026-04-12T00:00:10Z".to_string()),
                completed_at: None,
                error_message: None,
                result_summary_json: Some("{}".to_string()),
            },
        )
        .await
        .expect("seed replay job should insert");

        let service = ReplayService::new(&pool);
        let recovered = service
            .recover_stale_publishing_jobs()
            .await
            .expect("recovery should succeed");
        assert_eq!(recovered, 1);

        let job = sqlite::get_replay_job(&pool, "job-1")
            .await
            .expect("job should still exist");
        assert_eq!(job.status, "delivery_unknown");
        assert!(job.completed_at.is_some());
        assert!(job
            .error_message
            .unwrap_or_default()
            .contains("interrupted"));

        let events = sqlite::list_replay_job_events(&pool, "job-1")
            .await
            .expect("events should list");
        assert!(events
            .iter()
            .any(|event| event.event_type == "delivery_unknown_recovered"));

        let audits = sqlite::list_audit_events(
            &pool,
            &ListAuditEventsRequest {
                cluster_profile_id: Some("cluster-1".to_string()),
                event_type: Some("replay_delivery_unknown_recovered".to_string()),
                outcome: Some("delivery_unknown".to_string()),
                start_at: None,
                end_at: None,
                limit: Some(10),
            },
        )
        .await
        .expect("audit list should work");
        assert_eq!(audits.len(), 1);
    }

    #[tokio::test]
    async fn records_successful_live_delivery_lifecycle_for_mtls_cluster() {
        let pool = create_test_pool().await;
        seed_default_replay_policy(&pool).await;

        let ca_path = create_temp_file("ca", TEST_CERT_PEM);
        let cert_path = create_temp_file("client-cert", TEST_CERT_PEM);
        let key_path = create_temp_file("client-key", TEST_PRIVATE_KEY_PEM);

        seed_cluster_profile(
            &pool,
            "mtls",
            Some((
                ca_path.to_string_lossy().as_ref(),
                cert_path.to_string_lossy().as_ref(),
                key_path.to_string_lossy().as_ref(),
            )),
        )
        .await;

        let service = ReplayService::new(&pool);
        let result = service
            .create_replay_job_with_executor(
                sample_request("sandbox.orders.replayed"),
                success_executor,
            )
            .await
            .expect("live replay should succeed");

        assert_eq!(result.job.status, "delivered");
        assert!(result.job.started_at.is_some());
        assert!(result.job.completed_at.is_some());
        assert!(result.job.error_message.is_none());
        assert!(result.audit_ref.is_some());
        assert!(result
            .event_history
            .iter()
            .any(|event| event.event_type == "accepted"));
        assert!(result
            .event_history
            .iter()
            .any(|event| event.event_type == "publishing"));
        assert!(result
            .event_history
            .iter()
            .any(|event| event.event_type == "delivery_confirmed"));

        let audits = sqlite::list_audit_events(
            &pool,
            &ListAuditEventsRequest {
                cluster_profile_id: Some("cluster-1".to_string()),
                event_type: Some("replay_delivery_confirmed".to_string()),
                outcome: Some("delivered".to_string()),
                start_at: None,
                end_at: None,
                limit: Some(10),
            },
        )
        .await
        .expect("delivery confirmed audit should list");
        assert_eq!(audits.len(), 1);

        let _ = fs::remove_file(ca_path);
        let _ = fs::remove_file(cert_path);
        let _ = fs::remove_file(key_path);
    }

    #[tokio::test]
    async fn records_failed_live_delivery_lifecycle_for_mtls_cluster() {
        let pool = create_test_pool().await;
        seed_default_replay_policy(&pool).await;

        let ca_path = create_temp_file("ca-fail", TEST_CERT_PEM);
        let cert_path = create_temp_file("client-cert-fail", TEST_CERT_PEM);
        let key_path = create_temp_file("client-key-fail", TEST_PRIVATE_KEY_PEM);

        seed_cluster_profile(
            &pool,
            "mtls",
            Some((
                ca_path.to_string_lossy().as_ref(),
                cert_path.to_string_lossy().as_ref(),
                key_path.to_string_lossy().as_ref(),
            )),
        )
        .await;

        let service = ReplayService::new(&pool);
        let result = service
            .create_replay_job_with_executor(
                sample_request("sandbox.orders.failed"),
                failure_executor,
            )
            .await
            .expect("failed replay should still persist lifecycle");

        assert_eq!(result.job.status, "failed");
        assert!(result.job.started_at.is_some());
        assert!(result.job.completed_at.is_some());
        assert!(result
            .job
            .error_message
            .unwrap_or_default()
            .contains("broker unavailable"));
        assert!(result
            .event_history
            .iter()
            .any(|event| event.event_type == "delivery_failed"));

        let audits = sqlite::list_audit_events(
            &pool,
            &ListAuditEventsRequest {
                cluster_profile_id: Some("cluster-1".to_string()),
                event_type: Some("replay_delivery_failed".to_string()),
                outcome: Some("failed".to_string()),
                start_at: None,
                end_at: None,
                limit: Some(10),
            },
        )
        .await
        .expect("delivery failed audit should list");
        assert_eq!(audits.len(), 1);

        let _ = fs::remove_file(ca_path);
        let _ = fs::remove_file(cert_path);
        let _ = fs::remove_file(key_path);
    }

    #[tokio::test]
    async fn rejects_live_replay_outside_sandbox_policy() {
        let pool = create_test_pool().await;
        seed_default_replay_policy(&pool).await;
        seed_cluster_profile(&pool, "none", None).await;

        let service = ReplayService::new(&pool);
        let error = service
            .create_replay_job_with_executor(
                sample_request("prod.orders.replayed"),
                success_executor,
            )
            .await
            .expect_err("sandbox policy should reject non-sandbox target");

        assert!(error
            .to_string()
            .contains("only allows broker delivery replay to topics starting with 'sandbox.'"));

        let jobs = sqlite::list_replay_jobs(&pool, "cluster-1")
            .await
            .expect("replay jobs should list");
        assert!(jobs.is_empty());
    }

    #[tokio::test]
    async fn records_missing_source_message_as_failed_lifecycle() {
        let pool = create_test_pool().await;
        seed_default_replay_policy(&pool).await;
        seed_cluster_profile(&pool, "none", None).await;

        let service = ReplayService::new(&pool);
        let result = service
            .create_replay_job_with_executor(
                sample_request("sandbox.orders.missing-source"),
                missing_source_executor,
            )
            .await
            .expect("missing source should persist failed lifecycle");

        assert_eq!(result.job.status, "failed");
        assert!(result
            .job
            .error_message
            .unwrap_or_default()
            .contains("failed to load replay source message"));
        assert!(result
            .event_history
            .iter()
            .any(|event| event.event_type == "delivery_failed"));
    }
}
