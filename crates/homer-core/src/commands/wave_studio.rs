#[cfg(feature = "desktop")]
use crate::runtime::Manager;
use crate::runtime::{AppHandle, State};
use crate::{
    AppState,
    services::{
        memo_store,
        project_store::CommandError,
        settings, sfx_store, sound_store, wave_render, wave_store,
        wave_studio::{Project, Source},
        waveform::{self, PeakWindow},
    },
};
use std::sync::{
    Arc, LazyLock, Mutex,
    atomic::{AtomicBool, Ordering},
};
#[derive(Clone, serde::Serialize)]
pub struct OperationStatus {
    id: String,
    label: String,
    progress: u8,
}
static OPERATIONS: LazyLock<
    Mutex<std::collections::HashMap<String, (Arc<AtomicBool>, OperationStatus)>>,
> = LazyLock::new(|| Mutex::new(Default::default()));
struct Operation {
    id: String,
    cancel: Arc<AtomicBool>,
}
impl Operation {
    fn new(id: Option<String>, label: &str) -> Result<Self, CommandError> {
        let id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        crate::services::audio_assets::id(&id)?;
        let mut ops = OPERATIONS
            .lock()
            .map_err(|_| CommandError::internal("Operation registry unavailable"))?;
        if ops.len() >= 4 || ops.contains_key(&id) {
            return Err(CommandError::new(
                "IMPORT_BUSY",
                "Wait for the current imports to finish.",
            ));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        ops.insert(
            id.clone(),
            (
                cancel.clone(),
                OperationStatus {
                    id: id.clone(),
                    label: label.into(),
                    progress: 0,
                },
            ),
        );
        Ok(Self { id, cancel })
    }
    fn reporter(&self) -> Arc<dyn Fn(u8) + Send + Sync> {
        let id = self.id.clone();
        Arc::new(move |progress| {
            if let Ok(mut ops) = OPERATIONS.lock() {
                if let Some((_, s)) = ops.get_mut(&id) {
                    s.progress = progress;
                }
            }
        })
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        if let Ok(mut ops) = OPERATIONS.lock() {
            ops.remove(&self.id);
        }
    }
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_operation_status(request_id: String) -> Option<OperationStatus> {
    OPERATIONS
        .lock()
        .ok()
        .and_then(|ops| ops.get(&request_id).map(|(_, s)| s.clone()))
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_cancel_operation(request_id: String) {
    if let Ok(ops) = OPERATIONS.lock() {
        if let Some((flag, _)) = ops.get(&request_id) {
            flag.store(true, Ordering::SeqCst);
        }
    }
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_list(app: AppHandle) -> Result<Vec<Project>, CommandError> {
    wave_store::list(&app)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_get(app: AppHandle, id: String) -> Result<Project, CommandError> {
    wave_store::load(&app, &id)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_create(app: AppHandle, name: String) -> Result<Project, CommandError> {
    wave_store::create(&app, &name)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_save(
    app: AppHandle,
    state: State<'_, AppState>,
    project: Project,
    expected_revision: u64,
) -> Result<Project, CommandError> {
    let _guard = state
        .project_write_lock
        .lock()
        .map_err(|_| CommandError::internal("Project lock unavailable"))?;
    wave_store::save(&app, project, expected_revision)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub async fn wave_import(
    app: AppHandle,
    kind: String,
    path: Option<String>,
    id: Option<String>,
    request_id: Option<String>,
) -> Result<Source, CommandError> {
    let operation = Operation::new(request_id, "Preparing audio")?;
    crate::runtime::async_runtime::spawn_blocking(move || {
        let (path, name) = match kind.as_str() {
            "file" => {
                let p = std::path::PathBuf::from(
                    path.ok_or_else(|| CommandError::new("INVALID_IMPORT", "Choose audio."))?,
                );
                let n = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Imported audio")
                    .to_string();
                (p, n)
            }
            "memo" => {
                let m = memo_store::load(&app, &id.unwrap_or_default())?;
                let take = memo_store::selected(&m)?;
                if m.deleted || !matches!(take.state.as_str(), "source" | "accepted") {
                    return Err(CommandError::new(
                        "UNACCEPTED_AUDIO",
                        "Choose accepted audio first.",
                    ));
                }
                (memo_store::selected_path(&app, &m)?, m.name)
            }
            "sound" => {
                let wanted = id.unwrap_or_default();
                let asset = sound_store::list(&app)?
                    .into_iter()
                    .find(|a| a.id == wanted)
                    .ok_or_else(|| {
                        CommandError::new("SOUND_NOT_FOUND", "Choose an existing sound.")
                    })?;
                (
                    sound_store::asset_path(&app, &asset, true)?,
                    asset.prompt.chars().take(160).collect(),
                )
            }
            _ => {
                return Err(CommandError::new(
                    "INVALID_IMPORT",
                    "Unsupported audio source.",
                ));
            }
        };
        wave_store::import_controlled(
            &app,
            &path,
            &name,
            &settings::load(&app)?,
            operation.cancel.clone(),
            operation.reporter(),
        )
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
static PEAK_REQUESTS: std::sync::LazyLock<
    Mutex<std::collections::HashMap<String, Arc<AtomicBool>>>,
> = std::sync::LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
struct PeakRequest(String);
impl Drop for PeakRequest {
    fn drop(&mut self) {
        if let Ok(mut requests) = PEAK_REQUESTS.lock() {
            requests.remove(&self.0);
        }
    }
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_cancel_peaks(request_id: String) {
    if let Ok(requests) = PEAK_REQUESTS.lock() {
        if let Some(flag) = requests.get(&request_id) {
            flag.store(true, Ordering::SeqCst)
        }
    }
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub async fn wave_peaks(
    app: AppHandle,
    source_id: String,
    start_ms: u64,
    end_ms: u64,
    max_peaks: usize,
    request_id: Option<String>,
) -> Result<PeakWindow, CommandError> {
    let id = request_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut requests = PEAK_REQUESTS
            .lock()
            .map_err(|_| CommandError::internal("Waveform requests unavailable"))?;
        if requests.len() >= 16 || requests.contains_key(&id) {
            return Err(CommandError::new("BUSY", "Too many waveform requests."));
        }
        requests.insert(id.clone(), cancel.clone());
    }
    let request = PeakRequest(id);
    crate::runtime::async_runtime::spawn_blocking(move || {
        let _request = request;
        waveform::peaks(
            &wave_store::source_path(&app, &source_id)?,
            &wave_store::root(&app)?.join("peaks"),
            &settings::load(&app)?,
            start_ms,
            end_ms,
            max_peaks,
            cancel,
        )
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
static PREVIEW_CANCEL: std::sync::Mutex<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>> =
    std::sync::Mutex::new(None);
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_cancel_preview() {
    if let Ok(mut current) = PREVIEW_CANCEL.lock() {
        if let Some(flag) = current.take() {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub async fn wave_preview(
    app: AppHandle,
    project: Project,
    start_ms: f64,
    end_ms: f64,
) -> Result<crate::runtime::ipc::Response, CommandError> {
    if end_ms - start_ms > 10_001. {
        return Err(CommandError::new(
            "INVALID_PREVIEW_RANGE",
            "Preview chunks must be at most ten seconds.",
        ));
    }
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    *PREVIEW_CANCEL
        .lock()
        .map_err(|_| CommandError::internal("Preview unavailable"))? = Some(cancel.clone());
    crate::runtime::async_runtime::spawn_blocking(move || {
        wave_store::validate_sources(&app, &project)?;
        // Serialize preview cache publication only; project saves remain responsive.
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = LOCK
            .lock()
            .map_err(|_| CommandError::internal("Preview lock unavailable"))?;
        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(CommandError::new("CANCELLED", "Preview cancelled."));
        }
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_string(&project)
            .map_err(|e| CommandError::internal(e.to_string()))?
            .hash(&mut hash);
        start_ms.to_bits().hash(&mut hash);
        end_ms.to_bits().hash(&mut hash);
        let cache = wave_store::root(&app)?.join("preview");
        std::fs::create_dir_all(&cache)
            .map_err(|e| CommandError::io("Cannot create preview cache", e))?;
        let out = cache.join(format!("{:x}.wav", hash.finish()));
        if !out.exists() {
            let temp = cache.join(format!("{}.wav", uuid::Uuid::new_v4()));
            let result = wave_render::render(
                &project,
                &wave_store::root(&app)?,
                start_ms,
                end_ms,
                &temp,
                &settings::load(&app)?,
                cancel,
            );
            if let Err(error) = result {
                let _ = std::fs::remove_file(&temp);
                return Err(error);
            }
            std::fs::rename(&temp, &out)
                .map_err(|e| CommandError::io("Cannot publish preview", e))?;
        }
        let bytes = std::fs::read(&out).map_err(|e| CommandError::io("Cannot read preview", e))?;
        if bytes.len() > 4 * 1024 * 1024 {
            return Err(CommandError::new(
                "INVALID_PREVIEW",
                "Preview exceeded its memory limit.",
            ));
        }
        let mut files = std::fs::read_dir(&cache)
            .map_err(|e| CommandError::io("Cannot inspect preview cache", e))?
            .flatten()
            .collect::<Vec<_>>();
        files.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
        let remove = files.len().saturating_sub(24);
        for file in files.into_iter().take(remove) {
            let _ = std::fs::remove_file(file.path());
        }
        Ok(crate::runtime::ipc::Response::new(bytes))
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_export(
    app: AppHandle,
    state: State<'_, AppState>,
    project: Project,
    output_path: String,
) -> Result<String, CommandError> {
    wave_store::validate_sources(&app, &project)?;
    if project.timeline.duration() <= 0. {
        return Err(CommandError::new(
            "EMPTY_WAVE_PROJECT",
            "Import some audio first.",
        ));
    }
    Ok(state.jobs.enqueue(
        "wave_export",
        "Export Wave Studio mix".into(),
        move |control, progress| {
            control.boundary()?;
            progress(10);
            let output = std::path::PathBuf::from(output_path);
            let ext = output.extension().and_then(|x| x.to_str()).unwrap_or("");
            let temp =
                output.with_file_name(format!(".homer-wave-{}.{}", uuid::Uuid::new_v4(), ext));
            let result = (|| {
                wave_render::render(
                    &project,
                    &wave_store::root(&app)?,
                    0.,
                    project.timeline.duration(),
                    &temp,
                    &settings::load(&app)?,
                    control.cancelled.clone(),
                )?;
                control.boundary()?;
                let duration =
                    crate::services::speech::probe_duration(&temp, &settings::load(&app)?)?;
                if (duration as f64 - project.timeline.duration()).abs() > 150. {
                    return Err(CommandError::new(
                        "INVALID_EXPORT_DURATION",
                        "The exported duration did not match the timeline.",
                    ));
                }
                std::fs::rename(&temp, &output)
                    .map_err(|e| CommandError::io("Cannot publish mix", e))?;
                Ok(())
            })();
            if result.is_err() {
                let _ = std::fs::remove_file(temp);
            }
            result
        },
    ))
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub async fn wave_sfx_list(app: AppHandle) -> Result<Vec<sfx_store::Asset>, CommandError> {
    crate::runtime::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _guard = state
            .project_write_lock
            .lock()
            .map_err(|_| CommandError::internal("Library lock unavailable"))?;
        sfx_store::list(&app)
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub async fn wave_sfx_add(
    app: AppHandle,
    source: Source,
    category: String,
) -> Result<Vec<sfx_store::Asset>, CommandError> {
    crate::runtime::async_runtime::spawn_blocking(move || {
        let owned = wave_store::load_source(&app, &source.id)?;
        let state = app.state::<AppState>();
        let _guard = state
            .project_write_lock
            .lock()
            .map_err(|_| CommandError::internal("Library lock unavailable"))?;
        sfx_store::add(&app, owned, &category)
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_sfx_update(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    name: String,
    category: String,
    delete: bool,
) -> Result<Vec<sfx_store::Asset>, CommandError> {
    let _guard = state
        .project_write_lock
        .lock()
        .map_err(|_| CommandError::internal("Library lock unavailable"))?;
    sfx_store::update(&app, &id, &name, &category, delete)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_source_url(
    app: AppHandle,
    state: State<'_, AppState>,
    source_id: String,
) -> Result<String, CommandError> {
    let path = wave_store::source_path(&app, &source_id)?;
    if !path.is_file() {
        return Err(CommandError::new(
            "SOURCE_NOT_FOUND",
            "Audio source no longer exists.",
        ));
    }
    let id = format!("wave-{source_id}");
    state
        .audio_assets
        .write()
        .map_err(|_| CommandError::internal("Audio registry unavailable"))?
        .insert(id.clone(), path);
    Ok(format!("audio://localhost/{id}"))
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub async fn wave_normalize(
    app: AppHandle,
    project: Project,
    clip_id: String,
) -> Result<f64, CommandError> {
    crate::runtime::async_runtime::spawn_blocking(move || {
        crate::services::wave_processing::normalize(
            &app,
            &project,
            &clip_id,
            &settings::load(&app)?,
        )
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub async fn wave_join(
    app: AppHandle,
    project: Project,
    clip_ids: Vec<String>,
) -> Result<Source, CommandError> {
    crate::runtime::async_runtime::spawn_blocking(move || {
        crate::services::wave_processing::join(&app, &project, &clip_ids, &settings::load(&app)?)
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
fn enqueue_processing<F>(app: AppHandle, state: State<'_, AppState>, label: &str, task: F) -> String
where
    F: FnOnce(
            &AppHandle,
            &crate::services::jobs::JobControl,
            &dyn Fn(u8),
        ) -> Result<crate::services::wave_processing::ProcessingResult, CommandError>
        + Send
        + 'static,
{
    let (send, receive) = std::sync::mpsc::channel::<String>();
    let id = state
        .jobs
        .enqueue("wave_processing", label.into(), move |control, progress| {
            let id = receive
                .recv()
                .map_err(|e| CommandError::internal(e.to_string()))?;
            control.boundary()?;
            let result = task(&app, &control, &*progress)?;
            control.boundary()?;
            crate::services::wave_processing::write(&app, &id, &result)
        });
    let _ = send.send(id.clone());
    id
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_generate_speech(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
    voice_id: String,
    project_id: String,
    revision: u64,
) -> Result<String, CommandError> {
    if text.trim().is_empty() || text.len() > 100_000 {
        return Err(CommandError::new(
            "INVALID_TEXT",
            "Enter text shorter than 100,000 bytes.",
        ));
    }
    let settings = settings::load(&app)?;
    Ok(enqueue_processing(
        app,
        state,
        "Generate Wave speech",
        move |app, control, progress| {
            crate::services::wave_processing::generate(
                app,
                &text,
                &voice_id,
                &project_id,
                revision,
                &settings,
                control,
                progress,
            )
        },
    ))
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_convert_regions(
    app: AppHandle,
    state: State<'_, AppState>,
    mut project: Project,
    region_ids: Vec<String>,
    regenerate: bool,
) -> Result<String, CommandError> {
    wave_store::validate_sources(&app, &project)?;
    project
        .timeline
        .voices
        .retain(|r| region_ids.contains(&r.id) && (regenerate || r.production.is_none()));
    if project.timeline.voices.is_empty() {
        return Err(CommandError::new(
            "NO_PENDING_VOICES",
            "All selected passages are already generated or converted. Select Regenerate to replace one deliberately.",
        ));
    }
    let settings = settings::load(&app)?;
    Ok(enqueue_processing(
        app,
        state,
        "Apply tagged voices",
        move |app, control, progress| {
            crate::services::wave_processing::convert(app, &project, &settings, control, progress)
        },
    ))
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_processing_result(
    app: AppHandle,
    job_id: String,
) -> Result<crate::services::wave_processing::ProcessingResult, CommandError> {
    crate::services::wave_processing::read(&app, &job_id)
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub async fn wave_has_video(app: AppHandle, path: String) -> Result<bool, CommandError> {
    crate::runtime::async_runtime::spawn_blocking(move || {
        crate::services::wave_video::has_video(std::path::Path::new(&path), &settings::load(&app)?)
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub async fn wave_import_video(
    app: AppHandle,
    path: String,
    request_id: Option<String>,
) -> Result<crate::services::wave_studio::Video, CommandError> {
    let operation = Operation::new(request_id, "Preparing video")?;
    crate::runtime::async_runtime::spawn_blocking(move || {
        crate::services::wave_video::import_controlled(
            &app,
            std::path::Path::new(&path),
            &settings::load(&app)?,
            operation.cancel.clone(),
            operation.reporter(),
        )
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_video_url(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<String, CommandError> {
    let path = crate::services::wave_video::path(&app, &id)?;
    if !path.is_file() {
        return Err(CommandError::new(
            "VIDEO_NOT_FOUND",
            "Video reference is missing.",
        ));
    }
    let key = format!("wave-video-{id}");
    state
        .audio_assets
        .write()
        .map_err(|_| CommandError::internal("Media registry unavailable"))?
        .insert(key.clone(), path);
    Ok(format!("audio://localhost/{key}"))
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_deleted(app: AppHandle) -> Result<Vec<Project>, CommandError> {
    wave_store::deleted(&app)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    restore: bool,
) -> Result<(), CommandError> {
    let _guard = state
        .project_write_lock
        .lock()
        .map_err(|_| CommandError::internal("Project lock unavailable"))?;
    wave_store::trash(&app, &id, restore)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn wave_purge(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Option<String>,
) -> Result<usize, CommandError> {
    let _guard = state
        .project_write_lock
        .lock()
        .map_err(|_| CommandError::internal("Project lock unavailable"))?;
    wave_store::purge(&app, id.as_deref())
}
#[cfg(feature = "desktop")]
#[tauri::command]
pub fn wave_reveal(app: AppHandle, id: String, deleted: bool) -> Result<(), CommandError> {
    wave_store::reveal(&app, &id, deleted)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub async fn wave_save_copy(
    app: AppHandle,
    project: Project,
    path: String,
) -> Result<(), CommandError> {
    crate::runtime::async_runtime::spawn_blocking(move || {
        wave_store::save_copy(&app, &project, std::path::Path::new(&path))
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub async fn wave_open_copy(app: AppHandle, path: String) -> Result<Project, CommandError> {
    crate::runtime::async_runtime::spawn_blocking(move || {
        wave_store::open_copy(&app, std::path::Path::new(&path), &settings::load(&app)?)
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub async fn wave_export_bundle(
    app: AppHandle,
    project: Project,
    path: String,
) -> Result<(), CommandError> {
    crate::runtime::async_runtime::spawn_blocking(move || {
        crate::services::wave_bundle::export(&app, &project, std::path::Path::new(&path))
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub async fn wave_import_bundle(
    app: AppHandle,
    path: String,
    request_id: Option<String>,
) -> Result<Project, CommandError> {
    crate::runtime::async_runtime::spawn_blocking(move || {
        let operation = Operation::new(request_id, "Opening project")?;
        crate::services::wave_bundle::import(
            &app,
            std::path::Path::new(&path),
            &settings::load(&app)?,
            operation.cancel.clone(),
            operation.reporter(),
        )
    })
    .await
    .map_err(|e| CommandError::internal(e.to_string()))?
}
