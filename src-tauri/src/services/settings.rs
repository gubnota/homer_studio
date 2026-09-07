use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

use super::{process_runner, project_store::CommandError};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,
    pub llm: LlmSettings,
    pub speech: SpeechSettings,
    pub ffmpeg_path: Option<String>,
    pub ffprobe_path: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum LlmSettings {
    None,
    LlamaCpp {
        executable_path: String,
        model_path: String,
        context_size: u32,
        max_tokens: u32,
        gpu_layers: u32,
    },
    Ollama {
        base_url: String,
        model: String,
        context_size: u32,
        max_tokens: u32,
    },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechSettings {
    pub provider: String,
    pub voice_id: String,
    pub rate: u16,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDiagnostic {
    pub name: String,
    pub path: Option<String>,
    pub available: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            llm: LlmSettings::None,
            speech: SpeechSettings {
                provider: "macos_say".into(),
                voice_id: "Samantha".into(),
                rate: 180,
            },
            ffmpeg_path: None,
            ffprobe_path: None,
        }
    }
}

pub fn load(app: &AppHandle) -> Result<Settings, CommandError> {
    let path = path(app)?;
    if !path.exists() {
        return Ok(Settings::default());
    }
    let data = fs::read(&path).map_err(|error| CommandError::io("Cannot read settings", error))?;
    let settings: Settings = serde_json::from_slice(&data).map_err(|error| {
        CommandError::new(
            "INVALID_SETTINGS",
            format!("Settings file is invalid: {error}"),
        )
    })?;
    validate(&settings)?;
    Ok(settings)
}

pub fn save(app: &AppHandle, settings: Settings) -> Result<Settings, CommandError> {
    validate(&settings)?;
    let path = path(app)?;
    fs::create_dir_all(path.parent().unwrap())
        .map_err(|error| CommandError::io("Cannot create settings folder", error))?;
    let bytes = serde_json::to_vec_pretty(&settings)
        .map_err(|error| CommandError::internal(format!("Cannot serialize settings: {error}")))?;
    let temp = path.with_extension("tmp");
    fs::write(&temp, bytes).map_err(|error| CommandError::io("Cannot write settings", error))?;
    fs::rename(temp, path).map_err(|error| CommandError::io("Cannot publish settings", error))?;
    Ok(settings)
}

pub fn diagnostics(settings: &Settings) -> Vec<ToolDiagnostic> {
    let tools = [
        ("macOS speech", "say", None),
        ("FFmpeg", "ffmpeg", settings.ffmpeg_path.as_deref()),
        ("FFprobe", "ffprobe", settings.ffprobe_path.as_deref()),
        ("llama.cpp", "llama-cli", llama_override(&settings.llm)),
        ("Ollama", "ollama", None),
    ];
    tools
        .into_iter()
        .map(|(label, name, override_path)| {
            let path = process_runner::resolve_executable(name, override_path);
            ToolDiagnostic {
                name: label.into(),
                available: path.is_some(),
                path: path.map(|path| path.to_string_lossy().into_owned()),
            }
        })
        .collect()
}

fn llama_override(llm: &LlmSettings) -> Option<&str> {
    if let LlmSettings::LlamaCpp {
        executable_path, ..
    } = llm
    {
        Some(executable_path)
    } else {
        None
    }
}

fn validate(settings: &Settings) -> Result<(), CommandError> {
    if settings.schema_version != 1 {
        return Err(CommandError::new(
            "UNSUPPORTED_SETTINGS_VERSION",
            "Only settings version 1 is supported.",
        ));
    }
    if !(80..=500).contains(&settings.speech.rate) {
        return Err(CommandError::new(
            "INVALID_SPEECH_RATE",
            "Speech rate must be between 80 and 500 words per minute.",
        ));
    }
    if let LlmSettings::Ollama { base_url, .. } = &settings.llm {
        if !(base_url.starts_with("http://127.0.0.1:") || base_url.starts_with("http://localhost:"))
        {
            return Err(CommandError::new(
                "UNSAFE_OLLAMA_URL",
                "Ollama must use a loopback HTTP address.",
            ));
        }
    }
    Ok(())
}

fn path(app: &AppHandle) -> Result<PathBuf, CommandError> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join("settings.json"))
        .map_err(|error| CommandError::internal(format!("Cannot locate settings folder: {error}")))
}

#[allow(dead_code)]
fn _is_file(path: &Path) -> bool {
    path.is_file()
}
