use crate::app::state::AppState;
use crate::models::cluster::{
    ClusterConnectionTestRequest, ClusterConnectionTestResponse, ClusterProfileDto,
    ClusterProfileSummaryDto, CreateClusterProfileRequest, UpdateClusterProfileRequest,
};
use crate::models::error::{AppErrorDto, AppResult};
use crate::services::clusters::ClusterService;
use tauri::State;

#[tauri::command]
pub async fn list_clusters(
    state: State<'_, AppState>,
) -> Result<Vec<ClusterProfileSummaryDto>, AppErrorDto> {
    let result: AppResult<Vec<ClusterProfileSummaryDto>> =
        ClusterService::new(state.pool()).list_profiles().await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn get_cluster_profile(
    state: State<'_, AppState>,
    id: String,
) -> Result<ClusterProfileDto, AppErrorDto> {
    let result: AppResult<ClusterProfileDto> =
        ClusterService::new(state.pool()).get_profile(&id).await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn create_cluster_profile(
    state: State<'_, AppState>,
    request: CreateClusterProfileRequest,
) -> Result<ClusterProfileDto, AppErrorDto> {
    let result: AppResult<ClusterProfileDto> = ClusterService::new(state.pool())
        .create_profile(request)
        .await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn update_cluster_profile(
    state: State<'_, AppState>,
    request: UpdateClusterProfileRequest,
) -> Result<ClusterProfileDto, AppErrorDto> {
    let result: AppResult<ClusterProfileDto> = ClusterService::new(state.pool())
        .update_profile(request)
        .await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn test_cluster_connection(
    state: State<'_, AppState>,
    request: ClusterConnectionTestRequest,
) -> Result<ClusterConnectionTestResponse, AppErrorDto> {
    let result: AppResult<ClusterConnectionTestResponse> = ClusterService::new(state.pool())
        .test_connection(request)
        .await;

    result.map_err(Into::into)
}
