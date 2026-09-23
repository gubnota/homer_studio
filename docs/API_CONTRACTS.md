# API contracts

## Project manifest v1
- File: `project.json` at the project root, with `project.json.bak` holding the previous revision.
- Required fields: `schemaVersion`, UUID `id`, `title`, monotonic `revision`, millisecond timestamps, and ordered `chapters`.
- Chapter fields: stable UUID, title, zero-based order, relative source/processed/audio paths, segments, `audioStale`, measured `audioDurationMs`, `audioOrigin`, and `reviewStatus`.
- Segment fields: stable UUID, zero-based order, text, optional selected take ID, and `takes[]` with ID, project-relative audio path, and measured duration. Old projects default missing take/cue fields.
- Export history fields: stable UUID, relative M4A/timestamp paths, verified duration, creation time, source revision, and source content-update time.
- Project-owned paths must be relative normal path components. Absolute paths and parent traversal are rejected.
- Writes use a same-folder temporary file followed by rename. Mutations require the caller's expected revision.

## Implemented Tauri commands
- `read_manuscript(path) -> { name, text }`: UTF-8 `.txt`, `.md`, or `.markdown`, at most 20 MB.
- `create_project(parentPath, title, manuscript) -> ProjectSnapshot`.
- `open_project(rootPath) -> ProjectSnapshot`.
- `update_chapter(rootPath, expectedRevision, chapterId, title, sourceText) -> ProjectSnapshot`.
- `reorder_chapters(rootPath, expectedRevision, chapterIds) -> ProjectSnapshot`.
- `desktop_info() -> { platform, architecture, runtime }`.
- `get_settings()`, `save_settings(settings) -> SettingsV1`; settings are stored atomically in the app configuration folder.
- `tool_diagnostics() -> ToolDiagnostic[]`; resolution checks explicit overrides, process PATH, then bounded system and user Homebrew paths without a shell. Each result exposes `key`, `name`, effective `path`, `available`, `status`, `configuredPath`, and `detectedPath`. Ollama CLI and loopback server health are separate results.
- `list_jobs() -> JobRecord[]`, `control_job(jobId, action)` where action is `pause`, `resume`, or `cancel` for an active job; `dismiss_jobs(jobIds)` removes only terminal queue rows. Jobs carry bounded timestamped events, stage, progress, and errors.
- `process_text(instruction, text) -> TextCandidate`; uses the configured local llama.cpp or loopback Ollama provider and does not mutate project files.
- `accept_processed_text(rootPath, expectedRevision, chapterId, text) -> ProjectSnapshot`; stores a reviewed candidate separately from source text and regenerates segments.
- `list_voices() -> Voice[]`; reads the app-data voice library, including the built-in Chatterbox model voice. `create_voice(name)`, `add_voice_sample(voiceId,name,sourcePath)`, `add_recorded_voice_sample(voiceId,name,bytes)`, `select_voice_sample(voiceId,sampleId)`, `voice_sample_url(voiceId,sampleId)`, and `delete_voice(voiceId)` manage local reference samples. Source audio is normalized into an app-owned WAV before import returns; custom voices need a selected sample before use.
- `preview_voice(voiceId, rate) -> audio://localhost/<opaque-id>`; custom voice previews are synthesized in the background after adding or selecting a sample and cached as `voices/<voiceId>/preview.m4a`. Until ready, return `VOICE_PREVIEW_PENDING`; the UI can retry without holding a generation request open. The rate argument remains for IPC compatibility; neural generation does not use words-per-minute control.
- `generate_chapter_audio(...) -> jobId` and `import_chapter_audio(...) -> jobId`; queue conversion to canonical AAC/M4A and only commit a measured, valid result. `generate_segment_audio(...) -> jobId` saves and selects one take; `assemble_chapter_takes(...) -> jobId` joins selected takes in source order.
- `generate_chapters_audio(rootPath, expectedRevision, chapterIds) -> jobId` validates a nonempty, duplicate-free set of existing chapters with spoken text, snapshots text and speech settings, and runs them in project order in one queue job. Each successful chapter commit advances the expected revision; an external edit stops later commits with `REVISION_CONFLICT`. Completed chapters survive later failure or cancellation. Job progress covers the whole batch and bounded events name the active/completed chapter and any failure. Review's pending action includes missing or stale generated audio, while selected regeneration may replace existing or imported audio after confirmation.
- `segment_take_url(...) -> audio://localhost/<opaque-id>` plays one saved take. `select_segment_take(...) -> ProjectSnapshot` selects it and marks assembled chapter audio stale. `convert_segment_recording(..., bytes, rangeStartMs?, rangeEndMs?) -> jobId` requires a valid project revision, selected narrator sample, FFmpeg, and the Original worker; it saves an unselected preview take. Both optional range fields must be supplied together, span 0.1–20 seconds, and fit inside the selected section take; absent fields convert the whole recording into a replacement take.
- `export_chapter_audio(...)` writes a chosen M4A via the save dialog; `delete_generated_chapter_audio(...)` removes current generated chapter audio.
- `model_status(model) -> ModelStatus` and `install_model(model) -> jobId` cover pinned `turbo | original` checkpoints with byte progress, missing-file list, and disk use.
- `set_chapter_review(...) -> ProjectSnapshot`; accepts `approved` or `changes_requested` for current chapter audio.
- `audio_url(...) -> audio://localhost/<opaque-id>` and `audio_waveform(...) -> number[]`; expose only registered project audio, with byte-range playback and bounded peak data.
- `export_project(...) -> jobId`; re-probes all current approved chapters, tries verified stream-copy concatenation, falls back to one canonical AAC encode, then commits the M4A/timestamp pair with export history.
- `export_audio_url(...)` and `read_export_timestamps(...)`; read only manifest-owned export files.
- `sound_workers() -> WorkerHealth[]`, `list_sounds() -> SoundAsset[]`, `generate_sound(request) -> jobId`, `sound_audio_url(id) -> audio://localhost/<opaque-id>`, `export_sound(id, destination) -> void`. No project identifier is required.

