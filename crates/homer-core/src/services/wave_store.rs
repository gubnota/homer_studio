use super::{
    audio_assets,
    project_store::CommandError,
    settings::Settings,
    speech,
    wave_studio::{Project, Source, Timeline},
};
use crate::runtime::AppHandle;
#[cfg(feature = "desktop")]
use crate::runtime::Manager;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
pub fn root(app: &AppHandle) -> Result<PathBuf, CommandError> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("wave-studio"))
        .map_err(|e| CommandError::internal(e.to_string()))
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn source_path(app: &AppHandle, id: &str) -> Result<PathBuf, CommandError> {
    source_path_at(&root(app)?, id)
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStatus {
    pub folder: Option<String>,
    pub migrated_bytes: u64,
    pub reclaimed_bytes: u64,
}
fn location_path(base: &Path, id: &str) -> Result<PathBuf, CommandError> {
    audio_assets::id(id)?;
    Ok(base.join("locations").join(format!("{id}.json")))
}
fn checked_media(folder: &Path, id: &str) -> Result<PathBuf, CommandError> {
    audio_assets::id(id)?;
    let owned = folder.canonicalize().map_err(|_| {
        CommandError::new(
            "PROJECT_FOLDER_MISSING",
            "Locate the .wavehs project folder before continuing.",
        )
    })?;
    let media = owned.join("media");
    if media.exists()
        && media
            .canonicalize()
            .map_err(|e| CommandError::io("Cannot resolve media", e))?
            != media
    {
        return Err(CommandError::new(
            "UNSAFE_MEDIA",
            "Project media directory cannot be a link.",
        ));
    }
    let file = media.join(format!("{id}.wav"));
    if file.exists()
        && file
            .canonicalize()
            .map_err(|e| CommandError::io("Cannot resolve audio", e))?
            != file
    {
        return Err(CommandError::new(
            "UNSAFE_MEDIA",
            "Project audio cannot be a link.",
        ));
    }
    Ok(file)
}
pub fn project_source_path_at(
    base: &Path,
    project_id: &str,
    id: &str,
) -> Result<PathBuf, CommandError> {
    audio_assets::id(project_id)?;
    let binding = base.join("folders").join(format!("{project_id}.json"));
    if binding.exists() {
        let folder: PathBuf = serde_json::from_slice(
            &fs::read(binding).map_err(|e| CommandError::io("Cannot read project location", e))?,
        )
        .map_err(|e| CommandError::internal(e.to_string()))?;
        let path = checked_media(&folder, id)?;
        if !path.is_file() {
            return Err(CommandError::new(
                "PROJECT_MEDIA_MISSING",
                "Audio is missing from this .wavehs folder.",
            ));
        }
        return Ok(path);
    }
    source_path_at(base, id)
}
pub fn source_path_at(base: &Path, id: &str) -> Result<PathBuf, CommandError> {
    let location = location_path(base, id)?;
    let library = base.join("library");
    if library.join("media").join(format!("{id}.wav")).exists() {
        return checked_media(&library, id);
    }
    if location.exists() {
        let owners: Vec<String> = serde_json::from_slice(
            &fs::read(location).map_err(|e| CommandError::io("Cannot read audio location", e))?,
        )
        .map_err(|e| CommandError::internal(e.to_string()))?;
        for owner in owners {
            audio_assets::id(&owner)?;
            let binding = base.join("folders").join(format!("{owner}.json"));
            let bytes = match fs::read(binding) {
                Ok(bytes) => bytes,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(CommandError::io("Cannot read project location", e)),
            };
            let folder: PathBuf = serde_json::from_slice(&bytes)
                .map_err(|e| CommandError::internal(e.to_string()))?;
            if folder.is_dir() {
                let path = checked_media(&folder, id)?;
                if path.is_file() {
                    return Ok(path);
                }
            }
        }
        return Err(CommandError::new(
            "PROJECT_FOLDER_MISSING",
            "Locate the .wavehs folder containing this audio.",
        ));
    }
    audio_assets::path(base, id)
}
fn register_source(app: &AppHandle, source: &Source, project_id: &str) -> Result<(), CommandError> {
    let path = location_path(&root(app)?, &source.id)?;
    let mut owners: Vec<String> = if path.exists() {
        serde_json::from_slice(
            &fs::read(&path).map_err(|e| CommandError::io("Cannot read audio owners", e))?,
        )
        .map_err(|e| CommandError::internal(e.to_string()))?
    } else {
        vec![]
    };
    if !owners.iter().any(|id| id == project_id) {
        owners.push(project_id.into());
    }
    audio_assets::atomic_json(
        &root(app)?
            .join("sources")
            .join(format!("{}.json", source.id)),
        source,
    )?;
    audio_assets::atomic_json(&path, &owners)
}
pub fn require_package(app: &AppHandle, id: &str) -> Result<PathBuf, CommandError> {
    let folder = bound_folder(app, id)?.ok_or_else(|| {
        CommandError::new(
            "PROJECT_DESTINATION_REQUIRED",
            "Save this project as a .wavehs folder before adding audio.",
        )
    })?;
    let canonical = folder.canonicalize().map_err(|_| {
        CommandError::new(
            "PROJECT_FOLDER_MISSING",
            "Locate the .wavehs project folder before continuing.",
        )
    })?;
    if canonical != folder {
        return Err(CommandError::new(
            "UNSAFE_MEDIA",
            "Project folder location changed. Locate it again.",
        ));
    }
    let p = read_project(&folder.join("project.json"))?;
    if p.id != id {
        return Err(CommandError::new(
            "INVALID_DESTINATION",
            "This folder belongs to another project.",
        ));
    }
    Ok(folder)
}
pub fn storage(app: &AppHandle, id: &str) -> Result<StorageStatus, CommandError> {
    Ok(StorageStatus {
        folder: bound_folder(app, id)?.map(|p| p.to_string_lossy().into_owned()),
        migrated_bytes: 0,
        reclaimed_bytes: 0,
    })
}
fn read_project(path: &Path) -> Result<Project, CommandError> {
    if fs::metadata(path)
        .map_err(|e| CommandError::io("Cannot open Wave project", e))?
        .len()
        > 16 * 1024 * 1024
    {
        return Err(CommandError::new(
            "INVALID_WAVE_PROJECT",
            "Project is too large.",
        ));
    }
    let p: Project = serde_json::from_slice(
        &fs::read(path).map_err(|e| CommandError::io("Cannot open Wave project", e))?,
    )
    .map_err(|e| CommandError::new("INVALID_WAVE_PROJECT", e.to_string()))?;
    super::wave_studio::validate(&p)?;
    Ok(p)
}
pub fn locate(app: &AppHandle, id: &str, folder: &Path) -> Result<Project, CommandError> {
    let p = read_project(&folder.join("project.json"))?;
    if p.id != id {
        return Err(CommandError::new(
            "INVALID_DESTINATION",
            "Choose the folder for this project.",
        ));
    }
    for source in &p.sources {
        validate_pcm(&checked_media(folder, &source.id)?, source)?;
    }
    bind_folder(app, &p, folder)?;
    for source in &p.sources {
        register_source(app, source, id)?;
    }
    audio_assets::atomic_json(&project_path(app, id)?, &p)?;
    Ok(p)
}
fn project_path(app: &AppHandle, id: &str) -> Result<PathBuf, CommandError> {
    audio_assets::id(id)?;
    Ok(root(app)?.join("projects").join(format!("{id}.json")))
}
pub fn load(app: &AppHandle, id: &str) -> Result<Project, CommandError> {
    audio_assets::id(id)?;
    let path = if bound_folder(app, id)?.is_some() {
        require_package(app, id)?.join("project.json")
    } else {
        project_path(app, id)?
    };
    let p = read_project(&path)?;
    if p.id != id {
        return Err(CommandError::new(
            "INVALID_WAVE_PROJECT",
            "Project identity changed.",
        ));
    }
    Ok(p)
}
/// Used only to display recents and recover a moved/legacy package; never for editing a missing folder.
pub fn cached(app: &AppHandle, id: &str) -> Result<Project, CommandError> {
    read_project(&project_path(app, id)?)
}
pub fn create(
    app: &AppHandle,
    name: &str,
    destination: Option<&Path>,
) -> Result<Project, CommandError> {
    let p = Project {
        schema_version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        name: name.trim().to_string(),
        revision: 0,
        updated_at_ms: now(),
        sources: vec![],
        timeline: Timeline::default(),
        view: None,
        voice_original: None,
        videos: vec![],
    };
    super::wave_studio::validate(&p)?;
    let destination = package_destination(app, destination)?;
    save_copy_options(app, &p, &destination, false)?;
    bind_folder(app, &p, &destination)?;
    audio_assets::atomic_json(&project_path(app, &p.id)?, &p)?;
    Ok(p)
}
// Isolated desktop document smoke only: the harness sets a UUID-scoped application identifier.
pub fn smoke_destination_enabled() -> bool {
    std::env::var("HOMER_SMOKE_IDENTIFIER")
        .ok()
        .and_then(|id| {
            id.strip_prefix("com.homer-studio.smoke.")
                .map(str::to_owned)
        })
        .is_some_and(|id| uuid::Uuid::parse_str(&id).is_ok())
}
pub fn package_destination(
    app: &AppHandle,
    destination: Option<&Path>,
) -> Result<PathBuf, CommandError> {
    let path = match destination {
        Some(path) => path.to_path_buf(),
        #[cfg(all(feature = "server", not(feature = "desktop")))]
        None => root(app)?
            .join("packages")
            .join(format!("{}.wavehs", uuid::Uuid::new_v4())),
        #[cfg(any(feature = "desktop", not(feature = "server")))]
        None => {
            if smoke_destination_enabled() {
                root(app)?
                    .join("packages")
                    .join(format!("{}.wavehs", uuid::Uuid::new_v4()))
            } else {
                return Err(CommandError::new(
                    "PROJECT_DESTINATION_REQUIRED",
                    "Choose a .wavehs project folder.",
                ));
            }
        }
    };
    if !path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("wavehs"))
    {
        return Err(CommandError::new(
            "INVALID_DESTINATION",
            "Choose a folder ending in .wavehs.",
        ));
    }
    if path.exists() {
        return Err(CommandError::new(
            "DESTINATION_EXISTS",
            "Choose a new .wavehs project folder.",
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| CommandError::new("INVALID_DESTINATION", "Choose a project folder."))?;
    fs::create_dir_all(parent)
        .map_err(|e| CommandError::io("Cannot create project destination", e))?;
    Ok(path)
}
pub fn save(app: &AppHandle, mut p: Project, expected: u64) -> Result<Project, CommandError> {
    super::wave_studio::validate_video_placements(&p.videos)?;
    super::wave_studio::validate(&p)?;
    let old = load(app, &p.id)?;
    if old.revision != expected {
        return Err(CommandError::new(
            "REVISION_CONFLICT",
            "This Wave project was changed elsewhere. Reopen it before saving.",
        ));
    }
    let folder = require_package(app, &p.id)?;
    validate_sources(app, &p)?;
    p.revision = expected + 1;
    p.updated_at_ms = now();
    let include = fs::read(
        root(app)?
            .join("folders")
            .join(format!("{}.options.json", p.id)),
    )
    .ok()
    .and_then(|b| serde_json::from_slice::<bool>(&b).ok())
    .unwrap_or(false);
    save_copy_options(app, &p, &folder, include)?;
    audio_assets::atomic_json(&project_path(app, &p.id)?, &p)?;
    Ok(p)
}
pub fn list(app: &AppHandle) -> Result<Vec<Project>, CommandError> {
    let dir = root(app)?.join("projects");
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut result = vec![];
    for entry in fs::read_dir(dir)
        .map_err(|e| CommandError::io("Cannot list projects", e))?
        .take(500)
    {
        let entry = entry.map_err(|e| CommandError::io("Cannot inspect project", e))?;
        if let Some(id) = entry.path().file_stem().and_then(|s| s.to_str()) {
            if let Ok(p) = load(app, id).or_else(|_| cached(app, id)) {
                result.push(p);
            }
        }
    }
    result.sort_by_key(|p| std::cmp::Reverse(p.updated_at_ms));
    Ok(result)
}
pub fn load_source(app: &AppHandle, id: &str) -> Result<Source, CommandError> {
    audio_assets::id(id)?;
    let b = fs::read(root(app)?.join("sources").join(format!("{id}.json")))
        .map_err(|e| CommandError::io("Cannot read audio source", e))?;
    serde_json::from_slice(&b).map_err(|e| CommandError::internal(e.to_string()))
}
pub fn import(
    app: &AppHandle,
    path: &Path,
    name: &str,
    settings: &Settings,
    project_id: &str,
) -> Result<Source, CommandError> {
    import_controlled(
        app,
        path,
        name,
        settings,
        Default::default(),
        std::sync::Arc::new(|_| {}),
        project_id,
    )
}
pub fn import_controlled(
    app: &AppHandle,
    path: &Path,
    name: &str,
    settings: &Settings,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    report: std::sync::Arc<dyn Fn(u8) + Send + Sync>,
    project_id: &str,
) -> Result<Source, CommandError> {
    let folder = require_package(app, project_id)?;
    import_to(
        app,
        &folder,
        path,
        name,
        settings,
        cancel,
        report,
        Some(project_id),
    )
}
pub fn import_library(
    app: &AppHandle,
    path: &Path,
    name: &str,
    settings: &Settings,
) -> Result<Source, CommandError> {
    let folder = root(app)?.join("library");
    fs::create_dir_all(folder.join("media"))
        .map_err(|e| CommandError::io("Cannot create effects library", e))?;
    import_to(
        app,
        &folder,
        path,
        name,
        settings,
        Default::default(),
        std::sync::Arc::new(|_| {}),
        None,
    )
}
fn import_to(
    app: &AppHandle,
    folder: &Path,
    path: &Path,
    name: &str,
    settings: &Settings,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    report: std::sync::Arc<dyn Fn(u8) + Send + Sync>,
    project_id: Option<&str>,
) -> Result<Source, CommandError> {
    if !path.is_file()
        || !matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str(),
            "wav" | "mp3" | "m4a" | "aac" | "flac" | "aif" | "aiff" | "ogg" | "webm"
        )
    {
        return Err(CommandError::new(
            "INVALID_AUDIO_FILE",
            "Choose WAV, MP3, M4A, AAC or FLAC audio.",
        ));
    }
    let duration = speech::probe_duration(path, settings)?;
    if duration == 0 || duration > 86_400_000 {
        return Err(CommandError::new(
            "INVALID_AUDIO_DURATION",
            "Choose audio shorter than 24 hours.",
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let published = checked_media(&folder, &id)?;
    let output = published.with_file_name(format!(".{id}.part.wav"));
    fs::create_dir_all(output.parent().unwrap())
        .map_err(|e| CommandError::io("Cannot create audio library", e))?;
    let result = (|| {
        let executable =
            super::process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
                .ok_or_else(|| CommandError::new("FFMPEG_NOT_FOUND", "Set FFmpeg in Settings."))?;
        let encoded = super::process_runner::run_observed(
            &executable,
            &vec![
                "-progress".into(),
                "pipe:1".into(),
                "-v".into(),
                "error".into(),
                "-nostdin".into(),
                "-y".into(),
                "-i".into(),
                path.to_string_lossy().into(),
                "-map".into(),
                "0:a:0".into(),
                "-ar".into(),
                "48000".into(),
                "-ac".into(),
                "2".into(),
                "-c:a".into(),
                "pcm_f32le".into(),
                "-rf64".into(),
                "auto".into(),
                output.to_string_lossy().into(),
            ],
            std::time::Duration::from_secs(3600),
            cancel.clone(),
            Some((duration, report)),
        )?;
        if !encoded.success {
            return Err(CommandError::new("AUDIO_IMPORT_FAILED", encoded.stderr));
        }
        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(CommandError::new("JOB_CANCELLED", "Import cancelled."));
        }
        let s = Source {
            id: id.clone(),
            name: name.chars().take(160).collect(),
            duration_ms: speech::probe_duration(&output, settings)? as f64,
            channels: 2,
        };
        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(CommandError::new("JOB_CANCELLED", "Import cancelled."));
        }
        fs::rename(&output, &published)
            .map_err(|e| CommandError::io("Cannot publish project audio", e))?;
        if let Some(id) = project_id {
            register_source(app, &s, id)?;
        } else {
            audio_assets::atomic_json(
                &root(app)?.join("sources").join(format!("{}.json", s.id)),
                &s,
            )?;
        }
        Ok(s)
    })();
    if result.is_err() {
        let _ = fs::remove_file(output);
        let _ = fs::remove_file(published);
    }
    result
}

pub fn validate_sources(app: &AppHandle, p: &Project) -> Result<(), CommandError> {
    super::wave_studio::validate(p)?;
    for video in &p.videos {
        super::wave_video::validate(app, video)?;
    }
    for source in &p.sources {
        let owned = load_source(app, &source.id)?;
        if source.duration_ms != owned.duration_ms
            || source.channels != owned.channels
            || !source_path(app, &source.id)?.is_file()
        {
            return Err(CommandError::new(
                "INVALID_WAVE_SOURCE",
                "Source metadata does not match imported audio.",
            ));
        }
    }
    if bound_folder(app, &p.id)?.is_some() {
        ensure_project_media(app, p, &require_package(app, &p.id)?)?;
    }
    Ok(())
}

pub fn deleted(app: &AppHandle) -> Result<Vec<Project>, CommandError> {
    let dir = root(app)?.join("trash");
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut out = vec![];
    for e in fs::read_dir(dir)
        .map_err(|e| CommandError::io("Cannot read deleted projects", e))?
        .flatten()
        .take(500)
    {
        if let Ok(bytes) = fs::read(e.path()) {
            if bytes.len() <= 16 * 1024 * 1024 {
                if let Ok(p) = serde_json::from_slice::<Project>(&bytes) {
                    if super::wave_studio::validate(&p).is_ok() {
                        out.push(p)
                    }
                }
            }
        }
    }
    out.sort_by_key(|p| std::cmp::Reverse(p.updated_at_ms));
    Ok(out)
}
pub fn trash(app: &AppHandle, id: &str, restore: bool) -> Result<(), CommandError> {
    audio_assets::id(id)?;
    let active = project_path(app, id)?;
    let trash = root(app)?.join("trash").join(format!("{id}.json"));
    let (from, to) = if restore {
        (trash, active)
    } else {
        (active, trash)
    };
    if to.exists() {
        return Err(CommandError::new(
            "PROJECT_EXISTS",
            "This project already exists.",
        ));
    }
    fs::create_dir_all(to.parent().unwrap())
        .map_err(|e| CommandError::io("Cannot prepare projects", e))?;
    fs::rename(from, to).map_err(|e| CommandError::io("Cannot move project", e))
}
/// Permanently remove only trash manifests. Shared immutable media stays available
/// to active projects and memos that reference the same source.
pub fn purge(app: &AppHandle, id: Option<&str>) -> Result<usize, CommandError> {
    let dir = root(app)?.join("trash");
    if let Some(id) = id {
        audio_assets::id(id)?;
        fs::remove_file(dir.join(format!("{id}.json")))
            .map_err(|e| CommandError::io("Cannot delete project permanently", e))?;
        return Ok(1);
    }
    if !dir.exists() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in
        fs::read_dir(&dir).map_err(|e| CommandError::io("Cannot read deleted projects", e))?
    {
        let entry = entry.map_err(|e| CommandError::io("Cannot read deleted project", e))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if audio_assets::id(id).is_err() {
            continue;
        }
        if !entry
            .file_type()
            .map_err(|e| CommandError::io("Cannot inspect deleted project", e))?
            .is_file()
        {
            continue;
        }
        fs::remove_file(path)
            .map_err(|e| CommandError::io("Cannot delete project permanently", e))?;
        count += 1;
    }
    Ok(count)
}
#[cfg(feature = "desktop")]
pub fn reveal(app: &AppHandle, id: &str, deleted: bool) -> Result<(), CommandError> {
    audio_assets::id(id)?;
    let path = if deleted {
        root(app)?.join("trash").join(format!("{id}.json"))
    } else {
        bound_folder(app, id)?.unwrap_or(project_path(app, id)?)
    };
    if !path.exists() {
        return Err(CommandError::new(
            "PROJECT_NOT_FOUND",
            "Project is no longer available.",
        ));
    }
    let status = std::process::Command::new("/usr/bin/open")
        .arg("-R")
        .arg(path)
        .status()
        .map_err(|e| CommandError::io("Cannot reveal project in Finder", e))?;
    if !status.success() {
        return Err(CommandError::new(
            "REVEAL_FAILED",
            "Finder could not reveal this project.",
        ));
    }
    Ok(())
}

// Only app-owned bindings authorize automatic writes to a user-selected folder.
fn binding_path(app: &AppHandle, id: &str) -> Result<PathBuf, CommandError> {
    audio_assets::id(id)?;
    Ok(root(app)?.join("folders").join(format!("{id}.json")))
}
pub fn bind_folder(app: &AppHandle, p: &Project, folder: &Path) -> Result<(), CommandError> {
    let folder = folder
        .canonicalize()
        .map_err(|e| CommandError::io("Cannot resolve project folder", e))?;
    audio_assets::atomic_json(&binding_path(app, &p.id)?, &folder)
}
pub fn bound_folder(app: &AppHandle, id: &str) -> Result<Option<PathBuf>, CommandError> {
    let path = binding_path(app, id)?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(|e| CommandError::io("Cannot read project location", e))?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| CommandError::internal(e.to_string()))
}
pub fn save_folder(
    app: &AppHandle,
    p: &Project,
    destination: &Path,
    include_video: bool,
) -> Result<StorageStatus, CommandError> {
    if !destination
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("wavehs"))
    {
        return Err(CommandError::new(
            "INVALID_DESTINATION",
            "Choose a folder ending in .wavehs.",
        ));
    }
    let mut owned = p.clone();
    let results = root(app)?.join("results");
    if results.is_dir() {
        for entry in fs::read_dir(results)
            .map_err(|e| CommandError::io("Cannot inspect processing results", e))?
        {
            let path = entry
                .map_err(|e| CommandError::io("Cannot inspect processing result", e))?
                .path();
            if path.extension().and_then(|v| v.to_str()) != Some("json") {
                continue;
            }
            let value: serde_json::Value = serde_json::from_slice(
                &fs::read(path)
                    .map_err(|e| CommandError::io("Cannot read processing result", e))?,
            )
            .map_err(|e| CommandError::internal(e.to_string()))?;
            if value.get("projectId").and_then(|v| v.as_str()) == Some(p.id.as_str()) {
                if let Some(items) = value.get("sources").and_then(|v| v.as_array()) {
                    for item in items {
                        let source: Source = serde_json::from_value(item.clone())
                            .map_err(|e| CommandError::internal(e.to_string()))?;
                        if !owned.sources.iter().any(|s| s.id == source.id) {
                            owned.sources.push(source);
                        }
                    }
                }
            }
        }
    }
    let p = &owned;
    let mut migrated_bytes = 0;
    for source in &p.sources {
        let input = source_path(app, &source.id)?;
        let target = destination.join("media").join(format!("{}.wav", source.id));
        if !target.exists() {
            migrated_bytes += fs::metadata(&input)
                .map_err(|e| CommandError::io("Cannot inspect source", e))?
                .len();
        }
    }
    save_copy_options(app, p, destination, include_video)?;
    for source in &p.sources {
        verify_copy(
            &source_path(app, &source.id)?,
            &checked_media(destination, &source.id)?,
        )?;
    }
    audio_assets::atomic_json(
        &root(app)?
            .join("folders")
            .join(format!("{}.options.json", p.id)),
        &include_video,
    )?;
    bind_folder(app, p, destination)?;
    for source in &p.sources {
        register_source(app, source, &p.id)?;
    }
    audio_assets::atomic_json(&project_path(app, &p.id)?, p)?;
    let reclaimed_bytes = reclaim_legacy(app, &p.sources).unwrap_or(0);
    Ok(StorageStatus {
        folder: Some(
            destination
                .canonicalize()
                .map_err(|e| CommandError::io("Cannot resolve project", e))?
                .to_string_lossy()
                .into(),
        ),
        migrated_bytes,
        reclaimed_bytes,
    })
}
fn verify_copy(a: &Path, b: &Path) -> Result<(), CommandError> {
    if a == b {
        return Ok(());
    }
    use sha2::{Digest, Sha256};
    use std::io::Read;
    fn digest(path: &Path) -> Result<Vec<u8>, CommandError> {
        let mut f =
            fs::File::open(path).map_err(|e| CommandError::io("Cannot verify project media", e))?;
        let mut hash = Sha256::new();
        let mut buf = [0u8; 65536];
        loop {
            let n = f
                .read(&mut buf)
                .map_err(|e| CommandError::io("Cannot verify project media", e))?;
            if n == 0 {
                break;
            }
            hash.update(&buf[..n]);
        }
        Ok(hash.finalize().to_vec())
    }
    if digest(a)? != digest(b)? {
        return Err(CommandError::new(
            "INVALID_MEDIA",
            "Saved audio differs from its source. Original audio was retained.",
        ));
    }
    Ok(())
}
fn ensure_project_media(app: &AppHandle, p: &Project, folder: &Path) -> Result<(), CommandError> {
    for source in &p.sources {
        let target = checked_media(folder, &source.id)?;
        let input = source_path(app, &source.id)?;
        copy_media(
            folder,
            &input,
            &PathBuf::from(format!("media/{}.wav", source.id)),
        )?;
        verify_copy(&input, &target)?;
        register_source(app, source, &p.id)?;
    }
    Ok(())
}
/// Conservative collection: unreadable records block cleanup; only verified migrated candidates are removed.
fn reclaim_legacy(app: &AppHandle, sources: &[Source]) -> Result<u64, CommandError> {
    let base = root(app)?;
    let mut records = Vec::new();
    for name in ["projects", "trash", "results"] {
        let dir = base.join(name);
        if !dir.exists() {
            continue;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return Ok(0);
        };
        for entry in entries {
            let Ok(entry) = entry else { return Ok(0) };
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let Ok(bytes) = fs::read(path) else {
                return Ok(0);
            };
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
                return Ok(0);
            };
            records.push((name, value));
        }
    }
    let library = base.join("sfx.json");
    let sfx = if library.exists() {
        match fs::read(library) {
            Ok(b) => match serde_json::from_slice::<serde_json::Value>(&b) {
                Ok(v) => v.to_string(),
                Err(_) => return Ok(0),
            },
            Err(_) => return Ok(0),
        }
    } else {
        String::new()
    };
    let mut bytes = 0;
    for source in sources {
        let legacy = audio_assets::path(&base, &source.id)?;
        if !legacy.is_file() {
            continue;
        }
        if sfx.contains(&source.id) {
            continue;
        }
        let mut referenced = false;
        for (kind, value) in &records {
            if !value.to_string().contains(&source.id) {
                continue;
            }
            if *kind == "results" {
                referenced = true;
                break;
            }
            let Some(id) = value.get("id").and_then(|v| v.as_str()) else {
                return Ok(0);
            };
            let Some(folder) = bound_folder(app, id)? else {
                referenced = true;
                break;
            };
            match checked_media(&folder, &source.id).and_then(|path| verify_copy(&legacy, &path)) {
                Ok(()) => (),
                Err(_) => {
                    referenced = true;
                    break;
                }
            }
        }
        if !referenced {
            let size = fs::metadata(&legacy)
                .map_err(|e| CommandError::io("Cannot inspect legacy audio", e))?
                .len();
            if fs::remove_file(&legacy).is_ok() {
                bytes += size;
            }
        }
    }
    Ok(bytes)
}
pub fn save_copy(app: &AppHandle, p: &Project, destination: &Path) -> Result<(), CommandError> {
    save_copy_options(app, p, destination, false)
}
/// Publish immutable media atomically and reject redirected package paths.
pub fn copy_media(folder: &Path, source: &Path, relative: &Path) -> Result<(), CommandError> {
    let mut target = folder.to_path_buf();
    for component in relative.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(CommandError::new(
                "UNSAFE_PATH",
                "Invalid project media path.",
            ));
        };
        target.push(component);
        if fs::symlink_metadata(&target).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(CommandError::new(
                "UNSAFE_PATH",
                "Project media cannot use symbolic links.",
            ));
        }
    }
    if target.is_file() {
        return Ok(());
    }
    fs::create_dir_all(target.parent().unwrap())
        .map_err(|e| CommandError::io("Cannot create media folder", e))?;
    let temp = target.with_extension(format!("{}.part", uuid::Uuid::new_v4()));
    let result = (|| {
        fs::copy(source, &temp).map_err(|e| CommandError::io("Cannot copy project media", e))?;
        verify_copy(source, &temp)?;
        fs::rename(&temp, &target).map_err(|e| CommandError::io("Cannot publish project media", e))
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
pub fn save_copy_options(
    app: &AppHandle,
    p: &Project,
    destination: &Path,
    include_video: bool,
) -> Result<(), CommandError> {
    super::wave_studio::validate_video_placements(&p.videos)?;
    validate_sources(app, p)?;
    let existed = destination.exists();
    if existed {
        let bytes = fs::read(destination.join("project.json"))
            .map_err(|_| CommandError::new("DESTINATION_EXISTS", "Choose a new project folder."))?;
        let existing: Project = serde_json::from_slice(&bytes)
            .map_err(|_| CommandError::new("DESTINATION_EXISTS", "Choose a new project folder."))?;
        if existing.id != p.id {
            return Err(CommandError::new(
                "DESTINATION_EXISTS",
                "This folder belongs to another project.",
            ));
        }
    }
    let parent = destination
        .parent()
        .ok_or_else(|| CommandError::new("INVALID_DESTINATION", "Choose a project folder."))?;
    let work = if existed {
        destination.to_path_buf()
    } else {
        parent.join(format!(".homer-copy-{}", uuid::Uuid::new_v4()))
    };
    fs::create_dir_all(work.join("media"))
        .map_err(|e| CommandError::io("Cannot create media folder", e))?;
    let result = (|| {
        for source in &p.sources {
            copy_media(
                &work,
                &source_path(app, &source.id)?,
                &PathBuf::from(format!("media/{}.wav", source.id)),
            )?;
            verify_copy(
                &source_path(app, &source.id)?,
                &checked_media(&work, &source.id)?,
            )?;
        }
        for video in p.videos.iter().filter(|_| include_video) {
            copy_media(
                &work,
                &super::wave_video::original_path(app, video.asset_id())?,
                &PathBuf::from(format!("media/{}.mp4", video.id)),
            )?;
        }
        audio_assets::atomic_json(&work.join("video-links.json"), &p.videos)?;
        super::voice_store::export_portable(app, p, &work)?;
        audio_assets::atomic_json(&work.join("project.json"), p)?;
        if !existed {
            fs::rename(&work, destination)
                .map_err(|e| CommandError::io("Cannot publish project copy", e))?;
        }
        Ok(())
    })();
    if result.is_err() && !existed {
        let _ = fs::remove_dir_all(work);
    }
    result
}
pub fn open_copy(
    app: &AppHandle,
    folder: &Path,
    settings: &Settings,
) -> Result<Project, CommandError> {
    open_copy_controlled(
        app,
        folder,
        settings,
        false,
        Default::default(),
        std::sync::Arc::new(|_| {}),
    )
}
pub fn open_bundle_copy(
    app: &AppHandle,
    folder: &Path,
    settings: &Settings,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    report: std::sync::Arc<dyn Fn(u8) + Send + Sync>,
) -> Result<Project, CommandError> {
    open_copy_controlled(app, folder, settings, true, cancel, report)
}
fn open_copy_controlled(
    app: &AppHandle,
    folder: &Path,
    settings: &Settings,
    defer_registration: bool,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    report: std::sync::Arc<dyn Fn(u8) + Send + Sync>,
) -> Result<Project, CommandError> {
    let folder = folder
        .canonicalize()
        .map_err(|e| CommandError::io("Cannot resolve project folder", e))?;
    let file = folder.join("project.json");
    if fs::metadata(&file)
        .map_err(|e| CommandError::io("Cannot read project", e))?
        .len()
        > 16 * 1024 * 1024
    {
        return Err(CommandError::new(
            "INVALID_WAVE_PROJECT",
            "Project is too large.",
        ));
    }
    let mut p: Project = serde_json::from_slice(
        &fs::read(&file).map_err(|e| CommandError::io("Cannot read project", e))?,
    )
    .map_err(|e| CommandError::new("INVALID_WAVE_PROJECT", e.to_string()))?;
    super::wave_studio::validate(&p)?;
    let owned = |id: &str, ext: &str| -> Result<PathBuf, CommandError> {
        let path = folder
            .join("media")
            .join(format!("{id}.{ext}"))
            .canonicalize()
            .map_err(|e| CommandError::io("Cannot open project media", e))?;
        if !path.starts_with(&folder) {
            return Err(CommandError::new(
                "UNSAFE_MEDIA",
                "Media leaves the project folder.",
            ));
        }
        Ok(path)
    };
    if let Some(bound) = bound_folder(app, &p.id)? {
        if bound == folder {
            return locate(app, &p.id, &folder);
        }
        p.id = uuid::Uuid::new_v4().to_string();
    } else if project_path(app, &p.id)?.exists() {
        p.id = uuid::Uuid::new_v4().to_string();
    }
    struct ImportedFiles(Vec<PathBuf>);
    impl Drop for ImportedFiles {
        fn drop(&mut self) {
            for path in &self.0 {
                let _ = fs::remove_file(path);
            }
        }
    }
    let mut imported_files = ImportedFiles(Vec::new());
    let mut mapping = std::collections::HashMap::new();
    let total = p.sources.len() + p.videos.len();
    let mut completed = 0;
    for s in &mut p.sources {
        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(CommandError::new(
                "JOB_CANCELLED",
                "Project opening cancelled.",
            ));
        }
        let old = s.id.clone();
        let path = owned(&old, "wav")?;
        let imported = adopt_audio(app, &folder, &path, s)?;
        completed += 1;
        report((65 + completed * 30 / total.max(1)) as u8);
        let output = checked_media(&folder, &imported.id)?;
        if output != path {
            imported_files.0.push(output);
        }
        if (imported.duration_ms - s.duration_ms).abs() > 2. {
            return Err(CommandError::new(
                "INVALID_MEDIA",
                "Project audio duration changed.",
            ));
        }
        mapping.insert(old, imported.id.clone());
        *s = imported;
    }
    for t in std::iter::once(&mut p.timeline).chain(p.voice_original.iter_mut()) {
        for c in t.clips.iter_mut().chain(&mut t.sfx) {
            if let Some(id) = &mut c.source_id {
                if let Some(new) = mapping.get(id) {
                    *id = new.clone();
                }
            }
        }
        for r in &mut t.voices {
            if let Some(audio) = &mut r.audio {
                for clip in audio
                    .original
                    .iter_mut()
                    .chain(audio.versions.iter_mut().flat_map(|v| v.clips.iter_mut()))
                {
                    if let Some(id) = &mut clip.source_id {
                        if let Some(new) = mapping.get(id) {
                            *id = new.clone();
                        }
                    }
                }
                for (old, new) in &mapping {
                    audio.active_audio_key = audio.active_audio_key.replace(old, new);
                    for version in &mut audio.versions {
                        version.production.audio_key =
                            version.production.audio_key.replace(old, new);
                    }
                }
            }
            if let Some(production) = &mut r.production {
                for (old, new) in &mapping {
                    production.audio_key = production.audio_key.replace(old, new)
                }
            }
        }
    }
    for v in &mut p.videos {
        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(CommandError::new(
                "JOB_CANCELLED",
                "Project opening cancelled.",
            ));
        }
        let local_video = folder.join("media").join(format!("{}.mp4", v.id));
        let imported = if !local_video.exists() {
            // An archive may describe a missing link, but never authorize arbitrary external paths.
            if super::wave_video::validate(app, v).is_ok()
                && super::wave_video::path(app, v.asset_id())?.is_file()
            {
                v.clone()
            } else {
                super::wave_video::missing(app, v)?
            }
        } else {
            super::wave_video::import_controlled(
                app,
                &owned(&v.id, "mp4")?,
                settings,
                cancel.clone(),
                {
                    let report = report.clone();
                    let done = completed;
                    std::sync::Arc::new(move |value| {
                        report((65 + (done * 30 + value as usize * 30 / 100) / total.max(1)) as u8)
                    })
                },
            )?
        };
        if local_video.is_file() {
            super::wave_video::retain_original(app, &imported.id, &owned(&v.id, "mp4")?)?;
        }
        if imported.id != v.id {
            imported_files
                .0
                .push(super::wave_video::path(app, &imported.id)?);
        }
        if (imported.source_duration_ms.unwrap_or(imported.duration_ms)
            - v.source_duration_ms.unwrap_or(v.duration_ms))
        .abs()
            > 100.
        {
            return Err(CommandError::new(
                "INVALID_MEDIA",
                "Project video duration changed.",
            ));
        }
        v.asset_id = Some(imported.asset_id().to_string());
        v.source_duration_ms = Some(imported.source_duration_ms.unwrap_or(imported.duration_ms));
        completed += 1;
        report((65 + completed * 30 / total.max(1)) as u8);
    }
    if cancel.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(CommandError::new(
            "JOB_CANCELLED",
            "Project opening cancelled.",
        ));
    }
    p.revision = 0;
    p.updated_at_ms = now();
    super::wave_studio::validate(&p)?;
    // Bundle callers finish voice remapping and archive publication before registration.
    if !defer_registration {
        audio_assets::atomic_json(&folder.join("project.json"), &p)?;
        imported_files.0.clear();
        bind_folder(app, &p, &folder)?;
        for source in &p.sources {
            register_source(app, source, &p.id)?;
        }
        audio_assets::atomic_json(&project_path(app, &p.id)?, &p)?;
    }
    imported_files.0.clear();
    report(100);
    Ok(p)
}
// Bundle audio is already canonical float PCM. Reuse bytes rather than launching a decoder.
fn validate_pcm(path: &Path, source: &Source) -> Result<(), CommandError> {
    let reader = hound::WavReader::open(path)
        .map_err(|_| CommandError::new("INVALID_MEDIA", "Bundled audio is not a readable WAV."))?;
    let spec = reader.spec();
    let duration = reader.duration() as f64 * 1000. / spec.sample_rate.max(1) as f64;
    if spec.channels != 2
        || spec.sample_rate != 48000
        || spec.bits_per_sample != 32
        || spec.sample_format != hound::SampleFormat::Float
        || (duration - source.duration_ms).abs() > 2.
    {
        return Err(CommandError::new(
            "INVALID_MEDIA",
            "Bundled audio format or duration changed.",
        ));
    }
    let samples = reader.len() as u64;
    if fs::metadata(path)
        .map_err(|e| CommandError::io("Cannot inspect bundled audio", e))?
        .len()
        < samples * 4 + 44
    {
        return Err(CommandError::new(
            "INVALID_MEDIA",
            "Bundled audio is truncated.",
        ));
    }
    drop(reader);
    Ok(())
}
fn adopt_audio(
    app: &AppHandle,
    folder: &Path,
    path: &Path,
    source: &Source,
) -> Result<Source, CommandError> {
    validate_pcm(path, source)?;
    let mut imported = source.clone();
    if root(app)?
        .join("sources")
        .join(format!("{}.json", source.id))
        .exists()
    {
        imported.id = uuid::Uuid::new_v4().to_string();
    }
    let output = checked_media(folder, &imported.id)?;
    if output != path {
        copy_media(
            folder,
            path,
            &PathBuf::from(format!("media/{}.wav", imported.id)),
        )?;
        verify_copy(path, &output)?;
    }
    Ok(imported)
}

