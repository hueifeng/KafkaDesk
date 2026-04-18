use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ListAuditEventsRequest {
    #[serde(default)]
    pub cluster_profile_id: Option<String>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub start_at: Option<String>,
    #[serde(default)]
    pub end_at: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventSummaryDto {
    pub id: String,
    pub created_at: String,
    pub event_type: String,
    pub target_type: String,
    pub summary: String,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_ref: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventDetailDto {
    pub id: String,
    pub created_at: String,
    pub event_type: String,
    pub target_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_profile_id: Option<String>,
    pub outcome: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details_json: Option<String>,
}
