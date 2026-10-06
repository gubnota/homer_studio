#[cfg(feature = "desktop")]
use crate::runtime::Manager;
use crate::runtime::{AppHandle, State};
use crate::{
    AppState,
    services::{
        audio_assets,
        audio_edits::{self, AudioEdit},
        memo_store::{self, RecordingSession},
        project_store::CommandError,
        settings,
        waveform::{self, PeakWindow},
    },
};
use std::path::Path;
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn list_memos(
    app: AppHandle,
    include_deleted: bool,
) -> Result<Vec<RecordingSession>, CommandError> {
    memo_store::list(&app, include_deleted)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn create_memo(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    context: Option<memo_store::RecordingContext>,
) -> Result<RecordingSession, CommandError> {
    let _guard = state
        .project_write_lock
        .lock()
        .map_err(|_| CommandError::internal("Library lock unavailable"))?;
    if let Some(context) = &context {
        if serde_json::to_vec(context)
            .map_err(|e| CommandError::internal(e.to_string()))?
            .len()
            > 65536
        {
            return Err(CommandError::new(
                "INVALID_CONTEXT",
                "Recording context exceeds 64 KB.",
            ));
        }
    }
    let mut memo = memo_store::create(&app, &name)?;
    if let Some(context) = context {
        memo.context = context;
        memo_store::save(&app, &memo)?;
    }
    Ok(memo)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn get_memo(app: AppHandle, memo_id: String) -> Result<RecordingSession, CommandError> {
    memo_store::load(&app, &memo_id)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn update_memo(
    app: AppHandle,
    state: State<'_, AppState>,
    memo_id: String,
    expected_revision: u64,
    name: String,
    notes: String,
    favorite: bool,
    deleted: bool,
) -> Result<RecordingSession, CommandError> {
    let _guard = state
        .project_write_lock
        .lock()
        .map_err(|_| CommandError::internal("Library lock unavailable"))?;
    memo_store::update(
        &app,
        &memo_id,
        expected_revision,
        &name,
        &notes,
        favorite,
        deleted,
    )
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn choose_memo_take(
    app: AppHandle,
    state: State<'_, AppState>,
    memo_id: String,
    expected_revision: u64,
    take_id: String,
    action: String,
) -> Result<RecordingSession, CommandError> {
    let _guard = state
        .project_write_lock
        .lock()
        .map_err(|_| CommandError::internal("Library lock unavailable"))?;
    memo_store::choose(&app, &memo_id, expected_revision, &take_id, &action)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn import_memo_audio(
    app: AppHandle,
    state: State<'_, AppState>,
    memo_id: String,
    expected_revision: u64,
    source_path: String,
    name: String,
) -> Result<String, CommandError> {
    memo_store::revision(&memo_store::load(&app, &memo_id)?, expected_revision)?;
    Ok(state.jobs.enqueue(
        "audio",
        "Import recording".into(),
        move |control, _progress| {
            control.boundary()?;
            let state = app.state::<AppState>();
            let _guard = state
                .project_write_lock
                .lock()
                .map_err(|_| CommandError::internal("Library lock unavailable"))?;
            memo_store::revision(&memo_store::load(&app, &memo_id)?, expected_revision)?;
            memo_store::import_file(
                &app,
                &memo_id,
                Path::new(&source_path),
                &name,
                None,
                &settings::load(&app)?,
                control.cancelled,
            )?;
            Ok(())
        },
    ))
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn edit_memo_audio(
    app: AppHandle,
    state: State<'_, AppState>,
    memo_id: String,
    expected_revision: u64,
    edit: AudioEdit,
) -> Result<String, CommandError> {
    memo_store::revision(&memo_store::load(&app, &memo_id)?, expected_revision)?;
    Ok(state.jobs.enqueue(
        "audio",
        format!("Edit recording: {}", edit.kind),
        move |control, _progress| {
            control.boundary()?;
            let state = app.state::<AppState>();
            let _guard = state
                .project_write_lock
                .lock()
                .map_err(|_| CommandError::internal("Library lock unavailable"))?;
            let memo = memo_store::load(&app, &memo_id)?;
            memo_store::revision(&memo, expected_revision)?;
            let composition = audio_edits::edit(memo_store::selected(&memo)?, &edit)?;
            memo_store::publish_render(
                &app,
                &memo_id,
                expected_revision,
                composition,
                &edit.kind,
                false,
                None,
                None,
                &settings::load(&app)?,
                control.cancelled,
            )?;
            Ok(())
        },
    ))
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn replace_memo_range(
    app: AppHandle,
    state: State<'_, AppState>,
    memo_id: String,
    expected_revision: u64,
    replacement_id: String,
    start_ms: u64,
    end_ms: u64,
    crossfade_ms: u64,
) -> Result<String, CommandError> {
    let memo = memo_store::load(&app, &memo_id)?;
    memo_store::revision(&memo, expected_revision)?;
    let replacement = memo
        .takes
        .iter()
        .find(|t| t.id == replacement_id && matches!(t.state.as_str(), "source" | "accepted"))
        .ok_or_else(|| {
            CommandError::new(
                "TAKE_NOT_FOUND",
                "Choose a saved recording for the replacement.",
            )
        })?;
    let mut annotated = memo.takes.clone();
    if let Some(take) = annotated.iter_mut().find(|t| t.id == replacement_id) {
        take.cues = audio_edits::replacement_cues(
            memo_store::selected(&memo)?,
            start_ms,
            end_ms,
            take.duration_ms,
        );
    }
    let composition = audio_edits::replacement(
        memo_store::selected(&memo)?,
        replacement,
        start_ms,
        end_ms,
        crossfade_ms,
    )?;
    let cues = audio_edits::remap_cues(&composition, &annotated);
    Ok(state.jobs.enqueue(
        "audio_retake",
        "Preview recorded replacement".into(),
        move |control, _| {
            control.boundary()?;
            let state = app.state::<AppState>();
            let _guard = state
                .project_write_lock
                .lock()
                .map_err(|_| CommandError::internal("Library lock unavailable"))?;
            memo_store::publish_render(
                &app,
                &memo_id,
                expected_revision,
                composition,
                "Recorded replacement preview",
                true,
                None,
                Some(cues),
                &settings::load(&app)?,
                control.cancelled,
            )?;
            Ok(())
        },
    ))
}
fn take_path(
    app: &AppHandle,
    memo_id: &str,
    take_id: &str,
) -> Result<std::path::PathBuf, CommandError> {
    let memo = memo_store::load(app, memo_id)?;
    if !memo
        .takes
        .iter()
        .any(|t| t.id == take_id && t.state != "rejected")
    {
        return Err(CommandError::new(
            "TAKE_NOT_FOUND",
            "This take is unavailable.",
        ));
    }
    audio_assets::path(&audio_assets::root(app)?, take_id)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn memo_audio_url(
    app: AppHandle,
    state: State<'_, AppState>,
    memo_id: String,
    take_id: String,
) -> Result<String, CommandError> {
    let path = take_path(&app, &memo_id, &take_id)?;
    let id = format!("memo-{take_id}");
    state
        .audio_assets
        .write()
        .map_err(|_| CommandError::internal("Audio registry unavailable"))?
        .insert(id.clone(), path);
    Ok(format!("audio://localhost/{id}"))
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub async fn memo_waveform(
    app: AppHandle,
    memo_id: String,
    take_id: String,
    start_ms: u64,
    end_ms: u64,
    max_peaks: usize,
) -> Result<PeakWindow, CommandError> {
    crate::runtime::async_runtime::spawn_blocking(move || {
        let path = take_path(&app, &memo_id, &take_id)?;
        waveform::peaks(
            &path,
            &audio_assets::root(&app)?.join("peaks"),
            &settings::load(&app)?,
            start_ms,
            end_ms,
            max_peaks,
            Default::default(),
        )
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn export_memo(
    app: AppHandle,
    state: State<'_, AppState>,
    memo_id: String,
    expected_revision: u64,
    output_path: String,
) -> Result<String, CommandError> {
    let memo = memo_store::load(&app, &memo_id)?;
    memo_store::revision(&memo, expected_revision)?;
    let take = memo_store::selected(&memo)?;
    if memo.deleted || !matches!(take.state.as_str(), "source" | "accepted") {
        return Err(CommandError::new(
            "UNACCEPTED_AUDIO",
            "Accept this preview before exporting.",
        ));
    }
    let duration_ms = take.duration_ms;
    let source = memo_store::selected_path(&app, &memo)?;
    Ok(state.jobs.enqueue(
        "audio_export",
        "Export recording".into(),
        move |control, _progress| {
            control.boundary()?;
            let output = Path::new(&output_path);
            let extension = output.extension().and_then(|s| s.to_str()).unwrap_or("");
            let codec = match extension {
                "wav" => "pcm_s24le",
                "flac" => "flac",
                "m4a" => "aac",
                "mp3" => "libmp3lame",
                _ => {
                    return Err(CommandError::new(
                        "INVALID_EXPORT_FORMAT",
                        "Choose WAV, FLAC, M4A or MP3.",
                    ));
                }
            };
            let staging =
                output.with_file_name(format!(".homer-{}.{}", uuid::Uuid::new_v4(), extension));
            let result = audio_assets::ffmpeg(
                &settings::load(&app)?,
                vec![
                    "-v".into(),
                    "error".into(),
                    "-nostdin".into(),
                    "-y".into(),
                    "-i".into(),
                    source.to_string_lossy().into(),
                    "-c:a".into(),
                    codec.into(),
                    staging.to_string_lossy().into(),
                ],
                control.cancelled.clone(),
            );
            let result = result.and_then(|_| {
                control.boundary()?;
                let state = app.state::<AppState>();
                let _guard = state
                    .project_write_lock
                    .lock()
                    .map_err(|_| CommandError::internal("Library lock unavailable"))?;
                memo_store::revision(&memo_store::load(&app, &memo_id)?, expected_revision)?;
                let measured =
                    crate::services::speech::probe_duration(&staging, &settings::load(&app)?)?;
                if measured.abs_diff(duration_ms) > 150 {
                    return Err(CommandError::new(
                        "INVALID_EXPORT_DURATION",
                        "Exported audio duration changed unexpectedly.",
                    ));
                }
                std::fs::rename(&staging, output)
                    .map_err(|e| CommandError::io("Cannot publish export", e))
            });
            if result.is_err() {
                let _ = std::fs::remove_file(staging);
            }
            result
        },
    ))
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn memo_source_path(
    app: AppHandle,
    memo_id: String,
    expected_revision: u64,
) -> Result<String, CommandError> {
    let memo = memo_store::load(&app, &memo_id)?;
    memo_store::revision(&memo, expected_revision)?;
    let take = memo_store::selected(&memo)?;
    if memo.deleted || !["source", "accepted"].contains(&take.state.as_str()) {
        return Err(CommandError::new(
            "UNACCEPTED_AUDIO",
            "Choose accepted audio first.",
        ));
    }
    Ok(memo_store::selected_path(&app, &memo)?
        .to_string_lossy()
        .into_owned())
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn convert_memo_segment(
    app: AppHandle,
    state: State<'_, AppState>,
    memo_id: String,
    memo_revision: u64,
    root_path: String,
    expected_revision: u64,
    chapter_id: String,
    segment_id: String,
    range_start_ms: Option<u64>,
    range_end_ms: Option<u64>,
) -> Result<String, CommandError> {
    let path = memo_source_path(app.clone(), memo_id, memo_revision)?;
    if std::fs::metadata(&path)
        .map_err(|e| CommandError::io("Cannot read recording", e))?
        .len()
        > 20 * 1024 * 1024
    {
        return Err(CommandError::new(
            "INVALID_RECORDING",
            "Choose a recording shorter than 20 seconds.",
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| CommandError::io("Cannot read recording", e))?;
    super::production::convert_segment_recording(
        app,
        state,
        root_path,
        expected_revision,
        chapter_id,
        segment_id,
        bytes,
        range_start_ms,
        range_end_ms,
    )
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn update_memo_take(
    app: AppHandle,
    state: State<'_, AppState>,
    memo_id: String,
    expected_revision: u64,
    take_id: String,
    name: String,
    notes: String,
    favorite: bool,
    deleted: bool,
) -> Result<RecordingSession, CommandError> {
    let _lock = state
        .project_write_lock
        .lock()
        .map_err(|_| CommandError::internal("Library lock unavailable"))?;
    memo_store::update_take(
        &app,
        &memo_id,
        expected_revision,
        &take_id,
        &name,
        &notes,
        favorite,
        deleted,
    )
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn extract_memo_selection(
    app: AppHandle,
    state: State<'_, AppState>,
    memo_id: String,
    expected_revision: u64,
    start_ms: u64,
    end_ms: u64,
) -> Result<String, CommandError> {
    let memo = memo_store::load(&app, &memo_id)?;
    memo_store::revision(&memo, expected_revision)?;
    let take = memo_store::selected(&memo)?;
    if memo.deleted || !matches!(take.state.as_str(), "accepted" | "source") {
        return Err(CommandError::new(
            "UNACCEPTED_AUDIO",
            "Choose accepted audio before extracting.",
        ));
    }
    let composition = audio_edits::edit(
        take,
        &AudioEdit {
            kind: "trim".into(),
            start_ms,
            end_ms,
            value: 0.0,
        },
    )?;
    Ok(state.jobs.enqueue(
        "audio_extract",
        "Save selection as memo".into(),
        move |control, _| {
            control.boundary()?;
            let base = audio_assets::root(&app)?;
            let staging = base.join(format!(".extract-{}.wav", uuid::Uuid::new_v4()));
            let result = (|| {
                let settings = settings::load(&app)?;
                audio_edits::render(
                    &base,
                    &composition,
                    &memo.takes,
                    &staging,
                    &settings,
                    control.cancelled.clone(),
                )?;
                control.boundary()?;
                let state = app.state::<AppState>();
                let _lock = state
                    .project_write_lock
                    .lock()
                    .map_err(|_| CommandError::internal("Library lock unavailable"))?;
                memo_store::revision(&memo_store::load(&app, &memo_id)?, expected_revision)?;
                let created = memo_store::create(&app, "Extracted selection")?;
                match memo_store::import_file(
                    &app,
                    &created.id,
                    &staging,
                    "Selected passage",
                    Some(48000),
                    &settings,
                    control.cancelled.clone(),
                ) {
                    Ok(mut extracted) => {
                        extracted.context = memo.context.clone();
                        if let Some(take) = extracted.takes.first_mut() {
                            take.cues = audio_edits::remap_cues(&composition, &memo.takes);
                        }
                        memo_store::save(&app, &extracted)
                    }
                    Err(error) => {
                        let _ = std::fs::remove_file(
                            base.join("memos").join(format!("{}.json", created.id)),
                        );
                        Err(error)
                    }
                }
            })();
            let _ = std::fs::remove_file(staging);
            result
        },
    ))
}
