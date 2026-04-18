use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CorrelationRuleDto {
    pub id: String,
    pub name: String,
    pub cluster_profile_id: String,
    pub is_enabled: bool,
    pub match_strategy: String,
    pub scope_json: String,
    pub rule_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateCorrelationRuleRequest {
    pub name: String,
    pub cluster_profile_id: String,
    pub is_enabled: bool,
    pub match_strategy: String,
    pub scope_json: String,
    pub rule_json: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCorrelationRuleRequest {
    pub id: String,
    pub name: String,
    pub cluster_profile_id: String,
    pub is_enabled: bool,
    pub match_strategy: String,
    pub scope_json: String,
    pub rule_json: String,
}

impl CorrelationRuleDto {
    pub fn new(request: CreateCorrelationRuleRequest) -> Self {
        let now = Utc::now().to_rfc3339();

        Self {
            id: Uuid::new_v4().to_string(),
            name: request.name,
            cluster_profile_id: request.cluster_profile_id,
            is_enabled: request.is_enabled,
            match_strategy: request.match_strategy,
            scope_json: request.scope_json,
            rule_json: request.rule_json,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}
