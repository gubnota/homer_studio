use super::{
    audio_assets,
    project_store::CommandError,
    settings::Settings,
    wave_studio::{Clip, Project},
};
use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};
// Tempo variants are streamed once into an immutable native cache. Preview and
// export seek the same PCM, with sample-accurate envelopes on the 48 kHz grid.
pub fn arguments(
    p: &Project,
    base: &Path,
    start: f64,
    end: f64,
    output: &Path,
) -> Result<Vec<String>, CommandError> {
    super::wave_studio::validate(p)?;
    if !start.is_finite()
        || !end.is_finite()
        || start < 0.
        || end <= start
        || end > p.timeline.duration() + 1.
    {
        return Err(CommandError::new(
            "INVALID_RENDER_RANGE",
            "Choose an audio range inside the project.",
        ));
    }
    let seconds = (end - start) / 1000.;
    let mut args = vec![
        "-v".into(),
        "error".into(),
        "-nostdin".into(),
        "-y".into(),
        "-f".into(),
        "lavfi".into(),
        "-i".into(),
        format!("anullsrc=r=48000:cl=stereo:d={seconds:.9}"),
    ];
    let active: Vec<&Clip> = p
        .timeline
        .clips
        .iter()
        .chain(&p.timeline.sfx)
        .filter(|c| {
            c.source_id.is_some() && c.start_ms < end && c.end() > start && c.gain_db > -96.
        })
        .collect();
    let mut filters = vec![format!(
        "[0:a]atrim=end_sample={},asetpts=PTS-STARTPTS[base]",
        (seconds * 48000.).round() as u64
    )];
    let mut mix = "[base]".to_string();
    for (i, c) in active.iter().enumerate() {
        let local = ((start - c.start_ms) / 1000.).max(0.);
        let stop = ((end.min(c.end()) - c.start_ms) / 1000.).max(local);
        let seek = if c.speed == 1. {
            c.source_start_ms / 1000. + local
        } else {
            local
        };
        let input = if c.speed == 1. {
            audio_assets::path(base, c.source_id.as_ref().unwrap())?
        } else {
            tempo_path(base, c)
        };
        args.extend([
            "-ss".into(),
            format!("{seek:.9}"),
            "-t".into(),
            format!("{:.9}", stop - local + 0.05),
            "-i".into(),
            input.to_string_lossy().into(),
        ]);
        let fi = c.fade_in_ms / 1000.;
        let fo = c.fade_out_ms / 1000.;
        let d = c.duration() / 1000.;
        let gain = if c.gain_db <= -96. {
            0.
        } else {
            10f64.powf(c.gain_db / 20.)
        };
        let envelope = format!(
            "{gain:.9}*{}*{}",
            if fi > 0. {
                format!("min(1,(t+{local:.9})/{fi:.9})")
            } else {
                "1".into()
            },
            if fo > 0. {
                format!("max(0,min(1,({d:.9}-t-{local:.9})/{fo:.9}))")
            } else {
                "1".into()
            }
        );
        let delay = ((c.start_ms - start).max(0.) * 48.).round() as u64;
        filters.push(format!("[{}:a]atrim=end_sample={},asetpts=PTS-STARTPTS,aformat=sample_rates=48000:channel_layouts=stereo,aeval='val(0)*({})|val(1)*({})',adelay={}S:all=1[a{}]",i+1,((stop-local)*48000.).round() as u64,envelope,envelope,delay,i));
        mix.push_str(&format!("[a{i}]"));
    }
    filters.push(format!("{mix}amix=inputs={}:duration=first:normalize=0:dropout_transition=0,atrim=end_sample={}[out]",active.len()+1,(seconds*48000.).round() as u64));
    args.extend([
        "-filter_complex".into(),
        filters.join(";"),
        "-map".into(),
        "[out]".into(),
        "-ar".into(),
        "48000".into(),
    ]);
    let codec = match output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "wav" => "pcm_f32le",
        "flac" => "flac",
        "m4a" | "aac" => "aac",
        "mp3" => "libmp3lame",
        _ => {
            return Err(CommandError::new(
                "INVALID_EXPORT_FORMAT",
                "Choose WAV, FLAC, M4A, AAC or MP3.",
            ));
        }
    };
    args.extend(["-c:a".into(), codec.into()]);
    if codec == "aac" || codec == "libmp3lame" {
        args.extend(["-b:a".into(), "192k".into()]);
    }
    if codec == "pcm_f32le" {
        args.extend(["-rf64".into(), "auto".into()]);
    }
    args.push(output.to_string_lossy().into());
    Ok(args)
}
fn tempo_path(base: &Path, c: &Clip) -> std::path::PathBuf {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    c.source_id.hash(&mut hash);
    c.source_start_ms.to_bits().hash(&mut hash);
    c.source_end_ms.to_bits().hash(&mut hash);
    c.speed.to_bits().hash(&mut hash);
    base.join("tempo").join(format!("{:x}.wav", hash.finish()))
}
pub fn render(
    p: &Project,
    base: &Path,
    start: f64,
    end: f64,
    output: &Path,
    settings: &Settings,
    cancel: Arc<AtomicBool>,
) -> Result<(), CommandError> {
    super::wave_studio::validate(p)?;
    // Cache publication is serialized, streaming in FFmpeg rather than retaining
    // audiobook samples in either Rust or the renderer.
    static TEMPO_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    {
        let _guard = TEMPO_LOCK
            .lock()
            .map_err(|_| CommandError::internal("Tempo cache unavailable"))?;
        for c in p.timeline.clips.iter().chain(&p.timeline.sfx).filter(|c| {
            c.source_id.is_some() && c.speed != 1. && c.start_ms < end && c.end() > start
        }) {
            let path = tempo_path(base, c);
            if path.is_file() {
                continue;
            }
            std::fs::create_dir_all(path.parent().unwrap())
                .map_err(|e| CommandError::io("Cannot create tempo cache", e))?;
            let temp = path.with_file_name(format!("{}.wav", uuid::Uuid::new_v4()));
            let result = audio_assets::ffmpeg(
                settings,
                vec![
                    "-v".into(),
                    "error".into(),
                    "-nostdin".into(),
                    "-y".into(),
                    "-ss".into(),
                    format!("{:.9}", c.source_start_ms / 1000.),
                    "-t".into(),
                    format!("{:.9}", (c.source_end_ms - c.source_start_ms) / 1000.),
                    "-i".into(),
                    audio_assets::path(base, c.source_id.as_ref().unwrap())?
                        .to_string_lossy()
                        .into(),
                    "-af".into(),
                    format!(
                        "atempo={:.9},apad,atrim=end_sample={}",
                        c.speed,
                        (c.duration() * 48.).round() as u64
                    ),
                    "-ar".into(),
                    "48000".into(),
                    "-ac".into(),
                    "2".into(),
                    "-c:a".into(),
                    "pcm_f32le".into(),
                    "-rf64".into(),
                    "auto".into(),
                    temp.to_string_lossy().into(),
                ],
                cancel.clone(),
            );
            if let Err(error) = result {
                let _ = std::fs::remove_file(temp);
                return Err(error);
            }
            std::fs::rename(temp, path)
                .map_err(|e| CommandError::io("Cannot publish tempo audio", e))?;
        }
    }
    let result = audio_assets::ffmpeg(settings, arguments(p, base, start, end, output)?, cancel);
    // Bound disk cache while retaining every tempo source used by this snapshot.
    let used: std::collections::HashSet<_> = p
        .timeline
        .clips
        .iter()
        .chain(&p.timeline.sfx)
        .filter(|c| c.source_id.is_some() && c.speed != 1.)
        .map(|c| tempo_path(base, c))
        .collect();
    if let Ok(entries) = std::fs::read_dir(base.join("tempo")) {
        let mut files: Vec<_> = entries
            .flatten()
            .filter_map(|e| {
                e.metadata()
                    .ok()
                    .map(|m| (e.path(), m.len(), m.modified().ok()))
            })
            .collect();
        files.sort_by_key(|f| f.2);
        let mut total: u64 = files.iter().map(|f| f.1).sum();
        for (path, size, _) in files {
            if total <= 4 * 1024 * 1024 * 1024 {
                break;
            }
            if !used.contains(&path) && std::fs::remove_file(path).is_ok() {
                total = total.saturating_sub(size);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::wave_studio::{Source, Timeline};
    fn samples(path: &Path) -> Vec<f32> {
        hound::WavReader::open(path)
            .unwrap()
            .into_samples::<f32>()
            .map(Result::unwrap)
            .step_by(2)
            .collect()
    }
    fn rms(s: &[f32]) -> f64 {
        (s.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / s.len() as f64).sqrt()
    }
    #[test]
    fn renders_real_audio_with_silence_gain_fades_pitch_and_matching_preview() {
        let root = std::env::temp_dir().join(format!("homer-wave-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("assets")).unwrap();
        let source_id = uuid::Uuid::new_v4().to_string();
        let input = audio_assets::path(&root, &source_id).unwrap();
        std::fs::create_dir_all(input.parent().unwrap()).unwrap();
        let mut writer = hound::WavWriter::create(
            &input,
            hound::WavSpec {
                channels: 2,
                sample_rate: 48000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .unwrap();
        for i in 0..48000 * 4 {
            let value = (i as f64 * 440. * 2. * std::f64::consts::PI / 48000.).sin() as f32 * 0.25;
            writer.write_sample(value).unwrap();
            writer.write_sample(value).unwrap();
        }
        writer.finalize().unwrap();
        let source = Source {
            id: source_id.clone(),
            name: "Tone".into(),
            duration_ms: 4000.,
            channels: 2,
        };
        let clip = Clip {
            id: uuid::Uuid::new_v4().to_string(),
            source_id: Some(source_id),
            name: "Tone".into(),
            start_ms: 500.,
            source_start_ms: 0.,
            source_end_ms: 4000.,
            speed: 1.08,
            gain_db: -10.,
            fade_in_ms: 250.,
            fade_out_ms: 250.,
        };
        let mut p = Project {
            schema_version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            name: "Fixture".into(),
            revision: 0,
            updated_at_ms: 0,
            sources: vec![source],
            timeline: Timeline {
                clips: vec![clip],
                sfx: vec![],
                voices: vec![],
            },
        };
        let settings = Settings::default();
        let out = root.join("mix.wav");
        render(
            &p,
            &root,
            0.,
            p.timeline.duration(),
            &out,
            &settings,
            Default::default(),
        )
        .unwrap();
        let pcm = samples(&out);
        assert!((pcm.len() as f64 - p.timeline.duration() * 48.).abs() <= 1.);
        assert!(rms(&pcm[..20000]) < 1e-7, "Leading gap must stay silent");
        let middle = &pcm[48000..96000];
        assert!(
            (rms(middle) - 0.25 / 2f64.sqrt() * 10f64.powf(-10. / 20.)).abs() < 0.002,
            "Gain must remain at -10 dB"
        );
        let crossings = middle
            .windows(2)
            .filter(|w| w[0] <= 0. && w[1] > 0.)
            .count();
        assert!(
            (crossings as i32 - 440).abs() < 3,
            "Tempo must preserve the original pitch"
        );
        assert!(
            rms(&pcm[24000..26000]) < rms(&pcm[48000..50000]) * 0.4,
            "Fade-in must attenuate the beginning"
        );
        assert!(
            rms(&pcm[pcm.len() - 2000..]) < rms(middle) * 0.4,
            "Fade-out must attenuate the end"
        );
        // Both original and pitch-preserved speed windows must match export exactly.
        for speed in [1., 1.08] {
            p.timeline.clips[0].speed = speed;
            render(
                &p,
                &root,
                0.,
                p.timeline.duration(),
                &out,
                &settings,
                Default::default(),
            )
            .unwrap();
            let full = samples(&out);
            let left = root.join("left.wav");
            let right = root.join("right.wav");
            render(&p, &root, 0., 2000., &left, &settings, Default::default()).unwrap();
            render(
                &p,
                &root,
                2000.,
                p.timeline.duration(),
                &right,
                &settings,
                Default::default(),
            )
            .unwrap();
            let mut chunks = samples(&left);
            chunks.extend(samples(&right));
            assert_eq!(full.len(), chunks.len());
            assert!(full.iter().zip(&chunks).all(|(a, b)| (a - b).abs() < 1e-6));
        }
        // A separately aligned SFX clip mixes at its own gain without changing originals.
        p.timeline.clips.clear();
        let mut effect = Clip {
            id: uuid::Uuid::new_v4().to_string(),
            source_id: Some(p.sources[0].id.clone()),
            name: "Effect".into(),
            start_ms: 1000.,
            source_start_ms: 0.,
            source_end_ms: 1000.,
            speed: 1.,
            gain_db: -20.,
            fade_in_ms: 0.,
            fade_out_ms: 0.,
        };
        p.timeline.sfx.push(effect.clone());
        let original = std::fs::read(&input).unwrap();
        render(&p, &root, 0., 2000., &out, &settings, Default::default()).unwrap();
        let pcm = samples(&out);
        assert!(rms(&pcm[..48000]) < 1e-7);
        assert!((rms(&pcm[48000..]) - 0.25 / 2f64.sqrt() * 0.1).abs() < 1e-5);
        effect.gain_db = -96.;
        p.timeline.sfx[0] = effect;
        render(&p, &root, 0., 2000., &out, &settings, Default::default()).unwrap();
        assert!(rms(&samples(&out)) < 1e-7);
        assert_eq!(original, std::fs::read(&input).unwrap());
        let _ = std::fs::remove_dir_all(root);
    }
}
