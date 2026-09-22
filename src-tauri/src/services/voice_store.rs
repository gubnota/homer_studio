use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use tauri::{AppHandle, Manager};

use super::{process_runner, project_store::CommandError, settings::Settings, speech};

pub const DEFAULT_VOICE: &str = "chatterbox-default";

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSample {
    pub id: String,
    pub name: String,
    pub duration_ms: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    pub id: String,
    pub name: String,
    pub samples: Vec<VoiceSample>,
    pub selected_sample_id: Option<String>,
    pub built_in: bool,
}

#[derive(Default, Serialize, Deserialize)]
struct Library {
    schema_version: u32,
    voices: Vec<Voice>,
}

fn root(app: &AppHandle) -> Result<PathBuf, CommandError> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("voices"))
        .map_err(|error| CommandError::internal(format!("Cannot locate voice library: {error}")))
}

fn load(app: &AppHandle) -> Result<Library, CommandError> {
    let path = root(app)?.join("library.json");
    if !path.exists() {
        return Ok(Library {
            schema_version: 1,
            voices: Vec::new(),
        });
    }
    let data =
        fs::read(path).map_err(|error| CommandError::io("Cannot read voice library", error))?;
    let library: Library = serde_json::from_slice(&data)
        .map_err(|error| CommandError::new("INVALID_VOICE_LIBRARY", error.to_string()))?;
    if library.schema_version != 1 {
        return Err(CommandError::new(
            "INVALID_VOICE_LIBRARY",
            "Unsupported voice library version.",
        ));
    }
    Ok(library)
}

fn save(app: &AppHandle, library: &Library) -> Result<(), CommandError> {
    let directory = root(app)?;
    fs::create_dir_all(&directory)
        .map_err(|error| CommandError::io("Cannot create voice library", error))?;
    let path = directory.join("library.json");
    let temp = directory.join("library.tmp");
    fs::write(
        &temp,
        serde_json::to_vec_pretty(library)
            .map_err(|error| CommandError::internal(error.to_string()))?,
    )
    .map_err(|error| CommandError::io("Cannot save voice library", error))?;
    fs::rename(temp, path).map_err(|error| CommandError::io("Cannot publish voice library", error))
}

pub fn list(app: &AppHandle) -> Result<Vec<Voice>, CommandError> {
    let mut voices = vec![Voice {
        id: DEFAULT_VOICE.into(),
        name: "Chatterbox natural voice".into(),
        samples: Vec::new(),
        selected_sample_id: None,
        built_in: true,
    }];
    voices.extend(load(app)?.voices);
    Ok(voices)
}

pub fn create(app: &AppHandle, name: &str) -> Result<Voice, CommandError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(CommandError::new(
            "INVALID_VOICE_NAME",
            "Enter a voice name of 1–80 characters.",
        ));
    }
    let mut library = load(app)?;
    let voice = Voice {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.into(),
        samples: Vec::new(),
        selected_sample_id: None,
        built_in: false,
    };
    library.voices.push(voice.clone());
    save(app, &library)?;
    Ok(voice)
}

pub fn add_sample(
    app: &AppHandle,
    voice_id: &str,
    name: &str,
    source: &Path,
    settings: &Settings,
) -> Result<Voice, CommandError> {
    let mut library = load(app)?;
    let voice = library
        .voices
        .iter_mut()
        .find(|voice| voice.id == voice_id)
        .ok_or_else(|| CommandError::new("VOICE_NOT_FOUND", "Choose a custom voice first."))?;
    if !source.is_file()
        || fs::metadata(source).map(|m| m.len()).unwrap_or(u64::MAX) > 30 * 1024 * 1024
    {
        return Err(CommandError::new(
            "INVALID_VOICE_SAMPLE",
            "Choose an audio file smaller than 30 MB.",
        ));
    }
    let sample_id = uuid::Uuid::new_v4().to_string();
    let directory = root(app)?.join(&voice.id);
    fs::create_dir_all(&directory)
        .map_err(|error| CommandError::io("Cannot create sample folder", error))?;
    let destination = directory.join(format!("{sample_id}.wav"));
    let ffmpeg = process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
        .ok_or_else(|| {
            CommandError::new(
                "FFMPEG_NOT_FOUND",
                "Set FFmpeg in Settings before adding a voice sample.",
            )
        })?;
    let result = process_runner::run_bounded(
        &ffmpeg,
        &[
            "-v".into(),
            "error".into(),
            "-y".into(),
            "-i".into(),
            source.to_string_lossy().into_owned(),
            "-vn".into(),
            "-ac".into(),
            "1".into(),
            "-ar".into(),
            "24000".into(),
            "-c:a".into(),
            "pcm_s16le".into(),
            destination.to_string_lossy().into_owned(),
        ],
        Duration::from_secs(90),
        Default::default(),
    )?;
    if !result.success {
        let _ = fs::remove_file(&destination);
        return Err(CommandError::new("INVALID_VOICE_SAMPLE", result.stderr));
    }
    let duration = speech::probe_duration(&destination, settings)?;
    let size = fs::metadata(&destination)
        .map_err(|error| CommandError::io("Cannot inspect sample", error))?
        .len();
    if !(6_000..=20_000).contains(&duration) || size > 2 * 1024 * 1024 {
        let _ = fs::remove_file(&destination);
        return Err(CommandError::new(
            "INVALID_VOICE_SAMPLE",
            "Use a clear 6–20 second spoken sample smaller than 2 MB.",
        ));
    }
    let bytes =
        fs::read(&destination).map_err(|error| CommandError::io("Cannot inspect sample", error))?;
    if bytes.iter().skip(44).all(|byte| *byte == 0) {
        let _ = fs::remove_file(&destination);
        return Err(CommandError::new(
            "INVALID_VOICE_SAMPLE",
            "The sample appears silent.",
        ));
    }
    voice.samples.push(VoiceSample {
        id: sample_id.clone(),
        name: name.trim().chars().take(80).collect(),
        duration_ms: duration,
    });
    voice.selected_sample_id = Some(sample_id);
    let result = voice.clone();
    save(app, &library)?;
    Ok(result)
}

