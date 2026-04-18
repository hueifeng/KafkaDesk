use crate::models::audit::{AuditEventDetailDto, AuditEventSummaryDto, ListAuditEventsRequest};
use crate::models::error::{AppError, AppResult};
use crate::repositories::sqlite;
use sqlx::SqlitePool;

const DEFAULT_LIMIT: u32 = 100;
const MAX_LIMIT: u32 = 500;

pub struct AuditService<'a> {
    pool: &'a SqlitePool,
}

impl<'a> AuditService<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_audit_events(
        &self,
        request: ListAuditEventsRequest,
    ) -> AppResult<Vec<AuditEventSummaryDto>> {
        let normalized = normalize_request(request)?;
        sqlite::list_audit_events(self.pool, &normalized).await
    }

    pub async fn get_audit_event(&self, id: &str) -> AppResult<AuditEventDetailDto> {
        if id.trim().is_empty() {
            return Err(AppError::Validation(
                "audit event id is required".to_string(),
            ));
        }

        sqlite::get_audit_event(self.pool, id).await
    }
}

fn normalize_request(request: ListAuditEventsRequest) -> AppResult<ListAuditEventsRequest> {
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT);
    if limit == 0 || limit > MAX_LIMIT {
        return Err(AppError::Validation(format!(
            "audit limit must be between 1 and {MAX_LIMIT}"
        )));
    }

    let start_at = request
        .start_at
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let end_at = request
        .end_at
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if let (Some(start_at), Some(end_at)) = (&start_at, &end_at) {
        if start_at > end_at {
            return Err(AppError::Validation(
                "audit start time must be earlier than end time".to_string(),
            ));
        }
    }

    Ok(ListAuditEventsRequest {
        cluster_profile_id: request
            .cluster_profile_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        event_type: request
            .event_type
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        outcome: request
            .outcome
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        start_at,
        end_at,
        limit: Some(limit),
    })
}
