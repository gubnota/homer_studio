use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

use super::{llm, process_runner, project_store::CommandError};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,
    pub llm: LlmSettings,
    pub speech: SpeechSettings,
    pub ffmpeg_path: Option<String>,
    pub ffprobe_path: Option<String>,
    #[serde(default)]
    pub ollama_path: Option<String>,
    #[serde(default)]
    pub voice_presets: Vec<VoicePreset>,
    #[serde(default)]
    pub selected_voice_preset_id: Option<String>,
    #[serde(default)]
    pub sounds: SoundSettings,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundSettings {
    pub chatterbox_url: String,
    pub sfx_url: String,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            chatterbox_url: "http://127.0.0.1:8765".into(),
            sfx_url: "http://127.0.0.1:8766".into(),
        }
    }
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoicePreset {
    pub id: String,
    pub name: String,
    pub voice_id: String,
    pub rate: u16,
    pub built_in: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDiagnostic {
    pub key: String,
    pub name: String,
    pub path: Option<String>,
    pub available: bool,
    pub status: String,
    pub configured_path: Option<String>,
    pub detected_path: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            llm: LlmSettings::None,
            speech: SpeechSettings {
                provider: "chatterbox_turbo".into(),
                voice_id: super::voice_store::DEFAULT_VOICE.into(),
                rate: 180,
            },
            ffmpeg_path: None,
            ffprobe_path: None,
            ollama_path: None,
            voice_presets: Vec::new(),
            selected_voice_preset_id: None,
            sounds: SoundSettings::default(),
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
    let settings = migrate_speech(settings);
    validate(&settings)?;
    Ok(settings)
}

pub fn save(app: &AppHandle, settings: Settings) -> Result<Settings, CommandError> {
    let settings = migrate_speech(settings);
    validate(&settings)?;
    let voices = super::voice_store::list(app)?;
    let selected = voices.iter().find(|voice| voice.id == settings.speech.voice_id)
        .ok_or_else(|| CommandError::new("VOICE_NOT_FOUND", "Select a voice from the library."))?;
    if !selected.built_in && selected.selected_sample_id.is_none() {
        return Err(CommandError::new("VOICE_HAS_NO_SAMPLE", "Add a spoken sample before selecting this voice."));
    }
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
        (
            "ffmpeg",
            "FFmpeg",
            "ffmpeg",
            settings.ffmpeg_path.as_deref(),
        ),
        (
            "ffprobe",
            "FFprobe",
            "ffprobe",
            settings.ffprobe_path.as_deref(),
        ),
        (
            "llama",
            "llama.cpp",
            "llama-cli",
            llama_override(&settings.llm),
        ),
        (
            "ollama",
            "Ollama CLI",
            "ollama",
            settings.ollama_path.as_deref(),
        ),
    ];
    let mut diagnostics: Vec<_> = tools
        .into_iter()
        .map(|(key, label, name, configured)| {
            let detected = process_runner::resolve_executable(name, None);
            let resolved = process_runner::resolve_executable(name, configured);
            let configured_valid = configured
                .filter(|value| !value.trim().is_empty())
                .is_some_and(|_| resolved.is_some());
            let status = if configured.is_some_and(|value| !value.trim().is_empty()) {
                if configured_valid {
                    "configured"
                } else {
                    "invalid_configuration"
                }
            } else if detected.is_some() {
                "found_automatically"
            } else {
                "not_found"
            };
            ToolDiagnostic {
                key: key.into(),
                name: label.into(),
                available: resolved.is_some(),
                path: resolved.map(|path| path.to_string_lossy().into_owned()),
                status: status.into(),
                configured_path: configured.map(str::to_string),
                detected_path: detected.map(|path| path.to_string_lossy().into_owned()),
            }
        })
        .collect();
    let (base_url, selected_model) = match &settings.llm {
        LlmSettings::Ollama { base_url, model, .. } => (base_url.as_str(), Some(model.as_str())),
        _ => ("http://127.0.0.1:11434", None),
    };
    let status = match llm::ollama_models(base_url) {
        Err(_) => "service_unavailable",
        Ok(models) if models.is_empty() => "no_models",
        Ok(models) if selected_model.is_some_and(|model| model.trim().is_empty() || !models.iter().any(|installed| installed == model)) => "model_not_installed",
        Ok(_) => "service_ready",
    };
    diagnostics.push(ToolDiagnostic {
        key: "ollama_service".into(),
        name: "Ollama server".into(),
        path: Some(base_url.into()),
        available: status == "service_ready",
        status: status.into(),
        configured_path: Some(base_url.into()),
        detected_path: None,
    });
    let speech = super::sound_workers::health(&settings.sounds.chatterbox_url, "chatterbox_turbo");
    diagnostics.push(ToolDiagnostic {
        key: "speech".into(), name: "Chatterbox Turbo".into(), path: Some(settings.sounds.chatterbox_url.clone()),
        available: speech.ready, status: if speech.ready { "service_ready" } else { "service_unavailable" }.into(),
        configured_path: Some(settings.sounds.chatterbox_url.clone()), detected_path: None,
    });
    diagnostics
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
    validate_worker_url(&settings.sounds.chatterbox_url)?;
    validate_worker_url(&settings.sounds.sfx_url)?;
    if settings.schema_version != 1 {
        return Err(CommandError::new(
            "UNSUPPORTED_SETTINGS_VERSION",
            "Only settings version 1 is supported.",
        ));
    }
    if settings.speech.provider != "chatterbox_turbo" || settings.speech.voice_id.trim().is_empty() {
        return Err(CommandError::new("INVALID_SPEECH_PROVIDER", "Choose a Chatterbox voice."));
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

pub fn validate_worker_url(value: &str) -> Result<(), CommandError> {
    let port = value.strip_prefix("http://127.0.0.1:").ok_or_else(|| CommandError::new("UNSAFE_WORKER_URL", "Worker URL must be http://127.0.0.1:PORT."))?;
    if port.is_empty() || !port.bytes().all(|byte| byte.is_ascii_digit()) || port.parse::<u16>().ok().filter(|port| *port > 0).is_none() {
        return Err(CommandError::new("UNSAFE_WORKER_URL", "Worker URL must be http://127.0.0.1:PORT."));
    }
    Ok(())
}

fn migrate_speech(mut settings: Settings) -> Settings {
    if settings.speech.provider != "chatterbox_turbo" {
        settings.speech.provider = "chatterbox_turbo".into();
        settings.speech.voice_id = super::voice_store::DEFAULT_VOICE.into();
    }
    settings.voice_presets.clear();
    settings.selected_voice_preset_id = None;
    settings
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_macos_voice_without_changing_other_settings() {
        let mut settings = Settings::default();
        settings.speech.provider = "macos_say".into();
        settings.speech.voice_id = "Karen".into();
        settings.voice_presets.push(VoicePreset { id: "old".into(), name: "Old".into(), voice_id: "Karen".into(), rate: 180, built_in: true });
        let migrated = migrate_speech(settings);
        assert_eq!(migrated.speech.voice_id, crate::services::voice_store::DEFAULT_VOICE);
        assert!(migrated.voice_presets.is_empty());
    }
}
