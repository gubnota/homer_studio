use std::{fs, path::{Path, PathBuf}, process::{Command, Stdio}, time::Duration};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use super::{jobs::JobControl, project_store::CommandError};

const TURBO_REV: &str = "0fa248b9da33387654e294782997f0b31627eb84";
const ORIGINAL_REV: &str = "e2d6902dd4c1301892935d0a0277325551e8060e";
const TURBO: &[&str] = &["t3_turbo_v1.safetensors", "s3gen_meanflow.safetensors", "ve.safetensors", "conds.pt", "added_tokens.json", "merges.txt", "special_tokens_map.json", "tokenizer_config.json", "vocab.json"];
const ORIGINAL: &[&str] = &["ve.safetensors", "t3_cfg.safetensors", "s3gen.safetensors", "tokenizer.json", "conds.pt"];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus { pub model: String, pub path: String, pub installed: bool, pub bytes_on_disk: u64, pub downloaded_bytes: u64, pub total_bytes: Option<u64>, pub missing_files: Vec<String> }

#[derive(Serialize, Deserialize)]
struct ManifestEntry { file: String, bytes: u64 }

fn spec(model: &str) -> Result<(&'static str, &'static str, &'static [&'static str]), CommandError> {
    match model {
        "turbo" => Ok(("chatterbox-turbo", TURBO_REV, TURBO)),
        "original" => Ok(("chatterbox", ORIGINAL_REV, ORIGINAL)),
        _ => Err(CommandError::new("INVALID_MODEL", "Choose Turbo or Original Chatterbox.")),
    }
}

fn model_dir(app: &AppHandle, model: &str) -> Result<PathBuf, CommandError> {
    spec(model)?;
    let app_dir = app.path().app_data_dir().map_err(|error| CommandError::new("APP_DATA_UNAVAILABLE", error.to_string()))?;
    Ok(app_dir.join("models").join(if model == "turbo" { "chatterbox-turbo" } else { "chatterbox-original" }))
}

pub fn status(app: &AppHandle, model: &str) -> Result<ModelStatus, CommandError> {
    let (_, _, files) = spec(model)?;
    let dir = model_dir(app, model)?;
    let missing_files = files.iter().filter(|file| fs::metadata(dir.join(file)).map(|meta| !meta.is_file() || meta.len() == 0).unwrap_or(true)).map(|file| (*file).to_string()).collect::<Vec<_>>();
    let bytes_on_disk = fs::read_dir(&dir).ok().into_iter().flatten().filter_map(Result::ok).filter_map(|entry| entry.metadata().ok()).filter(|meta| meta.is_file()).map(|meta| meta.len()).sum();
    let manifest: Vec<ManifestEntry> = fs::read(dir.join(".download-manifest.json")).ok().and_then(|data| serde_json::from_slice(&data).ok()).unwrap_or_default();
    let downloaded_bytes = manifest.iter().filter(|entry| files.contains(&entry.file.as_str())).map(|entry| {
        let complete = fs::metadata(dir.join(&entry.file)).map(|meta| meta.len()).unwrap_or(0);
        let partial = fs::metadata(dir.join(format!("{}.part", entry.file))).map(|meta| meta.len()).unwrap_or(0);
        complete.max(partial).min(entry.bytes)
    }).sum();
    let total_bytes = (manifest.len() == files.len()).then(|| manifest.iter().map(|entry| entry.bytes).sum());
    Ok(ModelStatus { model: model.into(), path: dir.to_string_lossy().to_string(), installed: missing_files.is_empty(), bytes_on_disk, downloaded_bytes, total_bytes, missing_files })
}

fn curl_headers(url: &str) -> Result<(u64, Option<String>), CommandError> {
    let output = Command::new("/usr/bin/curl").args(["--fail", "--silent", "--show-error", "--location", "--head", "--max-time", "45", url]).output()
        .map_err(|error| CommandError::io("Cannot inspect model file", error))?;
    if !output.status.success() { return Err(CommandError::new("MODEL_DOWNLOAD_FAILED", "Cannot reach the pinned model checkpoint. Check the connection and retry.")); }
    let headers = String::from_utf8_lossy(&output.stdout);
    let mut length = None;
    let mut sha = None;
    for line in headers.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(value) = lower.strip_prefix("x-linked-size:").or_else(|| lower.strip_prefix("content-length:")) {
            if let Ok(value) = value.trim().parse::<u64>() { if value > 0 { length = Some(value); } }
        }
        if let Some(value) = lower.strip_prefix("x-linked-etag:") {
            let value = value.trim().trim_matches('"');
            if value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()) { sha = Some(value.to_string()); }
        }
    }
    Ok((length.ok_or_else(|| CommandError::new("MODEL_SIZE_UNKNOWN", "The model server did not report a file size; download was stopped."))?, sha))
}

