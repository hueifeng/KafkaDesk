use crate::models::cluster::{
    ClusterConnectionTestRequest, ClusterConnectionTestResponse, ClusterProfileDto,
    ClusterProfileSummaryDto, CreateClusterProfileRequest, UpdateClusterProfileRequest,
};
use crate::models::error::{AppError, AppResult};
use crate::models::validation::{
    summarize_validation_status, ValidationStageDto, ValidationStatusDto,
};
use crate::repositories::sqlite;
use crate::services::connectivity;
use crate::services::credentials::resolve_kafka_auth;
use crate::services::credentials::store_runtime_secret;
use crate::services::kafka_config::{
    apply_kafka_read_consumer_config_with_secret, normalize_optional_file_path,
    require_file_path,
};
use chrono::Utc;
use rdkafka::{
    consumer::{BaseConsumer, Consumer},
    ClientConfig,
};
use sqlx::SqlitePool;
use std::time::Duration;

pub struct ClusterService<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ClusterService<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_profiles(&self) -> AppResult<Vec<ClusterProfileSummaryDto>> {
        sqlite::list_cluster_profiles(self.pool).await
    }

    pub async fn get_profile(&self, id: &str) -> AppResult<ClusterProfileDto> {
        sqlite::get_cluster_profile(self.pool, id).await
    }

    pub async fn create_profile(
        &self,
        request: CreateClusterProfileRequest,
    ) -> AppResult<ClusterProfileDto> {
        validate_profile_input(&request)?;
        ensure_schema_registry_link(self.pool, request.schema_registry_profile_id.as_deref())
            .await?;

        if matches!(request.auth_mode.as_str(), "sasl-plain" | "sasl-scram") {
            if let (Some(reference), Some(secret)) = (
                request
                    .auth_credential_ref
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
                request
                    .auth_secret
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
            ) {
                store_runtime_secret(reference, secret)?;
            }
        }

        let profile = ClusterProfileDto::new(request);
        sqlite::insert_cluster_profile(self.pool, &profile).await?;

        let now = Utc::now().to_rfc3339();
        sqlite::seed_default_preferences(self.pool, &now).await?;

        Ok(profile)
    }

    pub async fn update_profile(
        &self,
        request: UpdateClusterProfileRequest,
    ) -> AppResult<ClusterProfileDto> {
        validate_profile_update(&request)?;
        ensure_schema_registry_link(self.pool, request.schema_registry_profile_id.as_deref())
            .await?;

        if matches!(request.auth_mode.as_str(), "sasl-plain" | "sasl-scram") {
            if let (Some(reference), Some(secret)) = (
                request
                    .auth_credential_ref
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
                request
                    .auth_secret
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
            ) {
                store_runtime_secret(reference, secret)?;
            }
        }

        let updated_at = Utc::now().to_rfc3339();

        sqlite::update_cluster_profile(self.pool, &request, &updated_at).await?;
        sqlite::get_cluster_profile(self.pool, &request.id).await
    }

    pub async fn test_connection(
        &self,
        request: ClusterConnectionTestRequest,
    ) -> AppResult<ClusterConnectionTestResponse> {
        let validation_request = CreateClusterProfileRequest {
            name: request.name.clone(),
            environment: request.environment.clone(),
            bootstrap_servers: request.bootstrap_servers.clone(),
            auth_mode: request.auth_mode.clone(),
            auth_credential_ref: request.auth_credential_ref.clone(),
            auth_secret: request.auth_secret.clone(),
            tls_mode: request.tls_mode.clone(),
            tls_ca_cert_path: request.tls_ca_cert_path.clone(),
            tls_client_cert_path: request.tls_client_cert_path.clone(),
            tls_client_key_path: request.tls_client_key_path.clone(),
            schema_registry_profile_id: request.schema_registry_profile_id.clone(),
            notes: request.notes.clone(),
            tags: request.tags.clone(),
        };

        let mut stages = Vec::new();

        if let Err(error) = validate_profile_input(&validation_request) {
            stages.push(validation_error_stage("profile-input", "配置校验", error));

            return Ok(ClusterConnectionTestResponse {
                ok: false,
                status: ValidationStatusDto::Failed,
                attempted_brokers: 0,
                reachable_brokers: 0,
                message: "集群校验未通过：请先修正配置输入。".to_string(),
                stages,
            });
        }

        stages.push(
            ValidationStageDto::passed("profile-input", "配置校验", "基础字段已通过语法校验。")
                .with_detail("名称、环境、Bootstrap 地址、认证方式和 TLS 模式均已提供。"),
        );

        match validate_schema_registry_link(
            self.pool,
            request.schema_registry_profile_id.as_deref(),
        )
        .await
        {
            Ok(stage) => stages.push(stage),
            Err(error) => return Err(error),
        }

        let reachability_report =
            connectivity::preflight_bootstrap_servers(&request.bootstrap_servers).await?;

        let bootstrap_stage = if reachability_report.reachable_brokers > 0 {
            ValidationStageDto::passed(
                "bootstrap-reachability",
                "Bootstrap 连通性",
                format!(
                    "{}/{} 个 Bootstrap 目标接受了 TCP 连接。",
                    reachability_report.reachable_brokers, reachability_report.attempted_brokers
                ),
            )
        } else {
            ValidationStageDto::failed(
                "bootstrap-reachability",
                "Bootstrap 连通性",
                format!(
                    "所有 {} 个 Bootstrap 目标的 TCP 连接均失败。",
                    reachability_report.attempted_brokers
                ),
                "connectivity_error",
                true,
            )
        }
        .with_detail(
            reachability_report
                .probes
                .iter()
                .map(|probe| format!("{}: {}", probe.target, probe.detail))
                .collect::<Vec<_>>()
                .join("；"),
        );
        stages.push(bootstrap_stage);

        let auth_stage = evaluate_auth_configuration_stage(&request);
        let auth_supported = auth_stage.status == ValidationStatusDto::Passed;
        stages.push(auth_stage);

        let tls_stage = evaluate_tls_stage(&request);
        let tls_supported = matches!(
            tls_stage.status,
            ValidationStatusDto::Passed | ValidationStatusDto::Warning
        );
        stages.push(tls_stage);

        if reachability_report.reachable_brokers > 0 && auth_supported && tls_supported {
            match fetch_metadata_capabilities(&request) {
                Ok((visible_topics, metadata_detail)) => {
                    stages.push(
                        ValidationStageDto::passed(
                            "broker-metadata",
                            "Broker 元数据",
                            "Kafka 元数据获取成功。",
                        )
                        .with_detail(metadata_detail),
                    );
                    stages.push(
                        ValidationStageDto::passed(
                            "topic-list",
                            "主题可见性",
                            format!("当前连接可见 {} 个主题。", visible_topics),
                        )
                        .with_detail("该结果基于元数据拉取，仅反映当前配置下可见的 Topic 列表。"),
                    );
                    match probe_consumer_group_capability(&request) {
                        Ok(group_count) => stages.push(
                            ValidationStageDto::passed(
                                "consumer-group-capability",
                                "消费组能力",
                                format!(
                                    "当前配置可执行 Consumer Group 列表读取，当前返回 {} 个组。",
                                    group_count
                                ),
                            )
                            .with_detail(
                                "该结果基于与分组页面相同的 Kafka 运行时链路，可用于提前暴露认证/TLS/权限问题。",
                            ),
                        ),
                        Err(error) => stages.push(
                            validation_error_stage(
                                "consumer-group-capability",
                                "消费组能力",
                                error,
                            )
                            .with_detail(
                                "当前配置尚无法复用到 Consumer Group 读取路径；请先修复该问题后再进入分组页面。",
                            ),
                        ),
                    }
                    match probe_message_query_capability(&request) {
                        Ok(detail) => stages.push(
                            ValidationStageDto::passed(
                                "message-query-capability",
                                "消息查询能力",
                                "当前配置可建立消息查询所需的 Kafka 读取客户端。",
                            )
                            .with_detail(detail),
                        ),
                        Err(error) => stages.push(
                            validation_error_stage(
                                "message-query-capability",
                                "消息查询能力",
                                error,
                            )
                            .with_detail(
                                "当前配置尚无法复用到消息查询路径；请先修复该问题后再执行受边界约束的消息读取。",
                            ),
                        ),
                    }
                }
                Err(error) => {
                    let (category, code, retriable) = classify_runtime_error(&error);
                    stages.push(
                        ValidationStageDto::failed(
                            "broker-metadata",
                            "Broker 元数据",
                            error.to_string(),
                            category,
                            retriable,
                        )
                        .with_detail(format!("错误代码：{}", code)),
                    );
                    stages.push(ValidationStageDto::skipped(
                        "topic-list",
                        "主题可见性",
                        "元数据获取失败，已跳过 Topic 可见性校验。",
                    ));
                    stages.push(ValidationStageDto::skipped(
                        "consumer-group-capability",
                        "消费组能力",
                        "Kafka 元数据未就绪，已跳过 Consumer Group 能力校验。",
                    ));
                    stages.push(ValidationStageDto::skipped(
                        "message-query-capability",
                        "消息查询能力",
                        "Kafka 元数据未就绪，已跳过消息查询能力校验。",
                    ));
                }
            }
        } else {
            stages.push(ValidationStageDto::skipped(
                "broker-metadata",
                "Broker 元数据",
                "前置条件未满足，已跳过 Kafka 元数据校验。",
            ));
            stages.push(ValidationStageDto::skipped(
                "topic-list",
                "主题可见性",
                "前置条件未满足，已跳过 Topic 可见性校验。",
            ));
            stages.push(ValidationStageDto::skipped(
                "consumer-group-capability",
                "消费组能力",
                "前置条件未满足，已跳过 Consumer Group 能力校验。",
            ));
            stages.push(ValidationStageDto::skipped(
                "message-query-capability",
                "消息查询能力",
                "前置条件未满足，已跳过消息查询能力校验。",
            ));
        }

        let status = summarize_validation_status(&stages);
        let ok = status == ValidationStatusDto::Passed;

        if ok {
            if let Some(profile_id) = request.profile_id.as_deref() {
                let connected_at = Utc::now().to_rfc3339();
                sqlite::mark_cluster_profile_connected(self.pool, profile_id, &connected_at)
                    .await?;
            }
        }

        let message = match status {
            ValidationStatusDto::Passed => {
                "集群校验通过：网络、认证、TLS 与 Kafka 元数据检查均已通过。"
                    .to_string()
            }
            ValidationStatusDto::Warning => {
                "集群校验完成，但仍存在阻塞性缺口或未实现能力。".to_string()
            }
            ValidationStatusDto::Failed => "集群校验失败：尚未进入可用状态。".to_string(),
            ValidationStatusDto::Skipped => "集群校验未执行。".to_string(),
        };

        Ok(ClusterConnectionTestResponse {
            ok,
            status,
            attempted_brokers: reachability_report.attempted_brokers,
            reachable_brokers: reachability_report.reachable_brokers,
            message,
            stages,
        })
    }
}

