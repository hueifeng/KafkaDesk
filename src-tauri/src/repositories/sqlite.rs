use crate::models::audit::{AuditEventDetailDto, AuditEventSummaryDto, ListAuditEventsRequest};
use crate::models::bookmark::MessageBookmarkDto;
use crate::models::cluster::{
    ClusterProfileDto, ClusterProfileSummaryDto, UpdateClusterProfileRequest,
};
use crate::models::correlation::{CorrelationRuleDto, UpdateCorrelationRuleRequest};
use crate::models::error::{AppError, AppResult};
use crate::models::replay::{
    AuditEventRecord, ReplayJobEventDto, ReplayJobRecord, ReplayJobRecoveryCandidate,
    ReplayJobSummaryDto,
};
use crate::models::saved_query::{SavedQueryDto, UpdateSavedQueryRequest};
use crate::models::schema_registry::{
    SchemaRegistryProfileDto, UpdateSchemaRegistryProfileRequest,
};
use serde_json::json;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, SqlitePool,
};
use std::{path::Path, str::FromStr};

pub async fn create_pool(database_path: &Path) -> AppResult<SqlitePool> {
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", database_path.display()))
        .map_err(|error| AppError::Internal(format!("invalid sqlite configuration: {error}")))?
        .create_if_missing(true);

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(AppError::Database)
}

pub async fn list_cluster_profiles(pool: &SqlitePool) -> AppResult<Vec<ClusterProfileSummaryDto>> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, environment, bootstrap_servers, auth_mode, auth_credential_ref, tls_mode,
               tls_ca_cert_path, tls_client_cert_path, schema_registry_profile_id, is_favorite, last_connected_at
        FROM cluster_profiles
        WHERE is_archived = 0
        ORDER BY is_favorite DESC, name ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(ClusterProfileSummaryDto {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                environment: row.try_get("environment")?,
                bootstrap_servers: row.try_get("bootstrap_servers")?,
                auth_mode: row.try_get("auth_mode")?,
                auth_credential_ref: row.try_get("auth_credential_ref")?,
                tls_mode: row.try_get("tls_mode")?,
                tls_ca_cert_path: row.try_get("tls_ca_cert_path")?,
                tls_client_cert_path: row.try_get("tls_client_cert_path")?,
                schema_registry_profile_id: row.try_get("schema_registry_profile_id")?,
                is_favorite: row.try_get::<i64, _>("is_favorite")? != 0,
                last_connected_at: row.try_get("last_connected_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::Database)
}

pub async fn insert_cluster_profile(
    pool: &SqlitePool,
    profile: &ClusterProfileDto,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO cluster_profiles (
            id, name, environment, bootstrap_servers, auth_mode, auth_credential_ref, tls_mode,
            tls_ca_cert_path, tls_client_cert_path, tls_client_key_path, schema_registry_profile_id, notes, tags_json, is_favorite,
            created_at, updated_at, last_connected_at, is_archived
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&profile.id)
    .bind(&profile.name)
    .bind(&profile.environment)
    .bind(&profile.bootstrap_servers)
    .bind(&profile.auth_mode)
    .bind(&profile.auth_credential_ref)
    .bind(&profile.tls_mode)
    .bind(&profile.tls_ca_cert_path)
    .bind(&profile.tls_client_cert_path)
    .bind(&profile.tls_client_key_path)
    .bind(&profile.schema_registry_profile_id)
    .bind(&profile.notes)
    .bind(serde_json::to_string(&profile.tags).map_err(|error| AppError::Internal(error.to_string()))?)
    .bind(if profile.is_favorite { 1 } else { 0 })
    .bind(&profile.created_at)
    .bind(&profile.updated_at)
    .bind(&profile.last_connected_at)
    .bind(if profile.is_archived { 1 } else { 0 })
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_cluster_profile(
    pool: &SqlitePool,
    request: &UpdateClusterProfileRequest,
    updated_at: &str,
) -> AppResult<()> {
    let result = sqlx::query(
        r#"
        UPDATE cluster_profiles
        SET name = ?, environment = ?, bootstrap_servers = ?, auth_mode = ?, auth_credential_ref = ?,
            tls_mode = ?, tls_ca_cert_path = ?, tls_client_cert_path = ?, tls_client_key_path = ?, schema_registry_profile_id = ?, notes = ?, tags_json = ?,
            is_favorite = ?, is_archived = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&request.name)
    .bind(&request.environment)
    .bind(&request.bootstrap_servers)
    .bind(&request.auth_mode)
    .bind(&request.auth_credential_ref)
    .bind(&request.tls_mode)
    .bind(&request.tls_ca_cert_path)
    .bind(&request.tls_client_cert_path)
    .bind(&request.tls_client_key_path)
    .bind(&request.schema_registry_profile_id)
    .bind(&request.notes)
    .bind(serde_json::to_string(&request.tags).map_err(|error| AppError::Internal(error.to_string()))?)
    .bind(if request.is_favorite { 1 } else { 0 })
    .bind(if request.is_archived { 1 } else { 0 })
    .bind(updated_at)
    .bind(&request.id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "cluster profile '{}' was not found",
            request.id
        )));
    }

    Ok(())
}

