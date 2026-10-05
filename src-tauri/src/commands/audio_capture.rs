use crate::services::{
    audio_capture::{CaptureDevice, CaptureService, CaptureState},
    project_store::CommandError,
};
use tauri::AppHandle;
#[tauri::command]
pub async fn audio_capture_devices() -> Result<Vec<CaptureDevice>, CommandError> {
    tauri::async_runtime::spawn_blocking(crate::services::audio_capture::devices)
        .await
        .map_err(|e| CommandError::internal(e.to_string()))?
}
#[tauri::command]
pub async fn audio_capture_permission(request: bool) -> Result<String, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::audio_capture::permission(request)
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
#[tauri::command]
pub async fn audio_capture_start(
    app: AppHandle,
    memo_id: String,
    device_id: Option<String>,
) -> Result<CaptureState, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        app.state::<CaptureService>()
            .start(app.clone(), memo_id, device_id)
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
#[tauri::command]
pub async fn audio_capture_control(
    app: AppHandle,
    action: String,
) -> Result<CaptureState, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        app.state::<CaptureService>().control(&action)
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
