use crate::app::state::AppState;
use crate::models::audit::{AuditEventDetailDto, AuditEventSummaryDto, ListAuditEventsRequest};
use crate::models::error::{AppErrorDto, AppResult};
use crate::services::audit::AuditService;
use tauri::State;

#[tauri::command]
pub async fn list_audit_events(
    state: State<'_, AppState>,
    request: ListAuditEventsRequest,
) -> Result<Vec<AuditEventSummaryDto>, AppErrorDto> {
    let result: AppResult<Vec<AuditEventSummaryDto>> = AuditService::new(state.pool())
        .list_audit_events(request)
        .await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn get_audit_event(
    state: State<'_, AppState>,
    id: String,
) -> Result<AuditEventDetailDto, AppErrorDto> {
    let result: AppResult<AuditEventDetailDto> =
        AuditService::new(state.pool()).get_audit_event(&id).await;

    result.map_err(Into::into)
}
