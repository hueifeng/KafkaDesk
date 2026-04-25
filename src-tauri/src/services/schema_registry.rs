use crate::models::error::{AppError, AppResult};
use crate::models::schema_registry::{
    CreateSchemaRegistryProfileRequest, SchemaRegistryConnectionTestRequest,
    SchemaRegistryConnectionTestResponse, SchemaRegistryProfileDto,
    UpdateSchemaRegistryProfileRequest,
};
use crate::models::validation::{
    summarize_validation_status, ValidationStageDto, ValidationStatusDto,
};
use crate::repositories::sqlite;
use chrono::Utc;
use reqwest::{Client, StatusCode};
use sqlx::SqlitePool;
use tokio::time::Duration;

use super::connectivity;
use super::credentials::{
    resolve_schema_registry_auth, store_runtime_secret, ResolvedSchemaRegistryAuth,
};

pub struct SchemaRegistryService<'a> {
    pool: &'a SqlitePool,
}

impl<'a> SchemaRegistryService<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_profiles(&self) -> AppResult<Vec<SchemaRegistryProfileDto>> {
        sqlite::list_schema_registry_profiles(self.pool).await
    }

    pub async fn create_profile(
        &self,
        request: CreateSchemaRegistryProfileRequest,
    ) -> AppResult<SchemaRegistryProfileDto> {
        validate_schema_registry_request(&request.name, &request.base_url, &request.auth_mode)?;

        if request.auth_mode != "none" {
            if let (Some(reference), Some(secret)) = (
                request
                    .credential_ref
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
                request
                    .credential_secret
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
            ) {
                store_runtime_secret(reference, secret)?;
            }
        }

        let profile = SchemaRegistryProfileDto::new(request);
        sqlite::insert_schema_registry_profile(self.pool, &profile).await?;
        Ok(profile)
    }

    pub async fn update_profile(
        &self,
        request: UpdateSchemaRegistryProfileRequest,
    ) -> AppResult<SchemaRegistryProfileDto> {
        validate_schema_registry_request(&request.name, &request.base_url, &request.auth_mode)?;

        if request.auth_mode != "none" {
            if let (Some(reference), Some(secret)) = (
                request
                    .credential_ref
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
                request
                    .credential_secret
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
            ) {
                store_runtime_secret(reference, secret)?;
            }
        }

        let updated_at = Utc::now().to_rfc3339();

        sqlite::update_schema_registry_profile(self.pool, &request, &updated_at).await?;
        sqlite::get_schema_registry_profile(self.pool, &request.id).await
    }

    pub async fn test_connection(
        &self,
        request: SchemaRegistryConnectionTestRequest,
    ) -> AppResult<SchemaRegistryConnectionTestResponse> {
        let mut stages = Vec::new();

        if let Err(error) =
            validate_schema_registry_request(&request.name, &request.base_url, &request.auth_mode)
        {
            stages.push(validation_error_stage("profile-input", "配置校验", error));

            return Ok(SchemaRegistryConnectionTestResponse {
                ok: false,
                status: ValidationStatusDto::Failed,
                target: "未解析".to_string(),
                message: "模式注册表校验未通过：请先修正配置输入。".to_string(),
                stages,
            });
        }

        stages.push(
            ValidationStageDto::passed(
                "profile-input",
                "配置校验",
                "名称、Base URL 和认证方式已通过语法校验。",
            )
            .with_detail("当前仅对无认证的实际 API 探活提供完整支持。"),
        );

        let endpoint = normalize_registry_endpoint(&request.base_url)?;

        let (credential_stage, resolved_auth) = evaluate_registry_credentials(
            &request.auth_mode,
            request.credential_ref.as_deref(),
            request.credential_secret.as_deref(),
        );
        let auth_ready = credential_stage.status == ValidationStatusDto::Passed;
        stages.push(credential_stage);

        let endpoint_probe = connectivity::probe_tcp_target(&endpoint.target).await;
        if endpoint_probe.reachable {
            stages.push(
                ValidationStageDto::passed(
                    "endpoint-reachability",
                    "Endpoint 连通性",
                    format!("Schema Registry 端点 {} 可建立 TCP 连接。", endpoint.target),
                )
                .with_detail(endpoint_probe.detail),
            );
        } else {
            stages.push(
                ValidationStageDto::failed(
                    "endpoint-reachability",
                    "Endpoint 连通性",
                    format!("Schema Registry 端点 {} 当前不可达。", endpoint.target),
                    "connectivity_error",
                    true,
                )
                .with_detail(endpoint_probe.detail),
            );
        }

        if endpoint_probe.reachable && auth_ready {
            match probe_registry_api(&endpoint.base_url, resolved_auth.as_ref()).await {
                Ok(api_detail) => stages.push(
                    ValidationStageDto::passed(
                        "registry-api",
                        "Registry API",
                        "Schema Registry API 探活成功。",
                    )
                    .with_detail(api_detail),
                ),
                Err(error) => {
                    let (category, code, retriable) = classify_registry_error(&error);
                    stages.push(
                        ValidationStageDto::failed(
                            "registry-api",
                            "Registry API",
                            error.to_string(),
                            category,
                            retriable,
                        )
                        .with_detail(format!("错误代码：{}", code)),
                    );
                }
            }
        } else {
            stages.push(ValidationStageDto::skipped(
                "registry-api",
                "Registry API",
                "前置条件未满足，已跳过 Schema Registry API 校验。",
            ));
        }

        let status = summarize_validation_status(&stages);
        let ok = status == ValidationStatusDto::Passed;
        let message = match status {
            ValidationStatusDto::Passed => "模式注册表校验通过：端点和 API 均可用。".to_string(),
            ValidationStatusDto::Warning => {
                "模式注册表校验完成，但仍存在阻塞性缺口或预留能力。".to_string()
            }
            ValidationStatusDto::Failed => "模式注册表校验失败：当前配置尚不可用。".to_string(),
            ValidationStatusDto::Skipped => "模式注册表校验未执行。".to_string(),
        };

        Ok(SchemaRegistryConnectionTestResponse {
            ok,
            status,
            target: endpoint.target,
            message,
            stages,
        })
    }
}

