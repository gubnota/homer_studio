use super::{
    audio_assets,
    project_store::CommandError,
    settings::Settings,
    speech,
    wave_studio::{Project, Source, Timeline},
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};
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
        audio_assets::ffmpeg(
            settings,
            vec![
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
            Default::default(),
        )?;
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
pub fn save_copy(app: &AppHandle, p: &Project, destination: &Path) -> Result<(), CommandError> {
    validate_sources(app, p)?;
    if destination.exists() {
        return Err(CommandError::new(
            "DESTINATION_EXISTS",
            "Choose a new folder name for this project copy.",
        ));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| CommandError::new("INVALID_DESTINATION", "Choose a project folder."))?;
    let work = parent.join(format!(".homer-copy-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&work).map_err(|e| CommandError::io("Cannot create project copy", e))?;
    let result = (|| {
        fs::create_dir(work.join("media"))
            .map_err(|e| CommandError::io("Cannot create media folder", e))?;
        for source in &p.sources {
            fs::copy(
                source_path(app, &source.id)?,
                work.join("media").join(format!("{}.wav", source.id)),
            )
            .map_err(|e| CommandError::io("Cannot copy project audio", e))?;
        }
        for video in &p.videos {
            fs::copy(
                super::wave_video::path(app, &video.id)?,
                work.join("media").join(format!("{}.mp4", video.id)),
            )
            .map_err(|e| CommandError::io("Cannot copy project video", e))?;
        }
        audio_assets::atomic_json(&work.join("project.json"), p)?;
        fs::rename(&work, destination)
            .map_err(|e| CommandError::io("Cannot publish project copy", e))
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(work);
    }
    result
}
pub fn open_copy(
    app: &AppHandle,
    folder: &Path,
    settings: &Settings,
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
    let mut mapping = std::collections::HashMap::new();
    for s in &mut p.sources {
        let old = s.id.clone();
        let imported = import(app, &owned(&old, "wav")?, &s.name, settings)?;
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
            if let Some(production) = &mut r.production {
                for (old, new) in &mapping {
                    production.audio_key = production.audio_key.replace(old, new)
                }
            }
        }
    }
    for v in &mut p.videos {
        let imported = super::wave_video::import(app, &owned(&v.id, "mp4")?, settings)?;
        if (imported.duration_ms - v.duration_ms).abs() > 100. {
            return Err(CommandError::new(
                "INVALID_MEDIA",
                "Project video duration changed.",
            ));
        }
        if let Some(view) = &mut p.view {
            if view.selected_id.as_ref() == Some(&v.id) {
                view.selected_id = Some(imported.id.clone());
            }
        }
        v.id = imported.id;
        v.duration_ms = imported.duration_ms;
    }
    p.id = uuid::Uuid::new_v4().to_string();
    p.revision = 0;
    p.updated_at_ms = now();
    validate_sources(app, &p)?;
    audio_assets::atomic_json(&project_path(app, &p.id)?, &p)?;
    Ok(p)
}
