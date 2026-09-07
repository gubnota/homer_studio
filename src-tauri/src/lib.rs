mod commands;
mod services;

use serde::Serialize;
use std::sync::Mutex;

pub struct AppState {
    pub project_write_lock: Mutex<()>,
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
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            project_write_lock: Mutex::new(()),
        })
        .invoke_handler(tauri::generate_handler![
            desktop_info,
            commands::project::create_project,
            commands::project::open_project,
            commands::project::update_chapter,
            commands::project::reorder_chapters,
            commands::project::read_manuscript
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Homer Studio");
}
