use crate::services::{
    llm::{self, TextCandidate},
    project_store::{self, CommandError, ProjectSnapshot},
    settings,
};
use crate::AppState;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn process_text(
    app: AppHandle,
    instruction: String,
    text: String,
) -> Result<TextCandidate, CommandError> {
    llm::process(&settings::load(&app)?.llm, &instruction, &text)
}

#[tauri::command]
pub fn accept_processed_text(
    root_path: String,
    expected_revision: u64,
    chapter_id: String,
    text: String,
) -> Result<ProjectSnapshot, CommandError> {
    project_store::accept_processed(&root_path, expected_revision, &chapter_id, &text)
}

#[tauri::command]
pub fn list_voices(
    app: AppHandle,
) -> Result<Vec<crate::services::voice_store::Voice>, CommandError> {
    crate::services::voice_store::list(&app)
}

#[tauri::command]
pub fn create_voice(
    app: AppHandle,
    name: String,
) -> Result<crate::services::voice_store::Voice, CommandError> {
    crate::services::voice_store::create(&app, &name)
}

#[tauri::command]
pub fn add_voice_sample(
    app: AppHandle,
    voice_id: String,
    name: String,
    source_path: String,
) -> Result<crate::services::voice_store::Voice, CommandError> {
    let voice = crate::services::voice_store::add_sample(
        &app,
        &voice_id,
        &name,
        Path::new(&source_path),
        &settings::load(&app)?,
    )?;
    queue_voice_preview(app, voice.id.clone());
    Ok(voice)
}

#[tauri::command]
pub fn add_recorded_voice_sample(
    app: AppHandle,
    voice_id: String,
    name: String,
    bytes: Vec<u8>,
) -> Result<crate::services::voice_store::Voice, CommandError> {
    if bytes.len() > 20 * 1024 * 1024 || bytes.is_empty() {
        return Err(CommandError::new(
            "INVALID_VOICE_SAMPLE",
            "Recording is empty or too large.",
        ));
    }
    let path = std::env::temp_dir().join(format!("homer-recording-{}.webm", uuid::Uuid::new_v4()));
    std::fs::write(&path, bytes)
        .map_err(|error| CommandError::io("Cannot stage recording", error))?;
    let result = crate::services::voice_store::add_sample(
        &app,
        &voice_id,
        &name,
        &path,
        &settings::load(&app)?,
    );
    let _ = std::fs::remove_file(path);
    let voice = result?;
    queue_voice_preview(app, voice.id.clone());
    Ok(voice)
}

fn queue_voice_preview(app: AppHandle, voice_id: String) {
    std::thread::spawn(move || {
        let Ok(settings) = settings::load(&app) else {
            return;
        };
        let Ok(output) = crate::services::voice_store::preview_path(&app, &voice_id) else {
            return;
        };
        let marker = output.with_file_name("preview.generating");
        let Ok(_guard) = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&marker)
        else {
            return;
        };
        let _ = std::fs::remove_file(&output);
        let temp = output.with_file_name("preview.pending.m4a");
        let _ = std::fs::remove_file(&temp);
        if crate::services::speech::generate(
            &app,
            "Welcome to Homer Studio. This is a preview of your audiobook voice.",
            &voice_id,
            &temp,
            &settings,
            &crate::services::jobs::JobControl::preview(),
        )
        .is_ok()
        {
            let _ = std::fs::rename(temp, output);
        }
        let _ = std::fs::remove_file(marker);
    });
}

#[tauri::command]
pub fn select_voice_sample(
    app: AppHandle,
    voice_id: String,
    sample_id: String,
) -> Result<crate::services::voice_store::Voice, CommandError> {
    let voice = crate::services::voice_store::select_sample(&app, &voice_id, &sample_id)?;
    queue_voice_preview(app, voice.id.clone());
    Ok(voice)
}

#[tauri::command]
pub fn voice_sample_url(
    app: AppHandle,
    state: State<'_, AppState>,
    voice_id: String,
    sample_id: String,
) -> Result<String, CommandError> {
    let path = crate::services::voice_store::sample_path(&app, &voice_id, &sample_id)?;
    let id = uuid::Uuid::new_v4().to_string();
    state
        .audio_assets
        .write()
        .map_err(|_| CommandError::internal("Audio registry unavailable"))?
        .insert(id.clone(), path);
    Ok(format!("audio://localhost/{id}"))
}

#[tauri::command]
pub fn delete_voice(app: AppHandle, voice_id: String) -> Result<(), CommandError> {
    crate::services::voice_store::delete(&app, &voice_id, &settings::load(&app)?.speech.voice_id)
}