fn validation_error_stage(key: &str, label: &str, error: AppError) -> ValidationStageDto {
    let (category, code, retriable) = classify_runtime_error(&error);

    ValidationStageDto::failed(key, label, error.to_string(), category, retriable)
        .with_detail(format!("错误代码：{}", code))
}

fn classify_runtime_error(error: &AppError) -> (&'static str, &'static str, bool) {
    match error {
        AppError::Validation(_) => ("validation_error", "validation.invalid_input", false),
        AppError::NotFound(_) => ("config_error", "config.not_found", false),
        AppError::Path(_) => ("config_error", "config.invalid_path", false),
        AppError::Network(message) => {
            let normalized = message.to_lowercase();
            if normalized.contains("ssl")
                || normalized.contains("tls")
                || normalized.contains("certificate")
            {
                ("tls_error", "connection.tls_failed", false)
            } else if normalized.contains("auth")
                || normalized.contains("sasl")
                || normalized.contains("permission")
            {
                ("auth_error", "connection.auth_failed", false)
            } else if normalized.contains("timed out") || normalized.contains("timeout") {
                ("timeout_error", "connection.timeout", true)
            } else {
                ("connectivity_error", "connection.unreachable", true)
            }
        }
        AppError::Database(_)
        | AppError::Migration(_)
        | AppError::Io(_)
        | AppError::Internal(_) => ("internal_error", "internal.unexpected", false),
    }
}