## Standalone sound library v1
- App data: `sound-assets/manifest.json`, `{ schemaVersion: 1, assets: SoundAsset[] }`. Each asset has UUID `id`, `prompt`, `category`, `provider`, `model`, `requestedDurationSeconds`, measured `durationMs`, optional `seed` and `negativePrompt`, `createdAtMs`, `masterPath`, and `previewPath`.
- Paths are exactly `clips/<id>/master.wav` and `clips/<id>/preview.m4a`. Files are published only after WAV/M4A validation; manifest writes use a same-folder temporary file and rename.
- `SoundRequest`: 1–500 nonblank prompt characters, category `speech | vocal_gesture`, maximum duration 1–120 seconds, optional integer seed 0–2147483647. Inline gestures are available in speech; the dedicated gesture category accepts one documented tag. New sound-effect requests and `negativePrompt` are rejected. Existing sound-effect assets remain available for playback, export, and deletion.

## Local audio worker protocol v2
- Each worker binds `127.0.0.1`. The Mac app accepts only `http://127.0.0.1:<port>`. `GET /v2/health` returns `protocolVersion`, `engine`, `model`, `ready`, `categories`, `maxDurationSeconds`, `message`, and `completedJobs` (zero if absent for older workers). Completed jobs count only successful, uncancelled audio generations in this worker process.
- `POST /v2/jobs` takes the `SoundRequest` JSON and returns HTTP 202 `{id}`. `GET /v2/jobs/<id>` returns `{id,status,error,format}` with `queued | running | completed | failed | cancelled`; `DELETE` cancels. A completed job provides WAV bytes at `GET /v2/jobs/<id>/audio`.
- `POST /v2/references` accepts a bounded WAV and returns an opaque reference ID. A Chatterbox speech or gesture job may include `referenceId`. Original voice conversion uses category `voice_conversion` plus both `sourceId` and `referenceId`; the worker consumes and deletes both temporary WAVs after the job. No path from a worker request is trusted.
- Chatterbox engine ID `chatterbox_turbo` supports speech and documented vocal tags. Original engine ID `chatterbox_original` supports English speech and voice conversion. The retired `stable_audio_open` worker source remains in the repository but is not offered by the app. Worker errors use JSON `{error}`. Responses and audio are size bounded; model packages and checkpoints are never downloaded in request handling.
- `POST /v2/shutdown` returns `{status:"stopping"}`, then ends the local worker server and process. On app exit the native client checks protocol v2 and the exact expected Chatterbox engine before using it, with one-second network timeouts. External Ollama processes and the retired effects worker are outside this lifecycle.
- Before uploading references for a new job, the native client restarts a default-port local Chatterbox worker after two completed jobs. It waits for the old server to stop and checks that the replacement is ready. Custom worker URLs are not restarted.

