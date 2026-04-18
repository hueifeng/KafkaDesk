use crate::app::state::AppState;
use crate::models::correlation::{
    CorrelationRuleDto, CreateCorrelationRuleRequest, UpdateCorrelationRuleRequest,
};
use crate::models::error::{AppErrorDto, AppResult};
use crate::services::correlation::CorrelationService;
use tauri::State;

#[tauri::command]
pub async fn list_correlation_rules(
    state: State<'_, AppState>,
) -> Result<Vec<CorrelationRuleDto>, AppErrorDto> {
    let result: AppResult<Vec<CorrelationRuleDto>> =
        CorrelationService::new(state.pool()).list_rules().await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn create_correlation_rule(
    state: State<'_, AppState>,
    request: CreateCorrelationRuleRequest,
) -> Result<CorrelationRuleDto, AppErrorDto> {
    let result: AppResult<CorrelationRuleDto> = CorrelationService::new(state.pool())
        .create_rule(request)
        .await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn update_correlation_rule(
    state: State<'_, AppState>,
    request: UpdateCorrelationRuleRequest,
) -> Result<CorrelationRuleDto, AppErrorDto> {
    let result: AppResult<CorrelationRuleDto> = CorrelationService::new(state.pool())
        .update_rule(request)
        .await;

    result.map_err(Into::into)
}
