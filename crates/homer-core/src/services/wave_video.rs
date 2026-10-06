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
    let duration = speech::probe_duration(input, settings)? as f64;
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
        audio_assets::atomic_json(
            &wave_store::root(app)?
                .join("videos")
                .join(format!("{id}.json")),
            &video,
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
    let owned: Video =
        serde_json::from_slice(&data).map_err(|e| CommandError::internal(e.to_string()))?;
    if owned.duration_ms != video.duration_ms || !path(app, &video.id)?.is_file() {
        return Err(CommandError::new(
            "INVALID_VIDEO",
            "Video reference no longer matches its source.",
        ));
    }
    Ok(())
}