fn evaluate_auth_configuration_stage(request: &ClusterConnectionTestRequest) -> ValidationStageDto {
    let credential_ref = normalized_optional_text(request.auth_credential_ref.as_deref());
    let secret_override = normalized_optional_text(request.auth_secret.as_deref());

    match request.auth_mode.as_str() {
        "none" => {
            if credential_ref.is_some() || secret_override.is_some() {
                ValidationStageDto::warning(
                    "auth-configuration",
                    "认证能力",
                    "当前配置无需认证；附带的凭据引用或 Secret 不会参与本次校验。",
                )
                .with_detail("如需验证 SASL，请切换到对应认证方式；当前元数据探测将按无认证链路执行。")
            } else {
                ValidationStageDto::passed(
                    "auth-configuration",
                    "认证能力",
                    "当前配置无需额外认证信息。",
                )
            }
        }
        "sasl-plain" | "sasl-scram" => {
            match resolve_kafka_auth(
                &request.auth_mode,
                credential_ref.as_deref(),
                secret_override.as_deref(),
            ) {
                Ok(Some(_)) => {
                    let detail = if secret_override.is_some() {
                        "已使用当前表单中的 Secret 进行临时凭据装配；保存后会写入系统 keyring。"
                    } else {
                        "已从系统 keyring 解析 credentialRef；后续元数据探测会复用同一 SASL 配置。"
                    };

                    ValidationStageDto::passed(
                        "auth-configuration",
                        "认证能力",
                        format!("认证方式为 {}，运行时 SASL 凭据已就绪。", request.auth_mode),
                    )
                    .with_detail(detail)
                }
                Ok(None) => ValidationStageDto::failed(
                    "auth-configuration",
                    "认证能力",
                    format!("认证方式为 {}，但未解析到可用 SASL 凭据。", request.auth_mode),
                    "config_error",
                    false,
                ),
                Err(error) => validation_error_stage("auth-configuration", "认证能力", error)
                    .with_detail(
                        "请填写 credentialRef，并确保系统 keyring 中存在 username:password 形式的 SASL secret；也可在本次测试时直接填写 Secret 覆盖现有条目。",
                    ),
            }
        }
        "mtls" => ValidationStageDto::passed(
            "auth-configuration",
            "认证能力",
            "mTLS 模式已受支持，将在 TLS 握手中装配客户端证书与私钥。",
        )
        .with_detail(
            "客户端证书、私钥与可选 CA 文件会在 TLS 能力阶段做本地路径校验，并在元数据探测时复用同一运行时配置。",
        ),
        other => ValidationStageDto::failed(
            "auth-configuration",
            "认证能力",
            format!("未知认证方式：{other}"),
            "validation_error",
            false,
        ),
    }
}