## Settings v1
- `llm`: `none`, `llama_cpp`, or `ollama`. Ollama URLs must use loopback HTTP.
- `speech`: `chatterbox_turbo | chatterbox_original` and an app-data voice ID. Legacy `macos_say`, rate, `voicePresets`, and `selectedVoicePresetId` fields are accepted for migration, but macOS synthesis and preset selection are no longer offered. Custom voices need a selected sample.
- Voice library: `voices/library.json` plus normalized, bounded local WAV sample files. One selected sample conditions each generation; distinct speakers are not blended.
- Optional explicit FFmpeg, FFprobe, and Ollama executable paths. llama.cpp keeps its executable and GGUF model paths in its provider settings.
- `sounds.chatterboxUrl` and `sounds.originalUrl` default to ports 8765 and 8767. The legacy `sounds.sfxUrl` field remains in persisted settings for migration but is no longer shown or used for generation. Worker URLs must be loopback HTTP.

## Job states
- `queued -> running -> completed | failed | cancelled`.
- One heavy worker runs at a time. Pause takes effect at the next task boundary; cancellation is cooperative and native child processes are killed.
- Process output retained by the runner is bounded to the newest 64 KiB.

`ProjectSnapshot` adds the absolute `rootPath` and chapter source/processed text to the persisted manifest fields.

Generated chapter audio stores optional `cues` on each chapter. A cue has `order`, clean spoken `text`, and `startMs`/`endMs`; legacy projects deserialize with no cues. Imported audio has an empty cue list. Narration accepts `[sigh]`, `[gasp]`, `[cough]`, `[laugh]`, `[chuckle]`, `[groan]`, and `[pause:100..5000]`; unsupported markers fail with `INVALID_SPEECH_MARKUP`.

## Implemented error codes
- `INVALID_PATH`, `UNSAFE_PROJECT_PATH`, `PROJECT_EXISTS`, `PROJECT_NOT_FOUND`.
- `INVALID_TITLE`, `EMPTY_MANUSCRIPT`, `MANUSCRIPT_TOO_LARGE`, `UNSUPPORTED_FILE`.
- `INVALID_PROJECT`, `UNSUPPORTED_PROJECT_VERSION`, `CHAPTER_NOT_FOUND`, `INVALID_CHAPTER_ORDER`.
- `REVISION_CONFLICT`, `IO_ERROR`, `INTERNAL`.
- `INVALID_BATCH` for empty, duplicate, or missing chapter selections.
- `INVALID_SETTINGS`, `UNSUPPORTED_SETTINGS_VERSION`, `INVALID_SPEECH_RATE`, `INVALID_VOICE_PRESET`, `VOICE_NOT_INSTALLED`, `UNSAFE_OLLAMA_URL`.
- `JOB_NOT_ACTIVE`, `INVALID_JOB_ACTION`, `JOB_CANCELLED`, `PROCESS_TIMEOUT`.
- `LLM_DISABLED`, `LLAMA_NOT_FOUND`, `MODEL_NOT_FOUND`, `MODEL_NOT_CONFIGURED`, `OLLAMA_UNAVAILABLE`.
- `EMPTY_INSTRUCTION`, `EMPTY_TEXT`, `LLM_INPUT_TOO_LARGE`, `EMPTY_LLM_OUTPUT`, `LLM_OUTPUT_TOO_LARGE`, `LLM_PROCESS_FAILED`, `INVALID_LLM_RESPONSE`.
- `INVALID_VOICE_SAMPLE`, `VOICE_NOT_FOUND`, `VOICE_LIST_FAILED`, `SPEECH_FAILED`, `AUDIO_NOT_FOUND`, `AUDIO_CONVERSION_FAILED`, `AUDIO_IMPORT_FAILED`, `AUDIO_PROBE_FAILED`, `WAVEFORM_FAILED`, `FFMPEG_NOT_FOUND`, `FFPROBE_NOT_FOUND`, `INVALID_REVIEW_STATUS`, `AUDIO_STALE`.
- `EXPORT_EMPTY`, `EXPORT_NOT_READY`, `EXPORT_NOT_FOUND`, `EXPORT_COPY_FAILED`, `EXPORT_ENCODE_FAILED`, `EXPORT_VERIFICATION_FAILED`.

