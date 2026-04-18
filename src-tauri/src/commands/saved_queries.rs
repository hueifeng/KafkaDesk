use crate::app::state::AppState;
use crate::models::error::{AppErrorDto, AppResult};
use crate::models::saved_query::{CreateSavedQueryRequest, SavedQueryDto, UpdateSavedQueryRequest};
use crate::services::saved_queries::SavedQueriesService;
use tauri::State;

#[tauri::command]
pub async fn list_saved_queries(
    state: State<'_, AppState>,
) -> Result<Vec<SavedQueryDto>, AppErrorDto> {
    let result: AppResult<Vec<SavedQueryDto>> = SavedQueriesService::new(state.pool())
        .list_saved_queries()
        .await;
    result.map_err(Into::into)
}

#[tauri::command]
pub async fn create_saved_query(
    state: State<'_, AppState>,
    request: CreateSavedQueryRequest,
) -> Result<SavedQueryDto, AppErrorDto> {
    let result: AppResult<SavedQueryDto> = SavedQueriesService::new(state.pool())
        .create_saved_query(request)
        .await;
    result.map_err(Into::into)
}

#[tauri::command]
pub async fn update_saved_query(
    state: State<'_, AppState>,
    request: UpdateSavedQueryRequest,
) -> Result<SavedQueryDto, AppErrorDto> {
    let result: AppResult<SavedQueryDto> = SavedQueriesService::new(state.pool())
        .update_saved_query(request)
        .await;
    result.map_err(Into::into)
}

#[tauri::command]
pub async fn delete_saved_query(state: State<'_, AppState>, id: String) -> Result<(), AppErrorDto> {
    let result: AppResult<()> = SavedQueriesService::new(state.pool())
        .delete_saved_query(&id)
        .await;
    result.map_err(Into::into)
}
