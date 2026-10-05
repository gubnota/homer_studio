use super::{
    audio_assets::{self, AudioComposition, AudioVariant},
    project_store::CommandError,
    settings::Settings,
};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioEdit {
    pub kind: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub value: f64,
}
pub fn duration(value: &AudioComposition) -> u64 {
    value
        .clips
        .iter()
        .enumerate()
        .map(|(i, c)| {
            (c.end_ms - c.start_ms).saturating_sub(if i == 0 { 0 } else { c.crossfade_ms })
        })
        .sum()
}
pub fn validate(value: &AudioComposition, takes: &[AudioVariant]) -> Result<(), CommandError> {
    if value.clips.is_empty() || value.clips.len() > 4096 {
        return Err(CommandError::new(
            "INVALID_COMPOSITION",
            "A composition needs 1–4096 clips.",
        ));
    }
    for (i, c) in value.clips.iter().enumerate() {
        let length = c.end_ms.saturating_sub(c.start_ms);
        if length == 0
            || !c.gain.is_finite()
            || !(0.0..=8.0).contains(&c.gain)
            || c.fade_in_ms + c.fade_out_ms > length
            || (i == 0 && c.crossfade_ms != 0)
            || c.crossfade_ms > length / 2
            || (i > 0
                && c.crossfade_ms > (value.clips[i - 1].end_ms - value.clips[i - 1].start_ms) / 2)
        {
            return Err(CommandError::new(
                "INVALID_COMPOSITION",
                "Invalid clip range, gain, fade or crossfade.",
            ));
        }
        if let Some(id) = &c.asset_id {
            let source = takes.iter().find(|t| &t.id == id).ok_or_else(|| {
                CommandError::new("MISSING_AUDIO_SOURCE", "A source take is missing.")
            })?;
            if c.end_ms > source.duration_ms {
                return Err(CommandError::new(
                    "INVALID_AUDIO_RANGE",
                    "Clip exceeds its source.",
                ));
            }
        }
    }
    Ok(())
}
pub fn edit(take: &AudioVariant, request: &AudioEdit) -> Result<AudioComposition, CommandError> {
    let start = request.start_ms;
    let end = request.end_ms;
    let length = take.duration_ms;
    if start > length || end > length || end < start || !request.value.is_finite() {
        return Err(CommandError::new(
            "INVALID_AUDIO_RANGE",
            "Choose a valid audio range.",
        ));
    }
    let make = |a, b| audio_assets::clip(Some(take.id.clone()), a, b);
    let mut clips = Vec::new();
    match request.kind.as_str() {
        "trim" => {
            if end > start {
                clips.push(make(start, end));
            }
        }
        "delete" | "silence" | "insert_silence" | "split" | "shorten_silence" => {
            if start > 0 {
                clips.push(make(0, start));
            }
            match request.kind.as_str() {
                "silence" => {
                    if end > start {
                        clips.push(audio_assets::clip(None, 0, end - start));
                    }
                }
                "insert_silence" => {
                    if !(1.0..=3_600_000.).contains(&request.value) {
                        return Err(CommandError::new(
                            "INVALID_SILENCE",
                            "Silence must be 1 ms–1 hour.",
                        ));
                    }
                    clips.push(audio_assets::clip(None, 0, request.value as u64));
                }
                "split" => {
                    if end > start {
                        clips.push(make(start, end));
                    }
                }
                "shorten_silence" => {
                    if request.value < 0. || request.value > (end - start) as f64 {
                        return Err(CommandError::new(
                            "INVALID_SILENCE",
                            "The shortened silence must fit the selection.",
                        ));
                    }
                    if request.value > 0. {
                        clips.push(audio_assets::clip(None, 0, request.value as u64));
                    }
                }
                _ => {}
            }
            let tail = if request.kind == "insert_silence" {
                start
            } else {
                end
            };
            if tail < length {
                clips.push(make(tail, length));
            }
        }
        "fade" | "gain" => {
            if start > 0 {
                clips.push(make(0, start));
            }
            if end > start {
                let mut c = make(start, end);
                if request.kind == "gain" {
                    c.gain = request.value as f32;
                } else {
                    let fade = request.value.max(0.) as u64;
                    c.fade_in_ms = fade;
                    c.fade_out_ms = fade;
                }
                clips.push(c);
            }
            if end < length {
                clips.push(make(end, length));
            }
        }
        _ => return Err(CommandError::new("INVALID_EDIT", "Unknown audio edit.")),
    }
    let result = AudioComposition { clips };
    validate(&result, std::slice::from_ref(take))?;
    Ok(result)
}
pub fn replacement(
    source: &AudioVariant,
    replacement: &AudioVariant,
    start: u64,
    end: u64,
    crossfade: u64,
) -> Result<AudioComposition, CommandError> {
    if end <= start || end > source.duration_ms || crossfade > 1000 {
        return Err(CommandError::new(
            "INVALID_AUDIO_RANGE",
            "Choose a valid replacement and crossfade up to one second.",
        ));
    }
    // Extend adjacent clips into the selected area: true overlap preserves the unselected timeline.
    let left = crossfade
        .min(start)
        .min((end - start) / 2)
        .min(replacement.duration_ms / 2);
    let right = crossfade
        .min(source.duration_ms - end)
        .min((end - start) / 2)
        .min(replacement.duration_ms.saturating_sub(left) / 2);
    let mut clips = vec![];
    if start > 0 {
        clips.push(audio_assets::clip(Some(source.id.clone()), 0, start + left));
    }
    let mut middle = audio_assets::clip(Some(replacement.id.clone()), 0, replacement.duration_ms);
    middle.crossfade_ms = if clips.is_empty() { 0 } else { left };
    clips.push(middle);
    if end < source.duration_ms {
        let mut c = audio_assets::clip(Some(source.id.clone()), end - right, source.duration_ms);
        c.crossfade_ms = right;
        clips.push(c);
    }
    let value = AudioComposition { clips };
    validate(&value, &[source.clone(), replacement.clone()])?;
    Ok(value)
}
// Carry sentence intervals through trims, insertions and measured replacements.
// Multiple surviving pieces of a sentence are represented by their bounding interval.
pub fn remap_cues(
    composition: &AudioComposition,
    takes: &[AudioVariant],
) -> Vec<super::project_store::LineCue> {
    let mut cues: Vec<super::project_store::LineCue> = vec![];
    let mut offset = 0_u64;
    for (i, clip) in composition.clips.iter().enumerate() {
        if i > 0 {
            offset = offset.saturating_sub(clip.crossfade_ms);
        }
        if let Some(source) = takes
            .iter()
            .find(|take| Some(&take.id) == clip.asset_id.as_ref())
        {
            for cue in &source.cues {
                let start = cue.start_ms.max(clip.start_ms);
                let end = cue.end_ms.min(clip.end_ms);
                if end <= start {
                    continue;
                }
                let mapped_start = offset + start - clip.start_ms;
                let mapped_end = offset + end - clip.start_ms;
                if let Some(existing) = cues
                    .iter_mut()
                    .find(|c| c.order == cue.order && c.text == cue.text)
                {
                    existing.start_ms = existing.start_ms.min(mapped_start);
                    existing.end_ms = existing.end_ms.max(mapped_end);
                } else {
                    cues.push(super::project_store::LineCue {
                        order: cue.order,
                        text: cue.text.clone(),
                        start_ms: mapped_start,
                        end_ms: mapped_end,
                    });
                }
            }
        }
        offset += clip.end_ms - clip.start_ms;
    }
    cues.sort_by_key(|cue| cue.start_ms);
    cues
}
pub fn replacement_cues(
    source: &AudioVariant,
    start: u64,
    end: u64,
    measured: u64,
) -> Vec<super::project_store::LineCue> {
    source
        .cues
        .iter()
        .filter_map(|cue| {
            let a = cue.start_ms.max(start);
            let z = cue.end_ms.min(end);
            if z <= a || end <= start {
                return None;
            }
            Some(super::project_store::LineCue {
                order: cue.order,
                text: cue.text.clone(),
                start_ms: ((a - start) as f64 * measured as f64 / (end - start) as f64).round()
                    as u64,
                end_ms: ((z - start) as f64 * measured as f64 / (end - start) as f64).round()
                    as u64,
            })
        })
        .collect()
}
pub fn render(
    base: &Path,
    composition: &AudioComposition,
    takes: &[AudioVariant],
    output: &Path,
    settings: &Settings,
    cancel: Arc<AtomicBool>,
) -> Result<u64, CommandError> {
    validate(composition, takes)?;
    let mut args = vec!["-v".into(), "error".into(), "-nostdin".into(), "-y".into()];
    let mut filters = vec![];
    for (i, c) in composition.clips.iter().enumerate() {
        let seconds = (c.end_ms - c.start_ms) as f64 / 1000.;
        if let Some(id) = &c.asset_id {
            args.extend([
                "-i".into(),
                audio_assets::path(base, id)?.to_string_lossy().into(),
            ]);
        } else {
            args.extend([
                "-f".into(),
                "lavfi".into(),
                "-t".into(),
                format!("{seconds:.6}"),
                "-i".into(),
                "anullsrc=r=48000:cl=mono".into(),
            ]);
        }
        let begin = if c.asset_id.is_some() {
            c.start_ms as f64 / 1000.
        } else {
            0.
        };
        let mut filter = format!(
            "[{i}:a]atrim=start={begin:.6}:duration={seconds:.6},asetpts=PTS-STARTPTS,aformat=sample_fmts=flt:sample_rates=48000:channel_layouts=mono,volume={}",
            c.gain
        );
        if c.fade_in_ms > 0 {
            filter.push_str(&format!(",afade=t=in:d={:.6}", c.fade_in_ms as f64 / 1000.));
        }
        if c.fade_out_ms > 0 {
            filter.push_str(&format!(
                ",afade=t=out:st={:.6}:d={:.6}",
                seconds - c.fade_out_ms as f64 / 1000.,
                c.fade_out_ms as f64 / 1000.
            ));
        }
        filters.push(format!("{filter}[c{i}]"));
    }
    let mut current = "c0".to_string();
    for (i, c) in composition.clips.iter().enumerate().skip(1) {
        let target = format!("j{i}");
        let operation = if c.crossfade_ms > 0 {
            format!(
                "acrossfade=d={:.6}:c1=tri:c2=tri",
                c.crossfade_ms as f64 / 1000.
            )
        } else {
            "concat=n=2:v=0:a=1".into()
        };
        filters.push(format!("[{current}][c{i}]{operation}[{target}]"));
        current = target;
    }
    args.extend([
        "-filter_complex".into(),
        filters.join(";"),
        "-map".into(),
        format!("[{current}]"),
        "-c:a".into(),
        "pcm_f32le".into(),
        output.to_string_lossy().into(),
    ]);
    audio_assets::ffmpeg(settings, args, cancel)?;
    let measured = super::speech::probe_duration(output, settings)?;
    if measured.abs_diff(duration(composition)) > 10 {
        return Err(CommandError::new("INVALID_RENDER_DURATION", "The rendered audio length does not match the edit."));
    }
    Ok(measured)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn take() -> AudioVariant {
        AudioVariant {
            favorite: false, notes: String::new(),
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            name: "original".into(),
            path: "".into(),
            duration_ms: 1000,
            sample_rate: 48000,
            original_sample_rate: None,
            channels: 1,
            created_at_ms: 0,
            state: "source".into(),
            composition: AudioComposition { clips: vec![] },
            processing: None,
            cues: vec![],
        }
    }
    #[test]
    fn edits_keep_sources_and_validate_bounds() {
        let t = take();
        let e = AudioEdit {
            kind: "delete".into(),
            start_ms: 200,
            end_ms: 600,
            value: 0.,
        };
        let c = edit(&t, &e).unwrap();
        assert_eq!(duration(&c), 600);
        assert!(c.clips.iter().all(|c| c.asset_id.as_ref() == Some(&t.id)));
        assert_eq!(t.duration_ms, 1000);
        assert!(edit(&t, &AudioEdit { end_ms: 2000, ..e }).is_err());
    }
    #[test]
    fn overlapping_replacement_preserves_unselected_duration() {
        let s = take();
        let mut r = take();
        r.duration_ms = 300;
        let c = replacement(&s, &r, 200, 600, 20).unwrap();
        assert_eq!(duration(&c), 900);
        assert_eq!(c.clips[1].crossfade_ms, 20);
    }
    #[test]
    fn empty_or_excessive_gain_is_rejected() {
        let t = take();
        assert!(
            edit(
                &t,
                &AudioEdit {
                    kind: "delete".into(),
                    start_ms: 0,
                    end_ms: 1000,
                    value: 0.
                }
            )
            .is_err()
        );
        assert!(
            edit(
                &t,
                &AudioEdit {
                    kind: "gain".into(),
                    start_ms: 0,
                    end_ms: 1000,
                    value: 20.
                }
            )
            .is_err()
        );
    }
    #[test]
    fn sentence_times_follow_measured_replacement_and_deleted_audio() {
        let mut source = take();
        source.cues = vec![
            super::super::project_store::LineCue {
                order: 0,
                text: "First".into(),
                start_ms: 0,
                end_ms: 400,
            },
            super::super::project_store::LineCue {
                order: 1,
                text: "Second".into(),
                start_ms: 400,
                end_ms: 1000,
            },
        ];
        let mut converted = take();
        converted.duration_ms = 300;
        converted.cues = replacement_cues(&source, 400, 1000, 300);
        let composition = replacement(&source, &converted, 400, 1000, 0).unwrap();
        let cues = remap_cues(&composition, &[source.clone(), converted]);
        assert_eq!(cues[1].start_ms, 400);
        assert_eq!(cues[1].end_ms, 700);
        let deleted = edit(
            &source,
            &AudioEdit {
                kind: "delete".into(),
                start_ms: 200,
                end_ms: 600,
                value: 0.,
            },
        )
        .unwrap();
        let cues = remap_cues(&deleted, &[source]);
        assert_eq!(cues[0].end_ms, 200);
        assert_eq!(cues[1].start_ms, 200);
        assert_eq!(cues[1].end_ms, 600);
    }
    #[test]
    fn ffmpeg_renders_replacement_and_silence_without_changing_sources() {
        let settings = Settings::default();
        let base = std::env::temp_dir().join(format!("Homer edit’s {}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(base.join("assets")).unwrap();
        let source = take();
        let mut recorded = take();
        recorded.duration_ms = 300;
        for variant in [&source, &recorded] {
            let path = audio_assets::path(&base, &variant.id).unwrap();
            audio_assets::ffmpeg(
                &settings,
                vec![
                    "-v".into(),
                    "error".into(),
                    "-nostdin".into(),
                    "-y".into(),
                    "-f".into(),
                    "lavfi".into(),
                    "-i".into(),
                    format!(
                        "sine=frequency=440:sample_rate=48000:duration={}",
                        variant.duration_ms as f64 / 1000.
                    ),
                    "-c:a".into(),
                    "pcm_f32le".into(),
                    path.to_string_lossy().into(),
                ],
                Default::default(),
            )
            .unwrap();
        }
        let original = std::fs::read(audio_assets::path(&base, &source.id).unwrap()).unwrap();
        let composition = replacement(&source, &recorded, 200, 600, 20).unwrap();
        let measured = render(
            &base,
            &composition,
            &[source.clone(), recorded],
            &base.join("retake.wav"),
            &settings,
            Default::default(),
        )
        .unwrap();
        assert!(measured.abs_diff(duration(&composition)) <= 2);
        let silence = edit(
            &source,
            &AudioEdit {
                kind: "silence".into(),
                start_ms: 200,
                end_ms: 600,
                value: 0.,
            },
        )
        .unwrap();
        assert_eq!(
            render(
                &base,
                &silence,
                &[source.clone()],
                &base.join("silenced.wav"),
                &settings,
                Default::default()
            )
            .unwrap(),
            1000
        );
        assert_eq!(
            original,
            std::fs::read(audio_assets::path(&base, &source.id).unwrap()).unwrap()
        );
        std::fs::remove_dir_all(base).unwrap();
    }
}
