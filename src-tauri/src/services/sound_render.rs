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
    #[serde(default)]
    pub negative_prompt: Option<String>,
}

pub fn validate(request: &SoundRequest) -> Result<(), CommandError> {
    let count = request.prompt.chars().count();
    if count == 0 || count > 500 || request.prompt.trim().is_empty() {
        return Err(CommandError::new("INVALID_SOUND_PROMPT", "Enter a prompt of 1–500 characters."));
    }
    if !request.duration_seconds.is_finite() || !(1.0..=120.0).contains(&request.duration_seconds) {
        return Err(CommandError::new("INVALID_SOUND_DURATION", "Choose a maximum duration from 1 to 120 seconds."));
    }
    match request.category.as_str() {
        "speech" => (),
        "vocal_gesture" => {
            if !["[clear throat]", "[sigh]", "[shush]", "[cough]", "[groan]", "[sniff]", "[gasp]", "[chuckle]", "[laugh]"].contains(&request.prompt.trim().to_lowercase().as_str()) {
                return Err(CommandError::new("INVALID_VOCAL_GESTURE", "Use one documented Chatterbox Turbo gesture tag."));
            }
        }
        _ => return Err(CommandError::new("INVALID_SOUND_CATEGORY", "Only speech and vocal gestures are supported.")),
    }
    if request.negative_prompt.is_some() {
        return Err(CommandError::new("INVALID_NEGATIVE_PROMPT", "Unwanted-sounds prompts are no longer supported."));
    }
    Ok(())
}

pub fn render(app: &AppHandle, settings: &Settings, request: &SoundRequest, control: &JobControl, progress: &dyn Fn(u8)) -> Result<(), CommandError> {
    validate(request)?;
    control.boundary()?;
    let url = &settings.sounds.chatterbox_url;
    let engine = "chatterbox_turbo";
    let health = sound_workers::health(url, engine);
    if !health.ready { return Err(CommandError::new("WORKER_UNAVAILABLE", health.message)); }
    if !health.categories.contains(&request.category) { return Err(CommandError::new("WORKER_CAPABILITY", "The selected worker cannot generate this kind of sound.")); }
    progress(10);
    let reference = super::voice_store::selected_sample(app, &settings.speech.voice_id)?;
    let id = uuid::Uuid::new_v4().to_string();
    let root = sound_store::root(app)?;
    let stage = root.join("staging").join(&id);
    fs::create_dir_all(&stage).map_err(|error| CommandError::io("Cannot stage sound clip", error))?;
    let result = (|| {
        let master = stage.join("master.wav");
        let preview = stage.join("preview.m4a");
        let prompts = if request.category == "speech" { split_speech(&request.prompt) } else { vec![request.prompt.clone()] };
        let minimum_count = (request.duration_seconds / 20.0).ceil() as usize;
        let count = if request.category == "speech" { prompts.len() } else { minimum_count };
        let mut list_entries = String::new();
        for index in 0..count {
            control.boundary()?;
            sound_workers::recycle_before_job(app, url, engine, control)?;
            let segment_seconds = (request.duration_seconds / count as f32).min(20.0).max(1.0);
            let reference_id = reference.as_ref().map(|bytes| sound_workers::upload_reference(url, bytes)).transpose()?;
            let prompt = if request.category == "speech" { &prompts[index] } else { &request.prompt };
            let payload = serde_json::json!({"prompt": prompt, "category": request.category, "durationSeconds": segment_seconds, "seed": request.seed.map(|seed| seed.wrapping_add(index as u32) & 0x7fff_ffff), "referenceId": reference_id, "negativePrompt": request.negative_prompt});
            let wav = sound_workers::generate(url, &payload, control)?;
            let original = stage.join(format!("worker-{index}.wav"));
            let normalized = stage.join(format!("segment-{index}.wav"));
            fs::write(&original, wav).map_err(|error| CommandError::io("Cannot stage worker audio", error))?;
            let original_ms = super::speech::probe_duration(&original, settings)?;
            if original_ms == 0 || original_ms > 22000 { return Err(CommandError::new("INVALID_WORKER_AUDIO", "Worker audio is empty or longer than 22 seconds.")); }
            convert(settings, &original, &normalized, "pcm_s24le", control)?;
            list_entries.push_str(&format!("file 'segment-{index}.wav'\n"));
            progress((10 + (index + 1) * 65 / count) as u8);
        }
        if count == 1 {
            fs::rename(stage.join("segment-0.wav"), &master).map_err(|error| CommandError::io("Cannot prepare sound clip", error))?;
        } else {
            let list = stage.join("segments.txt");
            fs::write(&list, list_entries).map_err(|error| CommandError::io("Cannot stage sound segments", error))?;
            let ffmpeg = process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
                .ok_or_else(|| CommandError::new("FFMPEG_NOT_FOUND", "Install FFmpeg or set its path in Settings."))?;
            let args = vec!["-v".into(), "error".into(), "-y".into(), "-f".into(), "concat".into(), "-safe".into(), "0".into(), "-i".into(), list.to_string_lossy().to_string(), "-c:a".into(), "pcm_s24le".into(), master.to_string_lossy().to_string()];
            let joined = process_runner::run_bounded(&ffmpeg, &args, Duration::from_secs(120), control.cancelled.clone())?;
            if !joined.success { return Err(CommandError::new("AUDIO_CONVERSION_FAILED", joined.stderr)); }
            fs::remove_file(list).ok();
        }
        for index in 0..count { fs::remove_file(stage.join(format!("worker-{index}.wav"))).ok(); fs::remove_file(stage.join(format!("segment-{index}.wav"))).ok(); }

        convert(settings, &master, &preview, "aac", control)?;
        let duration_ms = super::speech::probe_duration(&master, settings)?;
        if duration_ms == 0 || duration_ms > (request.duration_seconds * 1100.0) as u64 + 2000 { return Err(CommandError::new("INVALID_WORKER_AUDIO", "Converted audio is empty or too long.")); }
        let preview_ms = super::speech::probe_duration(&preview, settings)?;
        if preview_ms == 0 { return Err(CommandError::new("INVALID_WORKER_AUDIO", "Playback audio is empty.")); }
        control.boundary()?;
        progress(90);
        let asset = SoundAsset {
            id: id.clone(), prompt: request.prompt.trim().into(), category: request.category.clone(),
            provider: health.engine, model: health.model, requested_duration_seconds: request.duration_seconds,
            duration_ms, seed: request.seed, created_at_ms: sound_store::now_ms(),
            voice_id: Some(settings.speech.voice_id.clone()),
            negative_prompt: request.negative_prompt.clone(),
            master_path: format!("clips/{id}/master.wav"), preview_path: format!("clips/{id}/preview.m4a"),
        };
        sound_store::publish(app, asset, &stage)
    })();
    if result.is_err() { let _ = fs::remove_dir_all(&stage); }
    result
}

