use crate::models::cluster::ClusterProfileDto;
use crate::models::error::{AppError, AppResult};
use crate::models::message::{
    GetMessageDetailRequest, HeaderFilterDto, MessageDetailResponseDto, MessageHeaderDto,
    MessageRefDto, MessageSummaryDto, OffsetRangeDto, QueryMessagesRequest, TimeRangeDto,
};
use crate::models::schema_registry::SchemaRegistryProfileDto;
use crate::repositories::sqlite;
use crate::services::credentials::{resolve_schema_registry_auth, ResolvedSchemaRegistryAuth};
use crate::services::kafka_config::apply_kafka_read_consumer_config;
use apache_avro::{from_avro_datum, Schema as AvroSchema};
use chrono::{Local, NaiveDateTime, TimeZone};
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::{Headers, Message};
use rdkafka::topic_partition_list::{Offset, TopicPartitionList};
use rdkafka::ClientConfig;
use reqwest::blocking::Client as BlockingHttpClient;
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::io::Cursor;
use std::time::{Duration, Instant};

pub struct MessageService<'a> {
    pool: &'a SqlitePool,
}

struct PartitionBounds {
    start: i64,
    end_exclusive: i64,
}

struct DecodedPayloadOutcome {
    decode_status: String,
    payload_decoded: Option<String>,
    schema_info: Option<String>,
    payload_preview: Option<String>,
    related_hints: Vec<String>,
}

struct SchemaRegistryDecoder {
    profile: SchemaRegistryProfileDto,
    base_url: String,
    client: BlockingHttpClient,
    auth: Option<ResolvedSchemaRegistryAuth>,
    auth_resolution_error: Option<String>,
    cache: HashMap<u32, CachedSchema>,
}

#[derive(Clone)]
struct CachedSchema {
    schema_type: String,
    parsed_schema: Option<AvroSchema>,
    subject: Option<String>,
    has_references: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchemaByIdResponse {
    schema: String,
    #[serde(default)]
    schema_type: Option<String>,
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    references: Vec<SchemaReference>,
}

#[derive(Debug, Deserialize)]
struct SchemaReference {}

impl<'a> MessageService<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn query_messages(
        &self,
        request: QueryMessagesRequest,
    ) -> AppResult<Vec<MessageSummaryDto>> {
        validate_query_messages_request(&request)?;

        let profile = sqlite::get_cluster_profile(self.pool, &request.cluster_profile_id).await?;
        let schema_registry_profile =
            load_schema_registry_profile(self.pool, profile.schema_registry_profile_id.as_deref())
                .await?;

        tokio::task::spawn_blocking(move || {
            run_bounded_query(&profile, schema_registry_profile, request)
        })
        .await
        .map_err(|error| {
            AppError::Internal(format!("failed to join message query task: {error}"))
        })?
    }

    pub async fn get_message_detail(
        &self,
        request: GetMessageDetailRequest,
    ) -> AppResult<MessageDetailResponseDto> {
        validate_get_message_detail_request(&request)?;

        let profile =
            sqlite::get_cluster_profile(self.pool, &request.message_ref.cluster_profile_id).await?;
        let schema_registry_profile =
            load_schema_registry_profile(self.pool, profile.schema_registry_profile_id.as_deref())
                .await?;

        tokio::task::spawn_blocking(move || {
            load_message_detail(&profile, schema_registry_profile, request)
        })
        .await
        .map_err(|error| {
            AppError::Internal(format!("failed to join message detail task: {error}"))
        })?
    }
}

fn run_bounded_query(
    profile: &ClusterProfileDto,
    schema_registry_profile: Option<SchemaRegistryProfileDto>,
    request: QueryMessagesRequest,
) -> AppResult<Vec<MessageSummaryDto>> {
    let consumer = build_query_consumer(profile)?;
    let mut schema_decoder = SchemaRegistryDecoder::new(schema_registry_profile)?;
    let metadata = consumer
        .fetch_metadata(Some(&request.topic), Duration::from_secs(5))
        .map_err(|error| AppError::Network(format!("failed to load topic metadata: {error}")))?;

    let topic = metadata
        .topics()
        .iter()
        .find(|topic| topic.name() == request.topic)
        .ok_or_else(|| AppError::NotFound(format!("topic '{}' was not found", request.topic)))?;

    let available_partitions = topic
        .partitions()
        .iter()
        .map(|partition| partition.id())
        .collect::<HashSet<_>>();

    let target_partitions = if let Some(partitions) = request.partitions.clone() {
        if partitions.is_empty() {
            topic
                .partitions()
                .iter()
                .map(|partition| partition.id())
                .collect::<Vec<_>>()
        } else {
            partitions
        }
    } else {
        topic
            .partitions()
            .iter()
            .map(|partition| partition.id())
            .collect::<Vec<_>>()
    };

    for partition in &target_partitions {
        if !available_partitions.contains(partition) {
            return Err(AppError::Validation(format!(
                "partition '{}' does not exist on topic '{}'",
                partition, request.topic
            )));
        }
    }

    let mut bounds_map = HashMap::new();
    let mut assignment = TopicPartitionList::new();

    for partition in target_partitions {
        let bounds = resolve_partition_bounds(
            &consumer,
            &request.topic,
            partition,
            request.time_range.as_ref(),
            request.offset_range.as_ref(),
        )?;

        if bounds.start >= bounds.end_exclusive {
            continue;
        }

        assignment
            .add_partition_offset(&request.topic, partition, Offset::Offset(bounds.start))
            .map_err(|error| {
                AppError::Internal(format!(
                    "failed to assign partition '{}': {error}",
                    partition
                ))
            })?;
        bounds_map.insert(partition, bounds);
    }

    if bounds_map.is_empty() {
        return Ok(Vec::new());
    }

    consumer.assign(&assignment).map_err(|error| {
        AppError::Network(format!(
            "failed to assign bounded query partitions: {error}"
        ))
    })?;

    let key_filter = request.key_filter.clone().map(|value| value.to_lowercase());
    let header_filters = request.header_filters.clone().unwrap_or_default();
    let mut finished_partitions = HashSet::new();
    let mut results = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut idle_polls = 0;

    while Instant::now() < deadline
        && results.len() < request.max_results
        && finished_partitions.len() < bounds_map.len()
    {
        match consumer.poll(Duration::from_millis(200)) {
            Some(Ok(message)) => {
                idle_polls = 0;
                let partition = message.partition();
                let Some(bounds) = bounds_map.get(&partition) else {
                    continue;
                };

                if message.offset() >= bounds.end_exclusive {
                    finished_partitions.insert(partition);
                    continue;
                }

                if message.offset() < bounds.start {
                    continue;
                }

                if !matches_key_filter(&message, key_filter.as_deref()) {
                    continue;
                }

                if !matches_header_filters(&message, &header_filters) {
                    continue;
                }

                results.push(map_message_summary(
                    &request.cluster_profile_id,
                    &message,
                    schema_decoder.as_mut(),
                ));
            }
            Some(Err(_)) => {
                idle_polls += 1;
            }
            None => {
                idle_polls += 1;
            }
        }

        if idle_polls >= 10 {
            break;
        }
    }

    Ok(results)
}

