use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Manager};

use super::project_store::CommandError;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundAsset {
    pub id: String,
    pub prompt: String,
    pub category: String,
    pub provider: String,
    pub model: String,
    pub requested_duration_seconds: f32,
    pub duration_ms: u64,
    pub seed: Option<u32>,
    pub created_at_ms: u64,
    #[serde(default)]
    pub voice_id: Option<String>,
    #[serde(default)]
    pub negative_prompt: Option<String>,
    pub master_path: String,
    pub preview_path: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest { schema_version: u32, assets: Vec<SoundAsset> }

pub fn root(app: &AppHandle) -> Result<PathBuf, CommandError> {
    app.path().app_data_dir().map(|base| base.join("sound-assets"))
        .map_err(|error| CommandError::internal(format!("Cannot locate sound library: {error}")))
}

pub fn list(app: &AppHandle) -> Result<Vec<SoundAsset>, CommandError> {
    let path = root(app)?.join("manifest.json");
    if !path.exists() { return Ok(Vec::new()); }
    let bytes = fs::read(path).map_err(|error| CommandError::io("Cannot read sound library", error))?;
    let manifest: Manifest = serde_json::from_slice(&bytes)
        .map_err(|_| CommandError::new("INVALID_SOUND_LIBRARY", "Sound library metadata is invalid."))?;
    if manifest.schema_version != 1 { return Err(CommandError::new("UNSUPPORTED_SOUND_LIBRARY", "Sound library version is unsupported.")); }
    for asset in &manifest.assets {
        asset_path(app, asset, false)?;
        asset_path(app, asset, true)?;
    }
    Ok(manifest.assets)
}

pub fn find(app: &AppHandle, id: &str) -> Result<SoundAsset, CommandError> {
    uuid::Uuid::parse_str(id).map_err(|_| CommandError::new("INVALID_SOUND_ID", "Invalid sound clip ID."))?;
    list(app)?.into_iter().find(|asset| asset.id == id)
        .ok_or_else(|| CommandError::new("SOUND_NOT_FOUND", "Sound clip was not found."))
}

pub fn asset_path(app: &AppHandle, asset: &SoundAsset, master: bool) -> Result<PathBuf, CommandError> {
    uuid::Uuid::parse_str(&asset.id).map_err(|_| CommandError::new("INVALID_SOUND_LIBRARY", "Sound library has an invalid clip ID."))?;
    let expected = if master { "master.wav" } else { "preview.m4a" };
    let relative = if master { &asset.master_path } else { &asset.preview_path };
    if relative != &format!("clips/{}/{}", asset.id, expected) {
        return Err(CommandError::new("INVALID_SOUND_LIBRARY", "Sound library contains an unsafe audio path."));
    }
    Ok(root(app)?.join(relative))
}

pub fn publish(app: &AppHandle, asset: SoundAsset, staged: &Path) -> Result<(), CommandError> {
    let base = root(app)?;
    let target = base.join("clips").join(&asset.id);
    if target.exists() || list(app)?.iter().any(|existing| existing.id == asset.id) {
        return Err(CommandError::new("DUPLICATE_SOUND_ID", "Sound clip ID already exists."));
    }
    fs::create_dir_all(base.join("clips")).map_err(|error| CommandError::io("Cannot create sound library", error))?;
    fs::rename(staged, &target).map_err(|error| CommandError::io("Cannot publish sound files", error))?;
    let result = (|| {
        let mut assets = list(app)?;
        assets.push(asset);
        let data = serde_json::to_vec_pretty(&Manifest { schema_version: 1, assets })
            .map_err(|_| CommandError::internal("Cannot serialize sound library"))?;
        let temp = base.join(format!("manifest-{}.tmp", uuid::Uuid::new_v4()));
        fs::write(&temp, data).map_err(|error| CommandError::io("Cannot write sound library", error))?;
        fs::rename(&temp, base.join("manifest.json")).map_err(|error| CommandError::io("Cannot publish sound library", error))
    })();
    if result.is_err() { let _ = fs::remove_dir_all(target); }
    result
}

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
