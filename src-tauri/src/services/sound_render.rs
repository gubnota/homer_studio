use serde::{Deserialize, Serialize};
use std::{fs, path::Path, time::Duration};
use tauri::AppHandle;

use super::{jobs::JobControl, process_runner, project_store::CommandError, settings::Settings, sound_store::{self, SoundAsset}, sound_workers};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundRequest {
    pub prompt: String,
    pub category: String,
    pub duration_seconds: f32,
    pub seed: Option<u32>,
}

pub fn validate(request: &SoundRequest) -> Result<(), CommandError> {
    let count = request.prompt.chars().count();
    if count == 0 || count > 500 || request.prompt.trim().is_empty() {
        return Err(CommandError::new("INVALID_SOUND_PROMPT", "Enter a prompt of 1–500 characters."));
    }
    if !request.duration_seconds.is_finite() || !(1.0..=20.0).contains(&request.duration_seconds) {
        return Err(CommandError::new("INVALID_SOUND_DURATION", "Choose a duration between 1 and 20 seconds."));
    }
    match request.category.as_str() {
        "speech" | "sound_effect" => (),
        "vocal_gesture" => {
            if !["[sigh]", "[gasp]", "[cough]", "[laugh]", "[chuckle]", "[groan]"].contains(&request.prompt.trim().to_lowercase().as_str()) {
                return Err(CommandError::new("INVALID_VOCAL_GESTURE", "Use [sigh], [gasp], [cough], [laugh], [chuckle], or [groan]."));
            }
        }
        _ => return Err(CommandError::new("INVALID_SOUND_CATEGORY", "Choose speech, vocal gesture, or sound effect.")),
    }
    Ok(())
}

pub fn render(app: &AppHandle, settings: &Settings, request: &SoundRequest, control: &JobControl, progress: &dyn Fn(u8)) -> Result<(), CommandError> {
    validate(request)?;
    control.boundary()?;
    let (url, engine) = if request.category == "sound_effect" {
        (&settings.sounds.sfx_url, "stable_audio_open")
    } else { (&settings.sounds.chatterbox_url, "chatterbox_turbo") };
    let health = sound_workers::health(url, engine);
    if !health.ready { return Err(CommandError::new("WORKER_UNAVAILABLE", health.message)); }
    if !health.categories.contains(&request.category) { return Err(CommandError::new("WORKER_CAPABILITY", "The selected worker cannot generate this kind of sound.")); }
    progress(10);
    let payload = serde_json::json!({"prompt": request.prompt, "category": request.category, "durationSeconds": request.duration_seconds, "seed": request.seed});
    let wav = sound_workers::generate(url, &payload, control)?;
    control.boundary()?;
    progress(70);
    let id = uuid::Uuid::new_v4().to_string();
    let root = sound_store::root(app)?;
    let stage = root.join("staging").join(&id);
    fs::create_dir_all(&stage).map_err(|error| CommandError::io("Cannot stage sound clip", error))?;
    let result = (|| {
        let original = stage.join("worker.wav");
        let master = stage.join("master.wav");
        let preview = stage.join("preview.m4a");
        fs::write(&original, wav).map_err(|error| CommandError::io("Cannot stage worker audio", error))?;
        let original_ms = super::speech::probe_duration(&original, settings)?;
        if original_ms == 0 || original_ms > 22000 { return Err(CommandError::new("INVALID_WORKER_AUDIO", "Worker audio is empty or longer than 22 seconds.")); }
        convert(settings, &original, &master, "pcm_s24le", control)?;
        convert(settings, &master, &preview, "aac", control)?;
        let duration_ms = super::speech::probe_duration(&master, settings)?;
        if duration_ms == 0 || duration_ms > 22000 { return Err(CommandError::new("INVALID_WORKER_AUDIO", "Converted audio is empty or too long.")); }
        let preview_ms = super::speech::probe_duration(&preview, settings)?;
        if preview_ms == 0 { return Err(CommandError::new("INVALID_WORKER_AUDIO", "Playback audio is empty.")); }
        fs::remove_file(&original).map_err(|error| CommandError::io("Cannot clear temporary audio", error))?;
        control.boundary()?;
        progress(90);
        let asset = SoundAsset {
            id: id.clone(), prompt: request.prompt.trim().into(), category: request.category.clone(),
            provider: engine.into(), model: health.model, requested_duration_seconds: request.duration_seconds,
            duration_ms, seed: request.seed, created_at_ms: sound_store::now_ms(),
            master_path: format!("clips/{id}/master.wav"), preview_path: format!("clips/{id}/preview.m4a"),
        };
        sound_store::publish(app, asset, &stage)
    })();
    if result.is_err() { let _ = fs::remove_dir_all(&stage); }
    result
}

fn convert(settings: &Settings, source: &Path, destination: &Path, codec: &str, control: &JobControl) -> Result<(), CommandError> {
    let ffmpeg = process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
        .ok_or_else(|| CommandError::new("FFMPEG_NOT_FOUND", "Install FFmpeg or set its path in Settings."))?;
    let result = process_runner::run_bounded(&ffmpeg, &[
        "-v".into(), "error".into(), "-y".into(), "-i".into(), source.to_string_lossy().into_owned(),
        "-vn".into(), "-ar".into(), "48000".into(), "-ac".into(), "2".into(),
        "-c:a".into(), codec.into(), destination.to_string_lossy().into_owned(),
    ], Duration::from_secs(60), control.cancelled.clone())?;
    if !result.success { return Err(CommandError::new("AUDIO_CONVERSION_FAILED", result.stderr)); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsupported_gestures_and_lengths() {
        let mut request = SoundRequest { prompt: "heavy breathing".into(), category: "vocal_gesture".into(), duration_seconds: 3.0, seed: None };
        assert!(validate(&request).is_err());
        request.prompt = "[sigh]".into();
        assert!(validate(&request).is_ok());
        request.duration_seconds = 25.0;
        assert!(validate(&request).is_err());
    }
}
