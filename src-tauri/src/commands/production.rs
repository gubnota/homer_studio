use crate::services::{
    llm::{self, TextCandidate},
    project_store::{self, CommandError, ProjectSnapshot},
    settings,
};
use tauri::AppHandle;

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
