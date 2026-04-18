use serde::Serialize;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("path error: {0}")]
    Path(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorDto {
    pub category: String,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retriable: Option<bool>,
}

impl From<AppError> for AppErrorDto {
    fn from(value: AppError) -> Self {
        match value {
            AppError::Validation(message) => Self {
                category: "validation_error".to_string(),
                code: "validation.invalid_input".to_string(),
                message,
                details: None,
                retriable: Some(false),
            },
            AppError::NotFound(message) => Self {
                category: "config_error".to_string(),
                code: "config.not_found".to_string(),
                message,
                details: None,
                retriable: Some(false),
            },
            AppError::Database(error) => Self {
                category: "internal_error".to_string(),
                code: "internal.database".to_string(),
                message: error.to_string(),
                details: None,
                retriable: Some(false),
            },
            AppError::Migration(error) => Self {
                category: "internal_error".to_string(),
                code: "internal.migration".to_string(),
                message: error.to_string(),
                details: None,
                retriable: Some(false),
            },
            AppError::Io(error) => Self {
                category: "internal_error".to_string(),
                code: "internal.io".to_string(),
                message: error.to_string(),
                details: None,
                retriable: Some(false),
            },
            AppError::Path(message) => Self {
                category: "config_error".to_string(),
                code: "internal.path".to_string(),
                message,
                details: None,
                retriable: Some(false),
            },
            AppError::Network(message) => Self {
                category: "connectivity_error".to_string(),
                code: "connection.unreachable".to_string(),
                message,
                details: None,
                retriable: Some(true),
            },
            AppError::Internal(message) => Self {
                category: "internal_error".to_string(),
                code: "internal.unexpected".to_string(),
                message,
                details: None,
                retriable: Some(false),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppError, AppErrorDto};

    #[test]
    fn maps_validation_errors_to_non_retriable_dto() {
        let dto = AppErrorDto::from(AppError::Validation("name is required".to_string()));

        assert_eq!(dto.category, "validation_error");
        assert_eq!(dto.code, "validation.invalid_input");
        assert_eq!(dto.message, "name is required");
        assert_eq!(dto.retriable, Some(false));
        assert!(dto.details.is_none());
    }

    #[test]
    fn maps_network_errors_to_retriable_dto() {
        let dto = AppErrorDto::from(AppError::Network("cluster unreachable".to_string()));

        assert_eq!(dto.category, "connectivity_error");
        assert_eq!(dto.code, "connection.unreachable");
        assert_eq!(dto.message, "cluster unreachable");
        assert_eq!(dto.retriable, Some(true));
        assert!(dto.details.is_none());
    }
}
