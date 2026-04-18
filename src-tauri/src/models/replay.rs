use serde::{Deserialize, Serialize};

use crate::models::message::{MessageHeaderDto, MessageRefDto};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateReplayJobRequest {
    pub cluster_profile_id: String,
    pub source_message_ref: MessageRefDto,
    #[serde(default)]
    pub source_timestamp: Option<String>,
    pub target_topic: String,
    #[serde(default)]
    pub edited_key: Option<String>,
    #[serde(default)]
    pub edited_headers: Option<Vec<MessageHeaderDto>>,
    #[serde(default)]
    pub edited_payload: Option<String>,
    pub dry_run: bool,
    pub risk_acknowledged: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReplayJobSummaryDto {
    pub id: String,
    pub status: String,
    pub mode: String,
    pub target_topic: String,
    pub source_topic: String,
    pub source_partition: i32,
    pub source_offset: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_timestamp: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    pub risk_level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_summary_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_edit_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers_edit_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_edit_json: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReplayJobEventDto {
    pub id: String,
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_payload_json: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReplayJobDetailResponseDto {
    pub job: ReplayJobSummaryDto,
    pub event_history: Vec<ReplayJobEventDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_ref: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReplayJobRecord {
    pub id: String,
    pub cluster_profile_id: String,
    pub source_topic: String,
    pub source_partition: i32,
    pub source_offset: i64,
    pub source_timestamp: Option<String>,
    pub target_topic: String,
    pub status: String,
    pub mode: String,
    pub payload_edit_json: Option<String>,
    pub headers_edit_json: Option<String>,
    pub key_edit_json: Option<String>,
    pub dry_run: bool,
    pub requested_by_profile: Option<String>,
    pub risk_level: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub result_summary_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuditEventRecord {
    pub id: String,
    pub event_type: String,
    pub target_type: String,
    pub target_ref: Option<String>,
    pub actor_profile: Option<String>,
    pub cluster_profile_id: Option<String>,
    pub outcome: String,
    pub summary: String,
    pub details_json: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ReplayJobRecoveryCandidate {
    pub id: String,
    pub cluster_profile_id: String,
    pub target_topic: String,
    pub started_at: Option<String>,
}
