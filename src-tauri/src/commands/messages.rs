use crate::app::state::AppState;
use crate::models::error::{AppErrorDto, AppResult};
use crate::models::message::{
    GetMessageDetailRequest, MessageDetailResponseDto, MessageSummaryDto, QueryMessagesRequest,
};
use crate::services::messages::MessageService;
use tauri::State;

#[tauri::command]
pub async fn query_messages(
    state: State<'_, AppState>,
    request: QueryMessagesRequest,
) -> Result<Vec<MessageSummaryDto>, AppErrorDto> {
    let result: AppResult<Vec<MessageSummaryDto>> = MessageService::new(state.pool())
        .query_messages(request)
        .await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn get_message_detail(
    state: State<'_, AppState>,
    request: GetMessageDetailRequest,
) -> Result<MessageDetailResponseDto, AppErrorDto> {
    let result: AppResult<MessageDetailResponseDto> = MessageService::new(state.pool())
        .get_message_detail(request)
        .await;

    result.map_err(Into::into)
}