pub async fn get_cluster_profile(pool: &SqlitePool, id: &str) -> AppResult<ClusterProfileDto> {
    let row = sqlx::query(
        r#"
        SELECT id, name, environment, bootstrap_servers, auth_mode, auth_credential_ref, tls_mode,
               tls_ca_cert_path, tls_client_cert_path, tls_client_key_path, schema_registry_profile_id, notes, tags_json, is_favorite,
               created_at, updated_at, last_connected_at, is_archived
        FROM cluster_profiles
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("cluster profile '{}' was not found", id)))?;

    let tags_json: String = row.try_get("tags_json")?;

    Ok(ClusterProfileDto {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        environment: row.try_get("environment")?,
        bootstrap_servers: row.try_get("bootstrap_servers")?,
        auth_mode: row.try_get("auth_mode")?,
        auth_credential_ref: row.try_get("auth_credential_ref")?,
        tls_mode: row.try_get("tls_mode")?,
        tls_ca_cert_path: row.try_get("tls_ca_cert_path")?,
        tls_client_cert_path: row.try_get("tls_client_cert_path")?,
        tls_client_key_path: row.try_get("tls_client_key_path")?,
        schema_registry_profile_id: row.try_get("schema_registry_profile_id")?,
        notes: row.try_get("notes")?,
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        is_favorite: row.try_get::<i64, _>("is_favorite")? != 0,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        last_connected_at: row.try_get("last_connected_at")?,
        is_archived: row.try_get::<i64, _>("is_archived")? != 0,
    })
}

pub async fn list_schema_registry_profiles(
    pool: &SqlitePool,
) -> AppResult<Vec<SchemaRegistryProfileDto>> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, base_url, auth_mode, credential_ref, notes, created_at, updated_at
        FROM schema_registry_profiles
        ORDER BY name ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(SchemaRegistryProfileDto {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                base_url: row.try_get("base_url")?,
                auth_mode: row.try_get("auth_mode")?,
                credential_ref: row.try_get("credential_ref")?,
                notes: row.try_get("notes")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::Database)
}

