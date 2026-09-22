use crate::{AppState, services::project_store};
use serde::Serialize;
use std::{fs, path::Path};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn default_project_parent(app: AppHandle) -> Result<String, project_store::CommandError> {
    app.path().document_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| project_store::CommandError::new("PATH_UNAVAILABLE", format!("Cannot find the Documents folder: {error}")))
}

#[derive(Serialize)]
pub struct ManuscriptFile {
    name: String,
    text: String,
}

#[tauri::command]
pub fn read_manuscript(path: String) -> Result<ManuscriptFile, project_store::CommandError> {
    let file = Path::new(&path);
    let extension = file
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "txt" | "md" | "markdown") {
        return Err(project_store::CommandError::new(
            "UNSUPPORTED_FILE",
            "Choose a TXT or Markdown manuscript.",
        ));
    }
    let metadata = fs::metadata(file)
        .map_err(|error| project_store::CommandError::io("Cannot inspect manuscript", error))?;
    if metadata.len() > project_store::MAX_MANUSCRIPT_BYTES as u64 {
        return Err(project_store::CommandError::new(
            "MANUSCRIPT_TOO_LARGE",
            "The manuscript is larger than 20 MB.",
        ));
    }
    let text = fs::read_to_string(file)
        .map_err(|error| project_store::CommandError::io("Cannot read manuscript", error))?;
    project_store::validate_manuscript(&text)?;
    Ok(ManuscriptFile {
        name: file
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("manuscript.txt")
            .to_string(),
        text,
    })
}

#[tauri::command]
pub fn create_project(
    state: State<'_, AppState>,
    parent_path: String,
    title: String,
    manuscript: String,
) -> Result<project_store::ProjectSnapshot, project_store::CommandError> {
    let _guard = state
        .project_write_lock
        .lock()
        .map_err(|_| project_store::CommandError::internal("project lock is unavailable"))?;
    project_store::create(&parent_path, &title, &manuscript)
}

#[tauri::command]
pub fn open_project(
    state: State<'_, AppState>,
    root_path: String,
) -> Result<project_store::ProjectSnapshot, project_store::CommandError> {
    let _guard = state
        .project_write_lock
        .lock()
        .map_err(|_| project_store::CommandError::internal("project lock is unavailable"))?;
    project_store::open(&root_path)
}

#[tauri::command]
pub fn update_chapter(
    state: State<'_, AppState>,
    root_path: String,
    expected_revision: u64,
    chapter_id: String,
    title: String,
    source_text: String,
) -> Result<project_store::ProjectSnapshot, project_store::CommandError> {
    let _guard = state
        .project_write_lock
        .lock()
        .map_err(|_| project_store::CommandError::internal("project lock is unavailable"))?;
    project_store::update_chapter(
        &root_path,
        expected_revision,
        &chapter_id,
        &title,
        &source_text,
    )
}

#[tauri::command]
pub fn reorder_chapters(
    state: State<'_, AppState>,
    root_path: String,
    expected_revision: u64,
    chapter_ids: Vec<String>,
) -> Result<project_store::ProjectSnapshot, project_store::CommandError> {
    let _guard = state
        .project_write_lock
        .lock()
        .map_err(|_| project_store::CommandError::internal("project lock is unavailable"))?;
    project_store::reorder(&root_path, expected_revision, &chapter_ids)
}
