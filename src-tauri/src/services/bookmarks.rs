use crate::models::bookmark::{
    CreateMessageBookmarkRequest, ListMessageBookmarksRequest, MessageBookmarkDto,
};
use crate::models::error::{AppError, AppResult};
use crate::repositories::sqlite;
use sqlx::SqlitePool;

pub struct BookmarksService<'a> {
    pool: &'a SqlitePool,
}

impl<'a> BookmarksService<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_message_bookmarks(
        &self,
        request: ListMessageBookmarksRequest,
    ) -> AppResult<Vec<MessageBookmarkDto>> {
        if let Some(cluster_profile_id) = request.cluster_profile_id.as_deref() {
            sqlite::get_cluster_profile(self.pool, cluster_profile_id).await?;
        }

        sqlite::list_message_bookmarks(self.pool, request.cluster_profile_id.as_deref()).await
    }

    pub async fn create_message_bookmark(
        &self,
        request: CreateMessageBookmarkRequest,
    ) -> AppResult<MessageBookmarkDto> {
        validate_create_request(self.pool, &request).await?;

        if let Some(existing) =
            sqlite::find_message_bookmark_by_ref(self.pool, &request.message_ref).await?
        {
            return Ok(existing);
        }

        let bookmark = MessageBookmarkDto::new(request);
        sqlite::insert_message_bookmark(self.pool, &bookmark).await?;
        Ok(bookmark)
    }

    pub async fn delete_message_bookmark(&self, id: &str) -> AppResult<()> {
        if id.trim().is_empty() {
            return Err(AppError::Validation("bookmark id is required".to_string()));
        }

        sqlite::delete_message_bookmark(self.pool, id).await
    }
}

async fn validate_create_request(
    pool: &SqlitePool,
    request: &CreateMessageBookmarkRequest,
) -> AppResult<()> {
    if request.message_ref.cluster_profile_id.trim().is_empty() {
        return Err(AppError::Validation(
            "cluster profile id is required".to_string(),
        ));
    }
    if request.message_ref.topic.trim().is_empty() {
        return Err(AppError::Validation("topic is required".to_string()));
    }
    if request.message_ref.offset.trim().is_empty() {
        return Err(AppError::Validation("offset is required".to_string()));
    }

    request
        .message_ref
        .offset
        .parse::<i64>()
        .map_err(|_| AppError::Validation("offset must be a valid integer".to_string()))?;

    sqlite::get_cluster_profile(pool, &request.message_ref.cluster_profile_id).await?;
    Ok(())
}
