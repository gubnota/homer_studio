use serde::Serialize;
use std::{
    fs::{self, File},
    io::{BufReader, Read},
    path::{Path, PathBuf},
    time::Duration,
};

use super::{process_runner, project_store::CommandError, settings::Settings};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    pub id: String,
    pub language: String,
    pub sample: String,
}

pub fn list_voices() -> Result<Vec<Voice>, CommandError> {
    let output = process_runner::run_bounded(
        Path::new("/usr/bin/say"),
        &["-v".into(), "?".into()],
        Duration::from_secs(10),
        Default::default(),
    )?;
    if !output.success {
        return Err(CommandError::new("VOICE_LIST_FAILED", output.stderr));
    }
    Ok(parse_voices(&output.stdout))
}

pub fn ensure_installed_voice(voice_id: &str) -> Result<(), CommandError> {
    if voice_is_installed(voice_id, &list_voices()?) {
        Ok(())
    } else {
        Err(CommandError::new(
            "VOICE_NOT_INSTALLED",
            format!("The macOS voice ‘{voice_id}’ is not installed."),
        ))
    }
}

fn voice_is_installed(voice_id: &str, voices: &[Voice]) -> bool {
    voices.iter().any(|voice| voice.id == voice_id)
}

pub fn generate(
    text: &str,
    voice: &str,
    rate: u16,
    output: &Path,
    settings: &Settings,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<u64, CommandError> {
    if text.trim().is_empty() {
        return Err(CommandError::new(
            "EMPTY_TEXT",
            "There is no text to narrate.",
        ));
    }
    let work = output
        .parent()
        .ok_or_else(|| CommandError::internal("audio work folder is missing"))?;
    fs::create_dir_all(work)
        .map_err(|error| CommandError::io("Cannot create audio work folder", error))?;
    let text_path = work.join("speech.txt");
    let aiff_path = work.join("speech.aiff");
    fs::write(&text_path, text)
        .map_err(|error| CommandError::io("Cannot stage narration text", error))?;
    let say = process_runner::run_bounded(
        Path::new("/usr/bin/say"),
        &[
            "-v".into(),
            voice.into(),
            "-r".into(),
            rate.to_string(),
            "-o".into(),
            path_string(&aiff_path),
            "-f".into(),
            path_string(&text_path),
        ],
        Duration::from_secs(3600),
        cancelled.clone(),
    )?;
    if !say.success {
        return Err(CommandError::new("SPEECH_FAILED", concise(&say.stderr)));
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
            path_string(&aiff_path),
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
            "AUDIO_CONVERSION_FAILED",
            concise(&conversion.stderr),
        ));
    }
    probe_duration(output, settings)
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

fn parse_voices(text: &str) -> Vec<Voice> {
    text.lines()
        .filter_map(|line| {
            let locale_start = line.find(|character: char| character == '_' || character == '-')?;
            let prefix = &line[..locale_start];
            let id = prefix
                .trim_end()
                .rsplit_once("  ")
                .map(|(name, _)| name.trim())
                .unwrap_or(prefix.trim());
            let rest = &line[locale_start.saturating_sub(2)..];
            let mut parts = rest.splitn(2, '#');
            let language = parts.next()?.trim().split_whitespace().last()?.to_string();
            let sample = parts.next().unwrap_or_default().trim().to_string();
            (!id.is_empty()).then(|| Voice {
                id: id.into(),
                language,
                sample,
            })
        })
        .collect()
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
    fn parses_installed_voice_output() {
        let voices = parse_voices("Samantha             en_US    # Hello! My name is Samantha.\n");
        assert_eq!(voices[0].id, "Samantha");
        assert_eq!(voices[0].language, "en_US");
        assert!(voice_is_installed("Samantha", &voices));
        assert!(!voice_is_installed("Unavailable Voice", &voices));
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
