use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferencesDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_cluster_id: Option<String>,
    pub table_density: String,
    pub default_message_query_window_minutes: u32,
    pub preferred_trace_view: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAppPreferencesRequest {
    #[serde(default)]
    pub preferred_cluster_id: Option<String>,
    pub table_density: String,
    pub default_message_query_window_minutes: u32,
    pub preferred_trace_view: String,
}
