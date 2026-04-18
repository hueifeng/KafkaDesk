use crate::app::state::AppState;
use crate::models::error::{AppErrorDto, AppResult};
use crate::models::replay_policy::{ReplayPolicyDto, UpdateReplayPolicyRequest};
use crate::services::replay_policy::ReplayPolicyService;
use tauri::State;

#[tauri::command]
pub async fn get_replay_policy(state: State<'_, AppState>) -> Result<ReplayPolicyDto, AppErrorDto> {
    let result: AppResult<ReplayPolicyDto> = ReplayPolicyService::new(state.pool())
        .get_replay_policy()
        .await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn update_replay_policy(
    state: State<'_, AppState>,
    request: UpdateReplayPolicyRequest,
) -> Result<ReplayPolicyDto, AppErrorDto> {
    let result: AppResult<ReplayPolicyDto> = ReplayPolicyService::new(state.pool())
        .update_replay_policy(request)
        .await;

    result.map_err(Into::into)
}
