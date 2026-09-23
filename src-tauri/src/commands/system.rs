use crate::{
    AppState,
    services::{
        jobs::JobRecord,
        model_install::{self, ModelStatus},
        project_store::CommandError,
        settings::{self, Settings, ToolDiagnostic},
        sound_workers,
        worker_runtime::{self, RuntimeStatus},
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

#[tauri::command]
pub fn dismiss_jobs(
    state: State<'_, AppState>,
    job_ids: Vec<String>,
) -> Result<usize, CommandError> {
    state.jobs.dismiss(&job_ids)
}

#[tauri::command]
pub fn model_status(app: AppHandle, model: String) -> Result<ModelStatus, CommandError> {
    model_install::status(&app, &model)
}

#[tauri::command]
pub fn install_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model: String,
) -> Result<String, CommandError> {
    model_install::status(&app, &model)?;
    Ok(state.jobs.enqueue(
        "model",
        format!("Install {model} checkpoint"),
        move |control, progress| {
            model_install::install(&app, &model, &control, &*progress)?;
            let name = if model == "original" {
                "original"
            } else {
                "chatterbox"
            };
            if worker_runtime::status(&app, None)?.installed {
                let _ = sound_workers::start_local_worker(&app, name);
            }
            Ok(())
        },
    ))
}

#[tauri::command]
pub fn worker_runtime_status(
    app: AppHandle,
    python_path: Option<String>,
) -> Result<RuntimeStatus, CommandError> {
    worker_runtime::status(&app, python_path.as_deref())
}

#[tauri::command]
pub fn install_worker_runtime(
    app: AppHandle,
    state: State<'_, AppState>,
    python_path: Option<String>,
) -> Result<String, CommandError> {
    let selected = settings::load(&app)?.speech.provider;
    Ok(state.jobs.enqueue_with_report(
        "runtime",
        "Install Chatterbox Python runtime".into(),
        move |control, progress, report| {
            worker_runtime::install(&app, python_path.as_deref(), &control, &*progress, &*report)?;
            let model = if selected == "chatterbox_original" {
                "original"
            } else {
                "turbo"
            };
            if model_install::status(&app, model)?.installed {
                let name = if model == "original" {
                    "original"
                } else {
                    "chatterbox"
                };
                sound_workers::start_local_worker(&app, name)?;
            }
            Ok(())
        },
    ))
}