fn validation_error_stage(key: &str, label: &str, error: AppError) -> ValidationStageDto {
    let (category, code, retriable) = classify_registry_error(&error);

    ValidationStageDto::failed(key, label, error.to_string(), category, retriable)
        .with_detail(format!("错误代码：{}", code))
}

fn classify_registry_error(error: &AppError) -> (&'static str, &'static str, bool) {
    match error {
        AppError::Validation(_) => ("validation_error", "validation.invalid_input", false),
        AppError::NotFound(_) | AppError::Path(_) => ("config_error", "config.not_found", false),
        AppError::Network(message) => {
            let normalized = message.to_lowercase();
            if normalized.contains("401")
                || normalized.contains("403")
                || normalized.contains("auth")
            {
                ("auth_error", "connection.auth_failed", false)
            } else if normalized.contains("ssl")
                || normalized.contains("tls")
                || normalized.contains("certificate")
            {
                ("tls_error", "connection.tls_failed", false)
            } else if normalized.contains("timed out") || normalized.contains("timeout") {
                ("timeout_error", "connection.timeout", true)
            } else if normalized.contains("not implemented") {
                ("unsupported_feature", "feature.not_implemented", false)
            } else {
                ("connectivity_error", "connection.unreachable", true)
            }
        }
        AppError::Database(_)
        | AppError::Migration(_)
        | AppError::Io(_)
        | AppError::Internal(_) => ("internal_error", "internal.unexpected", false),
        AppError::Unsupported(_) => ("config_error", "config.unsupported", false),
    }
}