fn evaluate_tls_stage(request: &ClusterConnectionTestRequest) -> ValidationStageDto {
    let detail = match validate_tls_file_inputs(request) {
        Ok(detail) => detail,
        Err(error) => {
            return validation_error_stage("tls-configuration", "TLS 能力", error).with_detail(
                "请修复 CA / 客户端证书 / 私钥文件路径后再重试连接测试。",
            );
        }
    };

    if request.tls_mode != "system-default" || request.auth_mode == "mtls" {
        if let Err(error) = build_validation_consumer(request) {
            let normalized = error.to_string().to_lowercase();
            if normalized.contains("not supported in this build")
                || normalized.contains("openssl not available at build time")
            {
                return ValidationStageDto::failed(
                    "tls-configuration",
                    "TLS 能力",
                    "当前构建未启用 Kafka SSL/OpenSSL 运行时支持，无法按该配置执行 TLS / mTLS 握手。",
                    "unsupported_feature",
                    false,
                )
                .with_detail(error.to_string());
            }

            return validation_error_stage("tls-configuration", "TLS 能力", error).with_detail(
                "运行时无法组装当前 TLS 配置；请先修复本地配置后再重试连接测试。",
            );
        }
    }

    match request.tls_mode.as_str() {
        "system-default" => ValidationStageDto::passed(
            "tls-configuration",
            "TLS 能力",
            "当前配置使用默认传输设置。",
        )
        .with_detail(detail),
        "tls-required" => ValidationStageDto::passed(
            "tls-configuration",
            "TLS 能力",
            "将使用 SSL 模式执行 Kafka 元数据握手。",
        )
        .with_detail(detail),
        "tls-insecure" => ValidationStageDto::warning(
            "tls-configuration",
            "TLS 能力",
            "将尝试跳过证书校验进行 TLS 握手。",
        )
        .with_detail(format!(
            "{} 该模式仅用于排障，不应作为成熟产品默认配置。",
            detail
        )),
        other => ValidationStageDto::failed(
            "tls-configuration",
            "TLS 能力",
            format!("未知 TLS 模式：{other}"),
            "validation_error",
            false,
        ),
    }
}

fn validate_tls_file_inputs(request: &ClusterConnectionTestRequest) -> AppResult<String> {
    let mut details = Vec::new();

    if let Some(ca_path) =
        normalize_optional_file_path(request.tls_ca_cert_path.as_deref(), "TLS CA certificate")?
    {
        details.push(format!("自定义 CA 文件已就绪：{ca_path}。"));
    }

    if request.auth_mode == "mtls" {
        let cert_path = require_file_path(
            request.tls_client_cert_path.as_deref(),
            "TLS client certificate",
        )?;
        let key_path = require_file_path(
            request.tls_client_key_path.as_deref(),
            "TLS client private key",
        )?;

        details.push(format!(
            "客户端证书与私钥文件已就绪：{} / {}。",
            cert_path, key_path
        ));
    }

    if details.is_empty() {
        details.push("未指定额外证书文件，将直接复用系统默认 TLS 设置。".to_string());
    }

    Ok(details.join(" "))
}

