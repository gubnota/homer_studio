use crate::{AppState, services::{project_store::CommandError, settings, sound_render::{self, SoundRequest}, sound_store::{self, SoundAsset}, sound_workers::{self, WorkerHealth}}};
use std::{fs, path::Path};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn sound_workers(app: AppHandle) -> Result<Vec<WorkerHealth>, CommandError> {
    let settings = settings::load(&app)?;
    Ok(vec![sound_workers::health(&settings.sounds.chatterbox_url, "chatterbox_turbo")])
}

#[tauri::command]
pub fn list_sounds(app: AppHandle) -> Result<Vec<SoundAsset>, CommandError> { sound_store::list(&app) }

#[tauri::command]
pub fn delete_sounds(app: AppHandle, ids: Vec<String>) -> Result<Vec<SoundAsset>, CommandError> {
    sound_store::delete_many(&app, &ids)
}

#[tauri::command]
pub fn generate_sound(app: AppHandle, state: State<'_, AppState>, request: SoundRequest) -> Result<String, CommandError> {
    sound_render::validate(&request)?;
    let settings = settings::load(&app)?;
    let label = format!("{}: {}", request.category.replace('_', " "), request.prompt.chars().take(40).collect::<String>());
    Ok(state.jobs.enqueue("sound", label, move |control, progress| sound_render::render(&app, &settings, &request, &control, &*progress)))
}

