use std::{
    fs::{self, File},
    io::{BufReader, Read},
    path::{Path, PathBuf},
    time::Duration,
};

use super::{
    jobs::JobControl,
    process_runner,
    project_store::{CommandError, LineCue},
    settings::Settings,
    sound_workers, voice_store,
};
use tauri::AppHandle;

pub fn generate(
    app: &AppHandle,
    text: &str,
    voice_id: &str,
    output: &Path,
    settings: &Settings,
    control: &JobControl,
) -> Result<SpeechOutput, CommandError> {
    if text.trim().is_empty() {
        return Err(CommandError::new(
            "EMPTY_TEXT",
            "There is no text to narrate.",
        ));
    }
    let health = sound_workers::health(&settings.sounds.chatterbox_url, "chatterbox_turbo");
    if !health.ready {
        return Err(CommandError::new("WORKER_UNAVAILABLE", health.message));
    }
    let reference = voice_store::selected_sample(app, voice_id)?;
    let work = output
        .parent()
        .ok_or_else(|| CommandError::internal("Audio work folder is missing"))?
        .join(format!("narration-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&work).map_err(|error| CommandError::io("Cannot stage narration", error))?;
    let result = (|| {
        let lines: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        let parts = lines
            .iter()
            .map(|line| parse_markup(line))
            .collect::<Result<Vec<_>, _>>()?;
        let request_count = parts
            .iter()
            .flatten()
            .map(|part| match part {
                SpeechPart::Speech(value) => chunks(value, 280).len(),
                SpeechPart::Gesture(_) => 1,
                SpeechPart::Pause(_) => 0,
            })
            .sum::<usize>();
        if request_count > 500 {
            return Err(CommandError::new(
                "CHAPTER_TOO_LONG",
                "This chapter is too long for one narration job. Split it into smaller chapters.",
            ));
        }
        let ffmpeg = process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
            .ok_or_else(|| {
                CommandError::new(
                    "FFMPEG_NOT_FOUND",
                    "Set FFmpeg in Settings before creating narration.",
                )
            })?;
        let mut files = Vec::new();
        let mut cues = Vec::new();
        let mut index = 0usize;
        let mut elapsed_ms = 0u64;
        for (line_order, (line, line_parts)) in lines.iter().zip(parts).enumerate() {
            let line_start = files.len();
            for part in line_parts {
                match part {
                    SpeechPart::Speech(value) => {
                        for chunk in chunks(&value, 280) {
                            control.boundary()?;
                            let reference_id = reference
                                .as_ref()
                                .map(|bytes| {
                                    sound_workers::upload_reference(
                                        &settings.sounds.chatterbox_url,
                                        bytes,
                                    )
                                })
                                .transpose()?;
                            let payload = serde_json::json!({"prompt": chunk, "category": "speech", "durationSeconds": 20, "seed": null, "referenceId": reference_id});
                            let wav = sound_workers::generate(
                                &settings.sounds.chatterbox_url,
                                &payload,
                                control,
                            )?;
                            let name = format!("part-{index:04}.wav");
                            index += 1;
                            fs::write(work.join(&name), wav).map_err(|error| {
                                CommandError::io("Cannot stage speech audio", error)
                            })?;
                            files.push(name);
                        }
                    }
                    SpeechPart::Gesture(tag) => {
                        control.boundary()?;
                        let reference_id = reference
                            .as_ref()
                            .map(|bytes| {
                                sound_workers::upload_reference(
                                    &settings.sounds.chatterbox_url,
                                    bytes,
                                )
                            })
                            .transpose()?;
                        let payload = serde_json::json!({"prompt": tag, "category": "vocal_gesture", "durationSeconds": 3, "seed": null, "referenceId": reference_id});
                        let wav = sound_workers::generate(
                            &settings.sounds.chatterbox_url,
                            &payload,
                            control,
                        )?;
                        let name = format!("part-{index:04}.wav");
                        index += 1;
                        fs::write(work.join(&name), wav).map_err(|error| {
                            CommandError::io("Cannot stage vocal gesture", error)
                        })?;
                        files.push(name);
                    }
                    SpeechPart::Pause(ms) => {
                        control.boundary()?;
                        let name = format!("part-{index:04}.wav");
                        index += 1;
                        let silence = process_runner::run_bounded(
                            &ffmpeg,
                            &[
                                "-v".into(),
                                "error".into(),
                                "-y".into(),
                                "-f".into(),
                                "lavfi".into(),
                                "-i".into(),
                                "anullsrc=r=24000:cl=mono".into(),
                                "-t".into(),
                                format!("{:.3}", ms as f64 / 1000.0),
                                "-c:a".into(),
                                "pcm_s16le".into(),
                                path_string(&work.join(&name)),
                            ],
                            Duration::from_secs(15),
                            control.cancelled.clone(),
                        )?;
                        if !silence.success {
                            return Err(CommandError::new(
                                "AUDIO_CONVERSION_FAILED",
                                concise(&silence.stderr),
                            ));
                        }
                        files.push(name);
                    }
                }
            }
            let start_ms = elapsed_ms;
            for file in &files[line_start..] {
                elapsed_ms = elapsed_ms.saturating_add(probe_duration(&work.join(file), settings)?);
            }
            cues.push(LineCue {
                order: line_order,
                text: display_line(line),
                start_ms,
                end_ms: elapsed_ms,
            });
        }
        let list = work.join("parts.txt");
        fs::write(
            &list,
            files
                .iter()
                .map(|file| format!("file '{file}'\n"))
                .collect::<String>(),
        )
        .map_err(|error| CommandError::io("Cannot stage narration list", error))?;
        let conversion = process_runner::run_bounded(
            &ffmpeg,
            &[
                "-v".into(),
                "error".into(),
                "-y".into(),
                "-f".into(),
                "concat".into(),
                "-safe".into(),
                "1".into(),
                "-i".into(),
                path_string(&list),
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
            control.cancelled.clone(),
        )?;
        if !conversion.success {
            return Err(CommandError::new(
                "AUDIO_CONVERSION_FAILED",
                concise(&conversion.stderr),
            ));
        }
        let duration_ms = probe_duration(output, settings)?;
        Ok(SpeechOutput { duration_ms, cues })
    })();
    let _ = fs::remove_dir_all(work);
    if result.is_err() {
        let _ = fs::remove_file(output);
    }
    result
}

pub struct SpeechOutput {
    pub duration_ms: u64,
    pub cues: Vec<LineCue>,
}

fn display_line(line: &str) -> String {
    let mut visible = String::new();
    let mut rest = line;
    while let Some(open) = rest.find('[') {
        visible.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else {
            visible.push_str(&rest[open..]);
            rest = "";
            break;
        };
        let marker = after[..close].trim().to_lowercase();
        if !["sigh", "gasp", "cough", "laugh", "chuckle", "groan"].contains(&marker.as_str())
            && !marker.starts_with("pause:")
        {
            visible.push('[');
            visible.push_str(&after[..close + 1]);
        }
        rest = &after[close + 1..];
    }
    visible.push_str(rest);
    visible.trim().to_string()
}

#[derive(Debug, PartialEq, Eq)]
enum SpeechPart {
    Speech(String),
    Gesture(String),
    Pause(u64),
}

fn parse_markup(text: &str) -> Result<Vec<SpeechPart>, CommandError> {
    const TAGS: &[&str] = &["sigh", "gasp", "cough", "laugh", "chuckle", "groan"];
    let mut parts = Vec::new();
    let mut speech = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        speech.push_str(&rest[..open]);
        if open > 0 && rest[..open].ends_with('\\') {
            speech.pop();
            speech.push('[');
            rest = &rest[open + 1..];
            continue;
        }
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else {
            return Err(CommandError::new(
                "INVALID_SPEECH_MARKUP",
                "A narration marker is missing its closing bracket.",
            ));
        };
        let marker = after[..close].trim().to_lowercase();
        if TAGS.contains(&marker.as_str()) {
            if !speech.trim().is_empty() {
                parts.push(SpeechPart::Speech(std::mem::take(&mut speech)));
            }
            parts.push(SpeechPart::Gesture(format!("[{marker}]")));
        } else if let Some(duration) = marker
            .strip_prefix("pause:")
            .and_then(|value| value.parse::<u64>().ok())
        {
            if !(100..=5000).contains(&duration) {
                return Err(CommandError::new(
                    "INVALID_SPEECH_MARKUP",
                    "Pause duration must be between 100 and 5000 milliseconds.",
                ));
            }
            if !speech.trim().is_empty() {
                parts.push(SpeechPart::Speech(std::mem::take(&mut speech)));
            }
            parts.push(SpeechPart::Pause(duration));
        } else {
            return Err(CommandError::new("INVALID_SPEECH_MARKUP", format!("Unsupported narration marker [{marker}]. Use supported vocal tags or [pause:500].")));
        }
        rest = &after[close + 1..];
    }
    speech.push_str(rest);
    if !speech.trim().is_empty() {
        parts.push(SpeechPart::Speech(speech));
    }
    if parts.is_empty() {
        return Err(CommandError::new(
            "EMPTY_TEXT",
            "Add spoken text, a vocal gesture, or a pause.",
        ));
    }
    Ok(parts)
}

fn chunks(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > max_chars {
            chunks.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
        if current.chars().count() >= max_chars / 2 && word.ends_with(['.', '!', '?']) {
            chunks.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
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
    fn parses_gestures_and_pauses_and_rejects_unknown_cues() {
        let parts = parse_markup("Tomorrow [pause:800] [sigh] arrives").unwrap();
        assert_eq!(
            parts,
            vec![
                SpeechPart::Speech("Tomorrow ".into()),
                SpeechPart::Pause(800),
                SpeechPart::Gesture("[sigh]".into()),
                SpeechPart::Speech("  arrives".into())
            ]
        );
        assert!(parse_markup("Hello [pause:20]").is_err());
        assert!(parse_markup("Hello [sing]").is_err());
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