#[tauri::command]
pub fn preview_voice(
    app: AppHandle,
    state: State<'_, AppState>,
    voice_id: String,
    _rate: u16,
) -> Result<String, CommandError> {
    if voice_id != crate::services::voice_store::DEFAULT_VOICE {
        let output = crate::services::voice_store::preview_path(&app, &voice_id)?;
        if !output.is_file() {
            return Err(CommandError::new(
                "VOICE_PREVIEW_PENDING",
                "The voice preview is still being prepared. Try again in a moment.",
            ));
        }
        let id = uuid::Uuid::new_v4().to_string();
        state
            .audio_assets
            .write()
            .map_err(|_| CommandError::internal("Audio registry unavailable"))?
            .insert(id.clone(), output);
        return Ok(format!("audio://localhost/{id}"));
    }
    let directory = app
        .path()
        .app_cache_dir()
        .map_err(|error| CommandError::internal(format!("Cannot locate preview cache: {error}")))?
        .join("voice-previews");
    let mut assets = state
        .audio_assets
        .write()
        .map_err(|_| CommandError::internal("audio registry is unavailable"))?;
    prepare_preview_directory(&directory, &mut assets)?;
    drop(assets);
    let output = directory.join(format!("{}.m4a", uuid::Uuid::new_v4()));
    let settings = settings::load(&app)?;
    crate::services::speech::generate(
        &app,
        "Welcome to Homer Studio. This is a preview of your audiobook voice.",
        &voice_id,
        &output,
        &settings,
        &crate::services::jobs::JobControl::preview(),
    )?;
    let id = uuid::Uuid::new_v4().to_string();
    state
        .audio_assets
        .write()
        .map_err(|_| CommandError::internal("audio registry is unavailable"))?
        .insert(id.clone(), output);
    Ok(format!("audio://localhost/{id}"))
}

fn prepare_preview_directory(
    directory: &Path,
    assets: &mut HashMap<String, PathBuf>,
) -> Result<(), CommandError> {
    let _ = std::fs::remove_dir_all(directory);
    std::fs::create_dir_all(directory)
        .map_err(|error| CommandError::io("Cannot create preview cache", error))?;
    assets.retain(|_, path| !path.starts_with(directory));
    Ok(())
}

#[tauri::command]
pub fn generate_chapter_audio(
    app: AppHandle,
    state: State<'_, AppState>,
    root_path: String,
    expected_revision: u64,
    chapter_id: String,
) -> Result<String, CommandError> {
    let snapshot = project_store::open(&root_path)?;
    if snapshot.revision != expected_revision {
        return Err(CommandError::new(
            "REVISION_CONFLICT",
            "The project changed. Reload it before creating audio.",
        ));
    }
    let chapter = snapshot
        .chapters
        .iter()
        .find(|item| item.chapter.id == chapter_id)
        .ok_or_else(|| CommandError::new("CHAPTER_NOT_FOUND", "The chapter no longer exists."))?;
    let text = chapter
        .processed_text
        .clone()
        .unwrap_or_else(|| chapter.source_text.clone());
    let title = chapter.chapter.title.clone();
    let settings = settings::load(&app)?;
    let lock = state.project_write_lock.clone();
    let queued_root = root_path.clone();
    let queued_chapter = chapter_id.clone();
    Ok(state.jobs.enqueue(
        "speech",
        format!("Narrate {title}"),
        move |control, progress| {
            control.boundary()?;
            progress(10);
            let staged = crate::services::speech::staged_path(&queued_root, &queued_chapter);
            let duration = crate::services::speech::generate(
                &app,
                &text,
                &settings.speech.voice_id,
                &staged,
                &settings,
                &control,
            )?;
            control.boundary()?;
            progress(90);
            let _guard = lock
                .lock()
                .map_err(|_| CommandError::internal("project lock is unavailable"))?;
            project_store::commit_chapter_audio(
                &queued_root,
                expected_revision,
                &queued_chapter,
                &staged,
                duration,
                "generated",
            )?;
            Ok(())
        },
    ))
}

#[tauri::command]
pub fn import_chapter_audio(
    app: AppHandle,
    state: State<'_, AppState>,
    root_path: String,
    expected_revision: u64,
    chapter_id: String,
    source_path: String,
) -> Result<String, CommandError> {
    let snapshot = project_store::open(&root_path)?;
    if snapshot.revision != expected_revision {
        return Err(CommandError::new(
            "REVISION_CONFLICT",
            "The project changed. Reload it before importing audio.",
        ));
    }
    let title = snapshot
        .chapters
        .iter()
        .find(|item| item.chapter.id == chapter_id)
        .map(|item| item.chapter.title.clone())
        .ok_or_else(|| CommandError::new("CHAPTER_NOT_FOUND", "The chapter no longer exists."))?;
    let source = Path::new(&source_path).to_path_buf();
    let settings = settings::load(&app)?;
    let lock = state.project_write_lock.clone();
    let queued_root = root_path.clone();
    let queued_chapter = chapter_id.clone();
    Ok(state.jobs.enqueue(
        "audio_import",
        format!("Import audio for {title}"),
        move |control, progress| {
            control.boundary()?;
            progress(10);
            let staged = crate::services::speech::staged_path(&queued_root, &queued_chapter);
            let duration = crate::services::speech::import(
                &source,
                &staged,
                &settings,
                control.cancelled.clone(),
            )?;
            control.boundary()?;
            progress(90);
            let _guard = lock
                .lock()
                .map_err(|_| CommandError::internal("project lock is unavailable"))?;
            project_store::commit_chapter_audio(
                &queued_root,
                expected_revision,
                &queued_chapter,
                &staged,
                duration,
                "imported",
            )?;
            Ok(())
        },
    ))
}