fn load_message_detail(
    profile: &ClusterProfileDto,
    schema_registry_profile: Option<SchemaRegistryProfileDto>,
    request: GetMessageDetailRequest,
) -> AppResult<MessageDetailResponseDto> {
    let consumer = build_query_consumer(profile)?;
    let mut schema_decoder = SchemaRegistryDecoder::new(schema_registry_profile)?;
    let target_offset =
        request.message_ref.offset.parse::<i64>().map_err(|_| {
            AppError::Validation("message offset must be a valid integer".to_string())
        })?;

    let mut assignment = TopicPartitionList::new();
    assignment
        .add_partition_offset(
            &request.message_ref.topic,
            request.message_ref.partition,
            Offset::Offset(target_offset),
        )
        .map_err(|error| {
            AppError::Internal(format!(
                "failed to assign message detail partition: {error}"
            ))
        })?;

    consumer.assign(&assignment).map_err(|error| {
        AppError::Network(format!("failed to assign message detail consumer: {error}"))
    })?;

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match consumer.poll(Duration::from_millis(200)) {
            Some(Ok(message)) => {
                if message.partition() != request.message_ref.partition
                    || message.topic() != request.message_ref.topic
                {
                    continue;
                }

                if message.offset() < target_offset {
                    continue;
                }

                if message.offset() > target_offset {
                    return Err(AppError::NotFound(format!(
                        "message '{} / {} / {}' was not found",
                        request.message_ref.topic,
                        request.message_ref.partition,
                        request.message_ref.offset
                    )));
                }

                return Ok(map_message_detail(
                    message,
                    request.message_ref,
                    schema_decoder.as_mut(),
                ));
            }
            Some(Err(_)) | None => {}
        }
    }

    Err(AppError::NotFound(format!(
        "message '{} / {} / {}' was not found within the bounded query window",
        request.message_ref.topic, request.message_ref.partition, request.message_ref.offset
    )))
}

fn build_query_consumer(profile: &ClusterProfileDto) -> AppResult<BaseConsumer> {
    let mut config = ClientConfig::new();
    apply_kafka_read_consumer_config(&mut config, profile)?;

    config.create().map_err(|error| {
        AppError::Network(format!("failed to create message query consumer: {error}"))
    })
}