fn fetch_metadata_capabilities(request: &ClusterConnectionTestRequest) -> AppResult<(usize, String)> {
    let consumer = build_validation_consumer(request)?;
    let metadata = consumer
        .fetch_metadata(None, Duration::from_secs(5))
        .map_err(|error| AppError::Network(format!("failed to load Kafka metadata: {error}")))?;

    let visible_topics = metadata
        .topics()
        .iter()
        .filter(|topic| !topic.name().starts_with("__"))
        .count();
    Ok((
        visible_topics,
        format!("Kafka 元数据拉取成功，当前可见 Topic 数：{visible_topics}。"),
    ))
}

fn probe_consumer_group_capability(request: &ClusterConnectionTestRequest) -> AppResult<usize> {
    let consumer = build_capability_consumer(request, Some("traceforge-validation-group-probe"))?;
    let groups = consumer.fetch_group_list(None, Duration::from_secs(5)).map_err(|error| {
        AppError::Network(format!(
            "failed to load consumer groups during validation: {error}"
        ))
    })?;

    Ok(groups.groups().len())
}

fn probe_message_query_capability(request: &ClusterConnectionTestRequest) -> AppResult<String> {
    let consumer = build_capability_consumer(request, None)?;
    drop(consumer);

    Ok(
        "消息查询所需的受边界约束读取客户端已可建立；实际执行仍取决于消息页提供的 topic/partition/time/offset 边界。"
            .to_string(),
    )
}

fn build_capability_consumer(
    request: &ClusterConnectionTestRequest,
    group_id: Option<&str>,
) -> AppResult<BaseConsumer> {
    let profile = build_validation_profile(request);
    let mut config = ClientConfig::new();
    apply_kafka_read_consumer_config_with_secret(
        &mut config,
        &profile,
        normalized_optional_text(request.auth_secret.as_deref()).as_deref(),
    )?;

    if let Some(group_id) = group_id {
        config.set("group.id", group_id);
    }

    config.create().map_err(|error| {
        AppError::Network(format!("failed to create Kafka capability client: {error}"))
    })
}

fn build_validation_consumer(request: &ClusterConnectionTestRequest) -> AppResult<BaseConsumer> {
    let profile = build_validation_profile(request);
    let mut config = ClientConfig::new();
    apply_kafka_read_consumer_config_with_secret(
        &mut config,
        &profile,
        normalized_optional_text(request.auth_secret.as_deref()).as_deref(),
    )?;

    config.create().map_err(|error| {
        AppError::Network(format!("failed to create Kafka validation client: {error}"))
    })
}

fn build_validation_profile(request: &ClusterConnectionTestRequest) -> ClusterProfileDto {
    ClusterProfileDto {
        id: request
            .profile_id
            .clone()
            .unwrap_or_else(|| "cluster-validation-preview".to_string()),
        name: request.name.clone(),
        environment: request.environment.clone(),
        bootstrap_servers: request.bootstrap_servers.clone(),
        auth_mode: request.auth_mode.clone(),
        auth_credential_ref: normalized_optional_text(request.auth_credential_ref.as_deref()),
        tls_mode: request.tls_mode.clone(),
        tls_ca_cert_path: normalized_optional_text(request.tls_ca_cert_path.as_deref()),
        tls_client_cert_path: normalized_optional_text(request.tls_client_cert_path.as_deref()),
        tls_client_key_path: normalized_optional_text(request.tls_client_key_path.as_deref()),
        schema_registry_profile_id: request.schema_registry_profile_id.clone(),
        notes: request.notes.clone(),
        tags: request.tags.clone(),
        is_favorite: false,
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
        last_connected_at: None,
        is_archived: false,
    }
}

fn normalized_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn validate_profile_input(request: &CreateClusterProfileRequest) -> AppResult<()> {
    if request.name.trim().is_empty() {
        return Err(AppError::Validation(
            "cluster profile name is required".to_string(),
        ));
    }

    if request.environment.trim().is_empty() {
        return Err(AppError::Validation("environment is required".to_string()));
    }

    if request.bootstrap_servers.trim().is_empty() {
        return Err(AppError::Validation(
            "bootstrap servers are required".to_string(),
        ));
    }

    if request.auth_mode.trim().is_empty() {
        return Err(AppError::Validation("auth mode is required".to_string()));
    }

    if request.tls_mode.trim().is_empty() {
        return Err(AppError::Validation("TLS mode is required".to_string()));
    }

    Ok(())
}

