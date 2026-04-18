use crate::models::error::{AppError, AppResult};
use crate::models::saved_query::{CreateSavedQueryRequest, SavedQueryDto, UpdateSavedQueryRequest};
use crate::repositories::sqlite;
use chrono::Utc;
use serde_json::Value;
use sqlx::SqlitePool;

const SUPPORTED_QUERY_TYPE: &str = "messages";

pub struct SavedQueriesService<'a> {
    pool: &'a SqlitePool,
}

impl<'a> SavedQueriesService<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_saved_queries(&self) -> AppResult<Vec<SavedQueryDto>> {
        sqlite::list_saved_queries(self.pool).await
    }

    pub async fn create_saved_query(
        &self,
        request: CreateSavedQueryRequest,
    ) -> AppResult<SavedQueryDto> {
        validate_saved_query_request(
            self.pool,
            &request.name,
            &request.query_type,
            &request.cluster_profile_id,
            &request.scope_json,
            &request.query_json,
        )
        .await?;

        let record = SavedQueryDto::new(request);
        sqlite::insert_saved_query(self.pool, &record).await?;
        Ok(record)
    }

    pub async fn update_saved_query(
        &self,
        request: UpdateSavedQueryRequest,
    ) -> AppResult<SavedQueryDto> {
        if request.id.trim().is_empty() {
            return Err(AppError::Validation(
                "saved query id is required".to_string(),
            ));
        }

        validate_saved_query_request(
            self.pool,
            &request.name,
            &request.query_type,
            &request.cluster_profile_id,
            &request.scope_json,
            &request.query_json,
        )
        .await?;
        let updated_at = Utc::now().to_rfc3339();
        sqlite::update_saved_query(self.pool, &request, &updated_at).await?;
        sqlite::get_saved_query(self.pool, &request.id).await
    }

    pub async fn delete_saved_query(&self, id: &str) -> AppResult<()> {
        if id.trim().is_empty() {
            return Err(AppError::Validation(
                "saved query id is required".to_string(),
            ));
        }
        sqlite::delete_saved_query(self.pool, id).await
    }
}

async fn validate_saved_query_request(
    pool: &SqlitePool,
    name: &str,
    query_type: &str,
    cluster_profile_id: &str,
    scope_json: &str,
    query_json: &str,
) -> AppResult<()> {
    if name.trim().is_empty() {
        return Err(AppError::Validation(
            "saved query name is required".to_string(),
        ));
    }
    if query_type != SUPPORTED_QUERY_TYPE {
        return Err(AppError::Validation(
            "only messages saved queries are supported right now".to_string(),
        ));
    }
    if cluster_profile_id.trim().is_empty() {
        return Err(AppError::Validation(
            "cluster profile id is required".to_string(),
        ));
    }
    sqlite::get_cluster_profile(pool, cluster_profile_id).await?;

    let scope = parse_json(scope_json, "scope_json")?;
    let query = parse_json(query_json, "query_json")?;

    let topic = scope
        .get("topic")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if topic.is_empty() {
        return Err(AppError::Validation(
            "messages saved query requires a topic in scope_json".to_string(),
        ));
    }

    let max_results = query.get("maxResults").and_then(Value::as_u64).unwrap_or(0);
    if max_results == 0 || max_results > 500 {
        return Err(AppError::Validation(
            "messages saved query maxResults must be between 1 and 500".to_string(),
        ));
    }

    Ok(())
}

fn parse_json(value: &str, field_name: &str) -> AppResult<Value> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!("{field_name} is required")));
    }

    serde_json::from_str::<Value>(value)
        .map_err(|error| AppError::Validation(format!("{field_name} must be valid JSON: {error}")))
}
