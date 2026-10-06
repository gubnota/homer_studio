use super::{audio_assets, project_store::CommandError, settings, wave_store, wave_studio::Source};
#[cfg(feature = "desktop")]
use crate::runtime::Manager;
use crate::runtime::{AppHandle, path::BaseDirectory};
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    #[serde(flatten)]
    pub source: Source,
    pub category: String,
    pub built_in: bool,
    #[serde(default)]
    pub resource: Option<String>,
}
const BUNDLED: [(&str, &str, &str); 11] = [
    ("sitcom_laugh01.m4a", "Sitcom laugh", "Audience"),
    ("intro.m4a", "Intro", "Intro"),
    ("outro.m4a", "Outro", "Outro"),
    ("rewind.webm", "Rewind", "Transitions"),
    ("shoosh_large.webm", "Large whoosh", "Transitions"),
    ("riser.webm", "Riser", "Transitions"),
    ("suspense.webm", "Suspense", "Atmosphere"),
    ("thud.webm", "Thud", "UI"),
    ("surprise_shocked.mp3", "Surprised / shocked", "Audience"),
    ("gong.mp3", "Gong", "Transitions"),
    ("closing_door.mp3", "Closing door", "Atmosphere"),
];
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
    let mut changed = false;
    for (file, name, category) in BUNDLED {
        // Match the immutable resource identity, even if the user renames it.
        if assets.iter().any(|a| {
            a.built_in
                && (a.resource.as_deref() == Some(file)
                    || (file == "sitcom_laugh01.m4a" && a.resource.is_none()))
        }) {
            continue;
        }
        let bundled = app
            .path()
            .resolve(format!("sfx/{file}"), BaseDirectory::Resource)
            .map_err(|e| CommandError::internal(e.to_string()))?;
        let fallback = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../resources/sfx")
            .join(file);
        let source = wave_store::import(
            app,
            if bundled.is_file() {
                &bundled
            } else {
                &fallback
            },
            name,
            &settings::load(app)?,
        )?;
        assets.push(Asset {
            source,
            category: category.into(),
            built_in: true,
            resource: Some(file.into()),
        });
        // Publish each imported resource so a retry never duplicates preceding entries.
        audio_assets::atomic_json(&p, &assets)?;
        changed = true;
    }
    if changed {
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
        resource: None,
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
            "Bundled sounds stay in the library.",
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
