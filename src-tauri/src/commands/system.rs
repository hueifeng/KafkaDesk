use crate::models::error::{AppErrorDto, AppResult};

#[tauri::command]
pub async fn ping() -> Result<String, AppErrorDto> {
    let value: AppResult<String> = Ok("traceforge-runtime-ok".to_string());
    value.map_err(Into::into)
}
