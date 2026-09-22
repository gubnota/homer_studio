use crate::{AppState, services::{project_store::CommandError, settings, sound_render::{self, SoundRequest}, sound_store::{self, SoundAsset}, sound_workers::{self, WorkerHealth}}};
use std::{fs, path::Path};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn sound_workers(app: AppHandle) -> Result<Vec<WorkerHealth>, CommandError> {
    let settings = settings::load(&app)?;
    Ok(vec![
        sound_workers::health(&settings.sounds.chatterbox_url, "chatterbox_turbo"),
        sound_workers::health(&settings.sounds.sfx_url, "sound_effect"),
    ])
}

#[tauri::command]
pub fn list_sounds(app: AppHandle) -> Result<Vec<SoundAsset>, CommandError> { sound_store::list(&app) }

#[tauri::command]
pub fn generate_sound(app: AppHandle, state: State<'_, AppState>, request: SoundRequest) -> Result<String, CommandError> {
    sound_render::validate(&request)?;
    let settings = settings::load(&app)?;
    let label = format!("{}: {}", request.category.replace('_', " "), request.prompt.chars().take(40).collect::<String>());
    Ok(state.jobs.enqueue("sound", label, move |control, progress| sound_render::render(&app, &settings, &request, &control, &*progress)))
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
