use crate::app::state::AppState;
use crate::models::error::{AppErrorDto, AppResult};
use crate::models::topic::{
    GetTopicDetailRequest, ListTopicsRequest, TopicDetailResponseDto, TopicSummaryDto,
};
use crate::services::topics::TopicService;
use tauri::State;

#[tauri::command]
pub async fn list_topics(
    state: State<'_, AppState>,
    request: ListTopicsRequest,
) -> Result<Vec<TopicSummaryDto>, AppErrorDto> {
    let result: AppResult<Vec<TopicSummaryDto>> =
        TopicService::new(state.pool()).list_topics(request).await;

    result.map_err(Into::into)
}

#[tauri::command]
pub async fn get_topic_detail(
    state: State<'_, AppState>,
    request: GetTopicDetailRequest,
) -> Result<TopicDetailResponseDto, AppErrorDto> {
    let result: AppResult<TopicDetailResponseDto> = TopicService::new(state.pool())
        .get_topic_detail(request)
        .await;

    result.map_err(Into::into)
}
