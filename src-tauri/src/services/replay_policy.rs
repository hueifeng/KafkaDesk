use crate::models::error::{AppError, AppResult};
use crate::models::replay_policy::{ReplayPolicyDto, UpdateReplayPolicyRequest};
use crate::repositories::sqlite;
use chrono::Utc;
use serde_json::json;
use sqlx::SqlitePool;

const DEFAULT_SANDBOX_PREFIX: &str = "sandbox.";
const DEFAULT_DELIVERY_TIMEOUT_SECONDS: u64 = 7;
const DEFAULT_MAX_RETRY_ATTEMPTS: u32 = 1;

pub struct ReplayPolicyService<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ReplayPolicyService<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_replay_policy(&self) -> AppResult<ReplayPolicyDto> {
        let values = sqlite::list_app_preferences(self.pool).await?;
        Ok(map_replay_policy(values))
    }

    pub async fn update_replay_policy(
        &self,
        request: UpdateReplayPolicyRequest,
    ) -> AppResult<ReplayPolicyDto> {
        validate_replay_policy_request(&request)?;

        let updated_at = Utc::now().to_rfc3339();
        sqlite::upsert_app_preference(
            self.pool,
            "replayAllowLiveReplay",
            json!(request.allow_live_replay),
            &updated_at,
        )
        .await?;
        sqlite::upsert_app_preference(
            self.pool,
            "replaySandboxOnly",
            json!(request.sandbox_only),
            &updated_at,
        )
        .await?;
        sqlite::upsert_app_preference(
            self.pool,
            "replaySandboxTopicPrefix",
            json!(request.sandbox_topic_prefix),
            &updated_at,
        )
        .await?;
        sqlite::upsert_app_preference(
            self.pool,
            "replayDeliveryTimeoutSeconds",
            json!(request.delivery_timeout_seconds),
            &updated_at,
        )
        .await?;
        sqlite::upsert_app_preference(
            self.pool,
            "replayMaxRetryAttempts",
            json!(request.max_retry_attempts),
            &updated_at,
        )
        .await?;
        sqlite::upsert_app_preference(
            self.pool,
            "replayRequireRiskAcknowledgement",
            json!(request.require_risk_acknowledgement),
            &updated_at,
        )
        .await?;

        self.get_replay_policy().await
    }
}

pub fn validate_replay_policy_request(request: &UpdateReplayPolicyRequest) -> AppResult<()> {
    if request.sandbox_only && request.sandbox_topic_prefix.trim().is_empty() {
        return Err(AppError::Validation(
            "sandbox topic prefix is required when sandbox-only mode is enabled".to_string(),
        ));
    }

    if request.delivery_timeout_seconds == 0 || request.delivery_timeout_seconds > 60 {
        return Err(AppError::Validation(
            "delivery timeout seconds must be between 1 and 60".to_string(),
        ));
    }

    if request.max_retry_attempts == 0 || request.max_retry_attempts > 5 {
        return Err(AppError::Validation(
            "max retry attempts must be between 1 and 5".to_string(),
        ));
    }

    Ok(())
}

fn map_replay_policy(values: Vec<(String, serde_json::Value)>) -> ReplayPolicyDto {
    let mut allow_live_replay = true;
    let mut sandbox_only = true;
    let mut sandbox_topic_prefix = DEFAULT_SANDBOX_PREFIX.to_string();
    let mut require_risk_acknowledgement = true;
    let mut delivery_timeout_seconds = DEFAULT_DELIVERY_TIMEOUT_SECONDS;
    let mut max_retry_attempts = DEFAULT_MAX_RETRY_ATTEMPTS;

    for (key, value) in values {
        match key.as_str() {
            "replayAllowLiveReplay" => {
                if let Some(raw) = value.as_bool() {
                    allow_live_replay = raw;
                }
            }
            "replaySandboxOnly" => {
                if let Some(raw) = value.as_bool() {
                    sandbox_only = raw;
                }
            }
            "replaySandboxTopicPrefix" => {
                if let Some(raw) = value.as_str() {
                    sandbox_topic_prefix = raw.to_string();
                }
            }
            "replayRequireRiskAcknowledgement" => {
                if let Some(raw) = value.as_bool() {
                    require_risk_acknowledgement = raw;
                }
            }
            "replayDeliveryTimeoutSeconds" => {
                if let Some(raw) = value.as_u64() {
                    delivery_timeout_seconds = raw;
                }
            }
            "replayMaxRetryAttempts" => {
                if let Some(raw) = value.as_u64() {
                    max_retry_attempts = raw as u32;
                }
            }
            _ => {}
        }
    }

    ReplayPolicyDto {
        allow_live_replay,
        sandbox_only,
        sandbox_topic_prefix,
        require_risk_acknowledgement,
        delivery_timeout_seconds,
        max_retry_attempts,
    }
}

#[cfg(test)]
mod tests {
    use super::validate_replay_policy_request;
    use crate::models::replay_policy::UpdateReplayPolicyRequest;

    fn valid_request() -> UpdateReplayPolicyRequest {
        UpdateReplayPolicyRequest {
            allow_live_replay: true,
            sandbox_only: true,
            sandbox_topic_prefix: "sandbox.".to_string(),
            require_risk_acknowledgement: true,
            delivery_timeout_seconds: 7,
            max_retry_attempts: 2,
        }
    }

    #[test]
    fn rejects_zero_delivery_timeout() {
        let mut request = valid_request();
        request.delivery_timeout_seconds = 0;
        let error = validate_replay_policy_request(&request).expect_err("zero timeout must fail");
        assert!(error.to_string().contains("delivery timeout seconds"));
    }

    #[test]
    fn rejects_zero_retry_attempts() {
        let mut request = valid_request();
        request.max_retry_attempts = 0;
        let error = validate_replay_policy_request(&request).expect_err("zero retries must fail");
        assert!(error.to_string().contains("max retry attempts"));
    }
}
