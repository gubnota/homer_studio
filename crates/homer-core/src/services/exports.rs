use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use super::{
    process_runner,
    project_store::{self, CommandError, ProjectSnapshot},
    settings::Settings,
    speech,
};

pub struct PreparedExport {
    pub audio_path: PathBuf,
    pub timestamps_path: PathBuf,
    pub duration_ms: u64,
}

pub fn assemble(
    root: &str,
    project: &ProjectSnapshot,
    settings: &Settings,
    cancelled: Arc<AtomicBool>,
) -> Result<PreparedExport, CommandError> {
    if project.chapters.is_empty() {
        return Err(CommandError::new(
            "EXPORT_EMPTY",
            "The project has no chapters.",
        ));
    }
    let mut inputs = Vec::with_capacity(project.chapters.len());
    let mut durations = Vec::with_capacity(project.chapters.len());
    for chapter in &project.chapters {
        if chapter.chapter.audio_stale
            || chapter.chapter.review_status.as_deref() != Some("approved")
        {
            return Err(CommandError::new(
                "EXPORT_NOT_READY",
                format!(
                    "Approve current audio for '{}' before exporting.",
                    chapter.chapter.title
                ),
            ));
        }
        let path = project_store::chapter_audio_path(root, &chapter.chapter.id)?;
        durations.push(speech::probe_duration(&path, settings)?);
        inputs.push(path);
    }
    let ffmpeg = process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
        .ok_or_else(|| {
            CommandError::new(
                "FFMPEG_NOT_FOUND",
                "Install FFmpeg or set its path in Settings.",
            )
        })?;
    let work = Path::new(root)
        .join(".work")
        .join(format!("export-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&work)
        .map_err(|error| CommandError::io("Cannot create export workspace", error))?;
    let output = work.join("full-audio.m4a");
    let timestamp_path = work.join("youtube-chapters.txt");
    let expected: u64 = durations.iter().sum();

    let copy_result = concat_copy(&ffmpeg, &inputs, &work, &output, cancelled.clone());
    let copied_duration = copy_result
        .ok()
        .and_then(|_| speech::probe_duration(&output, settings).ok())
        .filter(|duration| within_tolerance(*duration, expected, inputs.len()));
    let duration_ms = if let Some(duration) = copied_duration {
        duration
    } else {
        let _ = fs::remove_file(&output);
        concat_encode(&ffmpeg, &inputs, &output, cancelled)?;
        let duration = speech::probe_duration(&output, settings)?;
        if !within_tolerance(duration, expected, inputs.len()) {
            let _ = fs::remove_dir_all(&work);
            return Err(CommandError::new(
                "EXPORT_VERIFICATION_FAILED",
                "The exported duration does not match the chapter timeline.",
            ));
        }
        duration
    };
    fs::write(&timestamp_path, timestamps(project, &durations))
        .map_err(|error| CommandError::io("Cannot write chapter timestamps", error))?;
    Ok(PreparedExport {
        audio_path: output,
        timestamps_path: timestamp_path,
        duration_ms,
    })
}

fn concat_copy(
    ffmpeg: &Path,
    inputs: &[PathBuf],
    work: &Path,
    output: &Path,
    cancelled: Arc<AtomicBool>,
) -> Result<(), CommandError> {
    let list_path = work.join("concat.txt");
    let list = inputs
        .iter()
        .map(|path| format!("file '{}'", escape_concat(path)))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&list_path, list)
        .map_err(|error| CommandError::io("Cannot stage chapter list", error))?;
    let result = process_runner::run_bounded(
        ffmpeg,
        &[
            "-y".into(),
            "-f".into(),
            "concat".into(),
            "-safe".into(),
            "0".into(),
            "-i".into(),
            path_string(&list_path),
            "-c".into(),
            "copy".into(),
            path_string(output),
        ],
        Duration::from_secs(3600),
        cancelled,
    )?;
    result
        .success
        .then_some(())
        .ok_or_else(|| CommandError::new("EXPORT_COPY_FAILED", concise(&result.stderr)))
}

fn concat_encode(
    ffmpeg: &Path,
    inputs: &[PathBuf],
    output: &Path,
    cancelled: Arc<AtomicBool>,
) -> Result<(), CommandError> {
    let mut args = vec!["-y".into()];
    for input in inputs {
        args.extend(["-i".into(), path_string(input)]);
    }
    let streams = (0..inputs.len())
        .map(|index| format!("[{index}:a]"))
        .collect::<String>();
    args.extend([
        "-filter_complex".into(),
        format!("{streams}concat=n={}:v=0:a=1[out]", inputs.len()),
        "-map".into(),
        "[out]".into(),
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        "128k".into(),
        "-ar".into(),
        "48000".into(),
        "-ac".into(),
        "2".into(),
        path_string(output),
    ]);
    let result = process_runner::run_bounded(ffmpeg, &args, Duration::from_secs(3600), cancelled)?;
    result
        .success
        .then_some(())
        .ok_or_else(|| CommandError::new("EXPORT_ENCODE_FAILED", concise(&result.stderr)))
}

fn timestamps(project: &ProjectSnapshot, durations: &[u64]) -> String {
    let titles = project
        .chapters
        .iter()
        .map(|chapter| chapter.chapter.title.clone())
        .collect::<Vec<_>>();
    timestamp_text(&titles, durations)
}

fn timestamp_text(titles: &[String], durations: &[u64]) -> String {
    let mut start_ms = 0_u64;
    let mut lines = Vec::with_capacity(titles.len());
    for (index, (title, duration)) in titles.iter().zip(durations).enumerate() {
        let clean = title.split_whitespace().collect::<Vec<_>>().join(" ");
        let title = if clean.is_empty() {
            format!("Chapter {}", index + 1)
        } else {
            clean
        };
        lines.push(format!("{} {title}", format_time(start_ms / 1000)));
        start_ms += duration;
    }
    lines.join("\n") + "\n"
}

fn format_time(seconds: u64) -> String {
    if seconds < 3600 {
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    } else {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    }
}

fn within_tolerance(actual: u64, expected: u64, chapters: usize) -> bool {
    actual.abs_diff(expected) <= 300 + chapters as u64 * 50
}

fn escape_concat(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\'', "'\\''")
}
fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
fn concise(value: &str) -> String {
    value
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("Audio export failed.")
        .chars()
        .take(500)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    #[test]
    fn formats_boundaries_after_accumulating_fractional_durations() {
        let titles = vec!["Opening".into(), "  Middle\nPart ".into(), "".into()];
        assert_eq!(
            timestamp_text(&titles, &[272_900, 378_900, 235_000]),
            "00:00 Opening\n04:32 Middle Part\n10:51 Chapter 3\n"
        );
        assert_eq!(format_time(3_661), "1:01:01");
    }
    #[test]
    fn escapes_quotes_in_concat_paths() {
        assert_eq!(
            escape_concat(Path::new("/tmp/author's/audio.m4a")),
            "/tmp/author'\\''s/audio.m4a"
        );
    }

    #[test]
    fn ffmpeg_exports_quoted_unicode_paths_and_mixed_audio() {
        let Some(ffmpeg) = process_runner::resolve_executable("ffmpeg", None) else {
            return;
        };
        let root = std::env::temp_dir().join(format!("Homer author's–{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let first = root.join("first chapter.m4a");
        let second = root.join("second chapter.m4a");
        let mixed = root.join("mixed chapter.wav");
        fixture(&ffmpeg, &first, "1.25", "48000", "2");
        fixture(&ffmpeg, &second, "1.50", "48000", "2");
        fixture(&ffmpeg, &mixed, "0.75", "22050", "1");

        let copied = root.join("copied.m4a");
        concat_copy(
            &ffmpeg,
            &[first.clone(), second],
            &root,
            &copied,
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
        let settings = Settings::default();
        assert!(within_tolerance(
            speech::probe_duration(&copied, &settings).unwrap(),
            2_750,
            2
        ));

        let encoded = root.join("encoded.m4a");
        concat_encode(
            &ffmpeg,
            &[first, mixed],
            &encoded,
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
        assert!(within_tolerance(
            speech::probe_duration(&encoded, &settings).unwrap(),
            2_000,
            2
        ));
        fs::remove_dir_all(root).unwrap();
    }

    fn fixture(ffmpeg: &Path, path: &Path, duration: &str, rate: &str, channels: &str) {
        let result = process_runner::run_bounded(
            ffmpeg,
            &[
                "-y".into(),
                "-f".into(),
                "lavfi".into(),
                "-i".into(),
                format!("sine=frequency=440:duration={duration}"),
                "-ar".into(),
                rate.into(),
                "-ac".into(),
                channels.into(),
                path_string(path),
            ],
            Duration::from_secs(30),
            Default::default(),
        )
        .unwrap();
        assert!(result.success, "{}", result.stderr);
    }
}