fn validate_profile_update(request: &UpdateClusterProfileRequest) -> AppResult<()> {
    if request.id.trim().is_empty() {
        return Err(AppError::Validation(
            "cluster profile id is required".to_string(),
        ));
    }

    validate_profile_input(&CreateClusterProfileRequest {
        name: request.name.clone(),
        environment: request.environment.clone(),
        bootstrap_servers: request.bootstrap_servers.clone(),
        auth_mode: request.auth_mode.clone(),
        auth_credential_ref: request.auth_credential_ref.clone(),
        auth_secret: request.auth_secret.clone(),
        tls_mode: request.tls_mode.clone(),
        tls_ca_cert_path: request.tls_ca_cert_path.clone(),
        tls_client_cert_path: request.tls_client_cert_path.clone(),
        tls_client_key_path: request.tls_client_key_path.clone(),
        schema_registry_profile_id: request.schema_registry_profile_id.clone(),
        notes: request.notes.clone(),
        tags: request.tags.clone(),
    })
}

async fn validate_schema_registry_link(
    pool: &SqlitePool,
    schema_registry_profile_id: Option<&str>,
) -> AppResult<ValidationStageDto> {
    let Some(schema_registry_profile_id) = schema_registry_profile_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(ValidationStageDto::skipped(
            "schema-registry-link",
            "Schema Registry 关联",
            "当前集群未关联 Schema Registry 配置。",
        ));
    };

    match sqlite::get_schema_registry_profile(pool, schema_registry_profile_id).await {
        Ok(profile) => Ok(ValidationStageDto::passed(
            "schema-registry-link",
            "Schema Registry 关联",
            format!("已关联 Schema Registry 配置“{}”。", profile.name),
        )
        .with_detail(format!("Base URL：{}", profile.base_url))),
        Err(AppError::NotFound(message)) => Ok(ValidationStageDto::failed(
            "schema-registry-link",
            "Schema Registry 关联",
            message,
            "config_error",
            false,
        )
        .with_detail("请先选择一个存在的 Schema Registry Profile，或移除该关联。")),
        Err(error) => Err(error),
    }
}

