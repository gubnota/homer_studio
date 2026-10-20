use super::{
    audio_assets,
    jobs::JobControl,
    project_store::CommandError,
    settings::Settings,
    sound_workers, speech, voice_store, wave_render, wave_store,
    wave_studio::{Project, Source},
};
use crate::runtime::AppHandle;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, sync::Arc};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Replacement {
    pub start_ms: f64,
    pub end_ms: f64,
    pub source_id: String,
    #[serde(default)]
    pub region_id: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioComparison {
    pub sources: Vec<Source>,
    pub replacements: Vec<Replacement>,
    pub warnings: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingResult {
    pub kind: String,
    pub project_id: String,
    pub revision: u64,
    pub sources: Vec<Source>,
    pub replacements: Vec<Replacement>,
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub without_cleanup: Option<AudioComparison>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_cleanup: Option<AudioComparison>,
}
pub fn read(app: &AppHandle, id: &str) -> Result<ProcessingResult, CommandError> {
    audio_assets::id(id)?;
    let b = fs::read(
        wave_store::root(app)?
            .join("results")
            .join(format!("{id}.json")),
    )
    .map_err(|e| CommandError::io("Cannot open processing result", e))?;
    serde_json::from_slice(&b).map_err(|e| CommandError::internal(e.to_string()))
}
pub fn write(app: &AppHandle, id: &str, result: &ProcessingResult) -> Result<(), CommandError> {
    audio_assets::atomic_json(
        &wave_store::root(app)?
            .join("results")
            .join(format!("{id}.json")),
        result,
    )
}
fn stage<T>(
    app: &AppHandle,
    f: impl FnOnce(&Path) -> Result<T, CommandError>,
) -> Result<T, CommandError> {
    let work = wave_store::root(app)?
        .join("work")
        .join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&work).map_err(|e| CommandError::io("Cannot stage audio", e))?;
    let result = f(&work);
    let _ = fs::remove_dir_all(&work);
    result
}
pub fn normalize(
    app: &AppHandle,
    p: &Project,
    id: &str,
    settings: &Settings,
) -> Result<f64, CommandError> {
    wave_store::validate_sources(app, p)?;
    let clip = p
        .timeline
        .clips
        .iter()
        .chain(&p.timeline.sfx)
        .find(|c| c.id == id && c.source_id.is_some())
        .ok_or_else(|| CommandError::new("NO_AUDIO", "Choose an audio fragment."))?;
    stage(app, |work| {
        let mut isolated = p.clone();
        let mut c = clip.clone();
        c.gain_db = 0.;
        isolated.timeline.clips = vec![c];
        isolated.timeline.sfx.clear();
        isolated.timeline.voices.clear();
        isolated.voice_original = None;
        let out = work.join("peak.wav");
        wave_render::render(
            &isolated,
            &wave_store::root(app)?,
            clip.start_ms,
            clip.end(),
            &out,
            settings,
            Default::default(),
        )?;
        let mut reader =
            hound::WavReader::open(out).map_err(|e| CommandError::internal(e.to_string()))?;
        let mut peak = 0f64;
        for value in reader.samples::<f32>() {
            let sample = value.map_err(|e| CommandError::internal(e.to_string()))?;
            peak = peak.max(sample.abs() as f64)
        }
        normalization_gain(peak)
    })
}
fn normalization_gain(peak: f64) -> Result<f64, CommandError> {
    if !peak.is_finite() || peak <= 1e-10 {
        return Err(CommandError::new(
            "SILENT_AUDIO",
            "This fragment is silent; normalization cannot raise its level.",
        ));
    }
    Ok((-1. - 20. * peak.log10()).clamp(-96., 20.))
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedSelection {
    pub gain_db: f64,
    pub sources: Vec<Source>,
    pub baked: Vec<(String, String)>,
    pub warnings: Vec<String>,
}
pub fn normalize_selection(
    app: &AppHandle,
    p: &Project,
    ids: &[String],
    start: f64,
    end: f64,
    settings: &Settings,
) -> Result<NormalizedSelection, CommandError> {
    wave_store::validate_sources(app, p)?;
    if !start.is_finite() || !end.is_finite() || start < 0. || end <= start {
        return Err(CommandError::new(
            "INVALID_SELECTION",
            "Select an audio range first.",
        ));
    }
    let selected = |c: &&super::wave_studio::Clip| {
        c.source_id.is_some()
            && c.start_ms < end
            && c.end() > start
            && (ids.is_empty() || ids.contains(&c.id))
    };
    let mut isolated = p.clone();
    isolated.timeline.voices.clear();
    isolated.voice_original = None;
    isolated.timeline.clips = p.timeline.clips.iter().filter(selected).cloned().collect();
    isolated.timeline.sfx = if ids.is_empty() {
        vec![]
    } else {
        p.timeline.sfx.iter().filter(selected).cloned().collect()
    };
    let clips = isolated
        .timeline
        .clips
        .iter()
        .chain(&isolated.timeline.sfx)
        .cloned()
        .collect::<Vec<_>>();
    isolated.timeline.clips.push(super::wave_studio::Clip {
        id: uuid::Uuid::new_v4().to_string(),
        source_id: None,
        name: "Selection tail".into(),
        start_ms: 0.,
        source_start_ms: 0.,
        source_end_ms: end,
        speed: 1.,
        gain_db: 0.,
        fade_in_ms: 0.,
        fade_out_ms: 0.,
    });
    if clips.is_empty() {
        return Err(CommandError::new(
            "NO_AUDIO",
            "The selection contains no audio.",
        ));
    }
    stage(app, |work| {
        let out = work.join("selection.wav");
        wave_render::render(
            &isolated,
            &wave_store::root(app)?,
            start,
            end,
            &out,
            settings,
            Default::default(),
        )?;
        let peak = super::wave_voice_audio::levels(&out)?.peak;
        if peak <= 1e-10 {
            return Err(CommandError::new(
                "SILENT_AUDIO",
                "This selection is silent.",
            ));
        }
        let desired = -1. - 20. * peak.log10();
        let lower = clips
            .iter()
            .map(|c| -96. - c.gain_db)
            .fold(f64::NEG_INFINITY, f64::max);
        let upper = clips
            .iter()
            .map(|c| 20. - c.gain_db)
            .fold(f64::INFINITY, f64::min);
        let gain_db = desired.clamp(lower, upper);
        let mut result = NormalizedSelection {
            gain_db,
            sources: vec![],
            baked: vec![],
            warnings: vec![],
        };
        if (desired - gain_db).abs() > 0.001 {
            result.warnings.push("The fragment gain limits prevent reaching −1 dBFS while preserving the selection’s balance.".into());
        }
        // Bake partial clips at unity gain before splitting: this preserves their existing
        // fades and speed exactly, including envelopes crossing the selection boundary.
        for clip in clips.iter().filter(|c| c.start_ms < start || c.end() > end) {
            let mut one = p.clone();
            let mut c = clip.clone();
            c.gain_db = 0.;
            one.timeline.clips = vec![c];
            one.timeline.sfx.clear();
            one.timeline.voices.clear();
            one.voice_original = None;
            wave_render::render(
                &one,
                &wave_store::root(app)?,
                clip.start_ms,
                clip.end(),
                &out,
                settings,
                Default::default(),
            )?;
            let source = wave_store::import(app, &out, &clip.name, settings, &p.id)?;
            result.baked.push((clip.id.clone(), source.id.clone()));
            result.sources.push(source);
        }
        Ok(result)
    })
}
pub fn join(
    app: &AppHandle,
    p: &Project,
    ids: &[String],
    settings: &Settings,
) -> Result<Source, CommandError> {
    wave_store::validate_sources(app, p)?;
    let mut clips = p
        .timeline
        .clips
        .iter()
        .filter(|c| ids.contains(&c.id))
        .cloned()
        .collect::<Vec<_>>();
    clips.sort_by(|a, b| a.start_ms.total_cmp(&b.start_ms));
    if clips.len() < 2
        || clips.len() != ids.len()
        || clips
            .windows(2)
            .any(|w| (w[0].end() - w[1].start_ms).abs() > 0.05)
    {
        return Err(CommandError::new(
            "INVALID_JOIN",
            "Select at least two adjacent narration fragments. Include silence fragments to join across a gap.",
        ));
    }
    stage(app, |work| {
        let start = clips[0].start_ms;
        let end = clips.last().unwrap().end();
        let mut isolated = p.clone();
        isolated.timeline.clips = clips;
        isolated.timeline.sfx.clear();
        isolated.timeline.voices.clear();
        isolated.voice_original = None;
        let out = work.join("joined.wav");
        wave_render::render(
            &isolated,
            &wave_store::root(app)?,
            start,
            end,
            &out,
            settings,
            Default::default(),
        )?;
        wave_store::import(app, &out, "Joined narration", settings, &p.id)
    })
}
pub fn generate(
    app: &AppHandle,
    text: &str,
    voice: &str,
    project_id: &str,
    revision: u64,
    settings: &Settings,
    control: &JobControl,
    progress: &dyn Fn(u8),
) -> Result<ProcessingResult, CommandError> {
    audio_assets::id(project_id)?;
    wave_store::require_package(app, project_id)?;
    stage(app, |work| {
        let out = work.join("speech.m4a");
        speech::generate_with_progress(
            app,
            text,
            voice,
            &out,
            settings,
            control,
            &|done, total| progress((5 + done * 80 / total.max(1)) as u8),
        )?;
        control.boundary()?;
        let source = wave_store::import_controlled(
            app,
            &out,
            "Generated speech",
            settings,
            control.cancelled.clone(),
            Arc::new(|_| {}),
            project_id,
        )?;
        control.boundary()?;
        Ok(ProcessingResult {
            kind: "speech".into(),
            project_id: project_id.into(),
            revision,
            sources: vec![source],
            replacements: vec![],
            warnings: vec![],
            without_cleanup: None,
            with_cleanup: None,
        })
    })
}
fn conversion_clips(
    p: &Project,
    region: &super::wave_studio::VoiceRegion,
) -> Vec<super::wave_studio::Clip> {
    if let Some(audio) = &region.audio {
        audio
            .original
            .iter()
            .map(|clip| {
                let mut clip = clip.clone();
                clip.start_ms += region.start_ms;
                clip
            })
            .collect()
    } else {
        p.timeline.clips.clone()
    }
}
fn convert_chunk(
    app: &AppHandle,
    worker: &str,
    reference: &Option<Vec<u8>>,
    source: &Path,
    out: &Path,
    duration: f64,
    settings: &Settings,
    control: &JobControl,
) -> Result<(), CommandError> {
    sound_workers::recycle_before_job(app, worker, "chatterbox_original", control)?;
    // VC tokenizes 16 kHz mono input. Float encoding avoids integer quantization
    // and keeps a full 15-second source within the worker's 2 MB upload bound.
    let native_source = source.with_extension("vc-input.wav");
    audio_assets::ffmpeg(
        settings,
        vec![
            "-v".into(),
            "error".into(),
            "-nostdin".into(),
            "-y".into(),
            "-i".into(),
            source.to_string_lossy().into(),
            "-ac".into(),
            "1".into(),
            "-ar".into(),
            "16000".into(),
            "-c:a".into(),
            "pcm_f32le".into(),
            native_source.to_string_lossy().into(),
        ],
        control.cancelled.clone(),
    )?;
    let source_id = sound_workers::upload_reference(
        worker,
        &fs::read(&native_source).map_err(|e| CommandError::io("Cannot read narration", e))?,
    )?;
    let reference_id = reference
        .as_ref()
        .map(|bytes| sound_workers::upload_reference(worker, bytes))
        .transpose()?;
    let bytes = sound_workers::generate(
        worker,
        &serde_json::json!({"prompt":"Convert recorded delivery","category":"voice_conversion","durationSeconds":(duration/1000.).max(1.),"seed":null,"sourceId":source_id,"referenceId":reference_id}),
        control,
    )?;
    fs::write(out, bytes).map_err(|e| CommandError::io("Cannot stage converted voice", e))
}
fn tempo_for_duration(actual: f64, wanted: f64) -> Result<f64, CommandError> {
    let ratio = actual / wanted;
    if !ratio.is_finite() || !(0.85..=1.2).contains(&ratio) {
        return Err(CommandError::new(
            "CONVERSION_DURATION_MISMATCH",
            "The converted delivery differs too much in duration. Shorten or retag this passage and retry.",
        ));
    }
    Ok(ratio)
}
pub fn convert(
    app: &AppHandle,
    p: &Project,
    settings: &Settings,
    control: &JobControl,
    progress: &dyn Fn(u8),
    cleanup: bool,
) -> Result<ProcessingResult, CommandError> {
    wave_store::validate_sources(app, p)?;
    if p.timeline.voices.is_empty() {
        return Err(CommandError::new(
            "NO_VOICES",
            "Tag a narration passage with a voice first.",
        ));
    }
    let mut regions = p.timeline.voices.clone();
    regions.sort_by(|a, b| a.start_ms.total_cmp(&b.start_ms));
    if regions.windows(2).any(|w| w[0].end_ms > w[1].start_ms) {
        return Err(CommandError::new(
            "OVERLAPPING_VOICES",
            "Voice regions overlap. Assign one voice per passage.",
        ));
    }
    let references = regions
        .iter()
        .map(|r| voice_store::selected_sample(app, &r.voice_id))
        .collect::<Result<Vec<_>, _>>()?;
    speech::ensure_worker_ready(app, &settings.sounds.original_url, true, control)?;
    stage(app, |work| {
        let mut result = ProcessingResult {
            kind: "voices".into(),
            project_id: p.id.clone(),
            revision: p.revision,
            sources: vec![],
            replacements: vec![],
            warnings: vec![],
            without_cleanup: None,
            with_cleanup: None,
        };
        let mut bypass = AudioComparison {
            sources: vec![],
            replacements: vec![],
            warnings: vec![],
        };
        let count = regions
            .iter()
            .map(|r| ((r.end_ms - r.start_ms) / 15000.).ceil() as usize)
            .sum::<usize>();
        let mut done = 0;
        let mut isolated = p.clone();
        isolated.timeline.sfx.clear();
        isolated.timeline.voices.clear();
        isolated.voice_original = None;
        for (region, reference) in regions.iter().zip(&references) {
            // Every conversion starts from immutable passage audio, never an accepted conversion.
            isolated.timeline.clips = conversion_clips(p, region);
            let span = region.end_ms - region.start_ms;
            let chunks = (span / 15000.).ceil().max(1.) as usize;
            let chunk_ms = span / chunks as f64;
            let passage = work.join("passage.wav");
            let mut writer = hound::WavWriter::create(
                &passage,
                hound::WavSpec {
                    channels: 2,
                    sample_rate: 48000,
                    bits_per_sample: 32,
                    sample_format: hound::SampleFormat::Float,
                },
            )
            .map_err(|e| CommandError::internal(e.to_string()))?;
            let mut boundaries = vec![];
            for index in 0..chunks {
                control.boundary()?;
                let start = region.start_ms + index as f64 * chunk_ms;
                let end = if index + 1 == chunks {
                    region.end_ms
                } else {
                    start + chunk_ms
                };
                let rendered = work.join("original.wav");
                let output = work.join("converted.wav");
                let fitted = work.join("fitted.wav");
                wave_render::render(
                    &isolated,
                    &wave_store::root(app)?,
                    start,
                    end,
                    &rendered,
                    settings,
                    control.cancelled.clone(),
                )?;
                convert_chunk(
                    app,
                    &settings.sounds.original_url,
                    reference,
                    &rendered,
                    &output,
                    end - start,
                    settings,
                    control,
                )?;
                // Keep model output at its native rate until fitting it to the project format. Tiny rounding needs no stretch.
                let actual = speech::probe_duration(&output, settings)? as f64;
                let ratio = tempo_for_duration(actual, end - start)?;
                let tempo = if (actual - (end - start)).abs() > 2. {
                    format!("atempo={ratio:.9},")
                } else {
                    String::new()
                };
                let raw_levels = super::wave_voice_audio::levels(&output)?;
                let first = ((start - region.start_ms) * 48.).round() as u64;
                let last = ((end - region.start_ms) * 48.).round() as u64;
                super::wave_voice_audio::transcode(
                    settings,
                    control.cancelled.clone(),
                    &output,
                    &fitted,
                    &format!(
                        "{tempo}aresample=48000,apad,atrim=end_sample={}",
                        last - first
                    ),
                )?;
                let fit_levels = super::wave_voice_audio::levels(&fitted)?;
                if raw_levels.rms > 1e-7
                    && (20. * (fit_levels.rms / raw_levels.rms).log10()).abs() > 3.
                {
                    let warning="Fitting this conversion changed its level substantially. Compare it with the original before accepting.".to_string();
                    result.warnings.push(warning.clone());
                    bypass.warnings.push(warning);
                }
                let mut reader = hound::WavReader::open(&fitted)
                    .map_err(|e| CommandError::internal(e.to_string()))?;
                for sample in reader.samples::<f32>() {
                    writer
                        .write_sample(sample.map_err(|e| CommandError::internal(e.to_string()))?)
                        .map_err(|e| CommandError::internal(e.to_string()))?;
                }
                boundaries.push((start, end, first, last));
                done += 1;
                progress((5 + done * 85 / count.max(1)) as u8);
            }
            writer
                .finalize()
                .map_err(|e| CommandError::internal(e.to_string()))?;
            let original = work.join("passage-original.wav");
            wave_render::render(
                &isolated,
                &wave_store::root(app)?,
                region.start_ms,
                region.end_ms,
                &original,
                settings,
                control.cancelled.clone(),
            )?;
            process_variants(
                app,
                p,
                settings,
                control,
                work,
                region,
                &original,
                &passage,
                &boundaries,
                &mut result,
                &mut bypass,
            )?;
        }
        control.boundary()?;
        if cleanup {
            result.without_cleanup = Some(bypass);
        } else {
            let clean = AudioComparison {
                sources: std::mem::replace(&mut result.sources, bypass.sources),
                replacements: std::mem::replace(&mut result.replacements, bypass.replacements),
                warnings: std::mem::replace(&mut result.warnings, bypass.warnings),
            };
            result.with_cleanup = Some(clean);
        }
        Ok(result)
    })
}
fn process_variants(
    app: &AppHandle,
    p: &Project,
    settings: &Settings,
    control: &JobControl,
    work: &Path,
    region: &super::wave_studio::VoiceRegion,
    original: &Path,
    passage: &Path,
    boundaries: &[(f64, f64, u64, u64)],
    result: &mut ProcessingResult,
    bypass: &mut AudioComparison,
) -> Result<(), CommandError> {
    for cleanup in [true, false] {
        control.boundary()?;
        let processed = work.join("matched.wav");
        let warnings = super::wave_voice_audio::match_passage(
            settings,
            control.cancelled.clone(),
            original,
            passage,
            &processed,
            cleanup,
        )?;
        let (sources, replacements, notes) = if cleanup {
            (
                &mut result.sources,
                &mut result.replacements,
                &mut result.warnings,
            )
        } else {
            (
                &mut bypass.sources,
                &mut bypass.replacements,
                &mut bypass.warnings,
            )
        };
        notes.extend(warnings);
        for &(start, end, first, last) in boundaries {
            let part = work.join("matched-part.wav");
            super::wave_voice_audio::transcode(
                settings,
                control.cancelled.clone(),
                &processed,
                &part,
                &format!("atrim=start_sample={first}:end_sample={last},asetpts=PTS-STARTPTS"),
            )?;
            let source = wave_store::import_controlled(
                app,
                &part,
                &format!(
                    "{} · matched{}",
                    region.name,
                    if cleanup { " · cleaned" } else { "" }
                ),
                settings,
                control.cancelled.clone(),
                Arc::new(|_| {}),
                &p.id,
            )?;
            replacements.push(Replacement {
                start_ms: start,
                end_ms: end,
                source_id: source.id.clone(),
                region_id: Some(region.id.clone()),
            });
            sources.push(source);
        }
    }
    Ok(())
}
/// Repairs the active converted passage using preserved original levels; no model/worker access.
pub fn match_existing(
    app: &AppHandle,
    p: &Project,
    region_id: &str,
    settings: &Settings,
    control: &JobControl,
    progress: &dyn Fn(u8),
) -> Result<ProcessingResult, CommandError> {
    wave_store::validate_sources(app, p)?;
    let region = p
        .timeline
        .voices
        .iter()
        .find(|r| {
            r.id == region_id
                && r.production
                    .as_ref()
                    .is_some_and(|v| v.status == "converted")
                && r.audio.is_some()
        })
        .ok_or_else(|| {
            CommandError::new(
                "NO_CONVERSION",
                "Select an applied conversion with preserved original audio.",
            )
        })?;
    stage(app, |work| {
        let mut isolated = p.clone();
        isolated.timeline.sfx.clear();
        isolated.timeline.voices.clear();
        isolated.voice_original = None;
        let current = work.join("current.wav");
        let original = work.join("original.wav");
        wave_render::render(
            &isolated,
            &wave_store::root(app)?,
            region.start_ms,
            region.end_ms,
            &current,
            settings,
            control.cancelled.clone(),
        )?;
        isolated.timeline.clips = conversion_clips(p, region);
        wave_render::render(
            &isolated,
            &wave_store::root(app)?,
            region.start_ms,
            region.end_ms,
            &original,
            settings,
            control.cancelled.clone(),
        )?;
        progress(25);
        let mut result = ProcessingResult {
            kind: "voices".into(),
            project_id: p.id.clone(),
            revision: p.revision,
            sources: vec![],
            replacements: vec![],
            warnings: vec![],
            without_cleanup: None,
            with_cleanup: None,
        };
        let mut bypass = AudioComparison {
            sources: vec![],
            replacements: vec![],
            warnings: vec![],
        };
        let chunks = ((region.end_ms - region.start_ms) / 15000.).ceil().max(1.) as usize;
        let boundaries = (0..chunks)
            .map(|i| {
                let start =
                    region.start_ms + (region.end_ms - region.start_ms) * i as f64 / chunks as f64;
                let end = region.start_ms
                    + (region.end_ms - region.start_ms) * (i + 1) as f64 / chunks as f64;
                (
                    start,
                    end,
                    ((start - region.start_ms) * 48.).round() as u64,
                    ((end - region.start_ms) * 48.).round() as u64,
                )
            })
            .collect::<Vec<_>>();
        process_variants(
            app,
            p,
            settings,
            control,
            work,
            region,
            &original,
            &current,
            &boundaries,
            &mut result,
            &mut bypass,
        )?;
        result.without_cleanup = Some(bypass);
        progress(95);
        Ok(result)
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conversion_uses_passage_original_instead_of_active_conversion() {
        use super::super::wave_studio::{Clip, Timeline, VoiceAudio, VoiceRegion};
        let original = Clip {
            id: uuid::Uuid::new_v4().to_string(),
            source_id: Some("original".into()),
            name: "Original".into(),
            start_ms: 0.,
            source_start_ms: 0.,
            source_end_ms: 1000.,
            speed: 1.,
            gain_db: 0.,
            fade_in_ms: 0.,
            fade_out_ms: 0.,
        };
        let mut converted = original.clone();
        converted.source_id = Some("converted".into());
        converted.start_ms = 2000.;
        let project = Project {
            schema_version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            name: "Fixture".into(),
            revision: 0,
            updated_at_ms: 0,
            sources: vec![],
            videos: vec![],
            view: None,
            voice_original: None,
            timeline: Timeline {
                clips: vec![converted],
                ..Default::default()
            },
        };
        let mut region = VoiceRegion {
            id: uuid::Uuid::new_v4().to_string(),
            voice_id: "john".into(),
            name: "John".into(),
            color: "#112233".into(),
            start_ms: 2000.,
            end_ms: 3000.,
            production: None,
            audio: Some(VoiceAudio {
                original: vec![original],
                versions: vec![],
                active_audio_key: "[]".into(),
            }),
        };
        let clips = conversion_clips(&project, &region);
        assert_eq!(clips[0].source_id.as_deref(), Some("original"));
        assert_eq!(clips[0].start_ms, 2000.);
        region.audio = None;
        assert_eq!(
            conversion_clips(&project, &region)[0].source_id.as_deref(),
            Some("converted")
        );
    }
    #[test]
    fn normalization_and_duration_guards() {
        assert!((normalization_gain(0.5).unwrap() - 5.0206).abs() < 0.001);
        assert_eq!(normalization_gain(0.01).unwrap(), 20.);
        assert!(normalization_gain(0.).is_err());
        assert!(tempo_for_duration(18000., 10000.).is_err());
        assert_eq!(tempo_for_duration(10500., 10000.).unwrap(), 1.05);
    }
}
