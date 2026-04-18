use crate::models::message::MessageRefDto;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageBookmarkDto {
    pub id: String,
    pub message_ref: MessageRefDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ListMessageBookmarksRequest {
    #[serde(default)]
    pub cluster_profile_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageBookmarkRequest {
    pub message_ref: MessageRefDto,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

impl MessageBookmarkDto {
    pub fn new(request: CreateMessageBookmarkRequest) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_ref: request.message_ref,
            label: request
                .label
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            notes: request
                .notes
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            created_at: Utc::now().to_rfc3339(),
        }
    }
}