#[tauri::command]
pub fn convert_voice_clip(app: AppHandle, state: State<'_, AppState>, voice_id: String, source_path: Option<String>, bytes: Option<Vec<u8>>) -> Result<String, CommandError> {
    const MAX_BYTES: usize = 100 * 1024 * 1024;
    const MAX_DURATION_MS: u64 = 120_000;
    const CHUNK_MS: u64 = 15_000;
    let input = match (source_path, bytes) {
        (Some(path), None) => {
            let metadata = fs::metadata(&path).map_err(|error| CommandError::io("Cannot read recording", error))?;
            if !metadata.is_file() || metadata.len() > MAX_BYTES as u64 { return Err(CommandError::new("INVALID_RECORDING", "Choose an audio file smaller than 100 MB.")); }
            fs::read(path).map_err(|error| CommandError::io("Cannot read recording", error))?
        }
        (None, Some(bytes)) => bytes,
        _ => return Err(CommandError::new("INVALID_RECORDING", "Record or import one audio clip.")),
    };
    if input.is_empty() || input.len() > MAX_BYTES { return Err(CommandError::new("INVALID_RECORDING", "Choose an audio clip smaller than 100 MB.")); }
    let settings = settings::load(&app)?;
    let reference = crate::services::voice_store::selected_sample(&app, &voice_id)?
        .ok_or_else(|| CommandError::new("VOICE_HAS_NO_SAMPLE", "Choose a narrator with a voice sample."))?;
    let health = sound_workers::health(&settings.sounds.original_url, "chatterbox_original");
    if !health.ready || !health.categories.iter().any(|category| category == "voice_conversion") {
        return Err(CommandError::new("WORKER_UNAVAILABLE", "Start the Original Chatterbox worker before voice conversion."));
    }
    Ok(state.jobs.enqueue("voice_conversion", "Convert voice recording".into(), move |control, progress| {
        let ffmpeg = crate::services::process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
            .ok_or_else(|| CommandError::new("FFMPEG_NOT_FOUND", "Set FFmpeg in Settings before converting a recording."))?;
        let id = uuid::Uuid::new_v4().to_string();
        let root = sound_store::root(&app)?;
        fs::create_dir_all(&root).map_err(|error| CommandError::io("Cannot create clip library", error))?;
        let work = root.join(format!(".conversion-{id}"));
        fs::create_dir(&work).map_err(|error| CommandError::io("Cannot stage recording", error))?;
        let result = (|| {
            control.boundary()?;
            let source = work.join("source.input");
            let master = work.join("master.wav");
            let preview = work.join("preview.m4a");
            fs::write(&source, input).map_err(|error| CommandError::io("Cannot stage recording", error))?;
            let duration_ms = crate::services::speech::probe_duration(&source, &settings)?;
            if duration_ms < 1000 || duration_ms > MAX_DURATION_MS {
                return Err(CommandError::new("INVALID_RECORDING_DURATION", "Choose a recording between 1 second and 2 minutes."));
            }
            let chunk_count = duration_ms.div_ceil(CHUNK_MS);
            let evenly_sized_chunk_ms = duration_ms.div_ceil(chunk_count);
            let mut entries = String::new();
            let worker = &settings.sounds.original_url;
            for index in 0..chunk_count {
                control.boundary()?;
                let remaining_ms = duration_ms - index * evenly_sized_chunk_ms;
                let chunk_ms = remaining_ms.min(evenly_sized_chunk_ms);
                let wav = work.join(format!("source-{index}.wav"));
                let converted_path = work.join(format!("converted-{index}.wav"));
                let args = vec!["-v".into(), "error".into(), "-y".into(), "-ss".into(), format!("{:.3}", (index * evenly_sized_chunk_ms) as f64 / 1000.0), "-i".into(), source.to_string_lossy().to_string(), "-t".into(), format!("{:.3}", chunk_ms as f64 / 1000.0), "-ac".into(), "1".into(), "-ar".into(), "24000".into(), "-c:a".into(), "pcm_s16le".into(), wav.to_string_lossy().to_string()];
                let normalized = crate::services::process_runner::run_bounded(&ffmpeg, &args, std::time::Duration::from_secs(60), control.cancelled.clone())?;
                if !normalized.success { return Err(CommandError::new("RECORDING_CONVERSION_FAILED", normalized.stderr)); }
                sound_workers::recycle_before_job(&app, worker, "chatterbox_original", &control)?;
                let source_id = sound_workers::upload_reference(worker, &fs::read(&wav).map_err(|error| CommandError::io("Cannot read recording", error))?)?;
                let reference_id = sound_workers::upload_reference(worker, &reference)?;
                let request = serde_json::json!({"prompt":"Convert recorded delivery", "category":"voice_conversion", "durationSeconds":chunk_ms as f64 / 1000.0, "seed":null, "sourceId":source_id, "referenceId":reference_id});
                let output = sound_workers::generate(worker, &request, &control)?;
                fs::write(&converted_path, output).map_err(|error| CommandError::io("Cannot stage converted clip", error))?;
                entries.push_str(&format!("file 'converted-{index}.wav'\n"));
                progress((15 + ((index + 1) * 65 / chunk_count)) as u8);
            }
            control.boundary()?;
            let list = work.join("segments.txt");
            fs::write(&list, entries).map_err(|error| CommandError::io("Cannot list converted segments", error))?;
            let args = vec!["-v".into(), "error".into(), "-y".into(), "-f".into(), "concat".into(), "-safe".into(), "0".into(), "-i".into(), list.to_string_lossy().to_string(), "-c:a".into(), "pcm_s24le".into(), master.to_string_lossy().to_string()];
            let joined = crate::services::process_runner::run_bounded(&ffmpeg, &args, std::time::Duration::from_secs(120), control.cancelled.clone())?;
            if !joined.success { return Err(CommandError::new("AUDIO_CONVERSION_FAILED", joined.stderr)); }
            let args = vec!["-v".into(), "error".into(), "-y".into(), "-i".into(), master.to_string_lossy().to_string(), "-c:a".into(), "aac".into(), "-b:a".into(), "128k".into(), preview.to_string_lossy().to_string()];
            let encoded = crate::services::process_runner::run_bounded(&ffmpeg, &args, std::time::Duration::from_secs(60), control.cancelled.clone())?;
            if !encoded.success { return Err(CommandError::new("AUDIO_CONVERSION_FAILED", encoded.stderr)); }
            let duration = crate::services::speech::probe_duration(&preview, &settings)?;
            let asset = SoundAsset { id: id.clone(), prompt: "Converted voice recording".into(), category: "voice_conversion".into(), provider: "chatterbox_original".into(), model: "Chatterbox Original".into(), requested_duration_seconds: duration_ms as f32 / 1000.0, duration_ms: duration, seed: None, created_at_ms: sound_store::now_ms(), voice_id: Some(voice_id), negative_prompt: None, master_path: format!("clips/{id}/master.wav"), preview_path: format!("clips/{id}/preview.m4a") };
            fs::remove_file(source).ok();
            fs::remove_file(list).ok();
            for index in 0..chunk_count { fs::remove_file(work.join(format!("source-{index}.wav"))).ok(); fs::remove_file(work.join(format!("converted-{index}.wav"))).ok(); }
            control.boundary()?;
            progress(90);
            sound_store::publish(&app, asset, &work)
        })();
        if result.is_err() { let _ = fs::remove_dir_all(&work); }
        result
    }))
}

