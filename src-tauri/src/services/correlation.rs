use crate::models::correlation::{
    CorrelationRuleDto, CreateCorrelationRuleRequest, UpdateCorrelationRuleRequest,
};
use crate::models::error::{AppError, AppResult};
use crate::repositories::sqlite;
use chrono::Utc;
use sqlx::SqlitePool;

const ALLOWED_STRATEGIES: &[&str] = &[
    "header-match",
    "key-match",
    "decoded-field-match",
    "ordered-multi-topic",
];

pub struct CorrelationService<'a> {
    pool: &'a SqlitePool,
}

impl<'a> CorrelationService<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_rules(&self) -> AppResult<Vec<CorrelationRuleDto>> {
        sqlite::list_correlation_rules(self.pool).await
    }

    pub async fn create_rule(
        &self,
        request: CreateCorrelationRuleRequest,
    ) -> AppResult<CorrelationRuleDto> {
        validate_rule_request(
            self.pool,
            &request.name,
            &request.cluster_profile_id,
            &request.match_strategy,
            &request.scope_json,
            &request.rule_json,
        )
        .await?;

        let rule = CorrelationRuleDto::new(request);
        sqlite::insert_correlation_rule(self.pool, &rule).await?;
        Ok(rule)
    }

    pub async fn update_rule(
        &self,
        request: UpdateCorrelationRuleRequest,
    ) -> AppResult<CorrelationRuleDto> {
        if request.id.trim().is_empty() {
            return Err(AppError::Validation(
                "correlation rule id is required".to_string(),
            ));
        }

        validate_rule_request(
            self.pool,
            &request.name,
            &request.cluster_profile_id,
            &request.match_strategy,
            &request.scope_json,
            &request.rule_json,
        )
        .await?;
        let updated_at = Utc::now().to_rfc3339();

        sqlite::update_correlation_rule(self.pool, &request, &updated_at).await?;
        sqlite::get_correlation_rule(self.pool, &request.id).await
    }
}

async fn validate_rule_request(
    pool: &SqlitePool,
    name: &str,
    cluster_profile_id: &str,
    match_strategy: &str,
    scope_json: &str,
    rule_json: &str,
) -> AppResult<()> {
    if name.trim().is_empty() {
        return Err(AppError::Validation(
            "correlation rule name is required".to_string(),
        ));
    }

    if cluster_profile_id.trim().is_empty() {
        return Err(AppError::Validation(
            "cluster scope is required".to_string(),
        ));
    }
    sqlite::get_cluster_profile(pool, cluster_profile_id).await?;

    if !ALLOWED_STRATEGIES.contains(&match_strategy) {
        return Err(AppError::Validation(
            "unsupported correlation strategy".to_string(),
        ));
    }

    validate_json_field(scope_json, "scope_json")?;
    validate_json_field(rule_json, "rule_json")?;

    Ok(())
}

fn validate_json_field(value: &str, field_name: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!("{field_name} is required")));
    }

    serde_json::from_str::<serde_json::Value>(value).map_err(|error| {
        AppError::Validation(format!("{field_name} must be valid JSON: {error}"))
    })?;

    Ok(())
}