## Accepted baseline
- Versioned project manifest containing project identity, ordered chapters, segment records, audio references, and export history.
- Original chapter source in readable UTF-8 files; project-owned paths are relative.
- Stable IDs independent of displayed chapter numbers.
- Settings separate LLM provider selection from speech generation settings.
- Narrow Tauri command API for project operations, settings, jobs, audio import/playback, and export.
- Structured errors include a code, actionable message, and optional affected chapter/job ID.
- Jobs expose real state/progress and cancellation; interrupted jobs never become completed automatically.
- Precise measured durations are accumulated before timestamps are formatted.
- Exact payloads, filenames, state transitions, and error codes are specified in `docs/IMPLEMENTATION_PLAN.md`.

Add exact payloads, state transitions, and errors here as each later stage lands.

## Audio library and review additions (2026-09-23)
- `delete_sounds(ids) -> SoundAsset[]` removes selected app-owned clips. `convert_voice_clip(voiceId, sourcePath?, bytes?) -> jobId` accepts one recording (1–120 seconds, at most 100 MB), splits it into worker-sized segments, and publishes one `voice_conversion` asset. The two input forms are mutually exclusive.
- `audio_waveform_window(rootPath, chapterId, startMs, endMs) -> number[]` returns bounded peaks for a visible chapter range. The Review player keeps playback and selection within project-owned audio.
- Review bulk actions delete generated chapter audio only. Export history bulk deletion removes project-owned M4A and timestamp pairs. Bulk saves copy selected outputs into a chosen folder; originals remain owned by the project.
- Review refreshes chapter state as each batch chapter is committed, and saved chapter audio remains downloadable while later chapters render. Exports offers individual chapter downloads independently of the approved full-audiobook export gate.
- Sound Studio accepts a 1–120 second maximum for all kinds. Rust divides effects and gestures into worker jobs of at most 20 seconds, splits longer speech at word boundaries into short prompts, and concatenates verified WAV segments. Chatterbox Turbo determines the actual spoken duration from the text; the requested maximum does not stretch a short prompt. Standalone conversion accepts up to 120 seconds by dividing source recordings into segments of at most 15 seconds.
- The built-in Turbo narrator preview is a bundled model-generated M4A. Custom voice previews are cached per voice; when unavailable, the UI can play the copied selected sample and report the generation error.

## Packaged Chatterbox runtime (2026-09-23)
- `worker_runtime_status(pythonPath?) -> { path, installed, pythonPath, message }` reports the app-data venv and a discovered or selected Python 3.10 executable. `installed` requires the pinned requirements marker and venv interpreter; worker resources must exist in the app bundle.
- `install_worker_runtime(pythonPath?) -> jobId` explicitly creates the venv under app data, installs pinned packages, verifies imports, and records setup progress/errors in the existing job queue. Cancellation terminates the setup child. The selected worker starts after both packages and its checkpoint are installed.
- Worker scripts are bundled in `Contents/Resources/workers`; checkpoints remain under app data `models/`. No production command resolves workers through `CARGO_MANIFEST_DIR`.