#[cfg(all(test, feature = "server", not(feature = "desktop")))]
mod adoption_tests {
    use super::*;
    #[test]
    fn incremental_media_is_atomic_and_rejects_redirects() {
        let base = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&base).unwrap();
        let source = base.join("source.wav");
        let folder = base.join("project.wavehs");
        fs::create_dir(&folder).unwrap();
        fs::write(&source, b"original").unwrap();
        copy_media(&folder, &source, Path::new("media/audio.wav")).unwrap();
        let media = folder.join("media/audio.wav");
        let modified = fs::metadata(&media).unwrap().modified().unwrap();
        fs::write(&source, b"changed").unwrap();
        copy_media(&folder, &source, Path::new("media/audio.wav")).unwrap();
        assert_eq!(fs::read(&media).unwrap(), b"original");
        assert_eq!(fs::metadata(&media).unwrap().modified().unwrap(), modified);
        assert!(copy_media(&folder, &source, Path::new("../outside.wav")).is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&base, folder.join("redirect")).unwrap();
            assert!(copy_media(&folder, &source, Path::new("redirect/outside.wav")).is_err());
            assert!(!base.join("outside.wav").exists());
        }
        fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn owned_pcm_adoption_preserves_bytes_without_ffmpeg() {
        let directory = std::env::temp_dir().join(format!("wave-adopt-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let app = AppHandle::new(directory.join("data"), directory.clone());
        let input = directory.join("source.wav");
        let mut writer = hound::WavWriter::create(
            &input,
            hound::WavSpec {
                channels: 2,
                sample_rate: 48000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .unwrap();
        for n in 0..9600 {
            writer.write_sample((n as f32 / 9600.).sin()).unwrap();
        }
        writer.finalize().unwrap();
        let source = Source {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Original".into(),
            duration_ms: 100.,
            channels: 2,
        };
        let project = create(&app, "Test", None).unwrap();
        let folder = require_package(&app, &project.id).unwrap();
        let imported = adopt_audio(&app, &folder, &input, &source).unwrap();
        register_source(&app, &imported, &project.id).unwrap();
        let output = source_path(&app, &imported.id).unwrap();
        assert_eq!(fs::read(&input).unwrap(), fs::read(&output).unwrap());
        fs::remove_file(input).unwrap();
        assert!(output.is_file());
        let mut invalid = source;
        invalid.duration_ms = 500.;
        assert!(adopt_audio(&app, &folder, &output, &invalid).is_err());
        fs::remove_dir_all(directory).unwrap();
    }
    fn fixture() -> (PathBuf, AppHandle, Source) {
        let directory = std::env::temp_dir().join(format!("wave-storage-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let app = AppHandle::new(directory.join("data"), directory.clone());
        let source = Source {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Legacy".into(),
            duration_ms: 100.,
            channels: 2,
        };
        let input = audio_assets::path(&root(&app).unwrap(), &source.id).unwrap();
        fs::create_dir_all(input.parent().unwrap()).unwrap();
        let mut writer = hound::WavWriter::create(
            input,
            hound::WavSpec {
                channels: 2,
                sample_rate: 48000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .unwrap();
        for _ in 0..9600 {
            writer.write_sample(0.1f32).unwrap();
        }
        writer.finalize().unwrap();
        audio_assets::atomic_json(
            &root(&app)
                .unwrap()
                .join("sources")
                .join(format!("{}.json", source.id)),
            &source,
        )
        .unwrap();
        (directory, app, source)
    }
    fn legacy(app: &AppHandle, source: &Source) -> Project {
        let mut project = create(app, "Legacy", None).unwrap();
        fs::remove_file(
            root(app)
                .unwrap()
                .join("folders")
                .join(format!("{}.json", project.id)),
        )
        .unwrap();
        project.sources.push(source.clone());
        audio_assets::atomic_json(&project_path(app, &project.id).unwrap(), &project).unwrap();
        project
    }
    #[test]
    fn migration_preserves_shared_trash_and_results_then_reclaims_verified_bytes() {
        let (directory, app, source) = fixture();
        let first = legacy(&app, &source);
        let second = legacy(&app, &source);
        trash(&app, &second.id, false).unwrap();
        let global = audio_assets::path(&root(&app).unwrap(), &source.id).unwrap();
        let bytes = fs::read(&global).unwrap();
        let a = directory.join("first.wavehs");
        let status = save_folder(&app, &first, &a, false).unwrap();
        assert_eq!(status.migrated_bytes, bytes.len() as u64);
        assert_eq!(status.reclaimed_bytes, 0);
        assert!(global.is_file());
        assert_eq!(
            fs::read(project_source_path_at(&root(&app).unwrap(), &first.id, &source.id).unwrap())
                .unwrap(),
            bytes
        );
        trash(&app, &second.id, true).unwrap();
        let b = directory.join("second.wavehs");
        let status = save_folder(&app, &second, &b, false).unwrap();
        assert_eq!(status.reclaimed_bytes, bytes.len() as u64);
        assert!(!global.exists());
        fs::write(&global, &bytes).unwrap();
        let result = root(&app).unwrap().join("results").join("pending.json");
        audio_assets::atomic_json(
            &result,
            &serde_json::json!({"projectId":first.id,"sources":[source]}),
        )
        .unwrap();
        assert_eq!(
            save_folder(&app, &first, &a, false)
                .unwrap()
                .reclaimed_bytes,
            0
        );
        assert!(global.is_file());
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn package_manifest_is_authoritative_and_moved_package_requires_relocation() {
        let (directory, app, source) = fixture();
        let p = legacy(&app, &source);
        let folder = directory.join("owned.wavehs");
        save_folder(&app, &p, &folder, false).unwrap();
        let mut changed = p.clone();
        changed.name = "Edited externally".into();
        changed.revision = 9;
        audio_assets::atomic_json(&folder.join("project.json"), &changed).unwrap();
        assert_eq!(load(&app, &p.id).unwrap().revision, 9);
        assert!(save(&app, p.clone(), 0).is_err());
        let moved = directory.join("moved.wavehs");
        fs::rename(folder, &moved).unwrap();
        assert!(load(&app, &p.id).is_err());
        assert!(project_source_path_at(&root(&app).unwrap(), &p.id, &source.id).is_err());
        locate(&app, &p.id, &moved).unwrap();
        assert_eq!(load(&app, &p.id).unwrap().name, "Edited externally");
        assert!(
            project_source_path_at(&root(&app).unwrap(), &p.id, &source.id)
                .unwrap()
                .starts_with(moved.canonicalize().unwrap())
        );
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn migration_copy_conflict_and_unreadable_metadata_keep_legacy_audio() {
        let (directory, app, source) = fixture();
        let p = legacy(&app, &source);
        let folder = directory.join("conflict.wavehs");
        fs::create_dir_all(folder.join("media")).unwrap();
        audio_assets::atomic_json(&folder.join("project.json"), &p).unwrap();
        fs::write(
            folder.join("media").join(format!("{}.wav", source.id)),
            b"conflicting audio",
        )
        .unwrap();
        assert!(save_folder(&app, &p, &folder, false).is_err());
        assert!(bound_folder(&app, &p.id).unwrap().is_none());
        let bad = root(&app).unwrap().join("trash");
        fs::create_dir_all(&bad).unwrap();
        fs::write(bad.join("unknown.json"), b"invalid").unwrap();
        let valid = directory.join("valid.wavehs");
        assert_eq!(
            save_folder(&app, &p, &valid, false)
                .unwrap()
                .reclaimed_bytes,
            0
        );
        assert!(audio_assets::path(&root(&app).unwrap(), &source.id)
            .unwrap()
            .is_file());
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn reusable_library_survives_missing_project_package() {
        let (directory, app, source) = fixture();
        let base = root(&app).unwrap();
        let library = base.join("library");
        fs::create_dir_all(library.join("media")).unwrap();
        let media = library.join("media").join(format!("{}.wav", source.id));
        fs::copy(audio_assets::path(&base, &source.id).unwrap(), &media).unwrap();
        let project = create(&app, "Library user", None).unwrap();
        register_source(&app, &source, &project.id).unwrap();
        fs::remove_dir_all(require_package(&app, &project.id).unwrap()).unwrap();
        assert_eq!(
            source_path(&app, &source.id).unwrap(),
            media.canonicalize().unwrap()
        );
        assert!(project_source_path_at(&base, &project.id, &source.id).is_err());
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn cancelled_import_cannot_publish_project_audio() {
        let (directory, app, source) = fixture();
        let p = create(&app, "Cancelled", None).unwrap();
        let folder = require_package(&app, &p.id).unwrap();
        assert!(import_controlled(
            &app,
            &audio_assets::path(&root(&app).unwrap(), &source.id).unwrap(),
            "Cancelled",
            &Settings::default(),
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
            std::sync::Arc::new(|_| {}),
            &p.id
        )
        .is_err());
        assert_eq!(fs::read_dir(folder.join("media")).unwrap().count(), 0);
        fs::remove_dir_all(directory).unwrap();
    }
}
