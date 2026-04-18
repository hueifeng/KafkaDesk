use crate::app::state::AppState;
use crate::models::bookmark::{
    CreateMessageBookmarkRequest, ListMessageBookmarksRequest, MessageBookmarkDto,
};
use crate::models::error::{AppErrorDto, AppResult};
use crate::services::bookmarks::BookmarksService;
use tauri::State;

#[tauri::command]
pub async fn list_message_bookmarks(
    state: State<'_, AppState>,
    request: ListMessageBookmarksRequest,
) -> Result<Vec<MessageBookmarkDto>, AppErrorDto> {
    let result: AppResult<Vec<MessageBookmarkDto>> = BookmarksService::new(state.pool())
        .list_message_bookmarks(request)
        .await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn create_message_bookmark(
    state: State<'_, AppState>,
    request: CreateMessageBookmarkRequest,
) -> Result<MessageBookmarkDto, AppErrorDto> {
    let result: AppResult<MessageBookmarkDto> = BookmarksService::new(state.pool())
        .create_message_bookmark(request)
        .await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn delete_message_bookmark(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppErrorDto> {
    let result: AppResult<()> = BookmarksService::new(state.pool())
        .delete_message_bookmark(&id)
        .await;

    result.map_err(Into::into)
}