async fn ensure_schema_registry_link(
    pool: &SqlitePool,
    schema_registry_profile_id: Option<&str>,
) -> AppResult<()> {
    let Some(schema_registry_profile_id) = schema_registry_profile_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };

    sqlite::get_schema_registry_profile(pool, schema_registry_profile_id).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        build_capability_consumer, build_validation_consumer, evaluate_auth_configuration_stage,
        evaluate_tls_stage, probe_message_query_capability, ClusterConnectionTestRequest,
    };
    use crate::models::validation::ValidationStatusDto;
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    const TEST_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----\nMIICsDCCAZgCCQD4vKEpwC7YOjANBgkqhkiG9w0BAQsFADAaMRgwFgYDVQQDDA90\ncmFjZWZvcmdlLXRlc3QwHhcNMjYwNDE4MDczMTU0WhcNMjcwNDE4MDczMTU0WjAa\nMRgwFgYDVQQDDA90cmFjZWZvcmdlLXRlc3QwggEiMA0GCSqGSIb3DQEBAQUAA4IB\nDwAwggEKAoIBAQDB3uz/2t6m3/qJaENMNk1KW6L0nE6Sr6a16C9alMoNk53q8Glx\nQA7shqJLUj8AHpbMMMddlc/RXz4cYXKLMq5XH2YVJ+QKxjcXR0Q6Swu3cqoMO7n8\nfiOiA9biPR3/78KSEPd9zdqOs3DwMHr65v3oEI9tc1rWvam9Xz+4MrkBFbmF4wZR\nYZ4ac+lgB9ie1FL4S4cRHYqYzvumNdMDtFv8hDCmyPdKUlfEowxtueuQ5WGB82yP\nGd0LlYOvxt79MiR0sIQsxVnSY5F0nLcNW9Z+bCuGgEWchakiUb9Nj2BcJ88IAGxs\npitqC3v+noeQA4SWkc+7Byd4dZS4rLGYv8FhAgMBAAEwDQYJKoZIhvcNAQELBQAD\nggEBACk9WDt7D7thZkoT8VJkyukWx4uPGXczOfp0+hu2eP1TODurSQziwVj3xF3O\noSjN8HrWg3U0vGqZGgqIPxPknbmwk5fjVorwWelRlX2X7DMElsFeRMZSY9leLC10\ntqdEu8mIJsGzR/Aua56fo3dywhIglYG/8O0tcZYjdp6YczXWW64lPz2vVv+9ZVVj\nnVrKYbU118mkVhd7jmV9QR5KdBY1th6qVEzI340S7CQ2PdweT0kemFwBTCp5gvJ5\na3Xi8pKrQKJk/L2O6oxhXOCCGvWhdEvZ8mel2Qp/whg6MupIciDKozdf68yECrUW\nEhr3a4kltLXboZZ+DJx+KZCTRv0=\n-----END CERTIFICATE-----\n";
    const TEST_PRIVATE_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQDB3uz/2t6m3/qJ\naENMNk1KW6L0nE6Sr6a16C9alMoNk53q8GlxQA7shqJLUj8AHpbMMMddlc/RXz4c\nYXKLMq5XH2YVJ+QKxjcXR0Q6Swu3cqoMO7n8fiOiA9biPR3/78KSEPd9zdqOs3Dw\nMHr65v3oEI9tc1rWvam9Xz+4MrkBFbmF4wZRYZ4ac+lgB9ie1FL4S4cRHYqYzvum\nNdMDtFv8hDCmyPdKUlfEowxtueuQ5WGB82yPGd0LlYOvxt79MiR0sIQsxVnSY5F0\nnLcNW9Z+bCuGgEWchakiUb9Nj2BcJ88IAGxspitqC3v+noeQA4SWkc+7Byd4dZS4\nrLGYv8FhAgMBAAECggEAU/ogZuODtn0mpQaIwCZ1bFQtTg+26Us0x27/tBjnPOJI\ncVAaHHhG/qWC/2Vs7LxTTbeDZEJUdrjuypRbbXhSlGKBcYCKAzDBBFBWeXmwZZJB\nnnLwqTJqdO980RrwN8C/Y03+JnxYw59uuFwHqU8NhMxHlH13R7V4JRNxInS2NoVV\nXVfgcTjax8pdbdzKKwIn3AUk27SwSJwBlYuMKgDq741/L8PvyjOmolvzwM+aF2FO\n1gqb3xZKM1867psYo4Z09qdc8GyG+joPEbJW9rQW2nORUy1mqApXmO8qprt+K3yY\nhWQUjYFpngx7OKOv0RhRSwzm9swK03QbLKK5i5/IwQKBgQDokMmZxVKbnpTlI8Fl\n6H2pQAIPi1HPdTTMapxBlP5CsLgtkBiYu60LYevmAcSdkzbpVr7uqWxy7+Z0upao\n7kheHaqovcO8xm1n1BDOnEgnmFJ9wLFDi8qG9EQd4dusvJWb6u3dvvWVWQFh4Pz4\nZPKxGbfa7VHeFSk9wizXuCGQpQKBgQDVZ/nhoCOIzwF9bxR9Gko6fdeLf6ZTK0ht\nMJZ8kbDlYoSjRWjX6zXdoLL7mQ2y7avQ8mEYVyOcBCb5xGa43cF6eKy6W59lXQqI\nB1Z64gaKAsFgqbhpJRo4sfEsft9pip/oI49Y9B6vP9do9UbfBg4ZZm5fZdS5+MnY\nWE70VTB1DQKBgQCzQh7SkuD4qIRWFnhUp55sXbT47Ecz5EC9K5OjjUdqejKMlBwR\nZd+c/W5KDKTTXIyf0Mg8x4SbF0UIRmYocfp/6NgJVrPQBxZ/SFtoFdgcBPHYkjVQ\nPijuWstCSTv86iNbWfrcx/sdkcxZ+ISkpZLXZV5sti47Qw5V1xyfbgMZLQKBgQCq\nI92LTvtFpZSQhrEVFJK9k3r3kuvuPwHdW/F+m0EngKYy7bGrA7HMYsSP5vSPBQII\n8lUK7N5NEtpoI3eqR9JrbC553XZ1f/pXfVIrYmzIN24pPObznUsMjIG1cel44baf\ng0pUJz0Xh5Sb74FzagZvpcS1diBlrL5wJ+e60PhzOQKBgQCy9K28nvKeC+JO40ST\ngp5YqlQnRNaWfXHTwbeeYygYfgiY+lw7hmtInnLcu7s3uVepoWpJJPGeVG7MiW+1\nfN3BpyGmBc+fA8UbUl2RHZKvKaLTWGwujYqwbfSOtI56FpNBHfjy4HyAvzAHwB7a\nlHc4mf+dH3zMIvjxLn2jsGjjFw==\n-----END PRIVATE KEY-----\n";

    fn sample_request(auth_mode: &str, tls_mode: &str) -> ClusterConnectionTestRequest {
        ClusterConnectionTestRequest {
            profile_id: None,
            name: "Cluster Validation".to_string(),
            environment: "dev".to_string(),
            bootstrap_servers: "localhost:9092".to_string(),
            auth_mode: auth_mode.to_string(),
            auth_credential_ref: None,
            auth_secret: None,
            tls_mode: tls_mode.to_string(),
            tls_ca_cert_path: None,
            tls_client_cert_path: None,
            tls_client_key_path: None,
            schema_registry_profile_id: None,
            notes: None,
            tags: vec![],
        }
    }

    fn create_temp_file(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("traceforge-{name}-{}.pem", Uuid::new_v4()));
        fs::write(&path, contents).expect("temp cert material should write");
        path
    }

    #[test]
    fn auth_stage_passes_for_runtime_ready_sasl_profile() {
        let mut request = sample_request("sasl-scram", "tls-required");
        request.auth_credential_ref = Some("cluster-admin".to_string());
        request.auth_secret = Some("alice:secret".to_string());

        let stage = evaluate_auth_configuration_stage(&request);

        assert_eq!(stage.status, ValidationStatusDto::Passed);
        assert!(stage.message.contains("SASL 凭据已就绪"));
    }

    #[test]
    fn tls_stage_passes_for_existing_mtls_material() {
        let ca_path = create_temp_file("validation-ca", TEST_CERT_PEM);
        let cert_path = create_temp_file("validation-cert", TEST_CERT_PEM);
        let key_path = create_temp_file("validation-key", TEST_PRIVATE_KEY_PEM);

        let mut request = sample_request("mtls", "tls-required");
        request.tls_ca_cert_path = Some(ca_path.to_string_lossy().into_owned());
        request.tls_client_cert_path = Some(cert_path.to_string_lossy().into_owned());
        request.tls_client_key_path = Some(key_path.to_string_lossy().into_owned());

        let stage = evaluate_tls_stage(&request);

        assert_eq!(stage.status, ValidationStatusDto::Passed);
        assert!(stage
            .detail
            .unwrap_or_default()
            .contains("客户端证书与私钥文件已就绪"));

        let _ = fs::remove_file(ca_path);
        let _ = fs::remove_file(cert_path);
        let _ = fs::remove_file(key_path);
    }

    #[test]
    fn validation_consumer_builds_for_sasl_runtime_config() {
        let mut request = sample_request("sasl-plain", "tls-required");
        request.auth_credential_ref = Some("cluster-admin".to_string());
        request.auth_secret = Some("alice:secret".to_string());

        build_validation_consumer(&request)
            .expect("validation consumer should reuse authenticated runtime config");
    }

    #[test]
    fn validation_consumer_builds_for_mtls_runtime_config() {
        let ca_path = create_temp_file("validation-consumer-ca", TEST_CERT_PEM);
        let cert_path = create_temp_file("validation-consumer-cert", TEST_CERT_PEM);
        let key_path = create_temp_file("validation-consumer-key", TEST_PRIVATE_KEY_PEM);

        let mut request = sample_request("mtls", "tls-required");
        request.tls_ca_cert_path = Some(ca_path.to_string_lossy().into_owned());
        request.tls_client_cert_path = Some(cert_path.to_string_lossy().into_owned());
        request.tls_client_key_path = Some(key_path.to_string_lossy().into_owned());

        let consumer = build_validation_consumer(&request)
            .expect("validation consumer should accept mTLS runtime config");
        drop(consumer);

        let _ = fs::remove_file(ca_path);
        let _ = fs::remove_file(cert_path);
        let _ = fs::remove_file(key_path);
    }

    #[test]
    fn consumer_group_capability_client_builds_for_sasl_runtime_config() {
        let mut request = sample_request("sasl-plain", "tls-required");
        request.auth_credential_ref = Some("cluster-admin".to_string());
        request.auth_secret = Some("alice:secret".to_string());

        let consumer = build_capability_consumer(&request, Some("traceforge-validation-group-probe"))
            .expect("consumer-group capability client should reuse authenticated runtime config");
        drop(consumer);
    }

    #[test]
    fn message_query_capability_probe_builds_for_mtls_runtime_config() {
        let ca_path = create_temp_file("message-capability-ca", TEST_CERT_PEM);
        let cert_path = create_temp_file("message-capability-cert", TEST_CERT_PEM);
        let key_path = create_temp_file("message-capability-key", TEST_PRIVATE_KEY_PEM);

        let mut request = sample_request("mtls", "tls-required");
        request.tls_ca_cert_path = Some(ca_path.to_string_lossy().into_owned());
        request.tls_client_cert_path = Some(cert_path.to_string_lossy().into_owned());
        request.tls_client_key_path = Some(key_path.to_string_lossy().into_owned());

        let detail = probe_message_query_capability(&request)
            .expect("message-query capability probe should accept mTLS runtime config");
        assert!(detail.contains("topic/partition/time/offset"));

        let _ = fs::remove_file(ca_path);
        let _ = fs::remove_file(cert_path);
        let _ = fs::remove_file(key_path);
    }
}
