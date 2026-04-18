use crate::models::error::{AppError, AppResult};
use crate::repositories::sqlite::create_pool;
use crate::services::replay::ReplayService;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Clone)]
pub struct AppState {
    pool: SqlitePool,
}

impl AppState {
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn bootstrap(app: &AppHandle) -> AppResult<Self> {
        let data_dir = resolve_app_data_dir(app)?;
        std::fs::create_dir_all(&data_dir).map_err(AppError::Io)?;

        let db_path = data_dir.join("traceforge.sqlite3");

        let pool = tauri::async_runtime::block_on(async {
            let pool = create_pool(&db_path).await?;
            sqlx::migrate!("./migrations").run(&pool).await?;
            let _ = ReplayService::new(&pool)
                .recover_stale_publishing_jobs()
                .await?;
            Ok::<SqlitePool, AppError>(pool)
        })?;

        Ok(Self { pool })
    }
}

fn resolve_app_data_dir(app: &AppHandle) -> AppResult<PathBuf> {
    app.path()
        .app_data_dir()
        .map_err(|error| AppError::Path(error.to_string()))
}
