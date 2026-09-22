use std::{
    fs::{self, File},
    io::{BufReader, Read},
    path::{Path, PathBuf},
    time::Duration,
};

use super::{jobs::JobControl, process_runner, project_store::CommandError, settings::Settings, sound_workers, voice_store};
use tauri::AppHandle;

pub fn generate(
    app: &AppHandle,
    text: &str,
    voice_id: &str,
    output: &Path,
    settings: &Settings,
    control: &JobControl,
) -> Result<u64, CommandError> {
    if text.trim().is_empty() { return Err(CommandError::new("EMPTY_TEXT", "There is no text to narrate.")); }
    let health = sound_workers::health(&settings.sounds.chatterbox_url, "chatterbox_turbo");
    if !health.ready { return Err(CommandError::new("WORKER_UNAVAILABLE", health.message)); }
    let reference = voice_store::selected_sample(app, voice_id)?;
    let work = output.parent().ok_or_else(|| CommandError::internal("Audio work folder is missing"))?
        .join(format!("narration-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&work).map_err(|error| CommandError::io("Cannot stage narration", error))?;
    let result = (|| {
        let chunks = chunks(text, 280);
        if chunks.len() > 500 { return Err(CommandError::new("CHAPTER_TOO_LONG", "This chapter is too long for one narration job. Split it into smaller chapters.")); }
        let mut files = Vec::new();
        for (index, chunk) in chunks.iter().enumerate() {
            control.boundary()?;
            let reference_id = reference.as_ref().map(|bytes| sound_workers::upload_reference(&settings.sounds.chatterbox_url, bytes)).transpose()?;
            let payload = serde_json::json!({"prompt": chunk, "category": "speech", "durationSeconds": 20, "seed": null, "referenceId": reference_id});
            let wav = sound_workers::generate(&settings.sounds.chatterbox_url, &payload, control)?;
            let name = format!("part-{index:04}.wav");
            fs::write(work.join(&name), wav).map_err(|error| CommandError::io("Cannot stage speech audio", error))?;
            files.push(name);
        }
        let ffmpeg = process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
            .ok_or_else(|| CommandError::new("FFMPEG_NOT_FOUND", "Set FFmpeg in Settings before creating narration."))?;
        let list = work.join("parts.txt");
        fs::write(&list, files.iter().map(|file| format!("file '{file}'\n")).collect::<String>())
            .map_err(|error| CommandError::io("Cannot stage narration list", error))?;
        let conversion = process_runner::run_bounded(&ffmpeg, &[
            "-v".into(), "error".into(), "-y".into(), "-f".into(), "concat".into(), "-safe".into(), "1".into(),
            "-i".into(), path_string(&list), "-vn".into(), "-c:a".into(), "aac".into(), "-b:a".into(), "128k".into(),
            "-ar".into(), "48000".into(), "-ac".into(), "2".into(), path_string(output),
        ], Duration::from_secs(3600), control.cancelled.clone())?;
        if !conversion.success { return Err(CommandError::new("AUDIO_CONVERSION_FAILED", concise(&conversion.stderr))); }
        probe_duration(output, settings)
    })();
    let _ = fs::remove_dir_all(work);
    if result.is_err() { let _ = fs::remove_file(output); }
    result
}

fn chunks(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > max_chars {
            chunks.push(std::mem::take(&mut current));
        }
        if !current.is_empty() { current.push(' '); }
        current.push_str(word);
        if current.chars().count() >= max_chars / 2 && word.ends_with(['.', '!', '?']) {
            chunks.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() { chunks.push(current); }
    chunks
}

pub fn import(
    source: &Path,
    output: &Path,
    settings: &Settings,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<u64, CommandError> {
    if !source.is_file() {
        return Err(CommandError::new(
            "AUDIO_NOT_FOUND",
            "The selected audio file was not found.",
        ));
    }
    let ffmpeg = process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
        .ok_or_else(|| {
            CommandError::new(
                "FFMPEG_NOT_FOUND",
                "Install FFmpeg or set its path in Settings.",
            )
        })?;
    let conversion = process_runner::run_bounded(
        &ffmpeg,
        &[
            "-y".into(),
            "-i".into(),
            path_string(source),
            "-vn".into(),
            "-c:a".into(),
            "aac".into(),
            "-b:a".into(),
            "128k".into(),
            "-ar".into(),
            "48000".into(),
            "-ac".into(),
            "2".into(),
            path_string(output),
        ],
        Duration::from_secs(3600),
        cancelled,
    )?;
    if !conversion.success {
        return Err(CommandError::new(
            "AUDIO_IMPORT_FAILED",
            concise(&conversion.stderr),
        ));
    }
    probe_duration(output, settings)
}

pub fn waveform(path: &Path, settings: &Settings) -> Result<Vec<f32>, CommandError> {
    if !path.is_file() {
        return Err(CommandError::new(
            "AUDIO_NOT_FOUND",
            "The chapter audio file was not found.",
        ));
    }
    let ffmpeg = process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
        .ok_or_else(|| {
            CommandError::new(
                "FFMPEG_NOT_FOUND",
                "Install FFmpeg or set its path in Settings.",
            )
        })?;
    let raw_path = path.with_extension(format!("{}.raw", uuid::Uuid::new_v4()));
    let decode = process_runner::run_bounded(
        &ffmpeg,
        &[
            "-y".into(),
            "-i".into(),
            path_string(path),
            "-ac".into(),
            "1".into(),
            "-ar".into(),
            "4000".into(),
            "-f".into(),
            "s16le".into(),
            path_string(&raw_path),
        ],
        Duration::from_secs(120),
        Default::default(),
    )?;
    if !decode.success {
        let _ = fs::remove_file(&raw_path);
        return Err(CommandError::new(
            "WAVEFORM_FAILED",
            concise(&decode.stderr),
        ));
    }
    let result = waveform_from_raw(&raw_path, 120);
    let _ = fs::remove_file(&raw_path);
    result
}

fn waveform_from_raw(path: &Path, bins: usize) -> Result<Vec<f32>, CommandError> {
    let sample_count = fs::metadata(path)
        .map_err(|error| CommandError::io("Cannot inspect decoded audio", error))?
        .len() as usize
        / 2;
    if sample_count == 0 {
        return Err(CommandError::new(
            "WAVEFORM_FAILED",
            "The audio file contains no samples.",
        ));
    }
    let mut peaks = vec![0_u16; bins];
    let mut reader = BufReader::new(
        File::open(path).map_err(|error| CommandError::io("Cannot read decoded audio", error))?,
    );
    let mut buffer = [0_u8; 8192];
    let mut sample_index = 0_usize;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| CommandError::io("Cannot read decoded audio", error))?;
        if read == 0 {
            break;
        }
        for bytes in buffer[..read].chunks_exact(2) {
            let amplitude = i16::from_le_bytes([bytes[0], bytes[1]]).unsigned_abs();
            let bin = (sample_index * bins / sample_count).min(bins - 1);
            peaks[bin] = peaks[bin].max(amplitude);
            sample_index += 1;
        }
    }
    Ok(peaks
        .into_iter()
        .map(|peak| peak as f32 / i16::MAX as f32)
        .collect())
}

pub fn probe_duration(path: &Path, settings: &Settings) -> Result<u64, CommandError> {
    let ffprobe = process_runner::resolve_executable("ffprobe", settings.ffprobe_path.as_deref())
        .ok_or_else(|| {
        CommandError::new(
            "FFPROBE_NOT_FOUND",
            "Install FFprobe or set its path in Settings.",
        )
    })?;
    let result = process_runner::run_bounded(
        &ffprobe,
        &[
            "-v".into(),
            "error".into(),
            "-show_entries".into(),
            "format=duration".into(),
            "-of".into(),
            "default=noprint_wrappers=1:nokey=1".into(),
            path_string(path),
        ],
        Duration::from_secs(30),
        Default::default(),
    )?;
    if !result.success {
        return Err(CommandError::new(
            "AUDIO_PROBE_FAILED",
            concise(&result.stderr),
        ));
    }
    let seconds: f64 = result.stdout.trim().parse().map_err(|_| {
        CommandError::new(
            "AUDIO_PROBE_FAILED",
            "FFprobe returned an invalid duration.",
        )
    })?;
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err(CommandError::new(
            "AUDIO_PROBE_FAILED",
            "Audio duration must be positive.",
        ));
    }
    Ok((seconds * 1000.0).round() as u64)
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
fn concise(value: &str) -> String {
    value
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("Audio processing failed.")
        .chars()
        .take(500)
        .collect()
}
pub fn staged_path(root: &str, chapter_id: &str) -> PathBuf {
    Path::new(root)
        .join(".work")
        .join(format!("{chapter_id}-{}.m4a", uuid::Uuid::new_v4()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn splits_long_speech_without_losing_words() {
        let original = "A first sentence. A second sentence here. A final sentence.";
        let parts = chunks(original, 28);
        assert_eq!(parts.join(" "), original);
        assert!(parts.iter().all(|part| part.len() <= 28));
    }

    #[test]
    fn reduces_pcm_samples_to_normalized_peaks() {
        let path =
            std::env::temp_dir().join(format!("homer-waveform-{}.raw", uuid::Uuid::new_v4()));
        let samples = [0_i16, 8_000, -16_000, i16::MAX];
        let bytes: Vec<u8> = samples.into_iter().flat_map(i16::to_le_bytes).collect();
        fs::write(&path, bytes).unwrap();
        let peaks = waveform_from_raw(&path, 2).unwrap();
        fs::remove_file(path).unwrap();
        assert!((peaks[0] - 8_000.0 / i16::MAX as f32).abs() < 0.001);
        assert_eq!(peaks[1], 1.0);
    }
}