fn evaluate_registry_credentials(
    auth_mode: &str,
    credential_ref: Option<&str>,
    credential_secret: Option<&str>,
) -> (ValidationStageDto, Option<ResolvedSchemaRegistryAuth>) {
    let credential_ref = credential_ref
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match auth_mode {
        "none" => {
            if credential_ref.is_some() {
                (
                ValidationStageDto::warning(
                    "credential-reference",
                    "凭据引用",
                    "当前配置无需认证，凭据引用不会参与本次校验。",
                )
                .with_detail("当前认证方式无需 secret；如果此前写入过 keyring 条目，本次测试不会使用它。"),
                None,
                )
            } else {
                (
                ValidationStageDto::passed(
                    "credential-reference",
                    "凭据引用",
                    "当前配置无需额外凭据。",
                ),
                None,
                )
            }
        }
        "basic" | "bearer" => match credential_ref {
            Some(reference) => match resolve_schema_registry_auth(auth_mode, Some(reference), credential_secret) {
                Ok(Some(resolved_auth)) => (
                    ValidationStageDto::passed(
                        "credential-reference",
                        "凭据引用",
                        format!("认证方式为 {auth_mode}，已从系统 keyring 解析 credentialRef。"),
                    )
                    .with_detail(format!("凭据引用“{}”可用于运行时认证；secret 不会写入数据库。", reference)),
                    Some(resolved_auth),
                ),
                Ok(None) => (
                    ValidationStageDto::failed(
                        "credential-reference",
                        "凭据引用",
                        format!("认证方式为 {auth_mode}，但未解析到可用 secret。"),
                        "config_error",
                        false,
                    )
                    .with_detail("请提供 credentialRef，并确保系统 keyring 中存在对应 secret。"),
                    None,
                ),
                Err(error) => (
                    ValidationStageDto::failed(
                        "credential-reference",
                        "凭据引用",
                        error.to_string(),
                        classify_registry_error(&error).0,
                        false,
                    )
                    .with_detail("Basic Auth 应存为 username:password；Bearer 应存为 token。可在本次保存时直接填写 Secret 覆盖现有 keyring 条目。"),
                    None,
                ),
            },
            None => (
                ValidationStageDto::failed(
                    "credential-reference",
                    "凭据引用",
                    format!("认证方式为 {auth_mode}，但未提供 credentialRef。"),
                    "config_error",
                    false,
                )
                .with_detail("请填写 credentialRef，并在保存时提供 Secret 以写入系统 keyring。"),
                None,
            ),
        },
        other => (
        ValidationStageDto::failed(
            "credential-reference",
            "凭据引用",
            format!("未知认证方式：{other}"),
            "validation_error",
            false,
        ),
        None,
        ),
    }
}

async fn probe_registry_api(
    base_url: &str,
    auth: Option<&ResolvedSchemaRegistryAuth>,
) -> AppResult<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
        .map_err(|error| {
            AppError::Internal(format!("failed to build registry HTTP client: {error}"))
        })?;

    let request = client.get(format!("{}/subjects", base_url.trim_end_matches('/')));
    let request = if let Some(auth) = auth {
        auth.apply_async(request)
    } else {
        request
    };

    let response = request.send().await.map_err(|error| {
        AppError::Network(format!("schema registry API request failed: {error}"))
    })?;

    match response.status() {
        StatusCode::OK => Ok("`GET /subjects` 返回 200，API 可用。".to_string()),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(AppError::Network(format!(
            "schema registry API authentication failed with status {}",
            response.status()
        ))),
        status => Err(AppError::Network(format!(
            "schema registry API returned unexpected status {}",
            status
        ))),
    }
}

fn validate_schema_registry_request(name: &str, base_url: &str, auth_mode: &str) -> AppResult<()> {
    if name.trim().is_empty() {
        return Err(AppError::Validation(
            "schema registry profile name is required".to_string(),
        ));
    }

    if base_url.trim().is_empty() {
        return Err(AppError::Validation(
            "schema registry base URL is required".to_string(),
        ));
    }

    if !matches!(auth_mode, "none" | "basic" | "bearer") {
        return Err(AppError::Validation(
            "schema registry auth mode must be none, basic, or bearer".to_string(),
        ));
    }

    normalize_registry_endpoint(base_url)?;
    Ok(())
}

struct RegistryEndpoint {
    base_url: String,
    target: String,
}

fn normalize_registry_endpoint(base_url: &str) -> AppResult<RegistryEndpoint> {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "schema registry base URL is required".to_string(),
        ));
    }

    let normalized_base_url = if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    };

    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);

    let authority = without_scheme
        .split('/')
        .next()
        .ok_or_else(|| AppError::Validation("schema registry base URL is invalid".to_string()))?
        .trim();

    if authority.is_empty() {
        return Err(AppError::Validation(
            "schema registry base URL is invalid".to_string(),
        ));
    }

    let target = if authority.contains(':') {
        authority.to_string()
    } else if trimmed.starts_with("https://") {
        format!("{authority}:443")
    } else {
        format!("{authority}:80")
    };

    Ok(RegistryEndpoint {
        base_url: normalized_base_url,
        target,
    })
}