fn resolve_partition_bounds(
    consumer: &BaseConsumer,
    topic: &str,
    partition: i32,
    time_range: Option<&TimeRangeDto>,
    offset_range: Option<&OffsetRangeDto>,
) -> AppResult<PartitionBounds> {
    let (low, high) = consumer
        .fetch_watermarks(topic, partition, Duration::from_secs(1))
        .map_err(|error| {
            AppError::Network(format!(
                "failed to load watermarks for topic '{}' partition {}: {error}",
                topic, partition
            ))
        })?;

    let mut start = low;
    let mut end_exclusive = high;

    if let Some(offset_range) = offset_range {
        if let Some(start_offset) = offset_range.start_offset.as_ref() {
            let parsed = start_offset.parse::<i64>().map_err(|_| {
                AppError::Validation("startOffset must be a valid integer".to_string())
            })?;
            start = start.max(parsed);
        }

        if let Some(end_offset) = offset_range.end_offset.as_ref() {
            let parsed = end_offset.parse::<i64>().map_err(|_| {
                AppError::Validation("endOffset must be a valid integer".to_string())
            })?;
            end_exclusive = end_exclusive.min(parsed.saturating_add(1));
        }
    }

    if let Some(time_range) = time_range {
        if !time_range.start.trim().is_empty() {
            let start_ms = parse_local_datetime_millis(&time_range.start)?;
            let resolved = resolve_offset_for_time(consumer, topic, partition, start_ms)?;
            start = start.max(resolved.unwrap_or(high));
        }

        if !time_range.end.trim().is_empty() {
            let end_ms = parse_local_datetime_millis(&time_range.end)?;
            let resolved = resolve_offset_for_time(consumer, topic, partition, end_ms)?;
            end_exclusive = end_exclusive.min(resolved.unwrap_or(high));
        }
    }

    start = start.clamp(low, high);
    end_exclusive = end_exclusive.clamp(low, high);

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
                "failed to build timestamp lookup partition list: {error}"
            ))
        })?;

    let offsets = consumer
        .offsets_for_times(timestamps, Duration::from_secs(2))
        .map_err(|error| {
            AppError::Network(format!("failed to resolve timestamp bounds: {error}"))
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

fn matches_key_filter<M: Message>(message: &M, key_filter: Option<&str>) -> bool {
    let Some(key_filter) = key_filter else {
        return true;
    };

    let key = message
        .key()
        .map(|value| String::from_utf8_lossy(value).to_lowercase())
        .unwrap_or_default();

    key.contains(key_filter)
}

fn matches_header_filters<M: Message>(message: &M, header_filters: &[HeaderFilterDto]) -> bool {
    if header_filters.is_empty() {
        return true;
    }

    let Some(headers) = message.headers() else {
        return false;
    };

    header_filters.iter().all(|filter| {
        headers.iter().any(|header| {
            if header.key != filter.key {
                return false;
            }

            let value = header
                .value
                .map(|bytes| String::from_utf8_lossy(bytes).to_string())
                .unwrap_or_default();

            filter
                .value
                .as_ref()
                .map(|needle| value.contains(needle))
                .unwrap_or(true)
        })
    })
}

fn map_message_summary(
    cluster_profile_id: &str,
    message: &impl Message,
    schema_decoder: Option<&mut SchemaRegistryDecoder>,
) -> MessageSummaryDto {
    let decode_outcome = decode_payload(message.payload(), schema_decoder);
    let key_preview = message
        .key()
        .map(|key| truncate_preview(&String::from_utf8_lossy(key), 80));
    let timestamp = message
        .timestamp()
        .to_millis()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "0".to_string());

    MessageSummaryDto {
        message_ref: MessageRefDto {
            cluster_profile_id: cluster_profile_id.to_string(),
            topic: message.topic().to_string(),
            partition: message.partition(),
            offset: message.offset().to_string(),
        },
        timestamp,
        partition: message.partition(),
        offset: message.offset().to_string(),
        key_preview,
        decode_status: decode_outcome.decode_status,
        payload_preview: decode_outcome
            .payload_preview
            .map(|preview| truncate_preview(&preview, 160)),
    }
}

fn map_message_detail(
    message: impl Message,
    message_ref: MessageRefDto,
    schema_decoder: Option<&mut SchemaRegistryDecoder>,
) -> MessageDetailResponseDto {
    let decode_outcome = decode_payload(message.payload(), schema_decoder);
    let payload_raw = message
        .payload()
        .map(|payload| String::from_utf8_lossy(payload).to_string())
        .unwrap_or_default();
    let key_raw = message
        .key()
        .map(|key| String::from_utf8_lossy(key).to_string());
    let headers = message
        .headers()
        .map(|headers| {
            headers
                .iter()
                .map(|header| MessageHeaderDto {
                    key: header.key.to_string(),
                    value: header
                        .value
                        .map(|value| String::from_utf8_lossy(value).to_string())
                        .unwrap_or_default(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    MessageDetailResponseDto {
        message_ref,
        timestamp: message
            .timestamp()
            .to_millis()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "0".to_string()),
        key_raw,
        headers,
        payload_raw,
        payload_decoded: decode_outcome.payload_decoded,
        decode_status: decode_outcome.decode_status,
        schema_info: decode_outcome.schema_info,
        related_hints: Some(
            decode_outcome
                .related_hints
                .into_iter()
                .chain(std::iter::once("可继续进入回放与追踪流程".to_string()))
                .collect(),
        ),
    }
}

fn decode_payload(
    payload: Option<&[u8]>,
    schema_decoder: Option<&mut SchemaRegistryDecoder>,
) -> DecodedPayloadOutcome {
    let Some(bytes) = payload else {
        return DecodedPayloadOutcome {
            decode_status: "empty".to_string(),
            payload_decoded: None,
            schema_info: None,
            payload_preview: None,
            related_hints: vec!["消息体为空，没有可解码内容。".to_string()],
        };
    };

    let raw_utf8 = std::str::from_utf8(bytes)
        .ok()
        .map(|value| value.to_string());

    if let Some(decoder) = schema_decoder {
        if let Some((schema_id, encoded_payload)) = parse_confluent_wire_payload(bytes) {
            return decoder.decode(schema_id, encoded_payload, raw_utf8);
        }
    }

    fallback_decode_outcome(raw_utf8)
}

fn fallback_decode_outcome(raw_utf8: Option<String>) -> DecodedPayloadOutcome {
    match raw_utf8 {
        Some(text) => DecodedPayloadOutcome {
            decode_status: "utf8".to_string(),
            payload_decoded: None,
            schema_info: None,
            payload_preview: Some(text),
            related_hints: vec![
                "消息体可按 UTF-8 文本查看，但当前没有 Schema Registry 解码结果。".to_string(),
            ],
        },
        None => DecodedPayloadOutcome {
            decode_status: "binary".to_string(),
            payload_decoded: None,
            schema_info: None,
            payload_preview: None,
            related_hints: vec![
                "消息体是二进制数据；若需结构化解码，请确保使用受支持的 Schema Registry 编码。"
                    .to_string(),
            ],
        },
    }
}

fn parse_confluent_wire_payload(payload: &[u8]) -> Option<(u32, &[u8])> {
    if payload.len() <= 5 || payload[0] != 0 {
        return None;
    }

    let schema_id = u32::from_be_bytes([payload[1], payload[2], payload[3], payload[4]]);
    Some((schema_id, &payload[5..]))
}

async fn load_schema_registry_profile(
    pool: &SqlitePool,
    schema_registry_profile_id: Option<&str>,
) -> AppResult<Option<SchemaRegistryProfileDto>> {
    let Some(schema_registry_profile_id) = schema_registry_profile_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };

    sqlite::get_schema_registry_profile(pool, schema_registry_profile_id)
        .await
        .map(Some)
}

impl SchemaRegistryDecoder {
    fn new(profile: Option<SchemaRegistryProfileDto>) -> AppResult<Option<Self>> {
        let Some(profile) = profile else {
            return Ok(None);
        };

        let base_url = normalize_schema_registry_base_url(&profile.base_url)?;
        let client = BlockingHttpClient::builder()
            .timeout(Duration::from_secs(4))
            .build()
            .map_err(|error| {
                AppError::Internal(format!(
                    "failed to build schema registry decode client: {error}"
                ))
            })?;
        let auth_resolution = resolve_schema_registry_auth(
            &profile.auth_mode,
            profile.credential_ref.as_deref(),
            None,
        );
        let (auth, auth_resolution_error) = match auth_resolution {
            Ok(auth) => (auth, None),
            Err(error) => (None, Some(error.to_string())),
        };

        Ok(Some(Self {
            profile,
            base_url,
            client,
            auth,
            auth_resolution_error,
            cache: HashMap::new(),
        }))
    }

    fn decode(
        &mut self,
        schema_id: u32,
        payload: &[u8],
        raw_utf8: Option<String>,
    ) -> DecodedPayloadOutcome {
        if self.profile.auth_mode != "none" && self.auth.is_none() {
            return DecodedPayloadOutcome {
                decode_status: "schema-auth-failed".to_string(),
                payload_decoded: None,
                schema_info: Some(format!(
                    "{} · auth={} · schemaId {}",
                    self.profile.name, self.profile.auth_mode, schema_id
                )),
                payload_preview: raw_utf8,
                related_hints: vec![self.auth_resolution_error.clone().unwrap_or_else(|| {
                    "已检测到需要认证的 Schema Registry，但当前无法解析可用的系统凭据。".to_string()
                })],
            };
        }

        let schema = match self.load_schema(schema_id) {
            Ok(schema) => schema,
            Err(error) => {
                return DecodedPayloadOutcome {
                    decode_status: "schema-registry-error".to_string(),
                    payload_decoded: None,
                    schema_info: Some(format!("{} · schemaId {}", self.profile.name, schema_id)),
                    payload_preview: raw_utf8,
                    related_hints: vec![error.to_string()],
                };
            }
        };

        if schema.schema_type != "AVRO" {
            return DecodedPayloadOutcome {
                decode_status: "schema-unsupported".to_string(),
                payload_decoded: None,
                schema_info: Some(format!(
                    "{} · {} · schemaId {}{}",
                    self.profile.name,
                    schema.schema_type,
                    schema_id,
                    schema
                        .subject
                        .as_ref()
                        .map(|subject| format!(" · {subject}"))
                        .unwrap_or_default()
                )),
                payload_preview: raw_utf8,
                related_hints: vec![format!(
                    "当前版本只实现了 Avro 解码；schemaType={} 仍会保留原始内容展示。",
                    schema.schema_type
                )],
            };
        }

        if schema.has_references {
            return DecodedPayloadOutcome {
                decode_status: "schema-references-unsupported".to_string(),
                payload_decoded: None,
                schema_info: Some(format!(
                    "{} · AVRO · schemaId {}",
                    self.profile.name, schema_id
                )),
                payload_preview: raw_utf8,
                related_hints: vec!["当前版本尚未解析带 references 的 Avro schema。".to_string()],
            };
        }

        let Some(parsed_schema) = schema.parsed_schema.as_ref() else {
            return DecodedPayloadOutcome {
                decode_status: "schema-decode-failed".to_string(),
                payload_decoded: None,
                schema_info: Some(format!(
                    "{} · AVRO · schemaId {}",
                    self.profile.name, schema_id
                )),
                payload_preview: raw_utf8,
                related_hints: vec![
                    "Schema Registry 返回了 Avro schema，但当前无法构建本地解码器。".to_string(),
                ],
            };
        };

        let mut cursor = Cursor::new(payload);
        match from_avro_datum(parsed_schema, &mut cursor, None) {
            Ok(value) => match serde_json::Value::try_from(value) {
                Ok(json_value) => match serde_json::to_string_pretty(&json_value) {
                    Ok(decoded_json) => DecodedPayloadOutcome {
                        decode_status: "avro-decoded".to_string(),
                        payload_decoded: Some(decoded_json.clone()),
                        schema_info: Some(format!(
                            "{} · AVRO · schemaId {}{}",
                            self.profile.name,
                            schema_id,
                            schema
                                .subject
                                .as_ref()
                                .map(|subject| format!(" · {subject}"))
                                .unwrap_or_default()
                        )),
                        payload_preview: Some(decoded_json),
                        related_hints: vec![
                            "已通过 Schema Registry 成功完成 Avro 解码。".to_string()
                        ],
                    },
                    Err(error) => DecodedPayloadOutcome {
                        decode_status: "schema-decode-failed".to_string(),
                        payload_decoded: None,
                        schema_info: Some(format!(
                            "{} · AVRO · schemaId {}",
                            self.profile.name, schema_id
                        )),
                        payload_preview: raw_utf8,
                        related_hints: vec![format!(
                            "Avro 解码成功，但格式化为 JSON 文本失败：{error}"
                        )],
                    },
                },
                Err(error) => DecodedPayloadOutcome {
                    decode_status: "schema-decode-failed".to_string(),
                    payload_decoded: None,
                    schema_info: Some(format!(
                        "{} · AVRO · schemaId {}",
                        self.profile.name, schema_id
                    )),
                    payload_preview: raw_utf8,
                    related_hints: vec![format!("Avro 结果无法转换成 JSON 展示：{error}")],
                },
            },
            Err(error) => DecodedPayloadOutcome {
                decode_status: "schema-decode-failed".to_string(),
                payload_decoded: None,
                schema_info: Some(format!(
                    "{} · AVRO · schemaId {}",
                    self.profile.name, schema_id
                )),
                payload_preview: raw_utf8,
                related_hints: vec![format!(
                    "Schema Registry 已返回 schema，但 Avro 解码失败：{error}"
                )],
            },
        }
    }

    fn load_schema(&mut self, schema_id: u32) -> AppResult<CachedSchema> {
        if let Some(schema) = self.cache.get(&schema_id) {
            return Ok(schema.clone());
        }

        let request = self.client.get(format!(
            "{}/schemas/ids/{schema_id}",
            self.base_url.trim_end_matches('/')
        ));
        let request = if let Some(auth) = self.auth.as_ref() {
            auth.apply_blocking(request)
        } else {
            request
        };

        let response = request.send().map_err(|error| {
            AppError::Network(format!(
                "failed to query schema registry for schemaId {schema_id}: {error}"
            ))
        })?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(AppError::Network(format!(
                "schema registry authentication failed while fetching schemaId {schema_id}: {status}"
            )));
        }
        if status != StatusCode::OK {
            return Err(AppError::Network(format!(
                "schema registry returned status {status} for schemaId {schema_id}"
            )));
        }

        let schema_response: SchemaByIdResponse = response.json().map_err(|error| {
            AppError::Network(format!(
                "failed to deserialize schema registry response for schemaId {schema_id}: {error}"
            ))
        })?;

        let schema_type = schema_response
            .schema_type
            .unwrap_or_else(|| "AVRO".to_string())
            .to_uppercase();
        let has_references = !schema_response.references.is_empty();
        let parsed_schema = if schema_type == "AVRO" && !has_references {
            Some(
                AvroSchema::parse_str(&schema_response.schema).map_err(|error| {
                    AppError::Internal(format!(
                        "failed to parse Avro schemaId {schema_id}: {error}"
                    ))
                })?,
            )
        } else {
            None
        };

        let cached = CachedSchema {
            schema_type,
            parsed_schema,
            subject: schema_response.subject,
            has_references,
        };
        self.cache.insert(schema_id, cached.clone());
        Ok(cached)
    }
}

fn normalize_schema_registry_base_url(base_url: &str) -> AppResult<String> {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "schema registry base URL is required".to_string(),
        ));
    }

    Ok(
        if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
            trimmed.trim_end_matches('/').to_string()
        } else {
            format!("http://{}", trimmed.trim_end_matches('/'))
        },
    )
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

