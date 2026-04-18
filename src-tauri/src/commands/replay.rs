use crate::app::state::AppState;
use crate::models::error::{AppErrorDto, AppResult};
use crate::models::replay::{
    CreateReplayJobRequest, ReplayJobDetailResponseDto, ReplayJobSummaryDto,
};
use crate::services::replay::ReplayService;
use tauri::State;

#[tauri::command]
pub async fn create_replay_job(
    state: State<'_, AppState>,
    request: CreateReplayJobRequest,
) -> Result<ReplayJobDetailResponseDto, AppErrorDto> {
    let result: AppResult<ReplayJobDetailResponseDto> = ReplayService::new(state.pool())
        .create_replay_job(request)
        .await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn list_replay_jobs(
    state: State<'_, AppState>,
    cluster_profile_id: String,
) -> Result<Vec<ReplayJobSummaryDto>, AppErrorDto> {
    let result: AppResult<Vec<ReplayJobSummaryDto>> = ReplayService::new(state.pool())
        .list_replay_jobs(&cluster_profile_id)
        .await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn get_replay_job(
    state: State<'_, AppState>,
    id: String,
) -> Result<ReplayJobDetailResponseDto, AppErrorDto> {
    let result: AppResult<ReplayJobDetailResponseDto> =
        ReplayService::new(state.pool()).get_replay_job(&id).await;

    result.map_err(Into::into)
}
