// Explicit allowlist shared with the desktop command boundary.
use homer_core::{AppState, commands, runtime::AppHandle, services::project_store::CommandError};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
fn argument<T: DeserializeOwned>(args: &Value, key: &str) -> Result<T, CommandError> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|_| CommandError::new("INVALID_ARGUMENT", format!("Invalid {key}.")))
}
pub async fn dispatch(app: AppHandle, command: &str, args: Value) -> Result<Value, CommandError> {
    match command {
        "audio_engine_configs" => {
            let result = commands::audio_processing::audio_engine_configs(app.clone())?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "save_audio_engine" => {
            let result = commands::audio_processing::save_audio_engine(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "config")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "audio_engine_status" => {
            let result = commands::audio_processing::audio_engine_status(
                app.clone(),
                argument(&args, "config")?,
            )
            .await?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "setup_audio_engine" => {
            let result = commands::audio_processing::setup_audio_engine(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "engine")?,
                argument(&args, "pythonPath")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "list_voice_profiles" => {
            let result = commands::audio_processing::list_voice_profiles(app.clone())?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "save_voice_profile" => {
            let result = commands::audio_processing::save_voice_profile(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "profile")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "process_memo_audio" => {
            let result = commands::audio_processing::process_memo_audio(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "request")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "publish_memo_audio" => {
            let result = commands::audio_processing::publish_memo_audio(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "memoId")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "target")?,
                argument(&args, "rootPath")?,
                argument(&args, "projectRevision")?,
                argument(&args, "chapterId")?,
                argument(&args, "segmentId")?,
                argument(&args, "voiceId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "import_library_audio" => {
            let result = commands::audio_processing::import_library_audio(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "memoId")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "target")?,
                argument(&args, "rootPath")?,
                argument(&args, "chapterId")?,
                argument(&args, "soundId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_operation_status" => {
            let result =
                commands::wave_studio::wave_operation_status(argument(&args, "requestId")?);
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_cancel_operation" => {
            commands::wave_studio::wave_cancel_operation(argument(&args, "requestId")?);
            Ok(Value::Null)
        }
        "wave_list" => {
            let result = commands::wave_studio::wave_list(app.clone())?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_get" => {
            let result = commands::wave_studio::wave_get(app.clone(), argument(&args, "id")?)?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_create" => {
            let result = commands::wave_studio::wave_create(app.clone(), argument(&args, "name")?)?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_save" => {
            let result = commands::wave_studio::wave_save(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "project")?,
                argument(&args, "expectedRevision")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_import" => {
            let result = commands::wave_studio::wave_import(
                app.clone(),
                argument(&args, "kind")?,
                argument(&args, "path")?,
                argument(&args, "id")?,
                argument(&args, "requestId")?,
            )
            .await?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_cancel_peaks" => {
            commands::wave_studio::wave_cancel_peaks(argument(&args, "requestId")?);
            Ok(serde_json::Value::Null)
        }
        "wave_peaks" => {
            let result = commands::wave_studio::wave_peaks(
                app.clone(),
                argument(&args, "sourceId")?,
                argument(&args, "startMs")?,
                argument(&args, "endMs")?,
                argument(&args, "maxPeaks")?,
                argument(&args, "requestId")?,
            )
            .await?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_cancel_preview" => {
            commands::wave_studio::wave_cancel_preview();
            Ok(Value::Null)
        }
        "wave_preview" => {
            let result = commands::wave_studio::wave_preview(
                app.clone(),
                argument(&args, "project")?,
                argument(&args, "startMs")?,
                argument(&args, "endMs")?,
            )
            .await?;
            Ok(json!({"binary":result.0}))
        }
        "wave_export" => {
            let result = commands::wave_studio::wave_export(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "project")?,
                argument(&args, "outputPath")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_sfx_list" => {
            let result = commands::wave_studio::wave_sfx_list(app.clone()).await?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_sfx_add" => {
            let result = commands::wave_studio::wave_sfx_add(
                app.clone(),
                argument(&args, "source")?,
                argument(&args, "category")?,
            )
            .await?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_sfx_update" => {
            let result = commands::wave_studio::wave_sfx_update(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "id")?,
                argument(&args, "name")?,
                argument(&args, "category")?,
                argument(&args, "delete")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_source_url" => {
            let result = commands::wave_studio::wave_source_url(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "sourceId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_normalize" => {
            let result = commands::wave_studio::wave_normalize(
                app.clone(),
                argument(&args, "project")?,
                argument(&args, "clipId")?,
            )
            .await?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_join" => {
            let result = commands::wave_studio::wave_join(
                app.clone(),
                argument(&args, "project")?,
                argument(&args, "clipIds")?,
            )
            .await?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_generate_speech" => {
            let result = commands::wave_studio::wave_generate_speech(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "text")?,
                argument(&args, "voiceId")?,
                argument(&args, "projectId")?,
                argument(&args, "revision")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_convert_regions" => {
            let result = commands::wave_studio::wave_convert_regions(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "project")?,
                argument(&args, "regionIds")?,
                argument(&args, "regenerate")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_processing_result" => {
            let result = commands::wave_studio::wave_processing_result(
                app.clone(),
                argument(&args, "jobId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_has_video" => {
            let result =
                commands::wave_studio::wave_has_video(app.clone(), argument(&args, "path")?)
                    .await?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_import_video" => {
            let result = commands::wave_studio::wave_import_video(
                app.clone(),
                argument(&args, "path")?,
                argument(&args, "requestId")?,
            )
            .await?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_video_url" => {
            let result = commands::wave_studio::wave_video_url(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "id")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_deleted" => {
            let result = commands::wave_studio::wave_deleted(app.clone())?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_delete" => {
            let result = commands::wave_studio::wave_delete(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "id")?,
                argument(&args, "restore")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_purge" => {
            let result = commands::wave_studio::wave_purge(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "id")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_save_copy" => {
            let result = commands::wave_studio::wave_save_copy(
                app.clone(),
                argument(&args, "project")?,
                argument(&args, "path")?,
            )
            .await?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_open_copy" => {
            let result =
                commands::wave_studio::wave_open_copy(app.clone(), argument(&args, "path")?)
                    .await?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_export_bundle" => {
            let result = commands::wave_studio::wave_export_bundle(
                app.clone(),
                argument(&args, "project")?,
                argument(&args, "path")?,
            )
            .await?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "wave_import_bundle" => {
            let result = commands::wave_studio::wave_import_bundle(
                app.clone(),
                argument(&args, "path")?,
                argument(&args, "requestId")?,
            )
            .await?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "sound_workers" => {
            let result = commands::sounds::sound_workers(app.clone())?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "list_sounds" => {
            let result = commands::sounds::list_sounds(app.clone())?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "delete_sounds" => {
            let result = commands::sounds::delete_sounds(app.clone(), argument(&args, "ids")?)?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "generate_sound" => {
            let result = commands::sounds::generate_sound(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "request")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "convert_voice_clip" => {
            let result = commands::sounds::convert_voice_clip(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "voiceId")?,
                argument(&args, "sourcePath")?,
                argument(&args, "bytes")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "sound_audio_url" => {
            let result = commands::sounds::sound_audio_url(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "id")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "export_sound" => {
            let result = commands::sounds::export_sound(
                app.clone(),
                argument(&args, "id")?,
                argument(&args, "destination")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "export_sounds" => {
            let result = commands::sounds::export_sounds(
                app.clone(),
                argument(&args, "ids")?,
                argument(&args, "destinationDir")?,
                argument(&args, "format")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "get_settings" => {
            let result = commands::system::get_settings(app.clone())?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "save_settings" => {
            let result =
                commands::system::save_settings(app.clone(), argument(&args, "settings")?)?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "tool_diagnostics" => {
            let result = commands::system::tool_diagnostics(app.clone())?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "list_jobs" => {
            let result = commands::system::list_jobs(app.state::<AppState>());
            Ok(json!(result))
        }
        "control_job" => {
            let result = commands::system::control_job(
                app.state::<AppState>(),
                argument(&args, "jobId")?,
                argument(&args, "action")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "dismiss_jobs" => {
            let result =
                commands::system::dismiss_jobs(app.state::<AppState>(), argument(&args, "jobIds")?);
            Ok(json!(result))
        }
        "model_status" => {
            let result = commands::system::model_status(app.clone(), argument(&args, "model")?)?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "install_model" => {
            let result = commands::system::install_model(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "model")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "worker_runtime_status" => {
            let result = commands::system::worker_runtime_status(
                app.clone(),
                argument(&args, "pythonPath")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "install_worker_runtime" => {
            let result = commands::system::install_worker_runtime(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "pythonPath")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "process_text" => {
            let result = commands::production::process_text(
                app.clone(),
                argument(&args, "instruction")?,
                argument(&args, "text")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "accept_processed_text" => {
            let result = commands::production::accept_processed_text(
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "chapterId")?,
                argument(&args, "text")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "list_voices" => {
            let result = commands::production::list_voices(app.clone())?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "create_voice" => {
            let result = commands::production::create_voice(app.clone(), argument(&args, "name")?)?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "add_voice_sample" => {
            let result = commands::production::add_voice_sample(
                app.clone(),
                argument(&args, "voiceId")?,
                argument(&args, "name")?,
                argument(&args, "sourcePath")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "add_recorded_voice_sample" => {
            let result = commands::production::add_recorded_voice_sample(
                app.clone(),
                argument(&args, "voiceId")?,
                argument(&args, "name")?,
                argument(&args, "bytes")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "select_voice_sample" => {
            let result = commands::production::select_voice_sample(
                app.clone(),
                argument(&args, "voiceId")?,
                argument(&args, "sampleId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "voice_sample_url" => {
            let result = commands::production::voice_sample_url(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "voiceId")?,
                argument(&args, "sampleId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "delete_voice" => {
            let result =
                commands::production::delete_voice(app.clone(), argument(&args, "voiceId")?)?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "preview_voice" => {
            let result = commands::production::preview_voice(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "voiceId")?,
                argument(&args, "Rate")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "generate_chapter_audio" => {
            let result = commands::production::generate_chapter_audio(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "chapterId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "generate_chapters_audio" => {
            let result = commands::production::generate_chapters_audio(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "chapterIds")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "generate_segment_audio" => {
            let result = commands::production::generate_segment_audio(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "chapterId")?,
                argument(&args, "segmentId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "convert_segment_recording" => {
            let result = commands::production::convert_segment_recording(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "chapterId")?,
                argument(&args, "segmentId")?,
                argument(&args, "bytes")?,
                argument(&args, "rangeStartMs")?,
                argument(&args, "rangeEndMs")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "assemble_chapter_takes" => {
            let result = commands::production::assemble_chapter_takes(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "chapterId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "import_chapter_audio" => {
            let result = commands::production::import_chapter_audio(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "chapterId")?,
                argument(&args, "sourcePath")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "set_chapter_review" => {
            let result = commands::production::set_chapter_review(
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "chapterId")?,
                argument(&args, "status")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "audio_url" => {
            let result = commands::production::audio_url(
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "chapterId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "segment_take_url" => {
            let result = commands::production::segment_take_url(
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "chapterId")?,
                argument(&args, "segmentId")?,
                argument(&args, "takeId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "select_segment_take" => {
            let result = commands::production::select_segment_take(
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "chapterId")?,
                argument(&args, "segmentId")?,
                argument(&args, "takeId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "audio_waveform" => {
            let result = commands::production::audio_waveform(
                app.clone(),
                argument(&args, "rootPath")?,
                argument(&args, "chapterId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "audio_waveform_window" => {
            let result = commands::production::audio_waveform_window(
                app.clone(),
                argument(&args, "rootPath")?,
                argument(&args, "chapterId")?,
                argument(&args, "startMs")?,
                argument(&args, "endMs")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "export_chapter_audio" => {
            let result = commands::production::export_chapter_audio(
                argument(&args, "rootPath")?,
                argument(&args, "chapterId")?,
                argument(&args, "destination")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "delete_generated_chapter_audio" => {
            let result = commands::production::delete_generated_chapter_audio(
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "chapterId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "delete_generated_chapter_audio_many" => {
            let result = commands::production::delete_generated_chapter_audio_many(
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "chapterIds")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "delete_exports" => {
            let result = commands::production::delete_exports(
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "exportIds")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "save_export_file" => {
            let result = commands::production::save_export_file(
                argument(&args, "rootPath")?,
                argument(&args, "exportId")?,
                argument(&args, "kind")?,
                argument(&args, "destination")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "save_exports" => {
            let result = commands::production::save_exports(
                argument(&args, "rootPath")?,
                argument(&args, "exportIds")?,
                argument(&args, "destinationDir")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "save_chapters" => {
            let result = commands::production::save_chapters(
                argument(&args, "rootPath")?,
                argument(&args, "chapterIds")?,
                argument(&args, "destinationDir")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "export_project" => {
            let result = commands::production::export_project(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "export_audio_url" => {
            let result = commands::production::export_audio_url(
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "exportId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "read_export_timestamps" => {
            let result = commands::production::read_export_timestamps(
                argument(&args, "rootPath")?,
                argument(&args, "exportId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "update_voice" => {
            let result = commands::production::update_voice(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "voiceId")?,
                argument(&args, "name")?,
                argument(&args, "color")?,
                argument(&args, "notes")?,
                argument(&args, "provider")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "default_project_parent" => {
            let result = commands::project::default_project_parent(app.clone())?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "read_manuscript" => {
            let result = commands::project::read_manuscript(argument(&args, "path")?)?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "create_project" => {
            let result = commands::project::create_project(
                app.state::<AppState>(),
                argument(&args, "parentPath")?,
                argument(&args, "title")?,
                argument(&args, "manuscript")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "open_project" => {
            let result = commands::project::open_project(
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "update_chapter" => {
            let result = commands::project::update_chapter(
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "chapterId")?,
                argument(&args, "title")?,
                argument(&args, "sourceText")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "reorder_chapters" => {
            let result = commands::project::reorder_chapters(
                app.state::<AppState>(),
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "chapterIds")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "list_memos" => {
            let result =
                commands::audio::list_memos(app.clone(), argument(&args, "includeDeleted")?)?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "create_memo" => {
            let result = commands::audio::create_memo(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "name")?,
                argument(&args, "context")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "get_memo" => {
            let result = commands::audio::get_memo(app.clone(), argument(&args, "memoId")?)?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "update_memo" => {
            let result = commands::audio::update_memo(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "memoId")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "name")?,
                argument(&args, "notes")?,
                argument(&args, "favorite")?,
                argument(&args, "deleted")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "choose_memo_take" => {
            let result = commands::audio::choose_memo_take(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "memoId")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "takeId")?,
                argument(&args, "action")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "import_memo_audio" => {
            let result = commands::audio::import_memo_audio(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "memoId")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "sourcePath")?,
                argument(&args, "name")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "edit_memo_audio" => {
            let result = commands::audio::edit_memo_audio(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "memoId")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "edit")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "replace_memo_range" => {
            let result = commands::audio::replace_memo_range(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "memoId")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "replacementId")?,
                argument(&args, "startMs")?,
                argument(&args, "endMs")?,
                argument(&args, "crossfadeMs")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "memo_audio_url" => {
            let result = commands::audio::memo_audio_url(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "memoId")?,
                argument(&args, "takeId")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "memo_waveform" => {
            let result = commands::audio::memo_waveform(
                app.clone(),
                argument(&args, "memoId")?,
                argument(&args, "takeId")?,
                argument(&args, "startMs")?,
                argument(&args, "endMs")?,
                argument(&args, "maxPeaks")?,
            )
            .await?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "export_memo" => {
            let result = commands::audio::export_memo(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "memoId")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "outputPath")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "memo_source_path" => {
            let result = commands::audio::memo_source_path(
                app.clone(),
                argument(&args, "memoId")?,
                argument(&args, "expectedRevision")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "convert_memo_segment" => {
            let result = commands::audio::convert_memo_segment(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "memoId")?,
                argument(&args, "memoRevision")?,
                argument(&args, "rootPath")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "chapterId")?,
                argument(&args, "segmentId")?,
                argument(&args, "rangeStartMs")?,
                argument(&args, "rangeEndMs")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "update_memo_take" => {
            let result = commands::audio::update_memo_take(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "memoId")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "takeId")?,
                argument(&args, "name")?,
                argument(&args, "notes")?,
                argument(&args, "favorite")?,
                argument(&args, "deleted")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        "extract_memo_selection" => {
            let result = commands::audio::extract_memo_selection(
                app.clone(),
                app.state::<AppState>(),
                argument(&args, "memoId")?,
                argument(&args, "expectedRevision")?,
                argument(&args, "startMs")?,
                argument(&args, "endMs")?,
            )?;
            serde_json::to_value(result).map_err(|e| CommandError::internal(e.to_string()))
        }
        _ => Err(CommandError::new(
            "UNKNOWN_COMMAND",
            "This operation is unavailable.",
        )),
    }
}