fn file_sha256(path: &Path) -> Result<String, CommandError> {
    let output = Command::new("/usr/bin/shasum").arg("-a").arg("256").arg(path).output()
        .map_err(|error| CommandError::io("Cannot verify checkpoint", error))?;
    if !output.status.success() { return Err(CommandError::new("MODEL_VERIFY_FAILED", "Cannot verify the downloaded checkpoint.")); }
    Ok(String::from_utf8_lossy(&output.stdout).split_whitespace().next().unwrap_or("").to_string())
}

pub fn install(app: &AppHandle, model: &str, control: &JobControl, progress: &dyn Fn(u8)) -> Result<(), CommandError> {
    let (repo, revision, files) = spec(model)?;
    let dir = model_dir(app, model)?;
    fs::create_dir_all(&dir).map_err(|error| CommandError::io("Cannot create model folder", error))?;
    let mut manifest = Vec::new();
    for file in files {
        control.boundary()?;
        let url = format!("https://huggingface.co/ResembleAI/{repo}/resolve/{revision}/{file}");
        let (length, sha) = curl_headers(&url)?;
        manifest.push((file, url, length, sha));
    }
    let entries = manifest.iter().map(|(file, _, bytes, _)| ManifestEntry { file: (*file).to_string(), bytes: *bytes }).collect::<Vec<_>>();
    fs::write(dir.join(".download-manifest.json"), serde_json::to_vec(&entries).map_err(|error| CommandError::new("MODEL_MANIFEST_FAILED", error.to_string()))?)
        .map_err(|error| CommandError::io("Cannot save download details", error))?;
    let total: u64 = manifest.iter().map(|item| item.2).sum();
    let mut completed = 0u64;
    for (file, url, length, sha) in manifest {
        control.boundary()?;
        let target = dir.join(file);
        if fs::metadata(&target).map(|meta| meta.len() == length).unwrap_or(false)
            && sha.as_ref().map(|expected| file_sha256(&target).map(|actual| actual == *expected)).transpose()?.unwrap_or(true) {
            completed += length;
            progress(((completed.saturating_mul(95) / total.max(1)) as u8).min(95));
            continue;
        }
        let partial = dir.join(format!("{file}.part"));
        if fs::metadata(&partial).map(|meta| meta.len() > length).unwrap_or(false) { fs::remove_file(&partial).map_err(|error| CommandError::io("Cannot reset partial download", error))?; }
        let mut child = Command::new("/usr/bin/curl")
            .args(["--fail", "--location", "--silent", "--show-error", "--retry", "2", "--continue-at", "-", "--output"])
            .arg(&partial).arg(&url).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
            .map_err(|error| CommandError::io("Cannot start model download", error))?;
        loop {
            if control.cancelled.load(std::sync::atomic::Ordering::SeqCst) { let _ = child.kill(); let _ = child.wait(); return Err(CommandError::new("JOB_CANCELLED", "Model download cancelled. Retry will resume saved partial files.")); }
            if let Some(exit) = child.try_wait().map_err(|error| CommandError::io("Cannot inspect model download", error))? {
                if !exit.success() { return Err(CommandError::new("MODEL_DOWNLOAD_FAILED", format!("Download failed for {file}. Retry to resume."))); }
                break;
            }
            let partial_size = fs::metadata(&partial).map(|meta| meta.len()).unwrap_or(0).min(length);
            progress(((completed.saturating_add(partial_size).saturating_mul(95) / total.max(1)) as u8).min(95));
            std::thread::sleep(Duration::from_millis(300));
        }
        let actual = fs::metadata(&partial).map_err(|error| CommandError::io("Cannot verify download", error))?.len();
        if actual != length { return Err(CommandError::new("MODEL_VERIFY_FAILED", format!("{file} has {actual} bytes; expected {length}. Retry to resume."))); }
        if let Some(expected) = sha { if file_sha256(&partial)? != expected { let _ = fs::remove_file(&partial); return Err(CommandError::new("MODEL_VERIFY_FAILED", format!("{file} failed checksum verification. Retry the download."))); } }
        fs::rename(&partial, &target).map_err(|error| CommandError::io("Cannot save checkpoint", error))?;
        completed += length;
        progress(((completed.saturating_mul(95) / total.max(1)) as u8).min(95));
    }
    if !status(app, model)?.installed { return Err(CommandError::new("MODEL_INCOMPLETE", "Required checkpoint files are missing.")); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_allowlisted_models_and_filenames_are_available() {
        assert!(spec("../bad").is_err());
        for model in ["turbo", "original"] {
            let (_, revision, files) = spec(model).unwrap();
            assert_eq!(revision.len(), 40);
            assert!(files.iter().all(|file| !file.contains('/') && !file.contains("..")));
        }
    }
}
