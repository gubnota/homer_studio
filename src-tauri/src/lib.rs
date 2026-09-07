mod commands;
mod services;

use serde::Serialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
};

pub struct AppState {
    pub project_write_lock: Arc<Mutex<()>>,
    pub jobs: services::jobs::JobStore,
    pub audio_assets: Arc<RwLock<HashMap<String, PathBuf>>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopInfo {
    platform: &'static str,
    architecture: &'static str,
    runtime: &'static str,
}

#[tauri::command]
fn desktop_info() -> DesktopInfo {
    DesktopInfo {
        platform: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
        runtime: "Tauri 2",
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let audio_assets = Arc::new(RwLock::new(HashMap::new()));
    let protocol_assets = audio_assets.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("audio", move |_context, request| {
            services::audio_protocol::respond(&protocol_assets, request)
        })
        .manage(AppState {
            project_write_lock: Arc::new(Mutex::new(())),
            jobs: services::jobs::JobStore::new(),
            audio_assets,
        })
        .invoke_handler(tauri::generate_handler![
            desktop_info,
            commands::project::create_project,
            commands::project::open_project,
            commands::project::update_chapter,
            commands::project::reorder_chapters,
            commands::project::read_manuscript,
            commands::system::get_settings,
            commands::system::save_settings,
            commands::system::tool_diagnostics,
            commands::system::list_jobs,
            commands::system::control_job,
            commands::production::process_text,
            commands::production::accept_processed_text,
            commands::production::list_voices,
            commands::production::preview_voice,
            commands::production::generate_chapter_audio,
            commands::production::import_chapter_audio,
            commands::production::set_chapter_review,
            commands::production::audio_url,
            commands::production::audio_waveform,
            commands::production::export_project,
            commands::production::export_audio_url,
            commands::production::read_export_timestamps
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Homer Studio");
}
