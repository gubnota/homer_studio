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
            let speech = crate::services::speech::generate_with_progress(
                &app,
                &text,
                &settings.speech.voice_id,
                &staged,
                &settings,
                &control,
                &|done, total| progress((10 + done.saturating_mul(75) / total.max(1)).min(85) as u8),
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
                speech.duration_ms,
                "generated",
                speech.cues,
            )?;
            Ok(())
        },
    ))
}

#[tauri::command]
pub fn generate_segment_audio(
    app: AppHandle,
    state: State<'_, AppState>,
    root_path: String,
    expected_revision: u64,
    chapter_id: String,
    segment_id: String,
) -> Result<String, CommandError> {
    let snapshot = project_store::open(&root_path)?;
    if snapshot.revision != expected_revision { return Err(CommandError::new("REVISION_CONFLICT", "Reload the project before narrating.")); }
    let chapter = snapshot.chapters.iter().find(|item| item.chapter.id == chapter_id)
        .ok_or_else(|| CommandError::new("CHAPTER_NOT_FOUND", "The chapter no longer exists."))?;
    let segment = chapter.chapter.segments.iter().find(|item| item.id == segment_id)
        .ok_or_else(|| CommandError::new("SEGMENT_NOT_FOUND", "The segment no longer exists."))?;
    let text = segment.text.clone();
    let label = format!("Narrate {} · section {}", chapter.chapter.title, segment.order + 1);
    let settings = settings::load(&app)?;
    let lock = state.project_write_lock.clone();
    Ok(state.jobs.enqueue("speech", label, move |control, progress| {
        control.boundary()?;
        progress(10);
        let staged = crate::services::speech::staged_path(&root_path, &chapter_id);
        let output = crate::services::speech::generate_with_progress(&app, &text, &settings.speech.voice_id, &staged, &settings, &control,
            &|done, total| progress((10 + done.saturating_mul(75) / total.max(1)).min(85) as u8))?;
        control.boundary()?;
        progress(90);
        let _guard = lock.lock().map_err(|_| CommandError::internal("project lock is unavailable"))?;
        project_store::commit_segment_take(&root_path, expected_revision, &chapter_id, &segment_id, &staged, output.duration_ms, true)?;
        Ok(())
    }))
}

