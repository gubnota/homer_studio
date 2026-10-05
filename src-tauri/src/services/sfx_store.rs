use super::{audio_assets, project_store::CommandError, settings, wave_store, wave_studio::Source};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, path::BaseDirectory};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    #[serde(flatten)]
    pub source: Source,
    pub category: String,
    pub built_in: bool,
}
pub const CATEGORIES: [&str; 7] = [
    "Audience",
    "Transitions",
    "Atmosphere",
    "UI",
    "Intro",
    "Outro",
    "Custom",
];
fn path(app: &AppHandle) -> Result<std::path::PathBuf, CommandError> {
    Ok(wave_store::root(app)?.join("sfx.json"))
}
pub fn list(app: &AppHandle) -> Result<Vec<Asset>, CommandError> {
    let p = path(app)?;
    let mut assets: Vec<Asset> = if p.exists() {
        serde_json::from_slice(
            &std::fs::read(&p).map_err(|e| CommandError::io("Cannot read SFX library", e))?,
        )
        .map_err(|e| CommandError::internal(e.to_string()))?
    } else {
        vec![]
    };
    if !assets.iter().any(|a| a.built_in) {
        let bundled = app
            .path()
            .resolve("sfx/sitcom_laugh01.m4a", BaseDirectory::Resource)
            .map_err(|e| CommandError::internal(e.to_string()))?;
        let fallback = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources/sfx/sitcom_laugh01.m4a");
        let source = wave_store::import(
            app,
            if bundled.is_file() {
                &bundled
            } else {
                &fallback
            },
            "sitcom_laugh01",
            &settings::load(app)?,
        )?;
        assets.push(Asset {
            source,
            category: "Audience".into(),
            built_in: true,
        });
        audio_assets::atomic_json(&p, &assets)?;
    }
    Ok(assets)
}
pub fn add(app: &AppHandle, source: Source, category: &str) -> Result<Vec<Asset>, CommandError> {
    if !CATEGORIES.contains(&category) {
        return Err(CommandError::new(
            "INVALID_SFX_CATEGORY",
            "Choose a sound category.",
        ));
    }
    let mut all = list(app)?;
    all.push(Asset {
        source,
        category: category.into(),
        built_in: false,
    });
    audio_assets::atomic_json(&path(app)?, &all)?;
    Ok(all)
}
pub fn update(
    app: &AppHandle,
    id: &str,
    name: &str,
    category: &str,
    delete: bool,
) -> Result<Vec<Asset>, CommandError> {
    let mut all = list(app)?;
    let a = all
        .iter_mut()
        .find(|a| a.source.id == id)
        .ok_or_else(|| CommandError::new("SFX_NOT_FOUND", "Sound no longer exists."))?;
    if delete && a.built_in {
        return Err(CommandError::new(
            "BUILTIN_SFX",
            "The bundled laugh stays in the library.",
        ));
    }
    if !CATEGORIES.contains(&category) || name.trim().is_empty() || name.len() > 320 {
        return Err(CommandError::new(
            "INVALID_SFX",
            "Enter a name and category.",
        ));
    }
    a.source.name = name.trim().into();
    a.category = category.into();
    if delete {
        all.retain(|a| a.source.id != id);
    }
    audio_assets::atomic_json(&path(app)?, &all)?;
    Ok(all)
}
