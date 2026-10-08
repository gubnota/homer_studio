use super::{
    audio_assets, process_runner, project_store::CommandError, settings::Settings, speech,
    wave_store, wave_studio::Video,
};
use crate::runtime::AppHandle;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
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
            "stream=duration,codec_type,codec_name,pix_fmt:stream_disposition=attached_pic:format=duration".into(),
            "-of".into(),
            "json".into(),
            input.to_string_lossy().into(),
        ],
        Duration::from_secs(30),
        cancel.clone(),
    )?;
    let info: serde_json::Value = serde_json::from_str(&measured.stdout).unwrap_or_default();
    if !measured.success {
        return Err(CommandError::new("MEDIA_PROBE_FAILED", measured.stderr));
    }
    let stream = &info["streams"][0];
    if stream["codec_type"] != "video" || stream["disposition"]["attached_pic"] == 1 {
        return Err(CommandError::new(
            "NO_VIDEO",
            "This file has no video track.",
        ));
    }
    let compatible = stream["codec_name"] == "h264" && stream["pix_fmt"] == "yuv420p";
    let duration = match info["streams"][0]["duration"]
        .as_str()
        .and_then(|d| d.parse::<f64>().ok())
        .filter(|d| d.is_finite() && *d > 0.)
    {
        Some(seconds) => seconds * 1000.,
        None => {
            info["format"]["duration"]
                .as_str()
                .and_then(|d| d.parse::<f64>().ok())
                .unwrap_or(0.)
                * 1000.
        }
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
        if cancel.load(Ordering::SeqCst) {
            return Err(CommandError::new(
                "JOB_CANCELLED",
                "Video import cancelled.",
            ));
        }
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

pub fn export_duration(audio: f64, video: f64, mode: Option<&str>) -> Result<f64, CommandError> {
    let duration = match mode {
        None => video,
        Some("longest") => audio.max(video),
        Some("shortest") => audio.min(video),
        _ => {
            return Err(CommandError::new(
                "INVALID_EXPORT_LENGTH",
                "Unknown export length.",
            ));
        }
    };
    if !duration.is_finite() || duration <= 0. || duration > 86_400_000. {
        return Err(CommandError::new(
            "INVALID_EXPORT_DURATION",
            "The chosen export has no audio/video overlap or exceeds 24 hours.",
        ));
    }
    Ok(duration)
}
#[cfg(test)]
mod length_tests {
    use super::export_duration;
    #[test]
    fn chooses_length_and_keeps_legacy_video_duration() {
        assert_eq!(
            export_duration(8000., 1000., Some("longest")).unwrap(),
            8000.
        );
        assert_eq!(
            export_duration(8000., 1000., Some("shortest")).unwrap(),
            1000.
        );
        assert_eq!(
            export_duration(1000., 8000., Some("longest")).unwrap(),
            8000.
        );
        assert_eq!(
            export_duration(1000., 8000., Some("shortest")).unwrap(),
            1000.
        );
        assert_eq!(export_duration(8000., 1000., None).unwrap(), 1000.);
        assert!(export_duration(0., 1000., Some("shortest")).is_err());
        assert!(export_duration(1000., 1000., Some("other")).is_err());
    }
}
/// Replace audio and trim or hold the selected video to the chosen duration.
pub fn export_mix(
    app: &AppHandle,
    video: &Video,
    duration: f64,
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
            .chain([format!("{:.9}", duration / 1000.)])
            .chain(codec.iter().map(|s| s.to_string()))
            .chain(["-movflags", "+faststart"].into_iter().map(String::from))
            .chain([output.to_string_lossy().into_owned()])
            .collect::<Vec<_>>();
        let result = process_runner::run_observed(
            &tool,
            &args,
            Duration::from_secs(7200),
            cancel.clone(),
            Some((duration as u64, report.clone())),
        )?;
        Ok(result.success)
    };
    let extend = duration > video.duration_ms;
    let encoded = if extend {
        run(&[
            "-vf",
            &format!(
                "tpad=stop_mode=clone:stop_duration={}",
                (duration - video.duration_ms) / 1000.
            ),
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            "20",
            "-pix_fmt",
            "yuv420p",
        ])?
    } else {
        run(&["-c:v", "copy"])?
    };
    if !encoded
        && (extend
            || !run(&[
                "-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p",
            ])?)
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
    if (speech::probe_duration(output, settings)? as f64 - duration).abs() > 150. {
        return Err(CommandError::new(
            "INVALID_EXPORT_DURATION",
            "Export duration did not match the chosen length.",
        ));
    }
    Ok(())
}