fn split_speech(prompt: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    for word in prompt.split_whitespace() {
        let inside_tag = current.rfind('[').is_some_and(|open| current[open..].find(']').is_none());
        if !inside_tag && !current.is_empty() && current.chars().count() + 1 + word.chars().count() > 80 {
            parts.push(std::mem::take(&mut current));
        }
        if !current.is_empty() { current.push(' '); }
        current.push_str(word);
    }
    if !current.is_empty() { parts.push(current); }
    parts
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
        let mut request = SoundRequest { prompt: "heavy breathing".into(), category: "vocal_gesture".into(), duration_seconds: 3.0, seed: None, negative_prompt: None };
        assert!(validate(&request).is_err());
        request.prompt = "[sigh]".into();
        assert!(validate(&request).is_ok());
        request.duration_seconds = 25.0;
        assert!(validate(&request).is_ok());
        request.category = "sound_effect".into();
        assert!(validate(&request).is_err());
        request.duration_seconds = 121.0;
        assert!(validate(&request).is_err());
    }

    #[test]
    fn speech_chunks_preserve_words_and_gesture_tags() {
        let prompt = format!("{} [clear throat] {}", "Tomorrow returns. ".repeat(7), "A new day arrives. ".repeat(6));
        let parts = split_speech(&prompt);
        assert!(parts.len() > 1);
        assert_eq!(parts.join(" "), prompt.split_whitespace().collect::<Vec<_>>().join(" "));
        assert!(parts.iter().any(|part| part.contains("[clear throat]")));
    }
}
