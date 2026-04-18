use crate::app::state::AppState;
use crate::models::error::{AppErrorDto, AppResult};
use crate::models::group::{
    GetGroupDetailRequest, GroupDetailResponseDto, GroupSummaryDto, ListGroupsRequest,
};
use crate::services::groups::GroupService;
use tauri::State;

#[tauri::command]
pub async fn list_groups(
    state: State<'_, AppState>,
    request: ListGroupsRequest,
) -> Result<Vec<GroupSummaryDto>, AppErrorDto> {
    let result: AppResult<Vec<GroupSummaryDto>> =
        GroupService::new(state.pool()).list_groups(request).await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn get_group_detail(
    state: State<'_, AppState>,
    request: GetGroupDetailRequest,
) -> Result<GroupDetailResponseDto, AppErrorDto> {
    let result: AppResult<GroupDetailResponseDto> = GroupService::new(state.pool())
        .get_group_detail(request)
        .await;

    result.map_err(Into::into)
}
