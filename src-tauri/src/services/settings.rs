use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
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
                provider: "macos_say".into(),
                voice_id: "Samantha".into(),
                rate: 180,
            },
            ffmpeg_path: None,
            ffprobe_path: None,
            ollama_path: None,
            voice_presets: Vec::new(),
            selected_voice_preset_id: None,
        }
    }
}

pub fn load(app: &AppHandle) -> Result<Settings, CommandError> {
    let path = path(app)?;
    if !path.exists() {
        return Ok(with_voice_presets(Settings::default()));
    }
    let data = fs::read(&path).map_err(|error| CommandError::io("Cannot read settings", error))?;
    let settings: Settings = serde_json::from_slice(&data).map_err(|error| {
        CommandError::new(
            "INVALID_SETTINGS",
            format!("Settings file is invalid: {error}"),
        )
    })?;
    let settings = with_voice_presets(settings);
    validate(&settings)?;
    Ok(settings)
}

pub fn save(app: &AppHandle, settings: Settings) -> Result<Settings, CommandError> {
    let settings = with_voice_presets(settings);
    validate(&settings)?;
    super::speech::ensure_installed_voice(&settings.speech.voice_id)?;
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
        ("speech", "macOS speech", "say", None),
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
    let base_url = match &settings.llm {
        LlmSettings::Ollama { base_url, .. } => base_url.as_str(),
        _ => "http://127.0.0.1:11434",
    };
    let reachable = llm::ollama_reachable(base_url);
    diagnostics.push(ToolDiagnostic {
        key: "ollama_service".into(),
        name: "Ollama server".into(),
        path: Some(base_url.into()),
        available: reachable,
        status: if reachable {
            "service_reachable"
        } else {
            "service_unavailable"
        }
        .into(),
        configured_path: Some(base_url.into()),
        detected_path: None,
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
    let mut ids = HashSet::new();
    for preset in &settings.voice_presets {
        if preset.id.trim().is_empty()
            || preset.name.trim().is_empty()
            || preset.voice_id.trim().is_empty()
        {
            return Err(CommandError::new(
                "INVALID_VOICE_PRESET",
                "Voice presets need an ID, name, and installed macOS voice.",
            ));
        }
        if !ids.insert(&preset.id) {
            return Err(CommandError::new(
                "INVALID_VOICE_PRESET",
                "Voice preset IDs must be unique.",
            ));
        }
        if !(80..=500).contains(&preset.rate) {
            return Err(CommandError::new(
                "INVALID_VOICE_PRESET",
                "Voice preset rates must be between 80 and 500 words per minute.",
            ));
        }
    }
    if settings
        .selected_voice_preset_id
        .as_ref()
        .is_some_and(|selected| !ids.contains(selected))
    {
        return Err(CommandError::new(
            "INVALID_VOICE_PRESET",
            "The selected voice preset does not exist.",
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

fn with_voice_presets(mut settings: Settings) -> Settings {
    let installed: HashSet<String> = super::speech::list_voices()
        .unwrap_or_default()
        .into_iter()
        .map(|voice| voice.id)
        .collect();
    migrate_voice_presets(&mut settings, &installed);
    settings
}

fn migrate_voice_presets(settings: &mut Settings, installed: &HashSet<String>) {
    let built_ins = [
        ("built-in-samantha", "Warm narrator", "Samantha", 180),
        ("built-in-daniel", "Measured storyteller", "Daniel", 170),
        ("built-in-karen", "Clear narrator", "Karen", 180),
    ];
    for (id, name, voice_id, rate) in built_ins {
        if installed.contains(voice_id)
            && !settings.voice_presets.iter().any(|preset| preset.id == id)
        {
            settings.voice_presets.push(VoicePreset {
                id: id.into(),
                name: name.into(),
                voice_id: voice_id.into(),
                rate,
                built_in: true,
            });
        }
    }
    if settings.selected_voice_preset_id.is_none() {
        if let Some(preset) = settings.voice_presets.iter().find(|preset| {
            preset.voice_id == settings.speech.voice_id && preset.rate == settings.speech.rate
        }) {
            settings.selected_voice_preset_id = Some(preset.id.clone());
        } else if !settings.speech.voice_id.trim().is_empty() {
            let id = "migrated-current-voice".to_string();
            if !settings.voice_presets.iter().any(|preset| preset.id == id) {
                settings.voice_presets.push(VoicePreset {
                    id: id.clone(),
                    name: "My current voice".into(),
                    voice_id: settings.speech.voice_id.clone(),
                    rate: settings.speech.rate,
                    built_in: false,
                });
            }
            settings.selected_voice_preset_id = Some(id);
        }
    }
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
    fn seeds_only_installed_built_in_voices() {
        let mut settings = Settings::default();
        migrate_voice_presets(&mut settings, &HashSet::from(["Daniel".to_string()]));
        assert!(
            settings
                .voice_presets
                .iter()
                .any(|preset| preset.voice_id == "Daniel")
        );
        assert!(
            !settings
                .voice_presets
                .iter()
                .any(|preset| preset.voice_id == "Karen")
        );
        assert!(settings.voice_presets.iter().any(|preset| !preset.built_in));
    }

    #[test]
    fn migrates_an_existing_matching_voice_to_a_preset() {
        let mut settings = Settings::default();
        migrate_voice_presets(&mut settings, &HashSet::from(["Samantha".to_string()]));
        assert_eq!(
            settings.selected_voice_preset_id.as_deref(),
            Some("built-in-samantha")
        );
    }

    #[test]
    fn rejects_duplicate_preset_ids() {
        let mut settings = Settings::default();
        let preset = VoicePreset {
            id: "same".into(),
            name: "One".into(),
            voice_id: "Samantha".into(),
            rate: 180,
            built_in: false,
        };
        settings.voice_presets = vec![preset.clone(), preset];
        assert_eq!(
            validate(&settings).unwrap_err().code,
            "INVALID_VOICE_PRESET"
        );
    }
}
