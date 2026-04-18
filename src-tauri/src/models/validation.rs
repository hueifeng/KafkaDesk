use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ValidationStatusDto {
    Passed,
    Warning,
    Failed,
    Skipped,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ValidationStageDto {
    pub key: String,
    pub label: String,
    pub status: ValidationStatusDto,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retriable: Option<bool>,
}

impl ValidationStageDto {
    pub fn passed(key: &str, label: &str, message: impl Into<String>) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
            status: ValidationStatusDto::Passed,
            message: message.into(),
            detail: None,
            error_category: None,
            retriable: Some(false),
        }
    }

    pub fn warning(key: &str, label: &str, message: impl Into<String>) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
            status: ValidationStatusDto::Warning,
            message: message.into(),
            detail: None,
            error_category: None,
            retriable: Some(false),
        }
    }

    pub fn failed(
        key: &str,
        label: &str,
        message: impl Into<String>,
        error_category: impl Into<String>,
        retriable: bool,
    ) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
            status: ValidationStatusDto::Failed,
            message: message.into(),
            detail: None,
            error_category: Some(error_category.into()),
            retriable: Some(retriable),
        }
    }

    pub fn skipped(key: &str, label: &str, message: impl Into<String>) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
            status: ValidationStatusDto::Skipped,
            message: message.into(),
            detail: None,
            error_category: None,
            retriable: Some(false),
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

pub fn summarize_validation_status(stages: &[ValidationStageDto]) -> ValidationStatusDto {
    let has_failed = stages
        .iter()
        .any(|stage| stage.status == ValidationStatusDto::Failed);
    let has_warning = stages
        .iter()
        .any(|stage| stage.status == ValidationStatusDto::Warning);
    let has_passed = stages
        .iter()
        .any(|stage| stage.status == ValidationStatusDto::Passed);

    if has_failed {
        if has_passed || has_warning {
            ValidationStatusDto::Warning
        } else {
            ValidationStatusDto::Failed
        }
    } else if has_warning {
        ValidationStatusDto::Warning
    } else {
        ValidationStatusDto::Passed
    }
}
