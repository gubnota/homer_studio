use super::{
    audio_assets::{self, AudioComposition, AudioVariant, ProcessingHistory},
    audio_edits,
    project_store::CommandError,
    settings::Settings,
    sound_store::now_ms,
};
use crate::runtime::AppHandle;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingContext {
    #[serde(default)]
    pub project_id: Option<String>,
    pub context_type: Option<String>,
    pub chapter_id: Option<String>,
    pub segment_id: Option<String>,
    pub speaker_id: Option<String>,
    pub text: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingSession {
    #[serde(flatten)]
    pub context: RecordingContext,
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub notes: String,
    pub favorite: bool,
    pub deleted: bool,
    pub revision: u64,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub takes: Vec<AudioVariant>,
    pub selected_take_id: Option<String>,
    pub undo: Vec<String>,
    pub redo: Vec<String>,
}
fn title(name: &str) -> Result<String, CommandError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 160 {
        return Err(CommandError::new(
            "INVALID_MEMO_NAME",
            "Use a name of 1–160 characters.",
        ));
    }
    Ok(name.into())
}
pub fn load_at(base: &Path, id: &str) -> Result<RecordingSession, CommandError> {
    audio_assets::id(id)?;
    let metadata_path = base.join("memos").join(format!("{id}.json"));
    if fs::metadata(&metadata_path)
        .map_err(|e| CommandError::io("Cannot inspect recording", e))?
        .len()
        > 8 * 1024 * 1024
    {
        return Err(CommandError::new(
            "INVALID_MEMO",
            "Recording metadata is too large.",
        ));
    }
    let bytes =
        fs::read(metadata_path).map_err(|e| CommandError::io("Cannot open recording", e))?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(CommandError::new(
            "INVALID_MEMO",
            "Recording metadata is too large.",
        ));
    }
    let memo: RecordingSession = serde_json::from_slice(&bytes)
        .map_err(|e| CommandError::new("INVALID_MEMO", e.to_string()))?;
    if memo.schema_version != 1 || memo.id != id {
        return Err(CommandError::new(
            "INVALID_MEMO",
            "Unsupported recording metadata.",
        ));
    }
    let ids: std::collections::HashSet<_> = memo.takes.iter().map(|t| t.id.as_str()).collect();
    if ids.len() != memo.takes.len()
        || memo
            .selected_take_id
            .iter()
            .chain(memo.undo.iter())
            .chain(memo.redo.iter())
            .any(|id| !ids.contains(id.as_str()))
    {
        return Err(CommandError::new(
            "INVALID_MEMO",
            "Recording take references are invalid.",
        ));
    }
    for t in &memo.takes {
        audio_assets::id(&t.id)?;
        if t.path != format!("assets/{}.wav", t.id) {
            return Err(CommandError::new(
                "UNSAFE_AUDIO_PATH",
                "Invalid recording source path.",
            ));
        }
    }
    Ok(memo)
}
pub fn load(app: &AppHandle, id: &str) -> Result<RecordingSession, CommandError> {
    load_at(&audio_assets::root(app)?, id)
}
pub fn list(app: &AppHandle, include_deleted: bool) -> Result<Vec<RecordingSession>, CommandError> {
    let folder = audio_assets::root(app)?.join("memos");
    if !folder.exists() {
        return Ok(vec![]);
    }
    let mut result = vec![];
    for e in fs::read_dir(folder).map_err(|e| CommandError::io("Cannot list recordings", e))? {
        let entry = e.map_err(|e| CommandError::io("Cannot read recording entry", e))?;
        if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let id = entry
            .path()
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let memo = load(app, &id)?;
        if memo.context.project_id.is_none()
            && memo.context.chapter_id.is_none()
            && (include_deleted || !memo.deleted)
        {
            result.push(memo);
        }
    }
    result.sort_by_key(|m| std::cmp::Reverse(m.updated_at_ms));
    Ok(result)
}
pub fn save_at(base: &Path, memo: &RecordingSession) -> Result<(), CommandError> {
    audio_assets::id(&memo.id)?;
    audio_assets::atomic_json(&base.join("memos").join(format!("{}.json", memo.id)), memo)
}
pub fn save(app: &AppHandle, memo: &RecordingSession) -> Result<(), CommandError> {
    save_at(&audio_assets::root(app)?, memo)
}
pub fn create(app: &AppHandle, name: &str) -> Result<RecordingSession, CommandError> {
    let now = now_ms();
    let memo = RecordingSession {
        context: RecordingContext::default(),
        schema_version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        name: title(name)?,
        notes: String::new(),
        favorite: false,
        deleted: false,
        revision: 0,
        created_at_ms: now,
        updated_at_ms: now,
        takes: vec![],
        selected_take_id: None,
        undo: vec![],
        redo: vec![],
    };
    save(app, &memo)?;
    Ok(memo)
}
pub fn revision(memo: &RecordingSession, expected: u64) -> Result<(), CommandError> {
    if memo.revision != expected {
        Err(CommandError::new(
            "REVISION_CONFLICT",
            "This recording changed. Reload it before saving.",
        ))
    } else {
        Ok(())
    }
}
pub fn update(
    app: &AppHandle,
    id: &str,
    expected: u64,
    name: &str,
    notes: &str,
    favorite: bool,
    deleted: bool,
) -> Result<RecordingSession, CommandError> {
    let mut memo = load(app, id)?;
    revision(&memo, expected)?;
    if notes.len() > 64 * 1024 {
        return Err(CommandError::new(
            "INVALID_MEMO_NOTES",
            "Notes are too long.",
        ));
    }
    memo.name = title(name)?;
    memo.notes = notes.into();
    memo.favorite = favorite;
    memo.deleted = deleted;
    touch(&mut memo);
    save(app, &memo)?;
    Ok(memo)
}
pub fn selected(memo: &RecordingSession) -> Result<&AudioVariant, CommandError> {
    if memo.deleted {
        return Err(CommandError::new(
            "MEMO_DELETED",
            "Restore this memo before editing audio.",
        ));
    }
    memo.takes
        .iter()
        .find(|t| Some(&t.id) == memo.selected_take_id.as_ref())
        .ok_or_else(|| CommandError::new("NO_SELECTED_TAKE", "Record or import a take first."))
}
pub fn selected_path(
    app: &AppHandle,
    memo: &RecordingSession,
) -> Result<std::path::PathBuf, CommandError> {
    audio_assets::path(&audio_assets::root(app)?, &selected(memo)?.id)
}
fn touch(memo: &mut RecordingSession) {
    memo.revision += 1;
    memo.updated_at_ms = now_ms();
}
pub fn choose(
    app: &AppHandle,
    id: &str,
    expected: u64,
    take_id: &str,
    action: &str,
) -> Result<RecordingSession, CommandError> {
    let mut memo = load(app, id)?;
    revision(&memo, expected)?;
    match action {
        "undo" => {
            if let Some(previous) = memo.undo.pop() {
                if let Some(current) = memo.selected_take_id.take() {
                    memo.redo.push(current);
                }
                memo.selected_take_id = Some(previous);
            }
        }
        "redo" => {
            if let Some(next) = memo.redo.pop() {
                if let Some(current) = memo.selected_take_id.take() {
                    memo.undo.push(current);
                }
                memo.selected_take_id = Some(next);
            }
        }
        "reject" => {
            let t = memo
                .takes
                .iter_mut()
                .find(|t| t.id == take_id && t.state == "preview")
                .ok_or_else(|| {
                    CommandError::new("INVALID_PREVIEW", "Choose a preview to reject.")
                })?;
            t.state = "rejected".into();
        }
        "accept" | "select" => {
            let t = memo
                .takes
                .iter_mut()
                .find(|t| t.id == take_id && t.state != "rejected")
                .ok_or_else(|| CommandError::new("TAKE_NOT_FOUND", "The take is unavailable."))?;
            if t.state == "preview" {
                t.state = "accepted".into();
            }
            if memo.selected_take_id.as_deref() != Some(take_id) {
                if let Some(previous) = memo.selected_take_id.take() {
                    memo.undo.push(previous);
                    if memo.undo.len() > 100 {
                        memo.undo.remove(0);
                    }
                }
                memo.redo.clear();
                memo.selected_take_id = Some(take_id.into());
            }
        }
        _ => {
            return Err(CommandError::new(
                "INVALID_TAKE_ACTION",
                "Unknown take action.",
            ));
        }
    }
    touch(&mut memo);
    save(app, &memo)?;
    Ok(memo)
}
pub fn import_file(
    app: &AppHandle,
    memo_id: &str,
    source: &Path,
    name: &str,
    original_rate: Option<u32>,
    settings: &Settings,
    cancel: Arc<AtomicBool>,
) -> Result<RecordingSession, CommandError> {
    let mut memo = load(app, memo_id)?;
    if memo.deleted {
        return Err(CommandError::new(
            "MEMO_DELETED",
            "Restore this recording before adding takes.",
        ));
    }
    let base = audio_assets::root(app)?;
    fs::create_dir_all(base.join("assets"))
        .map_err(|e| CommandError::io("Cannot create audio storage", e))?;
    let id = uuid::Uuid::new_v4().to_string();
    let path = audio_assets::path(&base, &id)?;
    let result = (|| {
        let duration_ms = if original_rate.is_some() {
            // Capture is already mono float PCM. Preserve the exact native-rate bytes.
            fs::copy(source, &path)
                .map_err(|e| CommandError::io("Cannot preserve recording", e))?;
            super::speech::probe_duration(&path, settings)?
        } else {
            audio_assets::normalize(source, &path, settings, cancel.clone())?
        };
        let composition = AudioComposition {
            clips: vec![audio_assets::clip(Some(id.clone()), 0, duration_ms)],
        };
        let variant = AudioVariant {
            favorite: false,
            notes: String::new(),
            id: id.clone(),
            parent_id: None,
            name: title(name)?,
            path: format!("assets/{id}.wav"),
            duration_ms,
            sample_rate: original_rate.unwrap_or(48000),
            original_sample_rate: original_rate,
            channels: 1,
            created_at_ms: now_ms(),
            state: "source".into(),
            composition,
            processing: None,
            cues: vec![],
        };
        if let Some(prev) = memo.selected_take_id.take() {
            memo.undo.push(prev);
        }
        if memo.undo.len() > 100 {
            memo.undo.drain(..memo.undo.len() - 100);
        }
        memo.redo.clear();
        memo.selected_take_id = Some(id);
        memo.takes.push(variant);
        touch(&mut memo);
        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(CommandError::new("CANCELLED", "Audio operation cancelled."));
        }
        save(app, &memo)?;
        Ok(memo)
    })();
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}
pub fn publish_render(
    app: &AppHandle,
    memo_id: &str,
    expected: u64,
    composition: AudioComposition,
    name: &str,
    preview: bool,
    history: Option<ProcessingHistory>,
    cue_override: Option<Vec<super::project_store::LineCue>>,
    settings: &Settings,
    cancel: Arc<AtomicBool>,
) -> Result<RecordingSession, CommandError> {
    let mut memo = load(app, memo_id)?;
    revision(&memo, expected)?;
    let parent = selected(&memo)?.id.clone();
    let base = audio_assets::root(app)?;
    let id = uuid::Uuid::new_v4().to_string();
    let path = audio_assets::path(&base, &id)?;
    let result = (|| {
        let duration_ms = audio_edits::render(
            &base,
            &composition,
            &memo.takes,
            &path,
            settings,
            cancel.clone(),
        )?;
        let current = load(app, memo_id)?;
        revision(&current, expected)?;
        let variant = AudioVariant {
            favorite: false,
            notes: String::new(),
            id: id.clone(),
            parent_id: Some(parent.clone()),
            name: name.into(),
            path: format!("assets/{id}.wav"),
            duration_ms,
            sample_rate: 48000,
            original_sample_rate: None,
            channels: 1,
            created_at_ms: now_ms(),
            state: if preview { "preview" } else { "accepted" }.into(),
            cues: cue_override
                .unwrap_or_else(|| audio_edits::remap_cues(&composition, &memo.takes)),
            composition,
            processing: history,
        };
        memo.takes.push(variant);
        if !preview {
            memo.undo.push(parent);
            if memo.undo.len() > 100 {
                memo.undo.drain(..memo.undo.len() - 100);
            }
            memo.redo.clear();
            memo.selected_take_id = Some(id);
        }
        touch(&mut memo);
        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(CommandError::new("CANCELLED", "Audio operation cancelled."));
        }
        save(app, &memo)?;
        Ok(memo)
    })();
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}
pub fn publish_processed(
    app: &AppHandle,
    memo_id: &str,
    expected: u64,
    source: &Path,
    mut history: ProcessingHistory,
    start: u64,
    end: u64,
    crossfade: u64,
    settings: &Settings,
    cancel: Arc<AtomicBool>,
) -> Result<RecordingSession, CommandError> {
    let mut memo = load(app, memo_id)?;
    revision(&memo, expected)?;
    let parent = selected(&memo)?.clone();
    let base = audio_assets::root(app)?;
    let id = uuid::Uuid::new_v4().to_string();
    let path = audio_assets::path(&base, &id)?;
    let duration_ms = match audio_assets::normalize(source, &path, settings, cancel.clone()) {
        Ok(duration) => duration,
        Err(error) => {
            let _ = fs::remove_file(&path);
            return Err(error);
        }
    };
    history.source_start_ms = start;
    history.source_end_ms = end;
    let raw = AudioVariant {
        favorite: false,
        notes: String::new(),
        id: id.clone(),
        parent_id: Some(parent.id),
        name: "Processed selection".into(),
        path: format!("assets/{id}.wav"),
        duration_ms,
        sample_rate: 48000,
        original_sample_rate: None,
        channels: 1,
        created_at_ms: now_ms(),
        state: "preview".into(),
        composition: AudioComposition {
            clips: vec![audio_assets::clip(Some(id.clone()), 0, duration_ms)],
        },
        processing: Some(history.clone()),
        cues: audio_edits::replacement_cues(selected(&memo)?, start, end, duration_ms),
    };
    let composition = match audio_edits::replacement(selected(&memo)?, &raw, start, end, crossfade)
    {
        Ok(composition) => composition,
        Err(error) => {
            let _ = fs::remove_file(&path);
            return Err(error);
        }
    };
    memo.takes.push(raw); // Commit both new files together after rendering; a failed render leaves no published metadata.
    let final_id = uuid::Uuid::new_v4().to_string();
    let final_path = audio_assets::path(&base, &final_id)?;
    let result = (|| {
        let duration_ms = audio_edits::render(
            &base,
            &composition,
            &memo.takes,
            &final_path,
            settings,
            cancel.clone(),
        )?;
        revision(&load(app, memo_id)?, expected)?;
        memo.takes.push(AudioVariant {
            favorite: false,
            notes: String::new(),
            id: final_id.clone(),
            parent_id: memo.selected_take_id.clone(),
            name: format!("{} preview", history.preset),
            path: format!("assets/{final_id}.wav"),
            duration_ms,
            sample_rate: 48000,
            original_sample_rate: None,
            channels: 1,
            created_at_ms: now_ms(),
            state: "preview".into(),
            cues: audio_edits::remap_cues(&composition, &memo.takes),
            composition,
            processing: Some(history),
        });
        touch(&mut memo);
        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(CommandError::new("CANCELLED", "Audio operation cancelled."));
        }
        save(app, &memo)?;
        Ok(memo)
    })();
    if result.is_err() {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(final_path);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(feature = "server", not(feature = "desktop")))]
    #[test]
    fn project_recordings_are_not_global_memos() {
        let base = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let app = AppHandle::new(base.clone(), base.clone());
        let standalone = create(&app, "Standalone").unwrap();
        let mut project = create(&app, "Project recording").unwrap();
        project.context.project_id = Some(uuid::Uuid::new_v4().to_string());
        save(&app, &project).unwrap();
        let mut chapter = create(&app, "Book recording").unwrap();
        chapter.context.chapter_id = Some(uuid::Uuid::new_v4().to_string());
        save(&app, &chapter).unwrap();
        for include_deleted in [false, true] {
            let visible = list(&app, include_deleted).unwrap();
            assert_eq!(visible.len(), 1);
            assert_eq!(visible[0].id, standalone.id);
        }
        assert_eq!(
            load(&app, &project.id).unwrap().context.project_id,
            project.context.project_id
        );
        fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn revision_conflict_preserves_metadata() {
        let m = RecordingSession {
            context: RecordingContext::default(),
            schema_version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            name: "Memo".into(),
            notes: "".into(),
            favorite: false,
            deleted: false,
            revision: 2,
            created_at_ms: 0,
            updated_at_ms: 0,
            takes: vec![],
            selected_take_id: None,
            undo: vec![],
            redo: vec![],
        };
        assert!(revision(&m, 1).is_err());
        let base = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        save_at(&base, &m).unwrap();
        let loaded = load_at(&base, &m.id).unwrap();
        assert_eq!(loaded.revision, 2);
        fs::remove_dir_all(base).unwrap();
    }
}

