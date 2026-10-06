use super::{
    audio_assets, process_runner, project_store::CommandError, settings::Settings, speech,
    wave_store, wave_studio::Video,
};
use crate::runtime::AppHandle;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
pub fn path(app: &AppHandle, id: &str) -> Result<PathBuf, CommandError> {
    audio_assets::id(id)?;
    let root = wave_store::root(app)?;
    let p = root.join("videos").join(format!("{id}.mp4"));
    if p.exists()
        && !p
            .canonicalize()
            .map_err(|e| CommandError::io("Cannot resolve video", e))?
            .starts_with(
                root.canonicalize()
                    .map_err(|e| CommandError::io("Cannot resolve library", e))?,
            )
    {
        return Err(CommandError::new("UNSAFE_VIDEO", "Video leaves library."));
    }
    Ok(p)
}
/// Original paths come only from app-owned metadata created by an explicit import.
pub fn original_path(app: &AppHandle, id: &str) -> Result<PathBuf, CommandError> {
    audio_assets::id(id)?;
    let bytes = fs::read(
        wave_store::root(app)?
            .join("videos")
            .join(format!("{id}.json")),
    )
    .map_err(|e| CommandError::io("Cannot read video link", e))?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| CommandError::internal(e.to_string()))?;
    let original = value["originalPath"].as_str().map(PathBuf::from);
    let input = original.filter(|p| p.is_file()).unwrap_or(path(app, id)?);
    if !input.is_file() {
        return Err(CommandError::new(
            "VIDEO_NOT_FOUND",
            "Locate the linked video before exporting.",
        ));
    }
    Ok(input)
}
/// Bundled originals must survive disposal of the archive extraction directory.
pub fn retain_original(app: &AppHandle, id: &str, input: &Path) -> Result<(), CommandError> {
    let folder = wave_store::root(app)?;
    let relative = PathBuf::from(format!("videos/{id}.original"));
    wave_store::copy_media(&folder, input, &relative)?;
    let metadata = folder.join("videos").join(format!("{id}.json"));
    let mut value: serde_json::Value = serde_json::from_slice(
        &fs::read(&metadata).map_err(|e| CommandError::io("Cannot read video", e))?,
    )
    .map_err(|e| CommandError::internal(e.to_string()))?;
    value["originalPath"] = folder.join(relative).to_string_lossy().into_owned().into();
    audio_assets::atomic_json(&metadata, &value)
}
pub fn missing(app: &AppHandle, video: &Video) -> Result<Video, CommandError> {
    let mut video = video.clone();
    video.id = uuid::Uuid::new_v4().to_string();
    let mut value =
        serde_json::to_value(&video).map_err(|e| CommandError::internal(e.to_string()))?;
    value["missing"] = true.into();
    audio_assets::atomic_json(
        &wave_store::root(app)?
            .join("videos")
            .join(format!("{}.json", video.id)),
        &value,
    )?;
    Ok(video)
}
pub fn has_video(p: &Path, settings: &Settings) -> Result<bool, CommandError> {
    if !p.is_file() {
        return Err(CommandError::new(
            "MEDIA_NOT_FOUND",
            "Choose an existing file.",
        ));
    }
    let tool = process_runner::resolve_executable("ffprobe", settings.ffprobe_path.as_deref())
        .ok_or_else(|| CommandError::new("FFPROBE_NOT_FOUND", "Select FFprobe in Settings."))?;
    let result = process_runner::run_bounded(
        &tool,
        &[
            "-v".into(),
            "error".into(),
            "-select_streams".into(),
            "v:0".into(),
            "-show_entries".into(),
            "stream=codec_type:stream_disposition=attached_pic".into(),
            "-of".into(),
            "json".into(),
            p.to_string_lossy().into(),
        ],
        Duration::from_secs(30),
        Default::default(),
    )?;
    if !result.success {
        return Err(CommandError::new("MEDIA_PROBE_FAILED", result.stderr));
    }
    let data: serde_json::Value =
        serde_json::from_str(&result.stdout).map_err(|e| CommandError::internal(e.to_string()))?;
    Ok(data["streams"].as_array().is_some_and(|s| {
        s.iter()
            .any(|v| v["codec_type"] == "video" && v["disposition"]["attached_pic"] != 1)
    }))
}
pub fn import(app: &AppHandle, input: &Path, settings: &Settings) -> Result<Video, CommandError> {
    import_controlled(app, input, settings, Default::default(), Arc::new(|_| {}))
}
pub fn import_controlled(
    app: &AppHandle,
    input: &Path,
    settings: &Settings,
    cancel: Arc<AtomicBool>,
    report: Arc<dyn Fn(u8) + Send + Sync>,
) -> Result<Video, CommandError> {
    if !has_video(input, settings)? {
        return Err(CommandError::new(
            "NO_VIDEO",
            "This file has no video track.",
        ));
    }
    let probe = process_runner::resolve_executable("ffprobe", settings.ffprobe_path.as_deref())
        .ok_or_else(|| CommandError::new("FFPROBE_NOT_FOUND", "Set FFprobe in Settings."))?;
    let measured = process_runner::run_bounded(
        &probe,
        &[
            "-v".into(),
            "error".into(),
            "-select_streams".into(),
            "v:0".into(),
            "-show_entries".into(),
            "stream=duration".into(),
            "-of".into(),
            "json".into(),
            input.to_string_lossy().into(),
        ],
        Duration::from_secs(30),
        cancel.clone(),
    )?;
    let info: serde_json::Value = serde_json::from_str(&measured.stdout).unwrap_or_default();
    let duration = match info["streams"][0]["duration"]
        .as_str()
        .and_then(|d| d.parse::<f64>().ok())
        .filter(|d| d.is_finite() && *d > 0.)
    {
        Some(seconds) => seconds * 1000.,
        None => speech::probe_duration(input, settings)? as f64,
    };
    if duration <= 0. || duration > 86_400_000. {
        return Err(CommandError::new(
            "INVALID_VIDEO_DURATION",
            "Choose video shorter than 24 hours.",
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let output = path(app, &id)?;
    fs::create_dir_all(output.parent().unwrap())
        .map_err(|e| CommandError::io("Cannot create video library", e))?;
    let result = (|| {
        let tool = process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
            .ok_or_else(|| CommandError::new("FFMPEG_NOT_FOUND", "Set FFmpeg in Settings."))?;
        let probe = process_runner::resolve_executable("ffprobe", settings.ffprobe_path.as_deref())
            .ok_or_else(|| CommandError::new("FFPROBE_NOT_FOUND", "Set FFprobe in Settings."))?;
        let info = process_runner::run_bounded(
            &probe,
            &[
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=codec_name,pix_fmt",
                "-of",
                "json",
            ]
            .into_iter()
            .map(String::from)
            .chain([input.to_string_lossy().into_owned()])
            .collect::<Vec<_>>(),
            Duration::from_secs(30),
            cancel.clone(),
        )?;
        let data: serde_json::Value = serde_json::from_str(&info.stdout).unwrap_or_default();
        let stream = &data["streams"][0];
        let compatible = stream["codec_name"] == "h264" && stream["pix_fmt"] == "yuv420p";
        let run = |codec: &[&str]| -> Result<bool, CommandError> {
            let args = ["-v", "error", "-nostdin", "-y", "-progress", "pipe:1", "-i"]
                .into_iter()
                .map(String::from)
                .chain([input.to_string_lossy().into_owned()])
                .chain(["-map", "0:v:0", "-an"].into_iter().map(String::from))
                .chain(codec.iter().map(|s| s.to_string()))
                .chain(["-movflags", "+faststart"].into_iter().map(String::from))
                .chain([output.to_string_lossy().into_owned()])
                .collect::<Vec<_>>();
            Ok(process_runner::run_observed(
                &tool,
                &args,
                Duration::from_secs(7200),
                cancel.clone(),
                Some((duration as u64, report.clone())),
            )?
            .success)
        };
        let mut done = compatible && run(&["-c:v", "copy"])?;
        if !done {
            #[cfg(target_os = "macos")]
            {
                done = run(&[
                    "-vf",
                    "scale=w='min(1280,iw)':h='min(720,ih)':force_original_aspect_ratio=decrease:force_divisible_by=2",
                    "-c:v",
                    "h264_videotoolbox",
                    "-b:v",
                    "2500k",
                    "-pix_fmt",
                    "yuv420p",
                ])?;
            }
            if !done {
                done = run(&[
                    "-vf",
                    "scale=w='min(1280,iw)':h='min(720,ih)':force_original_aspect_ratio=decrease:force_divisible_by=2",
                    "-c:v",
                    "libx264",
                    "-preset",
                    "veryfast",
                    "-crf",
                    "23",
                    "-pix_fmt",
                    "yuv420p",
                ])?;
            }
        }
        if !done {
            return Err(CommandError::new(
                "VIDEO_IMPORT_FAILED",
                "Cannot prepare this video reference.",
            ));
        }
        if cancel.load(Ordering::SeqCst) {
            return Err(CommandError::new(
                "JOB_CANCELLED",
                "Video import cancelled.",
            ));
        }
        let video = Video {
            id: id.clone(),
            name: input
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Video reference")
                .chars()
                .take(160)
                .collect(),
            duration_ms: speech::probe_duration(&output, settings)? as f64,
            start_ms: 0.,
        };
        let mut metadata =
            serde_json::to_value(&video).map_err(|e| CommandError::internal(e.to_string()))?;
        metadata["originalPath"] = input
            .canonicalize()
            .map_err(|e| CommandError::io("Cannot link video", e))?
            .to_string_lossy()
            .into_owned()
            .into();
        audio_assets::atomic_json(
            &wave_store::root(app)?
                .join("videos")
                .join(format!("{id}.json")),
            &metadata,
        )?;
        Ok(video)
    })();
    if result.is_err() {
        let _ = fs::remove_file(output);
    }
    result
}
pub fn validate(app: &AppHandle, video: &Video) -> Result<(), CommandError> {
    let data = fs::read(
        wave_store::root(app)?
            .join("videos")
            .join(format!("{}.json", video.id)),
    )
    .map_err(|e| CommandError::io("Cannot read video reference", e))?;
    let metadata: serde_json::Value =
        serde_json::from_slice(&data).map_err(|e| CommandError::internal(e.to_string()))?;
    let owned: Video =
        serde_json::from_slice(&data).map_err(|e| CommandError::internal(e.to_string()))?;
    if owned.duration_ms != video.duration_ms
        || (metadata["missing"] != true && !path(app, &video.id)?.is_file())
    {
        return Err(CommandError::new(
            "INVALID_VIDEO",
            "Video reference no longer matches its source.",
        ));
    }
    Ok(())
}

/// Replace the soundtrack while keeping the selected video's exact duration.
pub fn export_mix(
    app: &AppHandle,
    video: &Video,
    audio: &Path,
    output: &Path,
    settings: &Settings,
    cancel: Arc<AtomicBool>,
    report: Arc<dyn Fn(u8) + Send + Sync>,
) -> Result<(), CommandError> {
    let input = original_path(app, &video.id)?;
    let tool = process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
        .ok_or_else(|| CommandError::new("FFMPEG_NOT_FOUND", "Set FFmpeg in Settings."))?;
    let run = |codec: &[&str]| -> Result<bool, CommandError> {
        let args = ["-v", "error", "-nostdin", "-y", "-progress", "pipe:1", "-i"]
            .into_iter()
            .map(String::from)
            .chain([input.to_string_lossy().into_owned()])
            .chain(["-i".into(), audio.to_string_lossy().into_owned()])
            .chain(
                [
                    "-map", "0:v:0", "-map", "1:a:0", "-c:a", "aac", "-b:a", "192k", "-ac", "2",
                    "-t",
                ]
                .into_iter()
                .map(String::from),
            )
            .chain([format!("{:.9}", video.duration_ms / 1000.)])
            .chain(codec.iter().map(|s| s.to_string()))
            .chain(["-movflags", "+faststart"].into_iter().map(String::from))
            .chain([output.to_string_lossy().into_owned()])
            .collect::<Vec<_>>();
        let result = process_runner::run_observed(
            &tool,
            &args,
            Duration::from_secs(7200),
            cancel.clone(),
            Some((video.duration_ms as u64, report.clone())),
        )?;
        Ok(result.success)
    };
    if !run(&["-c:v", "copy"])?
        && !run(&[
            "-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p",
        ])?
    {
        return Err(CommandError::new(
            "VIDEO_EXPORT_FAILED",
            "Cannot export this video with edited audio.",
        ));
    }
    if cancel.load(Ordering::SeqCst) {
        return Err(CommandError::new(
            "JOB_CANCELLED",
            "Video export cancelled.",
        ));
    }
    let duration = speech::probe_duration(output, settings)? as f64;
    if (duration - video.duration_ms).abs() > 150. {
        return Err(CommandError::new(
            "INVALID_EXPORT_DURATION",
            "Export duration did not match the selected video.",
        ));
    }
    Ok(())
}
