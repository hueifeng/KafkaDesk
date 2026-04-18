use crate::models::error::{AppError, AppResult};
use crate::models::preferences::{AppPreferencesDto, UpdateAppPreferencesRequest};
use crate::repositories::sqlite;
use chrono::Utc;
use serde_json::json;
use sqlx::SqlitePool;

const DEFAULT_QUERY_WINDOW_MINUTES: u32 = 30;
const MIN_QUERY_WINDOW_MINUTES: u32 = 5;
const MAX_QUERY_WINDOW_MINUTES: u32 = 240;
const DEFAULT_TABLE_DENSITY: &str = "compact";
const DEFAULT_TRACE_VIEW: &str = "timeline";

pub struct PreferencesService<'a> {
    pool: &'a SqlitePool,
}

impl<'a> PreferencesService<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_app_preferences(&self) -> AppResult<AppPreferencesDto> {
        let values = sqlite::list_app_preferences(self.pool).await?;

        Ok(map_preferences(values))
    }

    pub async fn update_app_preferences(
        &self,
        request: UpdateAppPreferencesRequest,
    ) -> AppResult<AppPreferencesDto> {
        validate_preferences_request(&request)?;

        let updated_at = Utc::now().to_rfc3339();

        sqlite::upsert_app_preference(
            self.pool,
            "preferredClusterId",
            request
                .preferred_cluster_id
                .as_ref()
                .map(|value| json!(value))
                .unwrap_or(serde_json::Value::Null),
            &updated_at,
        )
        .await?;
        sqlite::upsert_app_preference(
            self.pool,
            "tableDensity",
            json!(request.table_density),
            &updated_at,
        )
        .await?;
        sqlite::upsert_app_preference(
            self.pool,
            "defaultMessageQueryWindowMinutes",
            json!(request.default_message_query_window_minutes),
            &updated_at,
        )
        .await?;
        sqlite::upsert_app_preference(
            self.pool,
            "preferredTraceView",
            json!(request.preferred_trace_view),
            &updated_at,
        )
        .await?;

        self.get_app_preferences().await
    }
}

fn validate_preferences_request(request: &UpdateAppPreferencesRequest) -> AppResult<()> {
    if !matches!(request.table_density.as_str(), "compact" | "comfortable") {
        return Err(AppError::Validation(
            "table density must be compact or comfortable".to_string(),
        ));
    }

    if !(MIN_QUERY_WINDOW_MINUTES..=MAX_QUERY_WINDOW_MINUTES)
        .contains(&request.default_message_query_window_minutes)
    {
        return Err(AppError::Validation(format!(
            "default message query window must be between {MIN_QUERY_WINDOW_MINUTES} and {MAX_QUERY_WINDOW_MINUTES} minutes"
        )));
    }

    if !matches!(request.preferred_trace_view.as_str(), "timeline" | "table") {
        return Err(AppError::Validation(
            "preferred trace view must be timeline or table".to_string(),
        ));
    }

    Ok(())
}

fn map_preferences(values: Vec<(String, serde_json::Value)>) -> AppPreferencesDto {
    let mut preferred_cluster_id = None;
    let mut table_density = DEFAULT_TABLE_DENSITY.to_string();
    let mut default_message_query_window_minutes = DEFAULT_QUERY_WINDOW_MINUTES;
    let mut preferred_trace_view = DEFAULT_TRACE_VIEW.to_string();

    for (key, value) in values {
        match key.as_str() {
            "preferredClusterId" => {
                preferred_cluster_id = value.as_str().map(ToString::to_string);
            }
            "tableDensity" => {
                if let Some(raw) = value.as_str() {
                    table_density = raw.to_string();
                }
            }
            "defaultMessageQueryWindowMinutes" => {
                if let Some(raw) = value.as_u64() {
                    default_message_query_window_minutes = raw as u32;
                }
            }
            "preferredTraceView" => {
                if let Some(raw) = value.as_str() {
                    preferred_trace_view = raw.to_string();
                }
            }
            _ => {}
        }
    }

    AppPreferencesDto {
        preferred_cluster_id,
        table_density,
        default_message_query_window_minutes,
        preferred_trace_view,
    }
}
