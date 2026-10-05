use super::{
    audio_assets, memo_store, project_store::CommandError, sound_store::now_ms, voice_store,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceProfile {
    pub id: String,
    pub notes: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub reference_memo_ids: Vec<String>,
    pub selected_reference_memo_id: Option<String>,
    pub model_path: Option<String>,
    pub index_path: Option<String>,
    pub engine_options: Value,
}
pub fn list(app: &AppHandle) -> Result<Vec<VoiceProfile>, CommandError> {
    let path = audio_assets::root(app)?.join("profiles.json");
    let mut profiles: Vec<VoiceProfile> = if path.exists() {
        let bytes = std::fs::read(path).map_err(|e| CommandError::io("Cannot read profiles", e))?;
        if bytes.len() > 1024 * 1024 {
            return Err(CommandError::new(
                "INVALID_PROFILE",
                "Profile metadata exceeds its limit.",
            ));
        }
        serde_json::from_slice(&bytes).map_err(|e| CommandError::internal(e.to_string()))?
    } else {
        vec![]
    };
    let voices = voice_store::list(app)?;
    profiles.retain(|p| voices.iter().any(|v| v.id == p.id));
    for voice in voices {
        if !profiles.iter().any(|p| p.id == voice.id) {
            profiles.push(VoiceProfile {
                id: voice.id,
                notes: String::new(),
                created_at_ms: now_ms(),
                updated_at_ms: now_ms(),
                reference_memo_ids: vec![],
                selected_reference_memo_id: None,
                model_path: None,
                index_path: None,
                engine_options: serde_json::json!({}),
            });
        }
    }
    Ok(profiles)
}
pub fn save(app: &AppHandle, mut profile: VoiceProfile) -> Result<VoiceProfile, CommandError> {
    let mut profiles = list(app)?;
    let previous = profiles
        .iter()
        .find(|p| p.id == profile.id)
        .ok_or_else(|| {
            CommandError::new("VOICE_NOT_FOUND", "Choose an existing narrator voice.")
        })?;
    if !profile.engine_options.is_object() || profile.engine_options.to_string().len() > 65536 {
        return Err(CommandError::new("INVALID_PROFILE", "Engine options must be an object under 64 KB."));
    }
    if profile.notes.len() > 65536 || profile.reference_memo_ids.len() > 100 {
        return Err(CommandError::new(
            "INVALID_PROFILE",
            "Profile is too large.",
        ));
    }
    if let Some(id) = &profile.selected_reference_memo_id {
        if !profile.reference_memo_ids.contains(id) {
            return Err(CommandError::new(
                "INVALID_PROFILE",
                "Select a reference in this profile.",
            ));
        }
    }
    for id in &profile.reference_memo_ids {
        let memo = memo_store::load(app, id)?;
        let take = memo_store::selected(&memo)?;
        if memo.deleted || !matches!(take.state.as_str(), "source" | "accepted") {
            return Err(CommandError::new(
                "INVALID_REFERENCE",
                "Use an accepted recording as a reference.",
            ));
        }
    }
    for path in [&profile.model_path, &profile.index_path]
        .into_iter()
        .flatten()
    {
        if !std::path::Path::new(path).is_file() {
            return Err(CommandError::new(
                "MODEL_NOT_FOUND",
                "The selected model or index file is unavailable.",
            ));
        }
    }
    profile.created_at_ms = previous.created_at_ms;
    profile.updated_at_ms = now_ms();
    profiles.retain(|p| p.id != profile.id);
    profiles.push(profile.clone());
    audio_assets::atomic_json(&audio_assets::root(app)?.join("profiles.json"), &profiles)?;
    Ok(profile)
}
