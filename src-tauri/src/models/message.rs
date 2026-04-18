use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HeaderFilterDto {
    pub key: String,
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimeRangeDto {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OffsetRangeDto {
    #[serde(default)]
    pub start_offset: Option<String>,
    #[serde(default)]
    pub end_offset: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryMessagesRequest {
    pub cluster_profile_id: String,
    pub topic: String,
    #[serde(default)]
    pub partitions: Option<Vec<i32>>,
    #[serde(default)]
    pub time_range: Option<TimeRangeDto>,
    #[serde(default)]
    pub offset_range: Option<OffsetRangeDto>,
    #[serde(default)]
    pub key_filter: Option<String>,
    #[serde(default)]
    pub header_filters: Option<Vec<HeaderFilterDto>>,
    pub max_results: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageRefDto {
    pub cluster_profile_id: String,
    pub topic: String,
    pub partition: i32,
    pub offset: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageSummaryDto {
    pub message_ref: MessageRefDto,
    pub timestamp: String,
    pub partition: i32,
    pub offset: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_preview: Option<String>,
    pub decode_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_preview: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageHeaderDto {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GetMessageDetailRequest {
    pub message_ref: MessageRefDto,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageDetailResponseDto {
    pub message_ref: MessageRefDto,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_raw: Option<String>,
    pub headers: Vec<MessageHeaderDto>,
    pub payload_raw: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_decoded: Option<String>,
    pub decode_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_hints: Option<Vec<String>>,
}
