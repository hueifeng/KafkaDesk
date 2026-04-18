use crate::app::state::AppState;
use crate::models::error::{AppErrorDto, AppResult};
use crate::models::schema_registry::{
    CreateSchemaRegistryProfileRequest, SchemaRegistryConnectionTestRequest,
    SchemaRegistryConnectionTestResponse, SchemaRegistryProfileDto,
    UpdateSchemaRegistryProfileRequest,
};
use crate::services::schema_registry::SchemaRegistryService;
use tauri::State;

#[tauri::command]
pub async fn list_schema_registry_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<SchemaRegistryProfileDto>, AppErrorDto> {
    let result: AppResult<Vec<SchemaRegistryProfileDto>> = SchemaRegistryService::new(state.pool())
        .list_profiles()
        .await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn create_schema_registry_profile(
    state: State<'_, AppState>,
    request: CreateSchemaRegistryProfileRequest,
) -> Result<SchemaRegistryProfileDto, AppErrorDto> {
    let result: AppResult<SchemaRegistryProfileDto> = SchemaRegistryService::new(state.pool())
        .create_profile(request)
        .await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn update_schema_registry_profile(
    state: State<'_, AppState>,
    request: UpdateSchemaRegistryProfileRequest,
) -> Result<SchemaRegistryProfileDto, AppErrorDto> {
    let result: AppResult<SchemaRegistryProfileDto> = SchemaRegistryService::new(state.pool())
        .update_profile(request)
        .await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn test_schema_registry_profile(
    state: State<'_, AppState>,
    request: SchemaRegistryConnectionTestRequest,
) -> Result<SchemaRegistryConnectionTestResponse, AppErrorDto> {
    let result: AppResult<SchemaRegistryConnectionTestResponse> =
        SchemaRegistryService::new(state.pool())
            .test_connection(request)
            .await;

    result.map_err(Into::into)
}
