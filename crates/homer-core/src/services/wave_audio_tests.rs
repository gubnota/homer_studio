//! Real-FFmpeg regression coverage, using disposable projects and no AI worker.
use super::{
    jobs::JobControl,
    settings::Settings,
    wave_processing, wave_render, wave_store,
    wave_studio::{Clip, Project, VoiceAudio, VoiceProduction, VoiceRegion},
    wave_voice_audio as audio,
};
use crate::{
    commands::wave_studio::{wave_cancel_preview, wave_preview},
    runtime::AppHandle,
};
use std::{
    io::Cursor,
    path::{Path, PathBuf},
};
struct Fixture {
    root: PathBuf,
    app: AppHandle,
    project: Project,
    settings: Settings,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("homer-voice-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let app = AppHandle::new(root.join("data"), root.clone());
        let project =
            wave_store::create(&app, "Voice regression", Some(&root.join("project.wavehs")))
                .unwrap();
        Self {
            root,
            app,
            project,
            settings: Settings::default(),
        }
    }
    fn source(&mut self, name: &str, seconds: f64, amplitude: f64) -> Clip {
        let path = self.root.join(format!("{name}.wav"));
        write(&path, seconds, |i| speech(i) * amplitude);
        let s =
            wave_store::import(&self.app, &path, name, &self.settings, &self.project.id).unwrap();
        let c = Clip {
            id: uuid::Uuid::new_v4().to_string(),
            source_id: Some(s.id.clone()),
            name: name.into(),
            start_ms: 0.,
            source_start_ms: 0.,
            source_end_ms: s.duration_ms,
            speed: 1.,
            gain_db: 0.,
            fade_in_ms: 0.,
            fade_out_ms: 0.,
        };
        self.project.sources.push(s);
        c
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn speech(i: usize) -> f64 {
    let t = i as f64 / 48000.;
    let envelope = 0.4 + 0.6 * (t * 3.).sin().abs();
    envelope
        * ((t * 220. * std::f64::consts::TAU).sin()
            + 0.25 * (t * 710. * std::f64::consts::TAU).sin())
}
fn write(path: &Path, seconds: f64, f: impl Fn(usize) -> f64) {
    let mut w = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )
    .unwrap();
    for i in 0..(seconds * 48000.).round() as usize {
        let v = f(i) as f32;
        w.write_sample(v).unwrap();
        w.write_sample(v).unwrap();
    }
    w.finalize().unwrap();
}
fn samples(path: &Path) -> Vec<f32> {
    hound::WavReader::open(path)
        .unwrap()
        .samples::<f32>()
        .map(Result::unwrap)
        .collect()
}
#[test]
fn matching_preserves_duration_and_loudness_with_peak_ceiling_and_short_rms_fallback() {
    let f = Fixture::new();
    let original = f.root.join("original.wav");
    let input = f.root.join("input.wav");
    let out = f.root.join("out.wav");
    for seconds in [0.18, 3.] {
        write(&original, seconds, |i| 0.2 * speech(i));
        write(&input, seconds, |i| 0.015 * speech(i));
        audio::match_passage(
            &f.settings,
            Default::default(),
            &original,
            &input,
            &out,
            false,
        )
        .unwrap();
        let a = audio::levels(&original).unwrap();
        let b = audio::levels(&out).unwrap();
        assert_eq!(a.frames, b.frames);
        assert!((20. * (a.rms / b.rms).log10()).abs() < 0.2);
        let (a_lu, _) = audio::loudness(&f.settings, Default::default(), &original).unwrap();
        let (b_lu, tp) = audio::loudness(&f.settings, Default::default(), &out).unwrap();
        assert!(tp <= -0.99, "{tp}");
        if seconds > 0.4 {
            assert!((a_lu - b_lu).abs() < 1., "{a_lu} vs {b_lu}");
        }
    }
    // A transient requires limiting; a hot original can exceed the safe target.
    write(&original, 2., |i| 0.8 * speech(i));
    write(&input, 2., |i| {
        if i % 24000 < 50 {
            0.9
        } else {
            0.03 * speech(i)
        }
    });
    let warnings = audio::match_passage(
        &f.settings,
        Default::default(),
        &original,
        &input,
        &out,
        false,
    )
    .unwrap();
    assert_eq!(audio::levels(&out).unwrap().frames, 96000);
    assert!(
        audio::loudness(&f.settings, Default::default(), &out)
            .unwrap()
            .1
            <= -0.99
    );
    assert!(!warnings.is_empty());
    write(&input, 2., |_| 0.);
    assert_eq!(
        audio::match_passage(
            &f.settings,
            Default::default(),
            &original,
            &input,
            &out,
            true
        )
        .unwrap_err()
        .code,
        "SILENT_CONVERSION"
    );
}
#[test]
fn gentle_cleanup_reduces_hiss_without_changing_length() {
    let f = Fixture::new();
    let original = f.root.join("original.wav");
    let noisy = f.root.join("noisy.wav");
    let clean = f.root.join("cleaned.wav");
    let bypass = f.root.join("bypass.wav");
    let mut state = 123456789u32;
    let mut noise = vec![];
    for _ in 0..192000 {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        noise.push((state as f64 / u32::MAX as f64 - 0.5) * 0.008);
    }
    let voice = |i| {
        if (i / 48000) % 2 == 0 {
            0.12 * speech(i)
        } else {
            0.
        }
    };
    write(&original, 4., voice);
    write(&noisy, 4., |i| voice(i) + noise[i]);
    audio::match_passage(
        &f.settings,
        Default::default(),
        &original,
        &noisy,
        &clean,
        true,
    )
    .unwrap();
    audio::match_passage(
        &f.settings,
        Default::default(),
        &original,
        &noisy,
        &bypass,
        false,
    )
    .unwrap();
    let a = samples(&clean);
    let b = samples(&bypass);
    assert_eq!(a.len(), b.len());
    let energy = |v: &[f32]| {
        v[120000..180000]
            .iter()
            .map(|v| (*v as f64).powi(2))
            .sum::<f64>()
    };
    assert!(
        energy(&a) < energy(&b) * 0.9,
        "cleanup should reduce steady noise"
    );
    assert!(
        (audio::loudness(&f.settings, Default::default(), &clean)
            .unwrap()
            .0
            - audio::loudness(&f.settings, Default::default(), &original)
                .unwrap()
                .0)
            .abs()
            < 1.
    );
}
#[test]
fn normalization_measures_shared_rendered_peak_and_bakes_partial_envelopes() {
    let mut f = Fixture::new();
    let mut a = f.source("A", 2., 0.2);
    a.gain_db = -6.;
    a.fade_in_ms = 1400.;
    let mut b = f.source("B", 2., 0.1);
    b.start_ms = 2000.;
    let mut effect = f.source("Effect", 4., 0.5);
    effect.gain_db = 0.;
    f.project.timeline.clips = vec![a.clone(), b.clone()];
    f.project.timeline.sfx = vec![effect.clone()];
    let result =
        wave_processing::normalize_selection(&f.app, &f.project, &[], 500., 4000., &f.settings)
            .unwrap();
    assert_eq!(result.baked.len(), 1);
    assert_eq!(result.baked[0].0, a.id);
    assert_eq!(result.sources.len(), 1);
    let mut selected = f.project.clone();
    selected.timeline.sfx.clear();
    let before = f.root.join("before.wav");
    wave_render::render(
        &selected,
        &wave_store::root(&f.app).unwrap(),
        500.,
        4000.,
        &before,
        &f.settings,
        Default::default(),
    )
    .unwrap();
    assert!(
        (20. * audio::levels(&before).unwrap().peak.log10() + result.gain_db + 1.).abs() < 0.001
    );
    let baked = wave_store::source_path(&f.app, &result.sources[0].id).unwrap();
    let mut unity = f.project.clone();
    unity.timeline.clips = vec![a.clone()];
    unity.timeline.clips[0].gain_db = 0.;
    unity.timeline.sfx.clear();
    wave_render::render(
        &unity,
        &wave_store::root(&f.app).unwrap(),
        0.,
        2000.,
        &before,
        &f.settings,
        Default::default(),
    )
    .unwrap();
    assert_eq!(samples(&baked), samples(&before));
    let explicit = wave_processing::normalize_selection(
        &f.app,
        &f.project,
        &[effect.id],
        0.,
        4000.,
        &f.settings,
    )
    .unwrap();
    assert!(explicit.gain_db < result.gain_db);
    f.project.timeline.clips[0].gain_db = 20.;
    let limited =
        wave_processing::normalize_selection(&f.app, &f.project, &[], 0., 4000., &f.settings)
            .unwrap();
    assert!(limited.gain_db <= 0.);
}
#[tokio::test]
async fn twenty_second_existing_conversion_repairs_without_ai_and_preview_matches_render() {
    let mut f = Fixture::new();
    f.settings.sounds.original_url = "http://127.0.0.1:1".into();
    let original = f.source("Original", 20., 0.12);
    let converted = f.source("Converted", 20., 0.01);
    f.project.timeline.clips = vec![converted];
    let id = uuid::Uuid::new_v4().to_string();
    f.project.timeline.voices = vec![VoiceRegion {
        id: id.clone(),
        voice_id: "female-a".into(),
        name: "Female A".into(),
        color: "#112233".into(),
        start_ms: 0.,
        end_ms: 20000.,
        production: Some(VoiceProduction {
            status: "converted".into(),
            voice_id: "female-a".into(),
            audio_key: "[]".into(),
        }),
        audio: Some(VoiceAudio {
            original: vec![original],
            versions: vec![],
            active_audio_key: "[]".into(),
        }),
    }];
    let result = wave_processing::match_existing(
        &f.app,
        &f.project,
        &id,
        &f.settings,
        &JobControl::preview(),
        &|_| {},
    )
    .unwrap();
    assert_eq!(result.sources.len(), 2);
    assert_eq!(result.without_cleanup.as_ref().unwrap().sources.len(), 2);
    assert_eq!(result.replacements[0].end_ms, 10000.);
    assert_eq!(result.replacements[1].start_ms, 10000.);
    f.project.sources.extend(result.sources.clone());
    f.project.timeline.clips = result
        .replacements
        .iter()
        .map(|r| Clip {
            id: uuid::Uuid::new_v4().to_string(),
            source_id: Some(r.source_id.clone()),
            name: "Accepted".into(),
            start_ms: r.start_ms,
            source_start_ms: 0.,
            source_end_ms: r.end_ms - r.start_ms,
            speed: 1.,
            gain_db: 0.,
            fade_in_ms: 0.,
            fade_out_ms: 0.,
        })
        .collect();
    // Production metadata keys are opaque to native rendering, but their snapshots remain valid.
    let out = f.root.join("export.wav");
    wave_render::render(
        &f.project,
        &wave_store::root(&f.app).unwrap(),
        0.,
        20000.,
        &out,
        &f.settings,
        Default::default(),
    )
    .unwrap();
    let exported = samples(&out);
    assert!(audio::levels(&out).unwrap().rms > 0.04);
    for start in [0., 10000., 9500.] {
        let end = (start + 10000f64).min(20000.);
        let request = uuid::Uuid::new_v4().to_string();
        let bytes = wave_preview(f.app.clone(), f.project.clone(), start, end, Some(request))
            .await
            .unwrap()
            .0;
        let mut reader = hound::WavReader::new(Cursor::new(bytes)).unwrap();
        assert_eq!(reader.spec().sample_format, hound::SampleFormat::Int);
        let pcm = reader
            .samples::<i16>()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        assert_eq!(pcm.len(), ((end - start) * 96.) as usize);
        for (i, v) in pcm.iter().enumerate() {
            assert!((*v as f32 / 32768. - exported[start as usize * 96 + i]).abs() < 0.0001);
        }
    }
    let old = uuid::Uuid::new_v4().to_string();
    wave_cancel_preview(Some(old.clone()));
    assert!(
        wave_preview(f.app.clone(), f.project.clone(), 0., 10000., Some(old))
            .await
            .is_err()
    );
    let fresh = uuid::Uuid::new_v4().to_string();
    let prior = wave_preview(f.app.clone(), f.project.clone(), 0., 10000., Some(fresh))
        .await
        .unwrap()
        .0;
    f.project.timeline.clips[0].gain_db = -6.;
    let edited = wave_preview(
        f.app.clone(),
        f.project.clone(),
        0.,
        10000.,
        Some(uuid::Uuid::new_v4().to_string()),
    )
    .await
    .unwrap()
    .0;
    assert_ne!(prior, edited);
}
