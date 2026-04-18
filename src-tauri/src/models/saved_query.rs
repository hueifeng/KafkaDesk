use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SavedQueryDto {
    pub id: String,
    pub name: String,
    pub query_type: String,
    pub cluster_profile_id: String,
    pub scope_json: String,
    pub query_json: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub is_favorite: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateSavedQueryRequest {
    pub name: String,
    pub query_type: String,
    pub cluster_profile_id: String,
    pub scope_json: String,
    pub query_json: String,
    #[serde(default)]
    pub description: Option<String>,
    pub is_favorite: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSavedQueryRequest {
    pub id: String,
    pub name: String,
    pub query_type: String,
    pub cluster_profile_id: String,
    pub scope_json: String,
    pub query_json: String,
    #[serde(default)]
    pub description: Option<String>,
    pub is_favorite: bool,
    #[serde(default)]
    pub last_run_at: Option<String>,
}

impl SavedQueryDto {
    pub fn new(request: CreateSavedQueryRequest) -> Self {
        let now = Utc::now().to_rfc3339();

        Self {
            id: Uuid::new_v4().to_string(),
            name: request.name,
            query_type: request.query_type,
            cluster_profile_id: request.cluster_profile_id,
            scope_json: request.scope_json,
            query_json: request.query_json,
            description: request
                .description
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            is_favorite: request.is_favorite,
            created_at: now.clone(),
            updated_at: now,
            last_run_at: None,
        }
    }
}
