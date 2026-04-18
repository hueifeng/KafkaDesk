use crate::app::state::AppState;
use crate::models::error::{AppErrorDto, AppResult};
use crate::models::trace::{RunTraceQueryRequest, TraceQueryResultDto};
use crate::services::trace::TraceService;
use tauri::State;

#[tauri::command]
pub async fn run_trace_query(
    state: State<'_, AppState>,
    request: RunTraceQueryRequest,
) -> Result<TraceQueryResultDto, AppErrorDto> {
    let result: AppResult<TraceQueryResultDto> = TraceService::new(state.pool())
        .run_trace_query(request)
        .await;

    result.map_err(Into::into)
}