#[tauri::command]
pub fn convert_segment_recording(app: AppHandle, state: State<'_, AppState>, root_path: String, expected_revision: u64, chapter_id: String, segment_id: String, bytes: Vec<u8>) -> Result<String, CommandError> {
    if bytes.is_empty() || bytes.len() > 20 * 1024 * 1024 { return Err(CommandError::new("INVALID_RECORDING", "Record a passage shorter than 20 seconds.")); }
    let snapshot = project_store::open(&root_path)?;
    if snapshot.revision != expected_revision || !snapshot.chapters.iter().any(|chapter| chapter.chapter.id == chapter_id && chapter.chapter.segments.iter().any(|segment| segment.id == segment_id)) {
        return Err(CommandError::new("REVISION_CONFLICT", "Reload the chapter before converting this section."));
    }
    let settings = settings::load(&app)?;
    let narrator = crate::services::voice_store::selected_sample(&app, &settings.speech.voice_id)?
        .ok_or_else(|| CommandError::new("VOICE_HAS_NO_SAMPLE", "Choose a narrator voice with a recorded sample before voice conversion."))?;
    let health = crate::services::sound_workers::health(&settings.sounds.original_url, "chatterbox_original");
    if !health.ready || !health.categories.iter().any(|category| category == "voice_conversion") {
        return Err(CommandError::new("WORKER_UNAVAILABLE", "Start the updated Original Chatterbox worker with its local checkpoint before converting a recording."));
    }
    let lock = state.project_write_lock.clone();
    Ok(state.jobs.enqueue("speech", "Convert recorded delivery".into(), move |control, progress| {
        let ffmpeg = crate::services::process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
            .ok_or_else(|| CommandError::new("FFMPEG_NOT_FOUND", "Set FFmpeg in Settings before converting a recording."))?;
        let work = std::path::Path::new(&root_path).join(".work").join(format!("conversion-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&work).map_err(|error| CommandError::io("Cannot stage recording", error))?;
        let result = (|| {
            control.boundary()?;
            let source = work.join("source.webm");
            let wav = work.join("source.wav");
            let converted = work.join("converted.wav");
            let staged = work.join("converted.m4a");
            std::fs::write(&source, &bytes).map_err(|error| CommandError::io("Cannot stage recording", error))?;
            let args = vec!["-v".into(), "error".into(), "-y".into(), "-i".into(), source.to_string_lossy().to_string(), "-t".into(), "20".into(), "-ac".into(), "1".into(), "-ar".into(), "24000".into(), wav.to_string_lossy().to_string()];
            let conversion = crate::services::process_runner::run_bounded(&ffmpeg, &args, std::time::Duration::from_secs(60), control.cancelled.clone())?;
            if !conversion.success { return Err(CommandError::new("RECORDING_CONVERSION_FAILED", conversion.stderr)); }
            let source_wav = std::fs::read(&wav).map_err(|error| CommandError::io("Cannot read recording", error))?;
            progress(20);
            let worker_url = &settings.sounds.original_url;
            let source_id = crate::services::sound_workers::upload_reference(worker_url, &source_wav)?;
            let reference_id = crate::services::sound_workers::upload_reference(worker_url, &narrator)?;
            let request = serde_json::json!({"prompt":"Convert recorded delivery", "category":"voice_conversion", "durationSeconds":20, "seed":null, "sourceId":source_id, "referenceId":reference_id});
            let result_wav = crate::services::sound_workers::generate(worker_url, &request, &control)?;
            progress(75);
            std::fs::write(&converted, result_wav).map_err(|error| CommandError::io("Cannot stage converted take", error))?;
            let args = vec!["-v".into(), "error".into(), "-y".into(), "-i".into(), converted.to_string_lossy().to_string(), "-c:a".into(), "aac".into(), "-b:a".into(), "128k".into(), staged.to_string_lossy().to_string()];
            let encode = crate::services::process_runner::run_bounded(&ffmpeg, &args, std::time::Duration::from_secs(60), control.cancelled.clone())?;
            if !encode.success { return Err(CommandError::new("AUDIO_CONVERSION_FAILED", encode.stderr)); }
            let duration = crate::services::speech::probe_duration(&staged, &settings)?;
            control.boundary()?;
            progress(90);
            let _guard = lock.lock().map_err(|_| CommandError::internal("project lock is unavailable"))?;
            project_store::commit_segment_take(&root_path, expected_revision, &chapter_id, &segment_id, &staged, duration, false)?;
            Ok(())
        })();
        let _ = std::fs::remove_dir_all(work);
        result
    }))
}

#[tauri::command]
pub fn assemble_chapter_takes(
    app: AppHandle,
    state: State<'_, AppState>,
    root_path: String,
    expected_revision: u64,
    chapter_id: String,
) -> Result<String, CommandError> {
    if project_store::open(&root_path)?.revision != expected_revision { return Err(CommandError::new("REVISION_CONFLICT", "Reload the project before assembling.")); }
    let takes = project_store::selected_segment_takes(&root_path, &chapter_id)?;
    if takes.is_empty() { return Err(CommandError::new("NO_SEGMENTS", "This chapter has no segments.")); }
    let settings = settings::load(&app)?;
    let lock = state.project_write_lock.clone();
    Ok(state.jobs.enqueue("speech", "Assemble chapter takes".into(), move |control, progress| {
        control.boundary()?;
        let ffmpeg = crate::services::process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
            .ok_or_else(|| CommandError::new("FFMPEG_NOT_FOUND", "Set FFmpeg in Settings before assembling."))?;
        let staged = crate::services::speech::staged_path(&root_path, &chapter_id);
        let list = staged.with_extension("txt");
        let contents = takes.iter().map(|(_, path)| format!("file '{}'\n", path.to_string_lossy().replace('\'', "'\\''"))).collect::<String>();
        std::fs::write(&list, contents).map_err(|error| CommandError::io("Cannot stage take list", error))?;
        let args = vec!["-v".into(), "error".into(), "-y".into(), "-f".into(), "concat".into(), "-safe".into(), "0".into(), "-i".into(), list.to_string_lossy().to_string(), "-vn".into(), "-c:a".into(), "aac".into(), "-b:a".into(), "128k".into(), "-ar".into(), "48000".into(), "-ac".into(), "2".into(), staged.to_string_lossy().to_string()];
        progress(20);
        let result = crate::services::process_runner::run_bounded(&ffmpeg, &args, std::time::Duration::from_secs(3600), control.cancelled.clone())?;
        let _ = std::fs::remove_file(list);
        if !result.success { return Err(CommandError::new("AUDIO_CONVERSION_FAILED", result.stderr)); }
        control.boundary()?;
        progress(90);
        let duration = crate::services::speech::probe_duration(&staged, &settings)?;
        let mut offset = 0;
        let cues = takes.iter().enumerate().map(|(order, (segment, path))| {
            let measured = crate::services::speech::probe_duration(path, &settings)?;
            let cue = project_store::LineCue { order, text: crate::services::spoken_text::lines(&segment.text).join(" "), start_ms: offset, end_ms: offset + measured };
            offset += measured;
            Ok(cue)
        }).collect::<Result<Vec<_>, CommandError>>()?;
        let _guard = lock.lock().map_err(|_| CommandError::internal("project lock is unavailable"))?;
        project_store::commit_chapter_audio(&root_path, expected_revision, &chapter_id, &staged, duration, "generated", cues)?;
        Ok(())
    }))
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
                Vec::new(),
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
pub fn segment_take_url(state: State<'_, AppState>, root_path: String, chapter_id: String, segment_id: String, take_id: String) -> Result<String, CommandError> {
    let path = project_store::segment_take_path(&root_path, &chapter_id, &segment_id, &take_id)?;
    let id = uuid::Uuid::new_v4().to_string();
    state.audio_assets.write().map_err(|_| CommandError::internal("audio registry is unavailable"))?.insert(id.clone(), path);
    Ok(format!("audio://localhost/{id}"))
}

#[tauri::command]
pub fn select_segment_take(state: State<'_, AppState>, root_path: String, expected_revision: u64, chapter_id: String, segment_id: String, take_id: String) -> Result<ProjectSnapshot, CommandError> {
    let _guard = state.project_write_lock.lock().map_err(|_| CommandError::internal("project lock is unavailable"))?;
    project_store::select_segment_take(&root_path, expected_revision, &chapter_id, &segment_id, &take_id)
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
pub fn export_chapter_audio(
    root_path: String,
    chapter_id: String,
    destination: String,
) -> Result<(), CommandError> {
    let target = Path::new(&destination);
    if target.extension().and_then(|value| value.to_str()).map(|value| value.to_ascii_lowercase()) != Some("m4a".into()) {
        return Err(CommandError::new("INVALID_EXPORT_PATH", "Choose an M4A destination."));
    }
    if !target.parent().is_some_and(Path::is_dir) {
        return Err(CommandError::new("INVALID_EXPORT_PATH", "The destination folder does not exist."));
    }
    let source = project_store::chapter_audio_path(&root_path, &chapter_id)?;
    std::fs::copy(source, target).map_err(|error| CommandError::io("Cannot export chapter audio", error))?;
    Ok(())
}

#[tauri::command]
pub fn delete_generated_chapter_audio(
    state: State<'_, AppState>,
    root_path: String,
    expected_revision: u64,
    chapter_id: String,
) -> Result<ProjectSnapshot, CommandError> {
    let _guard = state.project_write_lock.lock()
        .map_err(|_| CommandError::internal("project lock is unavailable"))?;
    project_store::delete_generated_audio(&root_path, expected_revision, &chapter_id)
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
