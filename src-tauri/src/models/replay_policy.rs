use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReplayPolicyDto {
    pub allow_live_replay: bool,
    pub sandbox_only: bool,
    pub sandbox_topic_prefix: String,
    pub require_risk_acknowledgement: bool,
    pub delivery_timeout_seconds: u64,
    pub max_retry_attempts: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReplayPolicyRequest {
    pub allow_live_replay: bool,
    pub sandbox_only: bool,
    pub sandbox_topic_prefix: String,
    pub require_risk_acknowledgement: bool,
    pub delivery_timeout_seconds: u64,
    pub max_retry_attempts: u32,
}
