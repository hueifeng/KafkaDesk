use crate::models::validation::{ValidationStageDto, ValidationStatusDto};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClusterProfileSummaryDto {
    pub id: String,
    pub name: String,
    pub environment: String,
    pub bootstrap_servers: String,
    pub auth_mode: String,
    pub auth_credential_ref: Option<String>,
    pub tls_mode: String,
    pub tls_ca_cert_path: Option<String>,
    pub tls_client_cert_path: Option<String>,
    pub schema_registry_profile_id: Option<String>,
    pub is_favorite: bool,
    pub last_connected_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClusterProfileDto {
    pub id: String,
    pub name: String,
    pub environment: String,
    pub bootstrap_servers: String,
    pub auth_mode: String,
    pub auth_credential_ref: Option<String>,
    pub tls_mode: String,
    pub tls_ca_cert_path: Option<String>,
    pub tls_client_cert_path: Option<String>,
    pub tls_client_key_path: Option<String>,
    pub schema_registry_profile_id: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub is_favorite: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_connected_at: Option<String>,
    pub is_archived: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateClusterProfileRequest {
    pub name: String,
    pub environment: String,
    pub bootstrap_servers: String,
    pub auth_mode: String,
    pub auth_credential_ref: Option<String>,
    pub auth_secret: Option<String>,
    pub tls_mode: String,
    pub tls_ca_cert_path: Option<String>,
    pub tls_client_cert_path: Option<String>,
    pub tls_client_key_path: Option<String>,
    pub schema_registry_profile_id: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateClusterProfileRequest {
    pub id: String,
    pub name: String,
    pub environment: String,
    pub bootstrap_servers: String,
    pub auth_mode: String,
    pub auth_credential_ref: Option<String>,
    pub auth_secret: Option<String>,
    pub tls_mode: String,
    pub tls_ca_cert_path: Option<String>,
    pub tls_client_cert_path: Option<String>,
    pub tls_client_key_path: Option<String>,
    pub schema_registry_profile_id: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub is_favorite: bool,
    pub is_archived: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClusterConnectionTestRequest {
    pub profile_id: Option<String>,
    pub name: String,
    pub environment: String,
    pub bootstrap_servers: String,
    pub auth_mode: String,
    pub auth_credential_ref: Option<String>,
    pub auth_secret: Option<String>,
    pub tls_mode: String,
    pub tls_ca_cert_path: Option<String>,
    pub tls_client_cert_path: Option<String>,
    pub tls_client_key_path: Option<String>,
    pub schema_registry_profile_id: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClusterConnectionTestResponse {
    pub ok: bool,
    pub status: ValidationStatusDto,
    pub attempted_brokers: usize,
    pub reachable_brokers: usize,
    pub message: String,
    pub stages: Vec<ValidationStageDto>,
}

impl ClusterProfileDto {
    pub fn new(request: CreateClusterProfileRequest) -> Self {
        let now = Utc::now().to_rfc3339();

        Self {
            id: Uuid::new_v4().to_string(),
            name: request.name,
            environment: request.environment,
            bootstrap_servers: request.bootstrap_servers,
            auth_mode: request.auth_mode,
            auth_credential_ref: request.auth_credential_ref,
            tls_mode: request.tls_mode,
            tls_ca_cert_path: request.tls_ca_cert_path,
            tls_client_cert_path: request.tls_client_cert_path,
            tls_client_key_path: request.tls_client_key_path,
            schema_registry_profile_id: request.schema_registry_profile_id,
            notes: request.notes,
            tags: request.tags,
            is_favorite: false,
            created_at: now.clone(),
            updated_at: now,
            last_connected_at: None,
            is_archived: false,
        }
    }
}
