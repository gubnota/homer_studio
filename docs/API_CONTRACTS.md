# API contracts

## Project manifest v1
- File: `project.json` at the project root, with `project.json.bak` holding the previous revision.
- Required fields: `schemaVersion`, UUID `id`, `title`, monotonic `revision`, millisecond timestamps, and ordered `chapters`.
- Chapter fields: stable UUID, title, zero-based order, relative source/processed/audio paths, segments, `audioStale`, measured `audioDurationMs`, `audioOrigin`, and `reviewStatus`.
- Segment fields: stable UUID, zero-based order, text, and optional selected take path.
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
- `list_jobs() -> JobRecord[]`, `control_job(jobId, action)` where action is `pause`, `resume`, or `cancel` for an active job.
- `process_text(instruction, text) -> TextCandidate`; uses the configured local llama.cpp or loopback Ollama provider and does not mutate project files.
- `accept_processed_text(rootPath, expectedRevision, chapterId, text) -> ProjectSnapshot`; stores a reviewed candidate separately from source text and regenerates segments.
- `list_voices() -> Voice[]`; reads installed macOS voices.
- `preview_voice(voiceId, rate) -> audio://localhost/<opaque-id>`; validates an installed voice and 80–500 WPM, synthesizes a disposable local M4A through the normal FFmpeg/FFprobe path, registers it for playback, and removes older previews.
- `generate_chapter_audio(...) -> jobId` and `import_chapter_audio(...) -> jobId`; queue conversion to canonical AAC/M4A and only commit a measured, valid result.
- `set_chapter_review(...) -> ProjectSnapshot`; accepts `approved` or `changes_requested` for current chapter audio.
- `audio_url(...) -> audio://localhost/<opaque-id>` and `audio_waveform(...) -> number[]`; expose only registered project audio, with byte-range playback and bounded peak data.
- `export_project(...) -> jobId`; re-probes all current approved chapters, tries verified stream-copy concatenation, falls back to one canonical AAC encode, then commits the M4A/timestamp pair with export history.
- `export_audio_url(...)` and `read_export_timestamps(...)`; read only manifest-owned export files.
- `sound_workers() -> WorkerHealth[]`, `list_sounds() -> SoundAsset[]`, `generate_sound(request) -> jobId`, `sound_audio_url(id) -> audio://localhost/<opaque-id>`, `export_sound(id, destination) -> void`. No project identifier is required.

## Standalone sound library v1
- App data: `sound-assets/manifest.json`, `{ schemaVersion: 1, assets: SoundAsset[] }`. Each asset has UUID `id`, `prompt`, `category`, `provider`, `model`, `requestedDurationSeconds`, measured `durationMs`, optional `seed`, `createdAtMs`, `masterPath`, and `previewPath`.
- Paths are exactly `clips/<id>/master.wav` and `clips/<id>/preview.m4a`. Files are published only after WAV/M4A validation; manifest writes use a same-folder temporary file and rename.
- `SoundRequest`: 1–500 nonblank prompt characters, category `speech | vocal_gesture | sound_effect`, duration 1–20 seconds, optional integer seed 0–2147483647. Gestures accept one documented tag.

## Local audio worker protocol v1
- Each worker binds `127.0.0.1`. The Mac app accepts only `http://127.0.0.1:<port>`. `GET /v1/health` returns `protocolVersion`, `engine`, `model`, `ready`, `categories`, `maxDurationSeconds`, and `message`.
- `POST /v1/jobs` takes the `SoundRequest` JSON and returns HTTP 202 `{id}`. `GET /v1/jobs/<id>` returns `{id,status,error,format}` with `queued | running | completed | failed | cancelled`; `DELETE` cancels. A completed job provides WAV bytes at `GET /v1/jobs/<id>/audio`.
- Chatterbox engine ID `chatterbox_turbo` supports speech and documented vocal tags. Stable Audio engine ID `stable_audio_open` supports sound effects. Worker errors use JSON `{error}`. Responses and audio are size bounded; model packages and checkpoints are never downloaded in request handling.

## Settings v1
- `llm`: `none`, `llama_cpp`, or `ollama`. Ollama URLs must use loopback HTTP.
- `speech`: `macos_say`, installed voice ID, and 80–500 words per minute.
- `voicePresets`: ordered `{ id, name, voiceId, rate, builtIn }` records. IDs are non-empty and unique; rates are 80–500 WPM. Curated Samantha, Daniel, and Karen presets are added only when those voices are installed.
- `selectedVoicePresetId`: nullable preset ID. Selecting or saving a preset also updates the effective `speech.voiceId` and `speech.rate` for backward-compatible generation.
- Optional explicit FFmpeg, FFprobe, and Ollama executable paths. llama.cpp keeps its executable and GGUF model paths in its provider settings.
- `sounds.chatterboxUrl` and `sounds.sfxUrl` default to ports 8765 and 8766. Existing settings migrate through defaults. Worker URLs must be loopback HTTP.

## Job states
- `queued -> running -> completed | failed | cancelled`.
- One heavy worker runs at a time. Pause takes effect at the next task boundary; cancellation is cooperative and native child processes are killed.
- Process output retained by the runner is bounded to the newest 64 KiB.

`ProjectSnapshot` adds the absolute `rootPath` and chapter source/processed text to the persisted manifest fields.

## Implemented error codes
- `INVALID_PATH`, `UNSAFE_PROJECT_PATH`, `PROJECT_EXISTS`, `PROJECT_NOT_FOUND`.
- `INVALID_TITLE`, `EMPTY_MANUSCRIPT`, `MANUSCRIPT_TOO_LARGE`, `UNSUPPORTED_FILE`.
- `INVALID_PROJECT`, `UNSUPPORTED_PROJECT_VERSION`, `CHAPTER_NOT_FOUND`, `INVALID_CHAPTER_ORDER`.
- `REVISION_CONFLICT`, `IO_ERROR`, `INTERNAL`.
- `INVALID_SETTINGS`, `UNSUPPORTED_SETTINGS_VERSION`, `INVALID_SPEECH_RATE`, `INVALID_VOICE_PRESET`, `VOICE_NOT_INSTALLED`, `UNSAFE_OLLAMA_URL`.
- `JOB_NOT_ACTIVE`, `INVALID_JOB_ACTION`, `JOB_CANCELLED`, `PROCESS_TIMEOUT`.
- `LLM_DISABLED`, `LLAMA_NOT_FOUND`, `MODEL_NOT_FOUND`, `MODEL_NOT_CONFIGURED`, `OLLAMA_UNAVAILABLE`.
- `EMPTY_INSTRUCTION`, `EMPTY_TEXT`, `LLM_INPUT_TOO_LARGE`, `EMPTY_LLM_OUTPUT`, `LLM_OUTPUT_TOO_LARGE`, `LLM_PROCESS_FAILED`, `INVALID_LLM_RESPONSE`.
- `VOICE_LIST_FAILED`, `SPEECH_FAILED`, `AUDIO_NOT_FOUND`, `AUDIO_CONVERSION_FAILED`, `AUDIO_IMPORT_FAILED`, `AUDIO_PROBE_FAILED`, `WAVEFORM_FAILED`, `FFMPEG_NOT_FOUND`, `FFPROBE_NOT_FOUND`, `INVALID_REVIEW_STATUS`, `AUDIO_STALE`.
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
