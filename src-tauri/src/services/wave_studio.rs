use super::project_store::CommandError;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub name: String,
    pub duration_ms: f64,
    pub channels: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub id: String,
    pub source_id: Option<String>,
    pub name: String,
    pub start_ms: f64,
    pub source_start_ms: f64,
    pub source_end_ms: f64,
    pub speed: f64,
    pub gain_db: f64,
    pub fade_in_ms: f64,
    pub fade_out_ms: f64,
}
impl Clip {
    pub fn duration(&self) -> f64 {
        (self.source_end_ms - self.source_start_ms) / self.speed
    }
    pub fn end(&self) -> f64 {
        self.start_ms + self.duration()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceProduction {
    pub status: String,
    pub voice_id: String,
    pub audio_key: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceRegion {
    pub id: String,
    pub voice_id: String,
    pub name: String,
    pub color: String,
    pub start_ms: f64,
    pub end_ms: f64,
    #[serde(default)]
    pub production: Option<VoiceProduction>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Timeline {
    pub clips: Vec<Clip>,
    pub sfx: Vec<Clip>,
    pub voices: Vec<VoiceRegion>,
}
impl Timeline {
    pub fn duration(&self) -> f64 {
        self.clips
            .iter()
            .chain(&self.sfx)
            .map(Clip::end)
            .fold(0., f64::max)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub offset_ms: f64,
    pub span_ms: f64,
    pub playhead_ms: f64,
    pub selection: Option<[f64; 2]>,
    pub selected_id: Option<String>,
    #[serde(rename = "loop")]
    pub loop_: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub id: String,
    pub name: String,
    pub duration_ms: f64,
    pub start_ms: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub revision: u64,
    pub updated_at_ms: u64,
    pub sources: Vec<Source>,
    pub timeline: Timeline,
    #[serde(default)]
    pub view: Option<View>,
    #[serde(default)]
    pub voice_original: Option<Timeline>,
    #[serde(default)]
    pub videos: Vec<Video>,
}
pub fn validate(p: &Project) -> Result<(), CommandError> {
    let bad = || {
        CommandError::new(
            "INVALID_WAVE_PROJECT",
            "The timeline contains invalid or overlapping audio. Undo the last edit and try again.",
        )
    };
    super::audio_assets::id(&p.id)?;
    if p.schema_version != 1
        || p.name.trim().is_empty()
        || p.name.len() > 320
        || p.sources.len() > 4096
        || p.timeline.clips.len() + p.timeline.sfx.len() + p.timeline.voices.len() > 16384
    {
        return Err(bad());
    }
    if p.videos.len() > 128 {
        return Err(bad());
    }
    let mut video_ids = std::collections::HashSet::new();
    for v in &p.videos {
        super::audio_assets::id(&v.id)?;
        if !video_ids.insert(&v.id)
            || v.name.len() > 512
            || !v.duration_ms.is_finite()
            || v.duration_ms <= 0.
            || !v.start_ms.is_finite()
            || v.start_ms < 0.
            || v.start_ms + v.duration_ms > 86_400_000.
        {
            return Err(bad());
        }
    }
    let mut ids = std::collections::HashSet::new();
    for s in &p.sources {
        super::audio_assets::id(&s.id)?;
        if s.name.len() > 512
            || s.channels != 2
            || !ids.insert(&s.id)
            || !s.duration_ms.is_finite()
            || s.duration_ms <= 0.
            || s.duration_ms > 86_400_000.
        {
            return Err(bad());
        }
    }
    ids.clear();
    for c in p.timeline.clips.iter().chain(&p.timeline.sfx) {
        super::audio_assets::id(&c.id)?;
        if !ids.insert(&c.id)
            || [
                c.start_ms,
                c.source_start_ms,
                c.source_end_ms,
                c.speed,
                c.gain_db,
                c.fade_in_ms,
                c.fade_out_ms,
            ]
            .iter()
            .any(|x| !x.is_finite())
            || c.start_ms < 0.
            || c.source_start_ms < 0.
            || c.source_end_ms <= c.source_start_ms
            || !(0.85..=1.2).contains(&c.speed)
            || !(-96. ..=6.).contains(&c.gain_db)
            || c.fade_in_ms < 0.
            || c.fade_out_ms < 0.
            || c.fade_in_ms > c.duration() + 0.01
            || c.fade_out_ms > c.duration() + 0.01
            || c.end() > 86_400_000.
        {
            return Err(bad());
        }
        if let Some(id) = &c.source_id {
            if !p
                .sources
                .iter()
                .any(|s| s.id == *id && c.source_end_ms <= s.duration_ms + 1.)
            {
                return Err(bad());
            }
        }
    }
    let mut clips = p.timeline.clips.iter().collect::<Vec<_>>();
    clips.sort_by(|a, b| a.start_ms.total_cmp(&b.start_ms));
    if clips.windows(2).any(|w| w[0].end() > w[1].start_ms + 0.01) {
        return Err(bad());
    }
    for v in &p.timeline.voices {
        super::audio_assets::id(&v.id)?;
        if !ids.insert(&v.id)
            || !v.start_ms.is_finite()
            || !v.end_ms.is_finite()
            || v.start_ms < 0.
            || v.end_ms <= v.start_ms
            || v.end_ms > p.timeline.duration() + 1.
            || v.name.len() > 320
            || v.color.len() > 16
            || v.production.as_ref().is_some_and(|s| {
                !matches!(s.status.as_str(), "generated" | "converted")
                    || s.voice_id != v.voice_id
                    || s.audio_key.len() > 1024 * 1024
            })
        {
            return Err(bad());
        }
    }
    if let Some(v) = &p.view {
        if !v.offset_ms.is_finite()
            || v.offset_ms < 0.
            || !v.span_ms.is_finite()
            || !(250. ..=86_400_000.).contains(&v.span_ms)
            || !v.playhead_ms.is_finite()
            || v.playhead_ms < 0.
            || v.selection.is_some_and(|s| {
                !s[0].is_finite() || !s[1].is_finite() || s[0] < 0. || s[1] <= s[0]
            })
        {
            return Err(bad());
        }
    }
    if let Some(original) = &p.voice_original {
        let mut baseline = p.clone();
        baseline.timeline = original.clone();
        baseline.voice_original = None;
        baseline.view = None;
        validate(&baseline)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn project() -> Project {
        Project {
            schema_version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            name: "Narration".into(),
            revision: 0,
            updated_at_ms: 0,
            view: None,
            voice_original: None,
            videos: vec![],
            sources: vec![],
            timeline: Timeline {
                clips: vec![Clip {
                    id: uuid::Uuid::new_v4().to_string(),
                    source_id: None,
                    name: "Silence".into(),
                    start_ms: 0.,
                    source_start_ms: 0.,
                    source_end_ms: 500.,
                    speed: 1.,
                    gain_db: 0.,
                    fade_in_ms: 0.,
                    fade_out_ms: 0.,
                }],
                ..Default::default()
            },
        }
    }
    #[test]
    fn rejects_overlap_unsupported_schema_and_outside_annotations() {
        let mut p = project();
        assert!(validate(&p).is_ok());
        let mut c = p.timeline.clips[0].clone();
        c.id = uuid::Uuid::new_v4().to_string();
        c.start_ms = 100.;
        p.timeline.clips.push(c);
        assert!(validate(&p).is_err());
        p.timeline.clips.pop();
        p.schema_version = 2;
        assert!(validate(&p).is_err());
        p.schema_version = 1;
        p.timeline.voices.push(VoiceRegion {
            id: uuid::Uuid::new_v4().to_string(),
            voice_id: "narrator".into(),
            name: "Narrator".into(),
            color: "#718392".into(),
            start_ms: 0.,
            end_ms: 600.,
            production: None,
        });
        assert!(validate(&p).is_err());
    }
    #[test]
    fn rejects_nonfinite_speed_and_missing_immutable_source() {
        let mut p = project();
        p.timeline.clips[0].speed = f64::NAN;
        assert!(validate(&p).is_err());
        p.timeline.clips[0].speed = 1.;
        p.timeline.clips[0].source_id = Some(uuid::Uuid::new_v4().to_string());
        assert!(validate(&p).is_err());
    }
    #[test]
    fn legacy_projects_and_saved_view_and_original_roundtrip() {
        let mut p = project();
        let mut legacy = serde_json::to_value(&p).unwrap();
        legacy.as_object_mut().unwrap().remove("view");
        legacy.as_object_mut().unwrap().remove("voiceOriginal");
        legacy.as_object_mut().unwrap().remove("videos");
        let old: Project = serde_json::from_value(legacy).unwrap();
        assert!(old.view.is_none() && old.voice_original.is_none() && old.videos.is_empty());
        p.view = Some(View {
            offset_ms: 125.,
            span_ms: 1000.,
            playhead_ms: 250.,
            selection: Some([100., 300.]),
            selected_id: Some(p.timeline.clips[0].id.clone()),
            loop_: true,
        });
        p.voice_original = Some(p.timeline.clone());
        let restored: Project = serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
        assert!(validate(&restored).is_ok());
        assert!(restored.view.unwrap().loop_);
        assert_eq!(restored.voice_original.unwrap().clips.len(), 1);
        p.view.as_mut().unwrap().span_ms = 0.;
        assert!(validate(&p).is_err());
    }
    #[test]
    fn validates_video_bounds_and_production_roundtrip() {
        let mut p = project();
        p.videos.push(Video {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Reference".into(),
            start_ms: 0.,
            duration_ms: 1000.,
        });
        p.timeline.voices.push(VoiceRegion {
            id: uuid::Uuid::new_v4().to_string(),
            voice_id: "narrator".into(),
            name: "Narrator".into(),
            color: "#8d79b8".into(),
            start_ms: 0.,
            end_ms: 500.,
            production: Some(VoiceProduction {
                status: "converted".into(),
                voice_id: "narrator".into(),
                audio_key: "[]".into(),
            }),
        });
        let restored: Project = serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
        assert!(validate(&restored).is_ok());
        assert_eq!(
            restored.timeline.voices[0]
                .production
                .as_ref()
                .unwrap()
                .status,
            "converted"
        );
        p.videos[0].start_ms = 86_400_000.;
        assert!(validate(&p).is_err());
        p.videos[0].start_ms = 0.;
        p.timeline.voices[0].production.as_mut().unwrap().voice_id = "different".into();
        assert!(validate(&p).is_err());
    }
}