/// Normalize one segment at a time so long projects do not open every video at once.
pub fn export_timeline(
    app: &AppHandle,
    videos: &[Video],
    duration: f64,
    audio: &Path,
    output: &Path,
    settings: &Settings,
    cancel: Arc<AtomicBool>,
    progress: &dyn Fn(u8),
) -> Result<(), CommandError> {
    super::wave_studio::validate_video_placements(videos)?;
    let tool = process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
        .ok_or_else(|| CommandError::new("FFMPEG_NOT_FOUND", "Set FFmpeg in Settings."))?;
    let folder = output.with_extension(format!("{}.segments", uuid::Uuid::new_v4()));
    fs::create_dir(&folder).map_err(|e| CommandError::io("Cannot stage video", e))?;
    let result = (|| {
        let mut sorted: Vec<_> = videos.iter().collect();
        sorted.sort_by(|a, b| a.start_ms.total_cmp(&b.start_ms));
        let mut segments: Vec<(Option<&Video>, f64, f64)> = vec![];
        if let Some(first) = sorted.first() {
            if first.start_ms > 0. {
                segments.push((None, 0., first.start_ms.min(duration)));
            }
        }
        for (i, v) in sorted.iter().enumerate() {
            if v.start_ms >= duration {
                break;
            }
            segments.push((
                Some(v),
                v.start_ms,
                sorted
                    .get(i + 1)
                    .map(|n| n.start_ms)
                    .unwrap_or(duration)
                    .min(duration),
            ));
        }
        let mut list = String::new();
        for (i, (video, start, end)) in segments.iter().enumerate() {
            let frames = ((end / 1000. * 30.).round() - (start / 1000. * 30.).round()) as u64;
            if frames == 0 {
                continue;
            }
            let mut args: Vec<String> = ["-v", "error", "-nostdin", "-y", "-threads", "1"]
                .into_iter()
                .map(String::from)
                .collect();
            if let Some(v) = video {
                args.extend([
                    "-i".into(),
                    original_path(app, &v.id)?.to_string_lossy().into_owned(),
                ]);
            } else {
                args.extend(
                    ["-f", "lavfi", "-i", "color=c=black:s=1280x720:r=30"]
                        .into_iter()
                        .map(String::from),
                );
            }
            let filter = if let Some(v) = video {
                format!(
                    "trim=duration={},setpts=PTS-STARTPTS,scale=1280:720:force_original_aspect_ratio=decrease,pad=1280:720:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30,tpad=stop_mode=clone:stop_duration={}",
                    v.duration_ms / 1000.,
                    (end - start) / 1000.
                )
            } else {
                "setsar=1".into()
            };
            let name = format!("{i}.mp4");
            args.extend([
                "-an".into(),
                "-vf".into(),
                filter,
                "-frames:v".into(),
                frames.to_string(),
                "-c:v".into(),
                "libx264".into(),
                "-threads".into(),
                "1".into(),
                "-preset".into(),
                "veryfast".into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
                folder.join(&name).to_string_lossy().into_owned(),
            ]);
            let run = process_runner::run_bounded(
                &tool,
                &args,
                Duration::from_secs(7200),
                cancel.clone(),
            )?;
            if !run.success {
                return Err(CommandError::new("VIDEO_EXPORT_FAILED", run.stderr));
            }
            progress(30 + ((i + 1) * 55 / segments.len()) as u8);
            list.push_str(&format!("file '{name}'\n"));
        }
        let manifest = folder.join("list.txt");
        fs::write(&manifest, list).map_err(|e| CommandError::io("Cannot stage video list", e))?;
        let args: Vec<String> = [
            "-v", "error", "-nostdin", "-y", "-f", "concat", "-safe", "1", "-i",
        ]
        .into_iter()
        .map(String::from)
        .chain([
            manifest.to_string_lossy().into_owned(),
            "-i".into(),
            audio.to_string_lossy().into_owned(),
        ])
        .chain(
            [
                "-map", "0:v:0", "-map", "1:a:0", "-c:v", "copy", "-c:a", "aac", "-b:a", "192k",
                "-t",
            ]
            .into_iter()
            .map(String::from),
        )
        .chain([
            (duration / 1000.).to_string(),
            "-movflags".into(),
            "+faststart".into(),
            output.to_string_lossy().into_owned(),
        ])
        .collect();
        let run = process_runner::run_bounded(&tool, &args, Duration::from_secs(7200), cancel)?;
        if !run.success {
            return Err(CommandError::new("VIDEO_EXPORT_FAILED", run.stderr));
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(folder);
    result
}
