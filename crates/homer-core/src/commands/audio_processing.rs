#[cfg(feature = "desktop")]
use crate::runtime::Manager;
use crate::runtime::{AppHandle, State};
use crate::{
    AppState,
    services::{
        audio_assets::{self, ProcessingHistory},
        audio_processors::{self, EngineConfig},
        memo_store,
        project_store::{self, CommandError},
        settings, sound_store,
        voice_profiles::{self, VoiceProfile},
        voice_store,
    },
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::fs;
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn audio_engine_configs(app: AppHandle) -> Result<Vec<EngineConfig>, CommandError> {
    audio_processors::configurations(&app)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn save_audio_engine(
    app: AppHandle,
    state: State<'_, AppState>,
    config: EngineConfig,
) -> Result<(), CommandError> {
    let _lock = state
        .project_write_lock
        .lock()
        .map_err(|_| CommandError::internal("Library lock unavailable"))?;
    audio_processors::save(&app, config)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub async fn audio_engine_status(
    app: AppHandle,
    config: EngineConfig,
) -> Result<Value, CommandError> {
    audio_processors::validate(&config)?;
    crate::runtime::async_runtime::spawn_blocking(move || audio_processors::status(&app, &config))
        .await
        .map_err(|e| CommandError::internal(e.to_string()))
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn setup_audio_engine(
    app: AppHandle,
    state: State<'_, AppState>,
    engine: String,
    python_path: String,
) -> Result<String, CommandError> {
    Ok(state.jobs.enqueue_with_report("audio_setup",format!("Set up {engine}"),move|control,_,report|{control.boundary()?;report("Installing Python dependencies in an isolated environment; model files are selected separately.".into());audio_processors::setup(&app,&engine,&python_path,control.cancelled)}))
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn list_voice_profiles(app: AppHandle) -> Result<Vec<VoiceProfile>, CommandError> {
    voice_profiles::list(&app)
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn save_voice_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    profile: VoiceProfile,
) -> Result<VoiceProfile, CommandError> {
    let _lock = state
        .project_write_lock
        .lock()
        .map_err(|_| CommandError::internal("Library lock unavailable"))?;
    voice_profiles::save(&app, profile)
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingRequest {
    memo_id: String,
    expected_revision: u64,
    engine: String,
    operation: String,
    preset: String,
    profile_id: Option<String>,
    start_ms: u64,
    end_ms: u64,
    crossfade_ms: u64,
    params: Value,
    preprocess: bool,
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn process_memo_audio(
    app: AppHandle,
    state: State<'_, AppState>,
    mut request: ProcessingRequest,
) -> Result<String, CommandError> {
    let memo = memo_store::load(&app, &request.memo_id)?;
    memo_store::revision(&memo, request.expected_revision)?;
    let take = memo_store::selected(&memo)?;
    if memo.deleted
        || request.start_ms >= request.end_ms
        || request.end_ms > take.duration_ms
        || !audio_processors::ENGINES.contains(&request.engine.as_str())
        || !["convert", "enhance", "denoise"].contains(&request.operation.as_str())
    {
        return Err(CommandError::new(
            "INVALID_PROCESSING",
            "Choose a valid engine, operation and audio range.",
        ));
    }
    if (request.engine == "seed_vc" || request.engine == "rvc") && request.operation != "convert" {
        return Err(CommandError::new(
            "INVALID_PROCESSING",
            "This engine supports voice conversion.",
        ));
    }
    if (request.engine == "deepfilternet" || request.engine == "resemble_enhance")
        && request.operation == "convert"
    {
        return Err(CommandError::new(
            "INVALID_PROCESSING",
            "Choose cleanup for this engine.",
        ));
    }
    if !request.params.is_object() {
        return Err(CommandError::new(
            "INVALID_PROCESSING",
            "Parameters must be an object.",
        ));
    }
    let mut parameters = serde_json::Map::new();
    if let Some(config) = audio_processors::configurations(&app)?
        .into_iter()
        .find(|c| c.engine == request.engine)
    {
        if let Some(options) = config.options.as_object() {
            parameters.extend(options.clone());
        }
    }
    if let Some(id) = &request.profile_id {
        let profile = voice_profiles::list(&app)?
            .into_iter()
            .find(|p| &p.id == id)
            .ok_or_else(|| CommandError::new("VOICE_NOT_FOUND", "Voice profile unavailable."))?;
        if let Some(options) = profile
            .engine_options
            .get(&request.engine)
            .and_then(|v| v.as_object())
        {
            parameters.extend(options.clone());
        }
    }
    parameters.extend(request.params.as_object().unwrap().clone());
    request.params = Value::Object(parameters);
    Ok(state.jobs.enqueue_with_report("audio_processing",format!("{} · {}",request.engine,request.preset),move|control,_,report|{
  control.boundary()?;let config=audio_processors::configurations(&app)?.into_iter().find(|c|c.engine==request.engine).ok_or_else(||CommandError::new("ENGINE_NOT_FOUND","Configure the selected engine."))?;
  let snapshot=memo_store::load(&app,&request.memo_id)?;memo_store::revision(&snapshot,request.expected_revision)?;let source=memo_store::selected_path(&app,&snapshot)?;let settings=settings::load(&app)?;
  let staging=audio_assets::root(&app)?.join(format!(".processing-{}",uuid::Uuid::new_v4()));fs::create_dir_all(&staging).map_err(|e|CommandError::io("Cannot create processing workspace",e))?;
  let result=(||{
   let selected=staging.join("selection.wav");audio_assets::ffmpeg(&settings,vec!["-v".into(),"error".into(),"-nostdin".into(),"-y".into(),"-i".into(),source.to_string_lossy().into(),"-af".into(),format!("atrim=start={}:end={},asetpts=PTS-STARTPTS",request.start_ms as f64/1000.,request.end_ms as f64/1000.),"-c:a".into(),"pcm_f32le".into(),selected.to_string_lossy().into()],control.cancelled.clone())?;
   let mut input=selected.clone();if request.preprocess{let clean=audio_processors::configurations(&app)?.into_iter().find(|c|c.engine=="deepfilternet").unwrap();let cleaned=staging.join("preclean.wav");audio_processors::invoke(&app,&clean,json!({"protocol":1,"action":"process","config":clean,"operation":"denoise","params":{"attenuation":12},"sourcePath":input,"outputPath":cleaned}),control.cancelled.clone(),&report)?;input=cleaned;}
   let output=staging.join("processed.wav");let mut frame=json!({"protocol":1,"action":"process","config":config,"operation":request.operation,"params":request.params,"sourcePath":input,"outputPath":output});
   if let Some(id)=&request.profile_id{let profile=voice_profiles::list(&app)?.into_iter().find(|p|&p.id==id).ok_or_else(||CommandError::new("VOICE_NOT_FOUND","Voice profile unavailable."))?;if let Some(memo_id)=profile.selected_reference_memo_id{let reference=memo_store::load(&app,&memo_id)?;if reference.deleted||!matches!(memo_store::selected(&reference)?.state.as_str(),"source"|"accepted"){return Err(CommandError::new("INVALID_REFERENCE","Restore the reference recording first."));}frame["referencePath"]=json!(memo_store::selected_path(&app,&reference)?);}else if request.engine=="seed_vc"{let voice=voice_store::list(&app)?.into_iter().find(|v|&v.id==id).ok_or_else(||CommandError::new("VOICE_NOT_FOUND","The reference voice was removed."))?;if let Some(sample)=voice.selected_sample_id{frame["referencePath"]=json!(voice_store::sample_path(&app,id,&sample)?);}}
    frame["modelPath"]=json!(profile.model_path);frame["indexPath"]=json!(profile.index_path);}
   let result=audio_processors::invoke(&app,&config,frame,control.cancelled.clone(),&report)?;control.boundary()?;
   let state=app.state::<AppState>();let _lock=state.project_write_lock.lock().map_err(|_|CommandError::internal("Library lock unavailable"))?;
   memo_store::publish_processed(&app,&request.memo_id,request.expected_revision,&output,ProcessingHistory{engine:request.engine.clone(),version:result["version"].as_str().unwrap_or("unknown").into(),preset:request.preset.clone(),params:json!({"parameters":request.params,"preprocess":request.preprocess,"config":config}),profile_id:request.profile_id.clone(),source_start_ms:request.start_ms,source_end_ms:request.end_ms,created_at_ms:sound_store::now_ms(),backend:result["backend"].as_str().unwrap_or("cpu").into()},request.start_ms,request.end_ms,request.crossfade_ms,&settings,control.cancelled.clone())?;Ok(())})();let _=fs::remove_dir_all(staging);result
 }))
}
// Publishing takes a copy of the accepted render. Project and sound exports then
// consume exactly the selected audio, including measured processing duration.
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn publish_memo_audio(
    app: AppHandle,
    state: State<'_, AppState>,
    memo_id: String,
    expected_revision: u64,
    target: String,
    root_path: Option<String>,
    project_revision: Option<u64>,
    chapter_id: Option<String>,
    segment_id: Option<String>,
    voice_id: Option<String>,
) -> Result<String, CommandError> {
    let memo = memo_store::load(&app, &memo_id)?;
    memo_store::revision(&memo, expected_revision)?;
    let take = memo_store::selected(&memo)?.clone();
    if memo.deleted || !matches!(take.state.as_str(), "accepted" | "source") {
        return Err(CommandError::new(
            "UNACCEPTED_AUDIO",
            "Accept the audio before publishing.",
        ));
    }
    let source = memo_store::selected_path(&app, &memo)?;
    Ok(state.jobs.enqueue(
        "audio_publish",
        format!("Send recording to {target}"),
        move |control, _| {
            control.boundary()?;
            let staging =
                audio_assets::root(&app)?.join(format!(".publish-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&staging)
                .map_err(|e| CommandError::io("Cannot stage recording", e))?;
            let result = (|| {
                let master = staging.join("master.wav");
                fs::copy(source, &master)
                    .map_err(|e| CommandError::io("Cannot copy accepted audio", e))?;
                let state = app.state::<AppState>();
                let _lock = state
                    .project_write_lock
                    .lock()
                    .map_err(|_| CommandError::internal("Library lock unavailable"))?;
                memo_store::revision(&memo_store::load(&app, &memo_id)?, expected_revision)?;
                control.boundary()?;
                match target.as_str() {
                    "chapter" | "segment" => {
                        let root = root_path.as_deref().ok_or_else(|| {
                            CommandError::new("PROJECT_REQUIRED", "Choose an open book.")
                        })?;
                        let revision = project_revision.ok_or_else(|| {
                            CommandError::new("PROJECT_REQUIRED", "Project revision unavailable.")
                        })?;
                        let chapter = chapter_id.as_deref().ok_or_else(|| {
                            CommandError::new("CHAPTER_REQUIRED", "Choose a chapter.")
                        })?;
                        if target == "segment" {
                            project_store::commit_segment_take(
                                root,
                                revision,
                                chapter,
                                segment_id.as_deref().ok_or_else(|| {
                                    CommandError::new("SEGMENT_REQUIRED", "Choose a section.")
                                })?,
                                &master,
                                take.duration_ms,
                                true,
                            )?;
                        } else {
                            project_store::commit_chapter_audio(
                                root,
                                revision,
                                chapter,
                                &master,
                                take.duration_ms,
                                "recording",
                                if memo.context.chapter_id.as_deref() == Some(chapter) {
                                    take.cues.clone()
                                } else {
                                    vec![]
                                },
                            )?;
                        }
                    }
                    "voice" => {
                        voice_store::add_sample(
                            &app,
                            voice_id.as_deref().ok_or_else(|| {
                                CommandError::new("VOICE_REQUIRED", "Choose a custom voice.")
                            })?,
                            &memo.name,
                            &master,
                            &settings::load(&app)?,
                        )?;
                    }
                    "sound" => {
                        audio_assets::ffmpeg(
                            &settings::load(&app)?,
                            vec![
                                "-v".into(),
                                "error".into(),
                                "-nostdin".into(),
                                "-y".into(),
                                "-i".into(),
                                master.to_string_lossy().into(),
                                "-c:a".into(),
                                "aac".into(),
                                staging.join("preview.m4a").to_string_lossy().into(),
                            ],
                            control.cancelled.clone(),
                        )?;
                        control.boundary()?;
                        let id = uuid::Uuid::new_v4().to_string();
                        sound_store::publish(
                            &app,
                            sound_store::SoundAsset {
                                id: id.clone(),
                                prompt: memo.name.clone(),
                                category: "voice_conversion".into(),
                                provider: "audio_studio".into(),
                                model: take
                                    .processing
                                    .as_ref()
                                    .map(|p| p.engine.clone())
                                    .unwrap_or("recording".into()),
                                requested_duration_seconds: take.duration_ms as f32 / 1000.,
                                duration_ms: take.duration_ms,
                                seed: None,
                                created_at_ms: sound_store::now_ms(),
                                voice_id: take
                                    .processing
                                    .as_ref()
                                    .and_then(|p| p.profile_id.clone()),
                                negative_prompt: None,
                                master_path: format!("clips/{id}/master.wav"),
                                preview_path: format!("clips/{id}/preview.m4a"),
                            },
                            &staging,
                        )?;
                    }
                    _ => {
                        return Err(CommandError::new(
                            "INVALID_AUDIO_TARGET",
                            "Choose a book, sound library or voice.",
                        ));
                    }
                };
                Ok(())
            })();
            let _ = fs::remove_dir_all(staging);
            result
        },
    ))
}
#[cfg_attr(feature = "desktop", tauri::command)]
pub fn import_library_audio(
    app: AppHandle,
    state: State<'_, AppState>,
    memo_id: String,
    expected_revision: u64,
    target: String,
    root_path: Option<String>,
    chapter_id: Option<String>,
    sound_id: Option<String>,
) -> Result<String, CommandError> {
    let (source, cues) = match target.as_str() {
        "chapter" => {
            let root = root_path.as_deref().unwrap_or("");
            let chapter = chapter_id.as_deref().unwrap_or("");
            let snapshot = project_store::open(root)?;
            let cues = snapshot
                .chapters
                .iter()
                .find(|c| c.chapter.id == chapter)
                .ok_or_else(|| {
                    CommandError::new("CHAPTER_NOT_FOUND", "Choose an existing chapter.")
                })?
                .chapter
                .cues
                .clone();
            (project_store::chapter_audio_path(root, chapter)?, cues)
        }
        "sound" => {
            let asset = sound_store::find(&app, sound_id.as_deref().unwrap_or(""))?;
            (sound_store::asset_path(&app, &asset, true)?, vec![])
        }
        _ => {
            return Err(CommandError::new(
                "INVALID_AUDIO_TARGET",
                "Choose chapter audio or a sound clip.",
            ));
        }
    };
    memo_store::revision(&memo_store::load(&app, &memo_id)?, expected_revision)?;
    Ok(state.jobs.enqueue(
        "audio_import",
        "Import library audio".into(),
        move |control, _| {
            control.boundary()?;
            let state = app.state::<AppState>();
            let _lock = state
                .project_write_lock
                .lock()
                .map_err(|_| CommandError::internal("Library lock unavailable"))?;
            memo_store::revision(&memo_store::load(&app, &memo_id)?, expected_revision)?;
            let mut memo = memo_store::import_file(
                &app,
                &memo_id,
                &source,
                "Library audio",
                None,
                &settings::load(&app)?,
                control.cancelled,
            )?;
            memo.context.context_type = Some(target.clone());
            memo.context.chapter_id = chapter_id.clone();
            let selected = memo.selected_take_id.clone();
            if let Some(take) = memo
                .takes
                .iter_mut()
                .find(|t| Some(&t.id) == selected.as_ref())
            {
                take.cues = cues;
            }
            memo_store::save(&app, &memo)?;
            Ok(())
        },
    ))
}
