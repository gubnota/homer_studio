# API contracts

## Project manifest v1
- File: `project.json` at the project root, with `project.json.bak` holding the previous revision.
- Required fields: `schemaVersion`, UUID `id`, `title`, monotonic `revision`, millisecond timestamps, and ordered `chapters`.
- Chapter fields: stable UUID, title, zero-based order, relative source/processed/audio paths, segments, `audioStale`, measured `audioDurationMs`, `audioOrigin`, and `reviewStatus`.
- Segment fields: stable UUID, zero-based order, text, and optional selected take path.
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
- `tool_diagnostics() -> ToolDiagnostic[]`; resolution checks explicit overrides, process PATH, then standard Homebrew/system paths without a shell.
- `list_jobs() -> JobRecord[]`, `control_job(jobId, action)` where action is `pause`, `resume`, or `cancel` for an active job.
- `process_text(instruction, text) -> TextCandidate`; uses the configured local llama.cpp or loopback Ollama provider and does not mutate project files.
- `accept_processed_text(rootPath, expectedRevision, chapterId, text) -> ProjectSnapshot`; stores a reviewed candidate separately from source text and regenerates segments.
- `list_voices() -> Voice[]`; reads installed macOS voices.
- `generate_chapter_audio(...) -> jobId` and `import_chapter_audio(...) -> jobId`; queue conversion to canonical AAC/M4A and only commit a measured, valid result.
- `set_chapter_review(...) -> ProjectSnapshot`; accepts `approved` or `changes_requested` for current chapter audio.
- `audio_url(...) -> audio://localhost/<opaque-id>` and `audio_waveform(...) -> number[]`; expose only registered project audio, with byte-range playback and bounded peak data.

## Settings v1
- `llm`: `none`, `llama_cpp`, or `ollama`. Ollama URLs must use loopback HTTP.
- `speech`: `macos_say`, installed voice ID, and 80–500 words per minute.
- Optional explicit FFmpeg and FFprobe executable paths.

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
- `INVALID_SETTINGS`, `UNSUPPORTED_SETTINGS_VERSION`, `INVALID_SPEECH_RATE`, `UNSAFE_OLLAMA_URL`.
- `JOB_NOT_ACTIVE`, `INVALID_JOB_ACTION`, `JOB_CANCELLED`, `PROCESS_TIMEOUT`.
- `LLM_DISABLED`, `LLAMA_NOT_FOUND`, `MODEL_NOT_FOUND`, `MODEL_NOT_CONFIGURED`, `OLLAMA_UNAVAILABLE`.
- `EMPTY_INSTRUCTION`, `EMPTY_TEXT`, `LLM_INPUT_TOO_LARGE`, `EMPTY_LLM_OUTPUT`, `LLM_OUTPUT_TOO_LARGE`, `LLM_PROCESS_FAILED`, `INVALID_LLM_RESPONSE`.
- `VOICE_LIST_FAILED`, `SPEECH_FAILED`, `AUDIO_NOT_FOUND`, `AUDIO_CONVERSION_FAILED`, `AUDIO_IMPORT_FAILED`, `AUDIO_PROBE_FAILED`, `WAVEFORM_FAILED`, `FFMPEG_NOT_FOUND`, `FFPROBE_NOT_FOUND`, `INVALID_REVIEW_STATUS`, `AUDIO_STALE`.

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
