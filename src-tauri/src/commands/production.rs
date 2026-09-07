use crate::AppState;
use crate::services::{
    llm::{self, TextCandidate},
    project_store::{self, CommandError, ProjectSnapshot},
    settings,
};
use std::path::Path;
use tauri::{AppHandle, State};

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
pub fn list_voices() -> Result<Vec<crate::services::speech::Voice>, CommandError> {
    crate::services::speech::list_voices()
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
                &text,
                &settings.speech.voice_id,
                settings.speech.rate,
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
