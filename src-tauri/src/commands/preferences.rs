use crate::app::state::AppState;
use crate::models::error::{AppErrorDto, AppResult};
use crate::models::preferences::{AppPreferencesDto, UpdateAppPreferencesRequest};
use crate::services::preferences::PreferencesService;
use tauri::State;

#[tauri::command]
pub async fn get_app_preferences(
    state: State<'_, AppState>,
) -> Result<AppPreferencesDto, AppErrorDto> {
    let result: AppResult<AppPreferencesDto> = PreferencesService::new(state.pool())
        .get_app_preferences()
        .await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn update_app_preferences(
    state: State<'_, AppState>,
    request: UpdateAppPreferencesRequest,
) -> Result<AppPreferencesDto, AppErrorDto> {
    let result: AppResult<AppPreferencesDto> = PreferencesService::new(state.pool())
        .update_app_preferences(request)
        .await;

    result.map_err(Into::into)
}