pub async fn insert_schema_registry_profile(
    pool: &SqlitePool,
    profile: &SchemaRegistryProfileDto,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO schema_registry_profiles (id, name, base_url, auth_mode, credential_ref, notes, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&profile.id)
    .bind(&profile.name)
    .bind(&profile.base_url)
    .bind(&profile.auth_mode)
    .bind(&profile.credential_ref)
    .bind(&profile.notes)
    .bind(&profile.created_at)
    .bind(&profile.updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_schema_registry_profile(
    pool: &SqlitePool,
    request: &UpdateSchemaRegistryProfileRequest,
    updated_at: &str,
) -> AppResult<()> {
    let result = sqlx::query(
        r#"
        UPDATE schema_registry_profiles
        SET name = ?, base_url = ?, auth_mode = ?, credential_ref = ?, notes = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&request.name)
    .bind(&request.base_url)
    .bind(&request.auth_mode)
    .bind(&request.credential_ref)
    .bind(&request.notes)
    .bind(updated_at)
    .bind(&request.id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "schema registry profile '{}' was not found",
            request.id
        )));
    }

    Ok(())
}

pub async fn get_schema_registry_profile(
    pool: &SqlitePool,
    id: &str,
) -> AppResult<SchemaRegistryProfileDto> {
    let row = sqlx::query(
        r#"
        SELECT id, name, base_url, auth_mode, credential_ref, notes, created_at, updated_at
        FROM schema_registry_profiles
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("schema registry profile '{}' was not found", id)))?;

    Ok(SchemaRegistryProfileDto {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        base_url: row.try_get("base_url")?,
        auth_mode: row.try_get("auth_mode")?,
        credential_ref: row.try_get("credential_ref")?,
        notes: row.try_get("notes")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

pub async fn list_correlation_rules(pool: &SqlitePool) -> AppResult<Vec<CorrelationRuleDto>> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, cluster_profile_id, is_enabled, match_strategy, scope_json, rule_json, created_at, updated_at
        FROM correlation_rules
        ORDER BY updated_at DESC, name ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(CorrelationRuleDto {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                cluster_profile_id: row.try_get("cluster_profile_id")?,
                is_enabled: row.try_get::<i64, _>("is_enabled")? != 0,
                match_strategy: row.try_get("match_strategy")?,
                scope_json: row.try_get("scope_json")?,
                rule_json: row.try_get("rule_json")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::Database)
}

pub async fn insert_correlation_rule(
    pool: &SqlitePool,
    rule: &CorrelationRuleDto,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO correlation_rules (id, name, cluster_profile_id, is_enabled, match_strategy, scope_json, rule_json, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&rule.id)
    .bind(&rule.name)
    .bind(&rule.cluster_profile_id)
    .bind(if rule.is_enabled { 1 } else { 0 })
    .bind(&rule.match_strategy)
    .bind(&rule.scope_json)
    .bind(&rule.rule_json)
    .bind(&rule.created_at)
    .bind(&rule.updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_correlation_rule(
    pool: &SqlitePool,
    request: &UpdateCorrelationRuleRequest,
    updated_at: &str,
) -> AppResult<()> {
    let result = sqlx::query(
        r#"
        UPDATE correlation_rules
        SET name = ?, cluster_profile_id = ?, is_enabled = ?, match_strategy = ?, scope_json = ?, rule_json = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&request.name)
    .bind(&request.cluster_profile_id)
    .bind(if request.is_enabled { 1 } else { 0 })
    .bind(&request.match_strategy)
    .bind(&request.scope_json)
    .bind(&request.rule_json)
    .bind(updated_at)
    .bind(&request.id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "correlation rule '{}' was not found",
            request.id
        )));
    }

    Ok(())
}

pub async fn get_correlation_rule(pool: &SqlitePool, id: &str) -> AppResult<CorrelationRuleDto> {
    let row = sqlx::query(
        r#"
        SELECT id, name, cluster_profile_id, is_enabled, match_strategy, scope_json, rule_json, created_at, updated_at
        FROM correlation_rules
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("correlation rule '{}' was not found", id)))?;

    Ok(CorrelationRuleDto {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        cluster_profile_id: row.try_get("cluster_profile_id")?,
        is_enabled: row.try_get::<i64, _>("is_enabled")? != 0,
        match_strategy: row.try_get("match_strategy")?,
        scope_json: row.try_get("scope_json")?,
        rule_json: row.try_get("rule_json")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

pub async fn list_message_bookmarks(
    pool: &SqlitePool,
    cluster_profile_id: Option<&str>,
) -> AppResult<Vec<MessageBookmarkDto>> {
    let rows = sqlx::query(
        r#"
        SELECT id, cluster_profile_id, topic, partition, offset, label, notes, created_at
        FROM message_bookmarks
        WHERE (? IS NULL OR cluster_profile_id = ?)
        ORDER BY created_at DESC
        "#,
    )
    .bind(cluster_profile_id)
    .bind(cluster_profile_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(MessageBookmarkDto {
                id: row.try_get("id")?,
                message_ref: crate::models::message::MessageRefDto {
                    cluster_profile_id: row.try_get("cluster_profile_id")?,
                    topic: row.try_get("topic")?,
                    partition: row.try_get("partition")?,
                    offset: row.try_get::<i64, _>("offset")?.to_string(),
                },
                label: row.try_get("label")?,
                notes: row.try_get("notes")?,
                created_at: row.try_get("created_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::Database)
}

pub async fn find_message_bookmark_by_ref(
    pool: &SqlitePool,
    message_ref: &crate::models::message::MessageRefDto,
) -> AppResult<Option<MessageBookmarkDto>> {
    let offset = message_ref
        .offset
        .parse::<i64>()
        .map_err(|_| AppError::Validation("offset must be a valid integer".to_string()))?;

    let row = sqlx::query(
        r#"
        SELECT id, cluster_profile_id, topic, partition, offset, label, notes, created_at
        FROM message_bookmarks
        WHERE cluster_profile_id = ? AND topic = ? AND partition = ? AND offset = ?
        LIMIT 1
        "#,
    )
    .bind(&message_ref.cluster_profile_id)
    .bind(&message_ref.topic)
    .bind(message_ref.partition)
    .bind(offset)
    .fetch_optional(pool)
    .await?;

    row.map(|row| {
        Ok(MessageBookmarkDto {
            id: row.try_get("id")?,
            message_ref: crate::models::message::MessageRefDto {
                cluster_profile_id: row.try_get("cluster_profile_id")?,
                topic: row.try_get("topic")?,
                partition: row.try_get("partition")?,
                offset: row.try_get::<i64, _>("offset")?.to_string(),
            },
            label: row.try_get("label")?,
            notes: row.try_get("notes")?,
            created_at: row.try_get("created_at")?,
        })
    })
    .transpose()
    .map_err(AppError::Database)
}

pub async fn insert_message_bookmark(
    pool: &SqlitePool,
    bookmark: &MessageBookmarkDto,
) -> AppResult<()> {
    let offset = bookmark
        .message_ref
        .offset
        .parse::<i64>()
        .map_err(|_| AppError::Validation("offset must be a valid integer".to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO message_bookmarks (id, cluster_profile_id, topic, partition, offset, label, notes, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&bookmark.id)
    .bind(&bookmark.message_ref.cluster_profile_id)
    .bind(&bookmark.message_ref.topic)
    .bind(bookmark.message_ref.partition)
    .bind(offset)
    .bind(&bookmark.label)
    .bind(&bookmark.notes)
    .bind(&bookmark.created_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_message_bookmark(pool: &SqlitePool, id: &str) -> AppResult<()> {
    let result = sqlx::query(
        r#"
        DELETE FROM message_bookmarks
        WHERE id = ?
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "message bookmark '{}' was not found",
            id
        )));
    }

    Ok(())
}

pub async fn list_saved_queries(pool: &SqlitePool) -> AppResult<Vec<SavedQueryDto>> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, query_type, cluster_profile_id, scope_json, query_json, description,
               is_favorite, created_at, updated_at, last_run_at
        FROM saved_queries
        ORDER BY is_favorite DESC, updated_at DESC, name ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(SavedQueryDto {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                query_type: row.try_get("query_type")?,
                cluster_profile_id: row.try_get("cluster_profile_id")?,
                scope_json: row.try_get("scope_json")?,
                query_json: row.try_get("query_json")?,
                description: row.try_get("description")?,
                is_favorite: row.try_get::<i64, _>("is_favorite")? != 0,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                last_run_at: row.try_get("last_run_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::Database)
}

pub async fn insert_saved_query(pool: &SqlitePool, query: &SavedQueryDto) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO saved_queries (id, name, query_type, cluster_profile_id, scope_json, query_json, description,
                                   is_favorite, created_at, updated_at, last_run_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&query.id)
    .bind(&query.name)
    .bind(&query.query_type)
    .bind(&query.cluster_profile_id)
    .bind(&query.scope_json)
    .bind(&query.query_json)
    .bind(&query.description)
    .bind(if query.is_favorite { 1 } else { 0 })
    .bind(&query.created_at)
    .bind(&query.updated_at)
    .bind(&query.last_run_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_saved_query(
    pool: &SqlitePool,
    request: &UpdateSavedQueryRequest,
    updated_at: &str,
) -> AppResult<()> {
    let result = sqlx::query(
        r#"
        UPDATE saved_queries
        SET name = ?, query_type = ?, cluster_profile_id = ?, scope_json = ?, query_json = ?,
            description = ?, is_favorite = ?, updated_at = ?, last_run_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&request.name)
    .bind(&request.query_type)
    .bind(&request.cluster_profile_id)
    .bind(&request.scope_json)
    .bind(&request.query_json)
    .bind(&request.description)
    .bind(if request.is_favorite { 1 } else { 0 })
    .bind(updated_at)
    .bind(&request.last_run_at)
    .bind(&request.id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "saved query '{}' was not found",
            request.id
        )));
    }

    Ok(())
}

pub async fn get_saved_query(pool: &SqlitePool, id: &str) -> AppResult<SavedQueryDto> {
    let row = sqlx::query(
        r#"
        SELECT id, name, query_type, cluster_profile_id, scope_json, query_json, description,
               is_favorite, created_at, updated_at, last_run_at
        FROM saved_queries
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("saved query '{}' was not found", id)))?;

    Ok(SavedQueryDto {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        query_type: row.try_get("query_type")?,
        cluster_profile_id: row.try_get("cluster_profile_id")?,
        scope_json: row.try_get("scope_json")?,
        query_json: row.try_get("query_json")?,
        description: row.try_get("description")?,
        is_favorite: row.try_get::<i64, _>("is_favorite")? != 0,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        last_run_at: row.try_get("last_run_at")?,
    })
}

pub async fn delete_saved_query(pool: &SqlitePool, id: &str) -> AppResult<()> {
    let result = sqlx::query(
        r#"
        DELETE FROM saved_queries
        WHERE id = ?
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "saved query '{}' was not found",
            id
        )));
    }

    Ok(())
}

pub async fn mark_cluster_profile_connected(
    pool: &SqlitePool,
    id: &str,
    connected_at: &str,
) -> AppResult<()> {
    let result = sqlx::query(
        r#"
        UPDATE cluster_profiles
        SET last_connected_at = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(connected_at)
    .bind(connected_at)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "cluster profile '{}' was not found",
            id
        )));
    }

    Ok(())
}

pub async fn upsert_app_preference(
    pool: &SqlitePool,
    key: &str,
    value: serde_json::Value,
    updated_at: &str,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO app_preferences (key, value_json, updated_at)
        VALUES (?, ?, ?)
        ON CONFLICT(key) DO UPDATE SET
            value_json = excluded.value_json,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(key)
    .bind(value.to_string())
    .bind(updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn seed_default_preferences(pool: &SqlitePool, updated_at: &str) -> AppResult<()> {
    upsert_app_preference(
        pool,
        "defaultMessageQueryWindowMinutes",
        json!(30),
        updated_at,
    )
    .await?;
    upsert_app_preference(pool, "tableDensity", json!("compact"), updated_at).await?;
    upsert_app_preference(pool, "preferredTraceView", json!("timeline"), updated_at).await?;
    upsert_app_preference(pool, "replayAllowLiveReplay", json!(true), updated_at).await?;
    upsert_app_preference(pool, "replaySandboxOnly", json!(true), updated_at).await?;
    upsert_app_preference(
        pool,
        "replaySandboxTopicPrefix",
        json!("sandbox."),
        updated_at,
    )
    .await?;
    upsert_app_preference(
        pool,
        "replayRequireRiskAcknowledgement",
        json!(true),
        updated_at,
    )
    .await?;
    Ok(())
}

pub async fn list_app_preferences(
    pool: &SqlitePool,
) -> AppResult<Vec<(String, serde_json::Value)>> {
    let rows = sqlx::query(
        r#"
        SELECT key, value_json
        FROM app_preferences
        ORDER BY key ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            let key: String = row.try_get("key")?;
            let raw_value: String = row.try_get("value_json")?;
            let value = serde_json::from_str::<serde_json::Value>(&raw_value).map_err(|error| {
                AppError::Internal(format!(
                    "invalid preference payload for key '{key}': {error}"
                ))
            })?;

            Ok((key, value))
        })
        .collect()
}

pub async fn insert_replay_job(pool: &SqlitePool, record: &ReplayJobRecord) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO replay_jobs (
            id, cluster_profile_id, source_topic, source_partition, source_offset, source_timestamp,
            target_topic, status, mode, payload_edit_json, headers_edit_json, key_edit_json,
            dry_run, requested_by_profile, risk_level, created_at, started_at, completed_at,
            error_message, result_summary_json
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&record.id)
    .bind(&record.cluster_profile_id)
    .bind(&record.source_topic)
    .bind(record.source_partition)
    .bind(record.source_offset)
    .bind(&record.source_timestamp)
    .bind(&record.target_topic)
    .bind(&record.status)
    .bind(&record.mode)
    .bind(&record.payload_edit_json)
    .bind(&record.headers_edit_json)
    .bind(&record.key_edit_json)
    .bind(if record.dry_run { 1 } else { 0 })
    .bind(&record.requested_by_profile)
    .bind(&record.risk_level)
    .bind(&record.created_at)
    .bind(&record.started_at)
    .bind(&record.completed_at)
    .bind(&record.error_message)
    .bind(&record.result_summary_json)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_replay_job_execution(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    started_at: Option<&str>,
    completed_at: Option<&str>,
    error_message: Option<&str>,
    result_summary_json: Option<&str>,
) -> AppResult<()> {
    let rows_affected = sqlx::query(
        r#"
        UPDATE replay_jobs
        SET status = ?,
            started_at = ?,
            completed_at = ?,
            error_message = ?,
            result_summary_json = ?
        WHERE id = ?
        "#,
    )
    .bind(status)
    .bind(started_at)
    .bind(completed_at)
    .bind(error_message)
    .bind(result_summary_json)
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "replay job '{}' was not found",
            id
        )));
    }

    Ok(())
}

