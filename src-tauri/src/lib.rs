mod commands;
mod services;

use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};
use tauri::Manager;

pub use homer_core::AppState;
static OPEN_FILES: std::sync::LazyLock<Mutex<Vec<String>>> =
    std::sync::LazyLock::new(|| Mutex::new(Vec::new()));
#[tauri::command]
fn take_open_wave_files() -> Vec<String> {
    OPEN_FILES
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default()
}
fn queue_wave_file(path: String) {
    if path == ":open-dialog:" || path.to_lowercase().ends_with(".wavehs") {
        if let Ok(mut q) = OPEN_FILES.lock() {
            q.push(path)
        }
    }
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
    for path in std::env::args().skip(1) {
        queue_wave_file(path)
    }
    let audio_assets = Arc::new(RwLock::new(HashMap::new()));
    let protocol_assets = audio_assets.clone();
    let mut context = tauri::generate_context!();
    if let Ok(identifier) = std::env::var("HOMER_SMOKE_IDENTIFIER") {
        context.config_mut().identifier = identifier;
        if let Some(window) = context.config_mut().app.windows.first_mut() {
            window.data_directory =
                std::env::var_os("HOMER_SMOKE_WEB_DATA").map(std::path::PathBuf::from);
        }
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let menu = tauri::menu::Menu::default(app.handle())?;
            for item in menu.items()? {
                if let tauri::menu::MenuItemKind::Submenu(file) = item {
                    if file.text()? == "File" {
                        file.insert(
                            &tauri::menu::MenuItem::with_id(
                                app,
                                "open-wave-project",
                                "Open Wave Project…",
                                true,
                                Some("CmdOrCtrl+O"),
                            )?,
                            0,
                        )?;
                    }
                }
            }
            app.set_menu(menu)?;
            if std::env::var_os("HOMER_SMOKE_IDENTIFIER").is_some() {
                return Ok(());
            }
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let selected = services::settings::load(&handle)
                    .unwrap_or_default()
                    .speech
                    .provider;
                let name = if selected == "chatterbox_original" {
                    "original"
                } else {
                    "chatterbox"
                };
                if let Err(error) = services::sound_workers::start_local_worker(&handle, name) {
                    eprintln!("Could not start Chatterbox: {}", error.message);
                }
            });
            Ok(())
        })
        .on_menu_event(|_, event| {
            if event.id().as_ref() == "open-wave-project" {
                queue_wave_file(":open-dialog:".into())
            }
        })
        .register_uri_scheme_protocol("audio", move |_context, request| {
            services::audio_protocol::respond(&protocol_assets, request)
        })
        .manage(AppState {
            project_write_lock: Arc::new(Mutex::new(())),
            jobs: services::jobs::JobStore::new(),
            audio_assets,
        })
        .manage(services::audio_capture::CaptureService::default())
        .invoke_handler(tauri::generate_handler![
            desktop_info,
            take_open_wave_files,
            commands::wave_studio::wave_cancel_peaks,
            commands::wave_studio::wave_normalize,
            commands::wave_studio::wave_join,
            commands::wave_studio::wave_generate_speech,
            commands::wave_studio::wave_convert_regions,
            commands::wave_studio::wave_processing_result,
            commands::wave_studio::wave_list,
            commands::wave_studio::wave_operation_status,
            commands::wave_studio::wave_cancel_operation,
            commands::wave_studio::wave_has_video,
            commands::wave_studio::wave_import_video,
            commands::wave_studio::wave_video_url,
            commands::wave_studio::wave_deleted,
            commands::wave_studio::wave_purge,
            commands::wave_studio::wave_reveal,
            commands::wave_studio::wave_delete,
            commands::wave_studio::wave_save_copy,
            commands::wave_studio::wave_export_bundle,
            commands::wave_studio::wave_import_bundle,
            commands::wave_studio::wave_open_copy,
            commands::wave_studio::wave_get,
            commands::wave_studio::wave_create,
            commands::wave_studio::wave_save,
            commands::wave_studio::wave_import,
            commands::wave_studio::wave_peaks,
            commands::wave_studio::wave_preview,
            commands::wave_studio::wave_cancel_preview,
            commands::wave_studio::wave_export,
            commands::wave_studio::wave_sfx_list,
            commands::wave_studio::wave_sfx_add,
            commands::wave_studio::wave_sfx_update,
            commands::wave_studio::wave_source_url,
            commands::production::update_voice,
            commands::audio_processing::audio_engine_configs,
            commands::audio_processing::save_audio_engine,
            commands::audio_processing::audio_engine_status,
            commands::audio_processing::setup_audio_engine,
            commands::audio_processing::list_voice_profiles,
            commands::audio_processing::save_voice_profile,
            commands::audio_processing::process_memo_audio,
            commands::audio_processing::publish_memo_audio,
            commands::audio_processing::import_library_audio,
            commands::audio::memo_source_path,
            commands::audio::convert_memo_segment,
            commands::audio::list_memos,
            commands::audio::create_memo,
            commands::audio::get_memo,
            commands::audio::update_memo,
            commands::audio::choose_memo_take,
            commands::audio::import_memo_audio,
            commands::audio::edit_memo_audio,
            commands::audio::extract_memo_selection,
            commands::audio::update_memo_take,
            commands::audio::replace_memo_range,
            commands::audio::memo_audio_url,
            commands::audio::memo_waveform,
            commands::audio::export_memo,
            commands::audio_capture::audio_capture_devices,
            commands::audio_capture::audio_capture_permission,
            commands::audio_capture::audio_capture_start,
            commands::audio_capture::audio_capture_control,
            commands::project::create_project,
            commands::project::default_project_parent,
            commands::project::open_project,
            commands::project::update_chapter,
            commands::project::reorder_chapters,
            commands::project::read_manuscript,
            commands::system::get_settings,
            commands::system::save_settings,
            commands::system::tool_diagnostics,
            commands::system::list_jobs,
            commands::system::control_job,
            commands::system::dismiss_jobs,
            commands::system::model_status,
            commands::system::install_model,
            commands::system::worker_runtime_status,
            commands::system::install_worker_runtime,
            commands::production::process_text,
            commands::production::accept_processed_text,
            commands::production::list_voices,
            commands::production::create_voice,
            commands::production::add_voice_sample,
            commands::production::add_recorded_voice_sample,
            commands::production::select_voice_sample,
            commands::production::voice_sample_url,
            commands::production::delete_voice,
            commands::production::preview_voice,
            commands::production::generate_chapter_audio,
            commands::production::generate_chapters_audio,
            commands::production::generate_segment_audio,
            commands::production::assemble_chapter_takes,
            commands::production::import_chapter_audio,
            commands::production::export_chapter_audio,
            commands::production::delete_generated_chapter_audio,
            commands::production::delete_generated_chapter_audio_many,
            commands::production::delete_exports,
            commands::production::save_export_file,
            commands::production::save_exports,
            commands::production::save_chapters,
            commands::production::set_chapter_review,
            commands::production::audio_url,
            commands::production::segment_take_url,
            commands::production::select_segment_take,
            commands::production::convert_segment_recording,
            commands::production::audio_waveform,
            commands::production::audio_waveform_window,
            commands::production::export_project,
            commands::production::export_audio_url,
            commands::production::read_export_timestamps,
            commands::sounds::sound_workers,
            commands::sounds::list_sounds,
            commands::sounds::delete_sounds,
            commands::sounds::generate_sound,
            commands::sounds::convert_voice_clip,
            commands::sounds::sound_audio_url,
            commands::sounds::export_sound,
            commands::sounds::export_sounds
        ])
        .build(context)
        .expect("failed to build Homer Studio")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested {
                api, code: None, ..
            } = &event
            {
                if let Some(window) = app.get_webview_window("main") {
                    api.prevent_exit();
                    let _ = window.close();
                }
            }
            if let tauri::RunEvent::Exit = event {
                app.state::<services::audio_capture::CaptureService>()
                    .shutdown();
                let settings = services::settings::load(app).unwrap_or_default();
                for (url, engine) in [
                    (&settings.sounds.chatterbox_url, "chatterbox_turbo"),
                    (&settings.sounds.original_url, "chatterbox_original"),
                ] {
                    if let Err(error) = services::sound_workers::shutdown(url, engine) {
                        eprintln!("Could not stop {engine} worker: {}", error.message);
                    }
                }
            }
        });
}
