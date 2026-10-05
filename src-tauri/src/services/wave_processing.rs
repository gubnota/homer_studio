use super::{
    audio_assets,
    jobs::JobControl,
    project_store::CommandError,
    settings::Settings,
    sound_workers, speech, voice_store, wave_render, wave_store,
    wave_studio::{Project, Source},
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, sync::Arc, sync::atomic::AtomicBool};
use tauri::AppHandle;
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Replacement {
    pub start_ms: f64,
    pub end_ms: f64,
    pub source_id: String,
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
    Ok((-1. - 20. * peak.log10()).clamp(-96., 6.))
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
        wave_store::import(app, &out, "Joined narration", settings)
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
    wave_store::load(app, project_id)?;
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
        let source = wave_store::import(app, &out, "Generated speech", settings)?;
        control.boundary()?;
        Ok(ProcessingResult {
            kind: "speech".into(),
            project_id: project_id.into(),
            revision,
            sources: vec![source],
            replacements: vec![],
            warnings: vec![],
        })
    })
}
fn convert_chunk(
    app: &AppHandle,
    worker: &str,
    reference: &[u8],
    source: &Path,
    out: &Path,
    duration: f64,
    control: &JobControl,
) -> Result<(), CommandError> {
    sound_workers::recycle_before_job(app, worker, "chatterbox_original", control)?;
    let source_id = sound_workers::upload_reference(
        worker,
        &fs::read(source).map_err(|e| CommandError::io("Cannot read narration", e))?,
    )?;
    let reference_id = sound_workers::upload_reference(worker, reference)?;
    let bytes = sound_workers::generate(
        worker,
        &serde_json::json!({"prompt":"Convert recorded delivery","category":"voice_conversion","durationSeconds":duration/1000.,"seed":null,"sourceId":source_id,"referenceId":reference_id}),
        control,
    )?;
    fs::write(out, bytes).map_err(|e| CommandError::io("Cannot stage converted voice", e))
}
fn transcode(
    settings: &Settings,
    cancel: Arc<AtomicBool>,
    input: &Path,
    output: &Path,
    filter: Option<String>,
    mono: bool,
) -> Result<(), CommandError> {
    let mut args = vec![
        "-v".into(),
        "error".into(),
        "-nostdin".into(),
        "-y".into(),
        "-i".into(),
        input.to_string_lossy().into(),
    ];
    if let Some(filter) = filter {
        args.extend(["-af".into(), filter]);
    }
    args.extend([
        "-ac".into(),
        if mono { "1" } else { "2" }.into(),
        "-ar".into(),
        if mono { "24000" } else { "48000" }.into(),
        "-c:a".into(),
        if mono { "pcm_s16le" } else { "pcm_f32le" }.into(),
        output.to_string_lossy().into(),
    ]);
    audio_assets::ffmpeg(settings, args, cancel)
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
        .map(|r| {
            voice_store::selected_sample(app, &r.voice_id)?.ok_or_else(|| {
                CommandError::new(
                    "VOICE_HAS_NO_SAMPLE",
                    format!("{} needs a voice reference in Voice Lab.", r.name),
                )
            })
        })
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
            let span = region.end_ms - region.start_ms;
            let chunks = (span / 15000.).ceil().max(1.) as usize;
            let chunk_ms = span / chunks as f64;
            for index in 0..chunks {
                control.boundary()?;
                let start = region.start_ms + index as f64 * chunk_ms;
                let end = if index + 1 == chunks {
                    region.end_ms
                } else {
                    start + chunk_ms
                };
                let rendered = work.join("original.wav");
                let input = work.join("input.wav");
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
                transcode(
                    settings,
                    control.cancelled.clone(),
                    &rendered,
                    &input,
                    None,
                    true,
                )?;
                convert_chunk(
                    app,
                    &settings.sounds.original_url,
                    reference,
                    &input,
                    &output,
                    end - start,
                    control,
                )?;
                let actual = speech::probe_duration(&output, settings)? as f64;
                let ratio = tempo_for_duration(actual, end - start)?;
                let samples = ((end - start) * 48.).round() as u64;
                transcode(
                    settings,
                    control.cancelled.clone(),
                    &output,
                    &fitted,
                    Some(format!(
                        "atempo={ratio:.9},aresample=48000,apad,atrim=end_sample={samples}"
                    )),
                    false,
                )?;
                control.boundary()?;
                let source = wave_store::import(
                    app,
                    &fitted,
                    &format!("{} · converted", region.name),
                    settings,
                )?;
                result.replacements.push(Replacement {
                    start_ms: start,
                    end_ms: end,
                    source_id: source.id.clone(),
                });
                result.sources.push(source);
                done += 1;
                progress((5 + done * 85 / count.max(1)) as u8);
            }
        }
        control.boundary()?;
        Ok(result)
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalization_and_duration_guards() {
        assert!((normalization_gain(0.5).unwrap() - 5.0206).abs() < 0.001);
        assert_eq!(normalization_gain(0.01).unwrap(), 6.);
        assert!(normalization_gain(0.).is_err());
        assert!(tempo_for_duration(18000., 10000.).is_err());
        assert_eq!(tempo_for_duration(10500., 10000.).unwrap(), 1.05);
    }
}