pub fn selected_sample(app: &AppHandle, voice_id: &str) -> Result<Option<Vec<u8>>, CommandError> {
    if voice_id == DEFAULT_VOICE {
        return Ok(None);
    }
    let library = load(app)?;
    let voice = library
        .voices
        .iter()
        .find(|voice| voice.id == voice_id)
        .ok_or_else(|| CommandError::new("VOICE_NOT_FOUND", "Selected voice no longer exists."))?;
    let sample_id = voice.selected_sample_id.as_ref().ok_or_else(|| {
        CommandError::new(
            "VOICE_HAS_NO_SAMPLE",
            "Add a spoken sample to this voice first.",
        )
    })?;
    if !voice.samples.iter().any(|sample| &sample.id == sample_id) {
        return Err(CommandError::new(
            "VOICE_HAS_NO_SAMPLE",
            "Selected voice sample no longer exists.",
        ));
    }
    fs::read(root(app)?.join(&voice.id).join(format!("{sample_id}.wav")))
        .map(Some)
        .map_err(|error| CommandError::io("Cannot read voice sample", error))
}

pub fn preview_path(app: &AppHandle, voice_id: &str) -> Result<PathBuf, CommandError> {
    if voice_id == DEFAULT_VOICE {
        return Err(CommandError::new(
            "VOICE_PREVIEW_UNAVAILABLE",
            "The built-in voice preview is not cached.",
        ));
    }
    let library = load(app)?;
    if !library.voices.iter().any(|voice| voice.id == voice_id) {
        return Err(CommandError::new("VOICE_NOT_FOUND", "Voice not found."));
    }
    Ok(root(app)?.join(voice_id).join("preview.m4a"))
}

pub fn sample_path(
    app: &AppHandle,
    voice_id: &str,
    sample_id: &str,
) -> Result<PathBuf, CommandError> {
    let library = load(app)?;
    let voice = library
        .voices
        .iter()
        .find(|voice| voice.id == voice_id)
        .ok_or_else(|| CommandError::new("VOICE_NOT_FOUND", "Voice not found."))?;
    if !voice.samples.iter().any(|sample| sample.id == sample_id) {
        return Err(CommandError::new(
            "VOICE_SAMPLE_NOT_FOUND",
            "Sample not found.",
        ));
    }
    Ok(root(app)?.join(&voice.id).join(format!("{sample_id}.wav")))
}

pub fn select_sample(
    app: &AppHandle,
    voice_id: &str,
    sample_id: &str,
) -> Result<Voice, CommandError> {
    let mut library = load(app)?;
    let voice = library
        .voices
        .iter_mut()
        .find(|voice| voice.id == voice_id)
        .ok_or_else(|| CommandError::new("VOICE_NOT_FOUND", "Voice not found."))?;
    if !voice.samples.iter().any(|sample| sample.id == sample_id) {
        return Err(CommandError::new(
            "VOICE_SAMPLE_NOT_FOUND",
            "Sample not found.",
        ));
    }
    voice.selected_sample_id = Some(sample_id.into());
    let result = voice.clone();
    save(app, &library)?;
    Ok(result)
}

pub fn delete(app: &AppHandle, voice_id: &str, active_voice_id: &str) -> Result<(), CommandError> {
    if voice_id == active_voice_id {
        return Err(CommandError::new(
            "VOICE_IN_USE",
            "Select another voice before deleting this one.",
        ));
    }
    let mut library = load(app)?;
    let before = library.voices.len();
    library.voices.retain(|voice| voice.id != voice_id);
    if before == library.voices.len() {
        return Err(CommandError::new("VOICE_NOT_FOUND", "Voice not found."));
    }
    save(app, &library)?;
    let sample_dir = root(app)?.join(voice_id);
    if sample_dir.exists() {
        fs::remove_dir_all(sample_dir)
            .map_err(|error| CommandError::io("Cannot remove voice samples", error))?;
    }
    Ok(())
}