#[tauri::command]
pub fn set_chapter_review(
    state: State<'_, AppState>,
    root_path: String,
    expected_revision: u64,
    chapter_id: String,
    status: String,
) -> Result<ProjectSnapshot, CommandError> {
    let _guard = state
        .project_write_lock
        .lock()
        .map_err(|_| CommandError::internal("project lock is unavailable"))?;
    project_store::set_review_status(&root_path, expected_revision, &chapter_id, &status)
}

#[tauri::command]
pub fn audio_url(
    state: State<'_, AppState>,
    root_path: String,
    chapter_id: String,
) -> Result<String, CommandError> {
    let path = project_store::chapter_audio_path(&root_path, &chapter_id)?;
    let id = uuid::Uuid::new_v4().to_string();
    state
        .audio_assets
        .write()
        .map_err(|_| CommandError::internal("audio registry is unavailable"))?
        .insert(id.clone(), path);
    Ok(format!("audio://localhost/{id}"))
}

#[tauri::command]
pub fn audio_waveform(
    app: AppHandle,
    root_path: String,
    chapter_id: String,
) -> Result<Vec<f32>, CommandError> {
    let path = project_store::chapter_audio_path(&root_path, &chapter_id)?;
    crate::services::speech::waveform(&path, &settings::load(&app)?)
}

#[tauri::command]
pub fn export_project(
    app: AppHandle,
    state: State<'_, AppState>,
    root_path: String,
    expected_revision: u64,
) -> Result<String, CommandError> {
    let snapshot = project_store::open(&root_path)?;
    if snapshot.revision != expected_revision {
        return Err(CommandError::new(
            "REVISION_CONFLICT",
            "The project changed. Reload it before exporting.",
        ));
    }
    let title = snapshot.title.clone();
    let settings = settings::load(&app)?;
    let lock = state.project_write_lock.clone();
    let queued_root = root_path.clone();
    Ok(state.jobs.enqueue(
        "export",
        format!("Export {title}"),
        move |control, progress| {
            control.boundary()?;
            progress(10);
            let prepared = crate::services::exports::assemble(
                &queued_root,
                &snapshot,
                &settings,
                control.cancelled.clone(),
            )?;
            control.boundary()?;
            progress(90);
            let _guard = lock
                .lock()
                .map_err(|_| CommandError::internal("project lock is unavailable"))?;
            let committed = project_store::commit_export(
                &queued_root,
                expected_revision,
                &prepared.audio_path,
                &prepared.timestamps_path,
                prepared.duration_ms,
            );
            if committed.is_err() {
                if let Some(work) = prepared.audio_path.parent() {
                    let _ = std::fs::remove_dir_all(work);
                }
            }
            committed.map(|_| ())
        },
    ))
}

#[tauri::command]
pub fn export_audio_url(
    state: State<'_, AppState>,
    root_path: String,
    export_id: String,
) -> Result<String, CommandError> {
    let (path, _) = project_store::export_paths(&root_path, &export_id)?;
    let id = uuid::Uuid::new_v4().to_string();
    state
        .audio_assets
        .write()
        .map_err(|_| CommandError::internal("audio registry is unavailable"))?
        .insert(id.clone(), path);
    Ok(format!("audio://localhost/{id}"))
}

#[tauri::command]
pub fn read_export_timestamps(
    root_path: String,
    export_id: String,
) -> Result<String, CommandError> {
    let (_, path) = project_store::export_paths(&root_path, &export_id)?;
    std::fs::read_to_string(path)
        .map_err(|error| CommandError::io("Cannot read exported timestamps", error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_old_preview_files_and_registry_entries() {
        let root = std::env::temp_dir().join(format!("homer-preview-{}", uuid::Uuid::new_v4()));
        let directory = root.join("voice-previews");
        let old_preview = directory.join("old.m4a");
        let other_audio = root.join("chapter.m4a");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(&old_preview, b"old").unwrap();
        std::fs::write(&other_audio, b"chapter").unwrap();
        let mut assets = HashMap::from([
            ("preview".into(), old_preview.clone()),
            ("chapter".into(), other_audio.clone()),
        ]);

        prepare_preview_directory(&directory, &mut assets).unwrap();

        assert!(!old_preview.exists());
        assert!(directory.is_dir());
        assert!(!assets.contains_key("preview"));
        assert_eq!(assets.get("chapter"), Some(&other_audio));
        std::fs::remove_dir_all(root).unwrap();
    }
}