#[tauri::command]
pub fn sound_audio_url(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<String, CommandError> {
    let asset = sound_store::find(&app, &id)?;
    let path = sound_store::asset_path(&app, &asset, false)?;
    if !path.is_file() { return Err(CommandError::new("SOUND_AUDIO_MISSING", "This clip's playback file is missing.")); }
    let key = uuid::Uuid::new_v4().to_string();
    state.audio_assets.write().map_err(|_| CommandError::internal("audio registry is unavailable"))?.insert(key.clone(), path);
    Ok(format!("audio://localhost/{key}"))
}

#[tauri::command]
pub fn export_sound(app: AppHandle, id: String, destination: String) -> Result<(), CommandError> {
    let asset = sound_store::find(&app, &id)?;
    let target = Path::new(&destination);
    let extension = target.extension().and_then(|value| value.to_str()).unwrap_or("").to_ascii_lowercase();
    let master = match extension.as_str() {
        "wav" => true,
        "m4a" => false,
        _ => return Err(CommandError::new("INVALID_SOUND_EXPORT", "Save as .wav or .m4a.")),
    };
    let source = sound_store::asset_path(&app, &asset, master)?;
    if !source.is_file() { return Err(CommandError::new("SOUND_AUDIO_MISSING", "This clip's audio file is missing.")); }
    let parent = target.parent().ok_or_else(|| CommandError::new("INVALID_SOUND_EXPORT", "Choose a destination folder."))?;
    if !parent.is_dir() { return Err(CommandError::new("INVALID_SOUND_EXPORT", "Destination folder does not exist.")); }
    let temp = parent.join(format!(".homer-sound-{}.tmp", uuid::Uuid::new_v4()));
    fs::copy(source, &temp).map_err(|error| CommandError::io("Cannot copy sound clip", error))?;
    let result = fs::rename(&temp, target).map_err(|error| CommandError::io("Cannot save sound clip", error));
    if result.is_err() { let _ = fs::remove_file(temp); }
    result
}

#[tauri::command]
pub fn export_sounds(app: AppHandle, ids: Vec<String>, destination_dir: String, format: String) -> Result<Vec<String>, CommandError> {
    if ids.is_empty() || ids.len() > 500 { return Err(CommandError::new("INVALID_SOUND_EXPORT", "Select between 1 and 500 clips.")); }
    if !matches!(format.as_str(), "wav" | "m4a") { return Err(CommandError::new("INVALID_SOUND_EXPORT", "Choose WAV or M4A.")); }
    let dir = Path::new(&destination_dir);
    if !dir.is_dir() { return Err(CommandError::new("INVALID_SOUND_EXPORT", "Destination folder does not exist.")); }
    let mut exported = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for id in ids {
        if !seen.insert(id.clone()) { continue; }
        let asset = sound_store::find(&app, &id)?;
        let source = sound_store::asset_path(&app, &asset, format == "wav")?;
        if !source.is_file() { return Err(CommandError::new("SOUND_AUDIO_MISSING", format!("Audio for '{}' is missing; earlier exports are kept.", asset.prompt))); }
        let slug: String = asset.prompt.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else if c.is_whitespace() { '-' } else { '_' }).collect();
        let slug = slug.trim_matches('-').chars().take(48).collect::<String>();
        let base = format!("{}-{}", if slug.is_empty() { "sound" } else { &slug }, &asset.id[..8.min(asset.id.len())]);
        let mut target = dir.join(format!("{base}.{format}"));
        let mut suffix = 2;
        while target.exists() { target = dir.join(format!("{base}-{suffix}.{format}")); suffix += 1; }
        fs::copy(source, &target).map_err(|error| CommandError::io("Cannot export sound clip", error))?;
        exported.push(target.to_string_lossy().into_owned());
    }
    Ok(exported)
}