fn validate_query_messages_request(request: &QueryMessagesRequest) -> AppResult<()> {
    if request.cluster_profile_id.trim().is_empty() {
        return Err(AppError::Validation(
            "cluster profile id is required".to_string(),
        ));
    }
    if request.topic.trim().is_empty() {
        return Err(AppError::Validation("topic is required".to_string()));
    }
    if request.max_results == 0 || request.max_results > 500 {
        return Err(AppError::Validation(
            "maxResults must be between 1 and 500".to_string(),
        ));
    }

    let has_partitions = request
        .partitions
        .as_ref()
        .map(|items| !items.is_empty())
        .unwrap_or(false);
    let has_time = request
        .time_range
        .as_ref()
        .map(|range| !range.start.trim().is_empty() || !range.end.trim().is_empty())
        .unwrap_or(false);
    let has_offset = request
        .offset_range
        .as_ref()
        .map(|range| {
            range
                .start_offset
                .as_ref()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
                || range
                    .end_offset
                    .as_ref()
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false)
        })
        .unwrap_or(false);

    if !has_partitions && !has_time && !has_offset {
        return Err(AppError::Validation(
            "bounded query required: provide partitions, timeRange, or offsetRange".to_string(),
        ));
    }

    Ok(())
}

fn validate_get_message_detail_request(request: &GetMessageDetailRequest) -> AppResult<()> {
    if request.message_ref.cluster_profile_id.trim().is_empty() {
        return Err(AppError::Validation(
            "cluster profile id is required".to_string(),
        ));
    }
    if request.message_ref.topic.trim().is_empty() {
        return Err(AppError::Validation("topic is required".to_string()));
    }
    if request.message_ref.offset.trim().is_empty() {
        return Err(AppError::Validation("offset is required".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        build_query_consumer, decode_payload, BlockingHttpClient, HashMap,
        ResolvedSchemaRegistryAuth, SchemaRegistryDecoder,
    };
    use crate::models::cluster::ClusterProfileDto;
    use crate::models::schema_registry::SchemaRegistryProfileDto;
    use apache_avro::{to_avro_datum, types::Value, Schema as AvroSchema};
    use serde_json::json;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    const TEST_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----\nMIICsDCCAZgCCQD4vKEpwC7YOjANBgkqhkiG9w0BAQsFADAaMRgwFgYDVQQDDA90\ncmFjZWZvcmdlLXRlc3QwHhcNMjYwNDE4MDczMTU0WhcNMjcwNDE4MDczMTU0WjAa\nMRgwFgYDVQQDDA90cmFjZWZvcmdlLXRlc3QwggEiMA0GCSqGSIb3DQEBAQUAA4IB\nDwAwggEKAoIBAQDB3uz/2t6m3/qJaENMNk1KW6L0nE6Sr6a16C9alMoNk53q8Glx\nQA7shqJLUj8AHpbMMMddlc/RXz4cYXKLMq5XH2YVJ+QKxjcXR0Q6Swu3cqoMO7n8\nfiOiA9biPR3/78KSEPd9zdqOs3DwMHr65v3oEI9tc1rWvam9Xz+4MrkBFbmF4wZR\nYZ4ac+lgB9ie1FL4S4cRHYqYzvumNdMDtFv8hDCmyPdKUlfEowxtueuQ5WGB82yP\nGd0LlYOvxt79MiR0sIQsxVnSY5F0nLcNW9Z+bCuGgEWchakiUb9Nj2BcJ88IAGxs\npitqC3v+noeQA4SWkc+7Byd4dZS4rLGYv8FhAgMBAAEwDQYJKoZIhvcNAQELBQAD\nggEBACk9WDt7D7thZkoT8VJkyukWx4uPGXczOfp0+hu2eP1TODurSQziwVj3xF3O\noSjN8HrWg3U0vGqZGgqIPxPknbmwk5fjVorwWelRlX2X7DMElsFeRMZSY9leLC10\ntqdEu8mIJsGzR/Aua56fo3dywhIglYG/8O0tcZYjdp6YczXWW64lPz2vVv+9ZVVj\nnVrKYbU118mkVhd7jmV9QR5KdBY1th6qVEzI340S7CQ2PdweT0kemFwBTCp5gvJ5\na3Xi8pKrQKJk/L2O6oxhXOCCGvWhdEvZ8mel2Qp/whg6MupIciDKozdf68yECrUW\nEhr3a4kltLXboZZ+DJx+KZCTRv0=\n-----END CERTIFICATE-----\n";
    const TEST_PRIVATE_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQDB3uz/2t6m3/qJ\naENMNk1KW6L0nE6Sr6a16C9alMoNk53q8GlxQA7shqJLUj8AHpbMMMddlc/RXz4c\nYXKLMq5XH2YVJ+QKxjcXR0Q6Swu3cqoMO7n8fiOiA9biPR3/78KSEPd9zdqOs3Dw\nMHr65v3oEI9tc1rWvam9Xz+4MrkBFbmF4wZRYZ4ac+lgB9ie1FL4S4cRHYqYzvum\nNdMDtFv8hDCmyPdKUlfEowxtueuQ5WGB82yPGd0LlYOvxt79MiR0sIQsxVnSY5F0\nnLcNW9Z+bCuGgEWchakiUb9Nj2BcJ88IAGxspitqC3v+noeQA4SWkc+7Byd4dZS4\nrLGYv8FhAgMBAAECggEAU/ogZuODtn0mpQaIwCZ1bFQtTg+26Us0x27/tBjnPOJI\ncVAaHHhG/qWC/2Vs7LxTTbeDZEJUdrjuypRbbXhSlGKBcYCKAzDBBFBWeXmwZZJB\nnnLwqTJqdO980RrwN8C/Y03+JnxYw59uuFwHqU8NhMxHlH13R7V4JRNxInS2NoVV\nXVfgcTjax8pdbdzKKwIn3AUk27SwSJwBlYuMKgDq741/L8PvyjOmolvzwM+aF2FO\n1gqb3xZKM1867psYo4Z09qdc8GyG+joPEbJW9rQW2nORUy1mqApXmO8qprt+K3yY\nhWQUjYFpngx7OKOv0RhRSwzm9swK03QbLKK5i5/IwQKBgQDokMmZxVKbnpTlI8Fl\n6H2pQAIPi1HPdTTMapxBlP5CsLgtkBiYu60LYevmAcSdkzbpVr7uqWxy7+Z0upao\n7kheHaqovcO8xm1n1BDOnEgnmFJ9wLFDi8qG9EQd4dusvJWb6u3dvvWVWQFh4Pz4\nZPKxGbfa7VHeFSk9wizXuCGQpQKBgQDVZ/nhoCOIzwF9bxR9Gko6fdeLf6ZTK0ht\nMJZ8kbDlYoSjRWjX6zXdoLL7mQ2y7avQ8mEYVyOcBCb5xGa43cF6eKy6W59lXQqI\nB1Z64gaKAsFgqbhpJRo4sfEsft9pip/oI49Y9B6vP9do9UbfBg4ZZm5fZdS5+MnY\nWE70VTB1DQKBgQCzQh7SkuD4qIRWFnhUp55sXbT47Ecz5EC9K5OjjUdqejKMlBwR\nZd+c/W5KDKTTXIyf0Mg8x4SbF0UIRmYocfp/6NgJVrPQBxZ/SFtoFdgcBPHYkjVQ\nPijuWstCSTv86iNbWfrcx/sdkcxZ+ISkpZLXZV5sti47Qw5V1xyfbgMZLQKBgQCq\nI92LTvtFpZSQhrEVFJK9k3r3kuvuPwHdW/F+m0EngKYy7bGrA7HMYsSP5vSPBQII\n8lUK7N5NEtpoI3eqR9JrbC553XZ1f/pXfVIrYmzIN24pPObznUsMjIG1cel44baf\ng0pUJz0Xh5Sb74FzagZvpcS1diBlrL5wJ+e60PhzOQKBgQCy9K28nvKeC+JO40ST\ngp5YqlQnRNaWfXHTwbeeYygYfgiY+lw7hmtInnLcu7s3uVepoWpJJPGeVG7MiW+1\nfN3BpyGmBc+fA8UbUl2RHZKvKaLTWGwujYqwbfSOtI56FpNBHfjy4HyAvzAHwB7a\nlHc4mf+dH3zMIvjxLn2jsGjjFw==\n-----END PRIVATE KEY-----\n";

    struct TestRegistryServer {
        base_url: String,
        handle: thread::JoinHandle<()>,
    }

    impl TestRegistryServer {
        fn join(self) {
            self.handle
                .join()
                .expect("test registry server should complete");
        }
    }

    #[test]
    fn decode_payload_returns_avro_decoded_for_authenticated_schema_registry_fetch() {
        let schema_json =
            r#"{"type":"record","name":"User","fields":[{"name":"name","type":"string"}]}"#;
        let server = spawn_schema_registry_server(
            Some("Bearer token-value"),
            "200 OK",
            json!({
                "schema": schema_json,
                "schemaType": "AVRO",
                "subject": "users-value",
                "references": []
            })
            .to_string(),
        );
        let mut decoder = build_decoder(
            &server.base_url,
            "bearer",
            Some(ResolvedSchemaRegistryAuth::Bearer {
                token: "token-value".to_string(),
            }),
            None,
        );
        let payload = confluent_payload(7, &encode_avro_record(schema_json, "alice"));

        let outcome = decode_payload(Some(&payload), Some(&mut decoder));

        server.join();
        assert_eq!(outcome.decode_status, "avro-decoded");
        let payload_decoded = outcome
            .payload_decoded
            .expect("decoded payload should be present for avro success");
        assert!(payload_decoded.contains("\"name\": \"alice\""));
        assert_eq!(
            outcome.payload_preview.as_deref(),
            Some(payload_decoded.as_str())
        );
        assert_eq!(
            outcome.schema_info.as_deref(),
            Some("Registry One · AVRO · schemaId 7 · users-value")
        );
        assert_eq!(
            outcome.related_hints,
            vec!["已通过 Schema Registry 成功完成 Avro 解码。"]
        );
    }

    #[test]
    fn decode_payload_resolves_bearer_auth_via_decoder_construction() {
        let schema_json =
            r#"{"type":"record","name":"User","fields":[{"name":"name","type":"string"}]}"#;
        let server = spawn_schema_registry_server(
            Some("Bearer runtime-token-value"),
            "200 OK",
            json!({
                "schema": schema_json,
                "schemaType": "AVRO",
                "subject": "users-value",
                "references": []
            })
            .to_string(),
        );
        let credential_ref = format!("registry-token-{}", Uuid::new_v4());
        let env_key = test_secret_env_key(&credential_ref);
        unsafe { std::env::set_var(&env_key, "runtime-token-value") };
        let profile = SchemaRegistryProfileDto {
            id: "registry-1".to_string(),
            name: "Registry One".to_string(),
            base_url: server.base_url.clone(),
            auth_mode: "bearer".to_string(),
            credential_ref: Some(credential_ref),
            notes: None,
            created_at: "2026-04-18T00:00:00Z".to_string(),
            updated_at: "2026-04-18T00:00:00Z".to_string(),
        };
        let mut decoder = SchemaRegistryDecoder::new(Some(profile))
            .expect("decoder construction should succeed with keyring-backed auth")
            .expect("schema registry profile should create a decoder");
        let payload = confluent_payload(7, &encode_avro_record(schema_json, "alice"));

        let outcome = decode_payload(Some(&payload), Some(&mut decoder));

        unsafe { std::env::remove_var(&env_key) };
        server.join();
        assert_eq!(outcome.decode_status, "avro-decoded");
        assert!(outcome
            .payload_decoded
            .as_deref()
            .is_some_and(|payload| payload.contains("\"name\": \"alice\"")));
        assert_eq!(
            outcome.schema_info.as_deref(),
            Some("Registry One · AVRO · schemaId 7 · users-value")
        );
    }

    #[test]
    fn decode_payload_surfaces_auth_resolution_failure_before_registry_fetch() {
        let profile = SchemaRegistryProfileDto {
            id: "registry-1".to_string(),
            name: "Registry One".to_string(),
            base_url: "http://127.0.0.1:65535".to_string(),
            auth_mode: "basic".to_string(),
            credential_ref: None,
            notes: None,
            created_at: "2026-04-18T00:00:00Z".to_string(),
            updated_at: "2026-04-18T00:00:00Z".to_string(),
        };
        let mut decoder = SchemaRegistryDecoder::new(Some(profile))
            .expect("decoder should still build when auth resolution fails")
            .expect("schema registry profile should create a decoder");
        let payload = confluent_payload(
            7,
            &encode_avro_record(
                r#"{"type":"record","name":"User","fields":[{"name":"name","type":"string"}]}"#,
                "alice",
            ),
        );

        let outcome = decode_payload(Some(&payload), Some(&mut decoder));

        assert_eq!(outcome.decode_status, "schema-auth-failed");
        assert!(outcome.payload_decoded.is_none());
        assert_eq!(
            outcome.schema_info.as_deref(),
            Some("Registry One · auth=basic · schemaId 7")
        );
        assert!(outcome
            .related_hints
            .iter()
            .any(|hint| hint.contains("credentialRef is required for auth mode 'basic'")));
    }

    #[test]
    fn decode_payload_reports_schema_registry_error_for_authenticated_401_response() {
        let schema_json =
            r#"{"type":"record","name":"User","fields":[{"name":"name","type":"string"}]}"#;
        let server = spawn_schema_registry_server(
            Some("Bearer token-value"),
            "401 Unauthorized",
            json!({
                "error_code": 401,
                "message": "unauthorized"
            })
            .to_string(),
        );
        let mut decoder = build_decoder(
            &server.base_url,
            "bearer",
            Some(ResolvedSchemaRegistryAuth::Bearer {
                token: "token-value".to_string(),
            }),
            None,
        );
        let payload = confluent_payload(7, &encode_avro_record(schema_json, "alice"));

        let outcome = decode_payload(Some(&payload), Some(&mut decoder));

        server.join();
        assert_eq!(outcome.decode_status, "schema-registry-error");
        assert!(outcome.payload_decoded.is_none());
        assert_eq!(
            outcome.schema_info.as_deref(),
            Some("Registry One · schemaId 7")
        );
        assert!(outcome.related_hints.iter().any(|hint| hint.contains(
            "schema registry authentication failed while fetching schemaId 7: 401 Unauthorized"
        )));
    }

    #[test]
    fn build_query_consumer_supports_mtls_cluster_profile() {
        let ca_path = create_temp_file("messages-ca", TEST_CERT_PEM);
        let cert_path = create_temp_file("messages-cert", TEST_CERT_PEM);
        let key_path = create_temp_file("messages-key", TEST_PRIVATE_KEY_PEM);
        let profile = sample_mtls_profile(&cert_path, &key_path, &ca_path);

        let consumer = build_query_consumer(&profile)
            .expect("message query consumer should build for mTLS profile");
        drop(consumer);

        let _ = fs::remove_file(ca_path);
        let _ = fs::remove_file(cert_path);
        let _ = fs::remove_file(key_path);
    }

    fn build_decoder(
        base_url: &str,
        auth_mode: &str,
        auth: Option<ResolvedSchemaRegistryAuth>,
        auth_resolution_error: Option<&str>,
    ) -> SchemaRegistryDecoder {
        SchemaRegistryDecoder {
            profile: SchemaRegistryProfileDto {
                id: "registry-1".to_string(),
                name: "Registry One".to_string(),
                base_url: base_url.to_string(),
                auth_mode: auth_mode.to_string(),
                credential_ref: Some("registry-token".to_string()),
                notes: None,
                created_at: "2026-04-18T00:00:00Z".to_string(),
                updated_at: "2026-04-18T00:00:00Z".to_string(),
            },
            base_url: base_url.to_string(),
            client: BlockingHttpClient::builder()
                .timeout(Duration::from_secs(4))
                .build()
                .expect("test decoder HTTP client should build"),
            auth,
            auth_resolution_error: auth_resolution_error.map(str::to_string),
            cache: HashMap::new(),
        }
    }

    fn encode_avro_record(schema_json: &str, name: &str) -> Vec<u8> {
        let schema = AvroSchema::parse_str(schema_json).expect("test schema should parse");
        let value = Value::Record(vec![("name".to_string(), Value::String(name.to_string()))]);

        to_avro_datum(&schema, value).expect("test avro record should encode")
    }

    fn confluent_payload(schema_id: u32, encoded_payload: &[u8]) -> Vec<u8> {
        let mut payload = vec![0];
        payload.extend_from_slice(&schema_id.to_be_bytes());
        payload.extend_from_slice(encoded_payload);
        payload
    }

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

    fn spawn_schema_registry_server(
        expected_authorization: Option<&str>,
        status_line: &str,
        body: String,
    ) -> TestRegistryServer {
        let listener = TcpListener::bind("127.0.0.1:0")
            .expect("test registry server should bind to localhost");
        let address = listener
            .local_addr()
            .expect("test registry server should have a local address");
        let expected_authorization = expected_authorization.map(str::to_string);
        let status_line = status_line.to_string();

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener
                .accept()
                .expect("test registry server should accept one request");
            let request = read_http_request(&mut stream);

            assert!(request.starts_with("GET /schemas/ids/7 HTTP/1.1"));

            if let Some(expected_authorization) = expected_authorization {
                assert!(
                    request
                        .to_ascii_lowercase()
                        .contains(&format!("authorization: {}", expected_authorization).to_ascii_lowercase()),
                    "expected authorization header '{expected_authorization}' in request: {request}"
                );
            }

            let response = format!(
                "HTTP/1.1 {status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.as_bytes().len(),
                body,
            );
            stream
                .write_all(response.as_bytes())
                .expect("test registry server should write response");
        });

        TestRegistryServer {
            base_url: format!("http://{address}"),
            handle,
        }
    }

    fn read_http_request(stream: &mut std::net::TcpStream) -> String {
        let mut request = Vec::new();

        loop {
            let mut buffer = [0_u8; 1024];
            let read = stream
                .read(&mut buffer)
                .expect("test registry server should read request bytes");
            if read == 0 {
                break;
            }

            request.extend_from_slice(&buffer[..read]);

            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        String::from_utf8(request).expect("test registry request should be utf8")
    }

    fn test_secret_env_key(credential_ref: &str) -> String {
        let normalized = credential_ref
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect::<String>();

        format!("TRACEFORGE_TEST_SECRET_{normalized}")
    }
}