pub async fn insert_replay_job_event(
    pool: &SqlitePool,
    id: &str,
    replay_job_id: &str,
    event_type: &str,
    event_payload_json: Option<&str>,
    created_at: &str,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO replay_job_events (id, replay_job_id, event_type, event_payload_json, created_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(replay_job_id)
    .bind(event_type)
    .bind(event_payload_json)
    .bind(created_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_replay_jobs(
    pool: &SqlitePool,
    cluster_profile_id: &str,
) -> AppResult<Vec<ReplayJobSummaryDto>> {
    let rows = sqlx::query(
        r#"
        SELECT id, status, mode, target_topic, source_topic, source_partition, source_offset, source_timestamp,
               created_at, started_at, completed_at, risk_level, error_message, result_summary_json,
               payload_edit_json, headers_edit_json, key_edit_json
        FROM replay_jobs
        WHERE cluster_profile_id = ?
        ORDER BY created_at DESC
        "#,
    )
    .bind(cluster_profile_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(ReplayJobSummaryDto {
                id: row.try_get("id")?,
                status: row.try_get("status")?,
                mode: row.try_get("mode")?,
                target_topic: row.try_get("target_topic")?,
                source_topic: row.try_get("source_topic")?,
                source_partition: row.try_get("source_partition")?,
                source_offset: row.try_get::<i64, _>("source_offset")?.to_string(),
                source_timestamp: row.try_get("source_timestamp")?,
                created_at: row.try_get("created_at")?,
                started_at: row.try_get("started_at")?,
                completed_at: row.try_get("completed_at")?,
                risk_level: row.try_get("risk_level")?,
                error_message: row.try_get("error_message")?,
                result_summary_json: row.try_get("result_summary_json")?,
                payload_edit_json: row.try_get("payload_edit_json")?,
                headers_edit_json: row.try_get("headers_edit_json")?,
                key_edit_json: row.try_get("key_edit_json")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::Database)
}

pub async fn get_replay_job(pool: &SqlitePool, id: &str) -> AppResult<ReplayJobSummaryDto> {
    let row = sqlx::query(
        r#"
        SELECT id, status, mode, target_topic, source_topic, source_partition, source_offset, source_timestamp,
               created_at, started_at, completed_at, risk_level, error_message, result_summary_json,
               payload_edit_json, headers_edit_json, key_edit_json
        FROM replay_jobs
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("replay job '{}' was not found", id)))?;

    Ok(ReplayJobSummaryDto {
        id: row.try_get("id")?,
        status: row.try_get("status")?,
        mode: row.try_get("mode")?,
        target_topic: row.try_get("target_topic")?,
        source_topic: row.try_get("source_topic")?,
        source_partition: row.try_get("source_partition")?,
        source_offset: row.try_get::<i64, _>("source_offset")?.to_string(),
        source_timestamp: row.try_get("source_timestamp")?,
        created_at: row.try_get("created_at")?,
        started_at: row.try_get("started_at")?,
        completed_at: row.try_get("completed_at")?,
        risk_level: row.try_get("risk_level")?,
        error_message: row.try_get("error_message")?,
        result_summary_json: row.try_get("result_summary_json")?,
        payload_edit_json: row.try_get("payload_edit_json")?,
        headers_edit_json: row.try_get("headers_edit_json")?,
        key_edit_json: row.try_get("key_edit_json")?,
    })
}

pub async fn list_replay_jobs_by_status(
    pool: &SqlitePool,
    status: &str,
) -> AppResult<Vec<ReplayJobRecoveryCandidate>> {
    let rows = sqlx::query(
        r#"
        SELECT id, cluster_profile_id, target_topic, started_at
        FROM replay_jobs
        WHERE status = ?
        ORDER BY created_at ASC
        "#,
    )
    .bind(status)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(ReplayJobRecoveryCandidate {
                id: row.try_get("id")?,
                cluster_profile_id: row.try_get("cluster_profile_id")?,
                target_topic: row.try_get("target_topic")?,
                started_at: row.try_get("started_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::Database)
}

pub async fn list_replay_job_events(
    pool: &SqlitePool,
    replay_job_id: &str,
) -> AppResult<Vec<ReplayJobEventDto>> {
    let rows = sqlx::query(
        r#"
        SELECT id, event_type, event_payload_json, created_at
        FROM replay_job_events
        WHERE replay_job_id = ?
        ORDER BY created_at ASC
        "#,
    )
    .bind(replay_job_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(ReplayJobEventDto {
                id: row.try_get("id")?,
                event_type: row.try_get("event_type")?,
                event_payload_json: row.try_get("event_payload_json")?,
                created_at: row.try_get("created_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::Database)
}

pub async fn insert_audit_event(pool: &SqlitePool, record: &AuditEventRecord) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
            id, event_type, target_type, target_ref, actor_profile, cluster_profile_id,
            outcome, summary, details_json, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&record.id)
    .bind(&record.event_type)
    .bind(&record.target_type)
    .bind(&record.target_ref)
    .bind(&record.actor_profile)
    .bind(&record.cluster_profile_id)
    .bind(&record.outcome)
    .bind(&record.summary)
    .bind(&record.details_json)
    .bind(&record.created_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_audit_events(
    pool: &SqlitePool,
    request: &ListAuditEventsRequest,
) -> AppResult<Vec<AuditEventSummaryDto>> {
    let cluster_profile_id = request.cluster_profile_id.as_deref();
    let event_type = request.event_type.as_deref();
    let outcome = request.outcome.as_deref();
    let start_at = request.start_at.as_deref();
    let end_at = request.end_at.as_deref();
    let limit = i64::from(request.limit.unwrap_or(100));

    let rows = sqlx::query(
        r#"
        SELECT id, created_at, event_type, target_type, summary, outcome, actor_profile, cluster_profile_id, target_ref
        FROM audit_events
        WHERE (? IS NULL OR cluster_profile_id = ?)
          AND (? IS NULL OR event_type = ?)
          AND (? IS NULL OR outcome = ?)
          AND (? IS NULL OR created_at >= ?)
          AND (? IS NULL OR created_at <= ?)
        ORDER BY created_at DESC
        LIMIT ?
        "#,
    )
    .bind(cluster_profile_id)
    .bind(cluster_profile_id)
    .bind(event_type)
    .bind(event_type)
    .bind(outcome)
    .bind(outcome)
    .bind(start_at)
    .bind(start_at)
    .bind(end_at)
    .bind(end_at)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(AuditEventSummaryDto {
                id: row.try_get("id")?,
                created_at: row.try_get("created_at")?,
                event_type: row.try_get("event_type")?,
                target_type: row.try_get("target_type")?,
                summary: row.try_get("summary")?,
                outcome: row.try_get("outcome")?,
                actor_profile: row.try_get("actor_profile")?,
                cluster_profile_id: row.try_get("cluster_profile_id")?,
                target_ref: row.try_get("target_ref")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AppError::Database)
}

pub async fn get_audit_event(pool: &SqlitePool, id: &str) -> AppResult<AuditEventDetailDto> {
    let row = sqlx::query(
        r#"
        SELECT id, created_at, event_type, target_type, target_ref, actor_profile, cluster_profile_id,
               outcome, summary, details_json
        FROM audit_events
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("audit event '{}' was not found", id)))?;

    Ok(AuditEventDetailDto {
        id: row.try_get("id")?,
        created_at: row.try_get("created_at")?,
        event_type: row.try_get("event_type")?,
        target_type: row.try_get("target_type")?,
        target_ref: row.try_get("target_ref")?,
        actor_profile: row.try_get("actor_profile")?,
        cluster_profile_id: row.try_get("cluster_profile_id")?,
        outcome: row.try_get("outcome")?,
        summary: row.try_get("summary")?,
        details_json: row.try_get("details_json")?,
    })
}

pub async fn find_latest_audit_ref_for_target(
    pool: &SqlitePool,
    target_type: &str,
    target_ref: &str,
) -> AppResult<Option<String>> {
    let row = sqlx::query(
        r#"
        SELECT id
        FROM audit_events
        WHERE target_type = ? AND target_ref = ?
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(target_type)
    .bind(target_ref)
    .fetch_optional(pool)
    .await?;

    row.map(|row| row.try_get("id"))
        .transpose()
        .map_err(AppError::Database)
}
