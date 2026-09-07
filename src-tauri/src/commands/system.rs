use crate::{
    AppState,
    services::{
        jobs::JobRecord,
        project_store::CommandError,
        settings::{self, Settings, ToolDiagnostic},
    },
};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<Settings, CommandError> {
    settings::load(&app)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<Settings, CommandError> {
    settings::save(&app, settings)
}

#[tauri::command]
pub fn tool_diagnostics(app: AppHandle) -> Result<Vec<ToolDiagnostic>, CommandError> {
    Ok(settings::diagnostics(&settings::load(&app)?))
}

#[tauri::command]
pub fn list_jobs(state: State<'_, AppState>) -> Vec<JobRecord> {
    state.jobs.list()
}

#[tauri::command]
pub fn control_job(
    state: State<'_, AppState>,
    job_id: String,
    action: String,
) -> Result<(), CommandError> {
    state.jobs.control(&job_id, &action)
}
