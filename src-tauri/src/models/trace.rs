use crate::models::message::{MessageRefDto, TimeRangeDto};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RunTraceQueryRequest {
    pub cluster_profile_id: String,
    pub key_type: String,
    pub key_value: String,
    #[serde(default)]
    pub topic_scope: Option<Vec<String>>,
    pub time_range: TimeRangeDto,
    #[serde(default)]
    pub result_mode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TraceEventDto {
    pub message_ref: MessageRefDto,
    pub timestamp: String,
    pub topic: String,
    pub partition: i32,
    pub offset: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_preview: Option<String>,
    pub matched_by: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TraceQuerySummaryDto {
    pub key_type: String,
    pub key_value: String,
    pub scanned_topics: Vec<String>,
    pub matched_count: usize,
    pub result_mode: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TraceQueryResultDto {
    pub query_summary: TraceQuerySummaryDto,
    pub events: Vec<TraceEventDto>,
    pub timeline: Vec<TraceEventDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_notes: Option<Vec<String>>,
}
