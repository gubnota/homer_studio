//! Passage-wide cleanup and level matching. Project sources remain immutable.
use super::{audio_assets, process_runner, project_store::CommandError, settings::Settings};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

pub fn transcode(
    settings: &Settings,
    cancel: Arc<AtomicBool>,
    input: &Path,
    output: &Path,
    filter: &str,
) -> Result<(), CommandError> {
    audio_assets::ffmpeg(
        settings,
        vec![
            "-v".into(),
            "error".into(),
            "-nostdin".into(),
            "-y".into(),
            "-i".into(),
            input.to_string_lossy().into(),
            "-af".into(),
            filter.into(),
            "-ac".into(),
            "2".into(),
            "-ar".into(),
            "48000".into(),
            "-c:a".into(),
            "pcm_f32le".into(),
            output.to_string_lossy().into(),
        ],
        cancel,
    )
}
#[derive(Debug)]
pub struct Levels {
    pub rms: f64,
    pub peak: f64,
    pub frames: u64,
}
pub fn levels(path: &Path) -> Result<Levels, CommandError> {
    let mut r = hound::WavReader::open(path).map_err(|e| CommandError::internal(e.to_string()))?;
    let spec = r.spec();
    let count = r.len() as u64;
    let mut sum = 0.;
    let mut peak = 0f64;
    // Accept native model WAV and floating point project renders without re-encoding for analysis.
    let samples: Box<dyn Iterator<Item = Result<f64, hound::Error>> + '_> =
        if spec.sample_format == hound::SampleFormat::Float {
            Box::new(r.samples::<f32>().map(|v| v.map(|v| v as f64)))
        } else {
            let scale = 2f64.powi(spec.bits_per_sample as i32 - 1);
            Box::new(r.samples::<i32>().map(move |v| v.map(|v| v as f64 / scale)))
        };
    for s in samples {
        let v = s.map_err(|e| CommandError::internal(e.to_string()))?;
        if !v.is_finite() {
            return Err(CommandError::new(
                "INVALID_AUDIO",
                "Audio contains invalid samples.",
            ));
        }
        sum += v * v;
        peak = peak.max(v.abs());
    }
    Ok(Levels {
        rms: (sum / count.max(1) as f64).sqrt(),
        peak,
        frames: count / spec.channels as u64,
    })
}
pub(super) fn loudness(
    settings: &Settings,
    cancel: Arc<AtomicBool>,
    path: &Path,
) -> Result<(f64, f64), CommandError> {
    let ffmpeg = process_runner::resolve_executable("ffmpeg", settings.ffmpeg_path.as_deref())
        .ok_or_else(|| CommandError::new("FFMPEG_NOT_FOUND", "Install FFmpeg to process audio."))?;
    let args = vec![
        "-hide_banner".into(),
        "-nostdin".into(),
        "-i".into(),
        path.to_string_lossy().into(),
        "-af".into(),
        "loudnorm=I=-23:TP=-1:LRA=11:print_format=json".into(),
        "-f".into(),
        "null".into(),
        "-".into(),
    ];
    let result = process_runner::run_bounded(&ffmpeg, &args, Duration::from_secs(3600), cancel)?;
    if !result.success {
        return Err(CommandError::new(
            "AUDIO_ANALYSIS_FAILED",
            format!("Cannot measure audio: {}", result.stderr),
        ));
    }
    let json = result
        .stderr
        .rfind('{')
        .and_then(|i| {
            serde_json::Deserializer::from_str(&result.stderr[i..])
                .into_iter::<serde_json::Value>()
                .next()
                .and_then(Result::ok)
        })
        .ok_or_else(|| {
            CommandError::new("AUDIO_ANALYSIS_FAILED", "Cannot read loudness measurement.")
        })?;
    let number = |key: &str| {
        json[key]
            .as_str()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(f64::NEG_INFINITY)
    };
    Ok((number("input_i"), number("input_tp")))
}
/// Cleanup precedes measurement. Oversampled limiting has no automatic makeup gain.
pub fn match_passage(
    settings: &Settings,
    cancel: Arc<AtomicBool>,
    original: &Path,
    input: &Path,
    output: &Path,
    cleanup: bool,
) -> Result<Vec<String>, CommandError> {
    let cleaned = output.with_extension("clean.wav");
    let limited = output.with_extension("limit.wav");
    let target = levels(original)?;
    let samples = target.frames;
    transcode(
        settings,
        cancel.clone(),
        input,
        &cleaned,
        &format!(
            "{}apad,atrim=end_sample={samples}",
            if cleanup {
                "afftdn=nr=6:nf=-50:tn=1:gs=5,"
            } else {
                ""
            }
        ),
    )?;
    let current = levels(&cleaned)?;
    if current.rms < 1e-7 || current.peak < 1e-6 {
        return Err(CommandError::new(
            "SILENT_CONVERSION",
            "The converted voice is almost silent. Choose another reference or retry this passage.",
        ));
    }
    if target.rms < 1e-7 {
        return Err(CommandError::new(
            "SILENT_ORIGINAL",
            "The original passage is silent. Select a passage containing speech.",
        ));
    }
    let (target_lufs, _) = loudness(settings, cancel.clone(), original)?;
    let (current_lufs, current_tp) = loudness(settings, cancel.clone(), &cleaned)?;
    let gain = if samples >= 19200 && target_lufs.is_finite() && current_lufs.is_finite() {
        target_lufs - current_lufs
    } else {
        20. * (target.rms / current.rms).log10()
    };
    let limiting = if !current_tp.is_finite() || current_tp + gain > -1. {
        "aresample=192000,alimiter=limit=0.891250938:attack=5:release=50:level=false:latency=true,aresample=48000,"
    } else {
        ""
    };
    let limiter = format!("volume={gain:.8}dB,{limiting}apad,atrim=end_sample={samples}");
    transcode(settings, cancel.clone(), &cleaned, &limited, &limiter)?;
    let (achieved, tp) = loudness(settings, cancel.clone(), &limited)?;
    // Recheck after downsampling; interpolation can create small intersample overshoots.
    let correction = if tp.is_finite() {
        (-1. - tp).min(0.)
    } else {
        0.
    };
    transcode(
        settings,
        cancel,
        &limited,
        output,
        &format!("volume={correction:.8}dB,apad,atrim=end_sample={samples}"),
    )?;
    let mut warnings = vec![];
    if target_lufs.is_finite()
        && achieved.is_finite()
        && (target_lufs - achieved - correction).abs() > 1.
    {
        warnings.push(
            "The peak ceiling limits how closely this passage can match the original volume."
                .into(),
        );
    }
    let _ = std::fs::remove_file(cleaned);
    let _ = std::fs::remove_file(limited);
    Ok(warnings)
}
