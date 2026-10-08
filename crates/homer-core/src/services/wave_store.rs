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
    audio_assets::path(&root(app)?, id)
}
fn project_path(app: &AppHandle, id: &str) -> Result<PathBuf, CommandError> {
    audio_assets::id(id)?;
    Ok(root(app)?.join("projects").join(format!("{id}.json")))
}
pub fn load(app: &AppHandle, id: &str) -> Result<Project, CommandError> {
    let b = fs::read(project_path(app, id)?)
        .map_err(|e| CommandError::io("Cannot open Wave project", e))?;
    if b.len() > 16 * 1024 * 1024 {
        return Err(CommandError::new(
            "INVALID_WAVE_PROJECT",
            "Project is too large.",
        ));
    }
    let p: Project = serde_json::from_slice(&b)
        .map_err(|e| CommandError::new("INVALID_WAVE_PROJECT", e.to_string()))?;
    super::wave_studio::validate(&p)?;
    Ok(p)
}
pub fn create(app: &AppHandle, name: &str) -> Result<Project, CommandError> {
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
    audio_assets::atomic_json(&project_path(app, &p.id)?, &p)?;
    Ok(p)
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
    validate_sources(app, &p)?;
    p.revision = expected + 1;
    p.updated_at_ms = now();
    if let Some(folder) = bound_folder(app, &p.id)? {
        let include = fs::read(
            root(app)?
                .join("folders")
                .join(format!("{}.options.json", p.id)),
        )
        .ok()
        .and_then(|b| serde_json::from_slice::<bool>(&b).ok())
        .unwrap_or(false);
        save_copy_options(app, &p, &folder, include)?;
    }
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
            if let Ok(p) = load(app, id) {
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
) -> Result<Source, CommandError> {
    import_controlled(
        app,
        path,
        name,
        settings,
        Default::default(),
        std::sync::Arc::new(|_| {}),
    )
}
pub fn import_controlled(
    app: &AppHandle,
    path: &Path,
    name: &str,
    settings: &Settings,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    report: std::sync::Arc<dyn Fn(u8) + Send + Sync>,
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
    let output = source_path(app, &id)?;
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
        audio_assets::atomic_json(&root(app)?.join("sources").join(format!("{id}.json")), &s)?;
        Ok(s)
    })();
    if result.is_err() {
        let _ = fs::remove_file(output);
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
) -> Result<(), CommandError> {
    save_copy_options(app, p, destination, include_video)?;
    audio_assets::atomic_json(
        &root(app)?
            .join("folders")
            .join(format!("{}.options.json", p.id)),
        &include_video,
    )?;
    bind_folder(app, p, destination)
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
    adopt: bool,
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
        let imported = if adopt {
            adopt_audio(app, &path, s)?
        } else {
            import_controlled(
                app,
                &path,
                &s.name,
                settings,
                cancel.clone(),
                std::sync::Arc::new(|_| {}),
            )?
        };
        completed += 1;
        report((65 + completed * 30 / total.max(1)) as u8);
        imported_files.0.push(source_path(app, &imported.id)?);
        imported_files.0.push(
            root(app)?
                .join("sources")
                .join(format!("{}.json", imported.id)),
        );
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
    p.id = uuid::Uuid::new_v4().to_string();
    p.revision = 0;
    p.updated_at_ms = now();
    validate_sources(app, &p)?;
    audio_assets::atomic_json(&project_path(app, &p.id)?, &p)?;
    imported_files.0.clear();
    report(100);
    Ok(p)
}
// Bundle audio is already canonical float PCM. Reuse bytes rather than launching a decoder.
fn adopt_audio(app: &AppHandle, path: &Path, source: &Source) -> Result<Source, CommandError> {
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
    let mut imported = source.clone();
    imported.id = uuid::Uuid::new_v4().to_string();
    let output = source_path(app, &imported.id)?;
    fs::create_dir_all(output.parent().unwrap())
        .map_err(|e| CommandError::io("Cannot create audio library", e))?;
    // A hard link avoids a second large disk copy; scratch removal leaves the owned link intact.
    if fs::hard_link(path, &output).is_err() {
        fs::copy(path, &output).map_err(|e| CommandError::io("Cannot restore project audio", e))?;
    }
    if let Err(e) = audio_assets::atomic_json(
        &root(app)?
            .join("sources")
            .join(format!("{}.json", imported.id)),
        &imported,
    ) {
        let _ = fs::remove_file(output);
        return Err(e);
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
        let imported = adopt_audio(&app, &input, &source).unwrap();
        let output = source_path(&app, &imported.id).unwrap();
        assert_eq!(fs::read(&input).unwrap(), fs::read(&output).unwrap());
        fs::remove_file(input).unwrap();
        assert!(output.is_file());
        let mut invalid = source;
        invalid.duration_ms = 500.;
        assert!(adopt_audio(&app, &output, &invalid).is_err());
        fs::remove_dir_all(directory).unwrap();
    }
}