/// Take metadata never changes the source or composition.
pub fn update_take(
    app: &AppHandle,
    id: &str,
    expected: u64,
    take_id: &str,
    name: &str,
    notes: &str,
    favorite: bool,
    deleted: bool,
) -> Result<RecordingSession, CommandError> {
    let mut memo = load(app, id)?;
    revision(&memo, expected)?;
    if notes.len() > 65536 {
        return Err(CommandError::new(
            "INVALID_TAKE",
            "Take notes exceed 64 KB.",
        ));
    }
    let take = memo
        .takes
        .iter_mut()
        .find(|t| t.id == take_id)
        .ok_or_else(|| CommandError::new("TAKE_NOT_FOUND", "Choose an existing take."))?;
    take.name = title(name)?;
    take.notes = notes.into();
    take.favorite = favorite;
    if deleted {
        take.state = "rejected".into();
        if memo.selected_take_id.as_deref() == Some(take_id) {
            memo.selected_take_id = memo
                .takes
                .iter()
                .find(|t| t.id != take_id && t.state == "accepted")
                .map(|t| t.id.clone());
        }
        memo.undo.retain(|id| id != take_id);
        memo.redo.retain(|id| id != take_id);
    }
    touch(&mut memo);
    save(app, &memo)?;
    Ok(memo)
}
