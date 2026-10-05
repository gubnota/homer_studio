use super::{process_runner, project_store::CommandError, settings::Settings, speech};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
use tauri::{AppHandle, Manager};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioClip {
    pub asset_id: Option<String>,
    pub start_ms: u64,
    pub end_ms: u64,
    pub gain: f32,
    pub fade_in_ms: u64,
    pub fade_out_ms: u64,
    pub crossfade_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioComposition {
    pub clips: Vec<AudioClip>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingHistory {
    pub engine: String,
    pub version: String,
    pub preset: String,
    pub params: serde_json::Value,
    pub profile_id: Option<String>,
    pub source_start_ms: u64,
    pub source_end_ms: u64,
    pub created_at_ms: u64,
    pub backend: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioVariant {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub path: String,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub original_sample_rate: Option<u32>,
    pub channels: u16,
    pub created_at_ms: u64,
    pub state: String,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub notes: String,
    pub composition: AudioComposition,
    pub processing: Option<ProcessingHistory>,
    #[serde(default)]
    pub cues: Vec<super::project_store::LineCue>,
}
pub fn root(app: &AppHandle) -> Result<PathBuf, CommandError> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("audio-studio"))
        .map_err(|e| CommandError::internal(e.to_string()))
}
pub fn id(value: &str) -> Result<(), CommandError> {
    uuid::Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| CommandError::new("INVALID_AUDIO_ID", "Invalid audio ID."))
}
pub fn path(base: &Path, value: &str) -> Result<PathBuf, CommandError> {
    id(value)?;
    let target = base.join("assets").join(format!("{value}.wav"));
    if target.exists() {
        let canonical =
            fs::canonicalize(&target).map_err(|e| CommandError::io("Cannot resolve audio", e))?;
        let owned = fs::canonicalize(base)
            .map_err(|e| CommandError::io("Cannot resolve audio library", e))?;
        if !canonical.starts_with(owned) {
            return Err(CommandError::new(
                "UNSAFE_AUDIO_PATH",
                "Audio path leaves its library.",
            ));
        }
    }
    Ok(target)
}
pub fn ffmpeg(
    settings: &Settings,
    args: Vec<String>,
    cancel: Arc<AtomicBool>,
) -> Result<(), CommandError> {
    let tool = process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
        .ok_or_else(|| {
            CommandError::new(
                "FFMPEG_NOT_FOUND",
                "Install FFmpeg or select it in Settings.",
            )
        })?;
    let result = process_runner::run_bounded(&tool, &args, Duration::from_secs(3600), cancel)?;
    if !result.success {
        return Err(CommandError::new(
            "AUDIO_RENDER_FAILED",
            result.stderr.chars().take(2000).collect::<String>(),
        ));
    }
    Ok(())
}
pub fn normalize(
    source: &Path,
    output: &Path,
    settings: &Settings,
    cancel: Arc<AtomicBool>,
) -> Result<u64, CommandError> {
    if !source.is_file() {
        return Err(CommandError::new(
            "AUDIO_NOT_FOUND",
            "Choose an existing audio file.",
        ));
    }
    ffmpeg(
        settings,
        vec![
            "-v".into(),
            "error".into(),
            "-nostdin".into(),
            "-y".into(),
            "-i".into(),
            source.to_string_lossy().into(),
            "-map".into(),
            "0:a:0".into(),
            "-ac".into(),
            "1".into(),
            "-ar".into(),
            "48000".into(),
            "-c:a".into(),
            "pcm_f32le".into(),
            output.to_string_lossy().into(),
        ],
        cancel,
    )?;
    speech::probe_duration(output, settings)
}
pub fn clip(asset_id: Option<String>, start_ms: u64, end_ms: u64) -> AudioClip {
    AudioClip {
        asset_id,
        start_ms,
        end_ms,
        gain: 1.,
        fade_in_ms: 0,
        fade_out_ms: 0,
        crossfade_ms: 0,
    }
}
pub fn atomic_json<T: Serialize>(path: &Path, value: &T) -> Result<(), CommandError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| CommandError::io("Cannot create audio library", e))?;
    }
    let temp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let data =
        serde_json::to_vec_pretty(value).map_err(|e| CommandError::internal(e.to_string()))?;
    let mut file =
        fs::File::create(&temp).map_err(|e| CommandError::io("Cannot save audio metadata", e))?;
    use std::io::Write;
    file.write_all(&data)
        .and_then(|_| file.sync_all())
        .map_err(|e| CommandError::io("Cannot flush audio metadata", e))?;
    fs::rename(&temp, path).map_err(|e| CommandError::io("Cannot publish audio metadata", e))
}
