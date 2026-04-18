use crate::models::validation::{ValidationStageDto, ValidationStatusDto};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SchemaRegistryProfileDto {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub auth_mode: String,
    pub credential_ref: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateSchemaRegistryProfileRequest {
    pub name: String,
    pub base_url: String,
    pub auth_mode: String,
    #[serde(default)]
    pub credential_ref: Option<String>,
    #[serde(default)]
    pub credential_secret: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSchemaRegistryProfileRequest {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub auth_mode: String,
    #[serde(default)]
    pub credential_ref: Option<String>,
    #[serde(default)]
    pub credential_secret: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SchemaRegistryConnectionTestRequest {
    #[serde(default)]
    pub profile_id: Option<String>,
    pub name: String,
    pub base_url: String,
    pub auth_mode: String,
    #[serde(default)]
    pub credential_ref: Option<String>,
    #[serde(default)]
    pub credential_secret: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SchemaRegistryConnectionTestResponse {
    pub ok: bool,
    pub status: ValidationStatusDto,
    pub target: String,
    pub message: String,
    pub stages: Vec<ValidationStageDto>,
}

impl SchemaRegistryProfileDto {
    pub fn new(request: CreateSchemaRegistryProfileRequest) -> Self {
        let now = Utc::now().to_rfc3339();

        Self {
            id: Uuid::new_v4().to_string(),
            name: request.name,
            base_url: request.base_url,
            auth_mode: request.auth_mode,
            credential_ref: request.credential_ref,
            notes: request.notes,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}
