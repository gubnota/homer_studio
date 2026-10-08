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

## Voice production (schema 1)
- App data `audio-studio/`: `assets/<UUID>.wav`, `memos/<UUID>.json`, waveform caches, `engines.json`, `profiles.json`, isolated `runtimes/<engine>/`.
- RecordingSession: schemaVersion, UUID id, name, notes, favorite, deleted, revision, timestamps, takes, selectedTakeId, undo/redo. Optional contextType/chapterId/segmentId/speakerId/text defaults preserve older metadata.
- AudioVariant: immutable UUID WAV, measured duration/sample rate, parentId, source/preview/accepted/rejected state, name/favorite/notes, composition, cues and optional processing history. Newly recorded sources preserve the input rate; derived media uses mono 48 kHz float WAV.
- Composition clips: assetId (null means silence), startMs/endMs, gain, fadeInMs/fadeOutMs and incoming crossfadeMs. Duration equals sum of clip spans minus incoming overlaps. At most 4096 validated clips; originals remain intact.
- Processing history includes engine, adapter/package version, preset, merged params, profileId, source range, backend and timestamp.
- Preview creation preserves selectedTakeId; acceptance selects the new take and stores undo. Reject/delete never destroys WAVs needed by another composition. Active take deletion is forbidden.
- Memo commands: list_memos, create_memo, get_memo, update_memo, update_memo_take, choose_memo_take; import_memo_audio, edit_memo_audio, replace_memo_range, extract_memo_selection, memo_audio_url, memo_waveform, export_memo, memo_source_path and convert_memo_segment. Mutations require expectedRevision where applicable.
- Capture commands: audio_capture_devices, audio_capture_permission, audio_capture_start, audio_capture_control (status/pause/resume/stop/discard). `audio-capture` events report state, elapsedMs, peak/RMS, clipping and bounded recent peaks. Capture limit: one hour.
- Processor commands: audio_engine_configs, save_audio_engine, audio_engine_status, setup_audio_engine, process_memo_audio, list_voice_profiles, save_voice_profile, publish_memo_audio and import_library_audio.
- Queued audio jobs use existing job controls/events; processor stages are indeterminate unless meaningful progress exists. Cancellation terminates the process group, reaps it and prevents a new variant from being committed.
- Worker NDJSON protocol 1: one stdin request, optional stage messages and exactly one result or error. Request ≤1 MB; stdout frames ≤64 KB; retained stderr ≤8 KB. Status validates Python imports and local files. Inference sets Hugging Face offline flags.
- Publication requires accepted/source selected media. Targets: chapter, segment, sound or voice; project writes check project revision. Chatterbox reference samples retain the 6–20 second limit.
- Memo export accepts WAV (24-bit), FLAC, M4A/AAC or MP3; writes staging media, probes measured duration and atomically publishes after revision/cancellation checks.
- Waveform windows return durationMs/startMs/endMs and ≤2048 min/max peaks. Audio protocol responses are capped at 4 MB and support 206/416 byte-range semantics.
- Cue timings are remapped from composition spans; affected replacement cues are scaled to actual measured output, without claiming word alignment.

## Wave Studio schema 1
- App data `wave-studio/`: `projects/<UUID>.json`, immutable `sources/`, `sfx.json`, disk tempo variants and bounded preview caches. Projects carry schemaVersion, UUID, name, revision, timestamps, sources and timeline (`clips`, `sfx`, `voices`). Optional `view` and `voiceOriginal` retain schema-1 compatibility; old projects default to no stored view/baseline.
- Clip fields: UUID, sourceId (null = silence), name, startMs, sourceStartMs/sourceEndMs, speed, gainDb, fadeInMs/fadeOutMs. Effective duration is source span / speed; main clips cannot overlap. SFX can overlap.
- Bounds: finite times within 24 hours, speed 0.85–1.20, gain -96..+20 dB, fades within clip duration, unique IDs and owned stereo sources. Voice regions have voiceId/name/color and positive timeline bounds.
- `wave_list`, `wave_get`, `wave_create`, `wave_save` use revision checks and atomic JSON. `wave_import` accepts file, memo or saved sound and returns an owned measured source.
- `wave_peaks` returns at most 2048 min/max pairs. `wave_preview` returns bounded PCM WAV for at most 10001 ms; `wave_cancel_preview` cancels native preparation. Preview and export share sample-accurate gain/fade/mixing and cached pitch-preserving tempo processing.
- `wave_export` queues verified WAV/MP3/M4A/AAC/FLAC output using existing job cancellation. `wave_sfx_list/add/update`, `wave_source_url` expose reusable library assets through owned IDs.
- Explicit production commands: `wave_normalize(project, clipId) -> gainDb`; `wave_join(project, clipIds) -> Source`; `wave_generate_speech(text, voiceId, projectId, revision)` and `wave_convert_regions(project, regionIds, regenerate)` queue jobs. `wave_processing_result(jobId)` returns owned sources plus fixed-range replacements and origin project/revision. Results live under `wave-studio/results/`; acceptance belongs to the renderer and checks unchanged project/timeline.
- Earlier AI selection input: project ID + expected revision + selected timeline range + target voice ID + source spans. Results are immutable previews requiring explicit acceptance. Annotation changes do not automatically convert audio or download models.
- Voice metadata adds backward-compatible color, notes and modelProvider defaults; updates retain existing sample identity.

- Optional schema-1 `VoiceRegion.production` records generated/converted status, voiceId and audioKey. Audio identity ignores clip IDs, gain and fades; changed source/range/speed or voice invalidates completion. Conversion defaults to pending regions; regenerate is explicit. Built-in conversion may omit referenceId.
- Optional `videos` contains owned id/name/durationMs/startMs. Video import removes audio, bounds frames to 1280×720 and serves range-enabled MP4. Preview duration includes video; audio export excludes references.
- `wave_deleted`, `wave_delete(id, restore)` move project manifests into/out of recoverable trash. `wave_save_copy(project, directory)` writes a new portable directory including immutable sources and videos. `wave_open_copy(directory)` validates contained media and imports a new independent project with remapped media IDs.

## Platform transport and portable Wave projects (0.2.6)
- `WaveView.selectedIds?: string[]` complements legacy `selectedId`; older saved views migrate automatically. Join uses explicit IDs and asks for a resulting voice when tags differ.
- `.wavehs`: new streamed `WAVEHS02` lossless gzip payload, with SHA-256 per entry; legacy `WAVEHS01` remains readable. Contains project JSON, owned audio/video and referenced custom voice samples. Opening accepts optional `requestId` for operation progress/cancel; one renderer import at a time. Canonical bundled 48 kHz stereo float WAV is adopted without redundant decoding. Import remaps IDs, rejects traversal, duplicate entries, corrupt/trailing bytes and rolls back partial state. Limits: 20,000 entries, 100 GB total, 16 MB manifest.
- Desktop drag/Open/CLI paths enqueue `.wavehs` imports; original projects remain separate.
- Server `POST /api/login`: owner token, at least 24 ASCII alphanumeric characters; HttpOnly SameSite=Strict cookie. Bearer token also supported. Mutation Origin must match Host.
- `POST /api/command/{name}`: allowlisted command, camelCase JSON arguments; structured core errors. Binary previews are encoded for transport then restored to ArrayBuffer by platform adapter.
- `/api/upload`, `/api/download`, `/api/media/{id}`: owned file streaming, authenticated ranged audio/video. Top-level filesystem arguments are scoped to server data/resources.
- `/api/recording`: create a staged webm/m4a recording; PUT chunks up to 8 MB; stop queues existing memo import. Capture buffers and duration are bounded.
- `GET /api/projects`: bounded list of saved manuscript project folders under server data; Wave projects use shared list/get/save commands.
- Managed worker cancellation requests DELETE then verified local worker shutdown; restart is explicit when the worker is needed again. Unrelated services are not interrupted.

- `wave_purge {id: string | null}` permanently removes one trash manifest or clears all deleted manifests for null, while retaining shared media. `wave_reveal {id, deleted}` is desktop-only Finder reveal. Project restore/open must drain current saves and load the restored manifest before exporting. No schema changes.
- Renderer waveform cache keys include immutable source/take identity, window boundaries and peak count; failed/cancelled requests are never cached.

- Wave voice regions optionally persist `audio: {original, versions, activeAudioKey}`. Snapshots use passage-local clip times; versions pair voice ID, clips and production metadata (up to 64 unique voices). Conversion always renders the original snapshot. Voice/source IDs and provenance keys remap on portable import, including inactive cached voices. Legacy regions without snapshots remain readable and can recover from the old pre-conversion timeline.

## v0.3.2 project media
- Recording context accepts optional `projectId`; project must exist. App-wide memo listing excludes `projectId` and `chapterId`.
- `wave_save_copy` accepts optional `includeVideo` (default false) and binds a readable `.wavehs` folder for incremental autosave. `wave_export_bundle` remains the explicit binary portable archive path. Legacy archive import remains supported.
- `wave_export` accepts optional `videoId`; selected video determines the start window and legacy default length, with silent audio gaps. Output is MP4 with edited audio, excluding original video audio.
- Folder manifests contain relative media paths; original external video links remain app-owned and must be relinked on another machine.

## Audio/video timeline follow-up
- `wave_export` adds optional `videoTimeline: boolean` (default false), mutually exclusive with `videoId`. True exports from zero to max audio/video end as MP4: black before first video, prior final frame across gaps/tail, 1280×720 at 30 fps, edited stereo audio only. Existing selected-video exports are preserved.
- Video intervals may touch but must not overlap on save/save-copy and combined export. Legacy overlapping projects remain readable and require explicit Arrange before saving. No schema change.
- Separate soundtrack import keeps 0 dB gain and does not split narration. Main-lane drops explicitly insert at the drop position; soundtrack-lane drops add independent audio. File-picker dialog defaults to separate soundtrack. Multi-video imports run sequentially with progress, cancellation and retained successful clips after a later failure.

- `wave_export` accepts optional `lengthMode: "longest" | "shortest"`. MP4 longest uses max remaining audio/video duration; shortest uses min, rejecting empty windows. Selected video starts at its placement; combined video length ends at the last video end. Longest holds the final frame or pads audio silence. Omitted mode preserves legacy selected-video length and combined max duration. Unknown modes return `INVALID_EXPORT_LENGTH`.
- Selected SFX Split affects only that clip at playhead. Edge trims map timeline deltas through speed into source bounds, retain at least 1 ms playback, clamp fades and keep other clip placements. Start-edge trimming keeps the selected clip end fixed; Undo/Redo use existing audio history.

- Export UI blocks overlapping video placements before destination selection; explicit Arrange resolves them through existing placement helpers. Preparation uses indeterminate progress; enqueue success closes the dialog and shows queued/job progress. Preparation/command errors remain inline for retry. Native export commands/length policy are unchanged.

- Both selected and combined MP4 exports require audio and video stream durations within 150 ms of the chosen duration; missing/mismatched streams return INVALID_EXPORT_DURATION. Held coverage is presentation-only; no persisted schema changes.

## v0.3.6 non-destructive fragments
- Video placements add optional `assetId`, `sourceStartMs` (default 0), `sourceDurationMs` (full asset duration). Legacy records use placement ID as asset ID. Splits/copies keep unique placement IDs and share immutable assets; bounds are checked against app-owned metadata. Folder/archive import preserves placement IDs/ranges while remapping asset IDs. Relink preserves all placements of the replaced asset.
- Main audio may overlap and mixes through the same preview/export engine as SFX. Cross-lane moves preserve requested placement time and do not ripple other fragments. Renderer keeps existing soundtrack row assignments during the editing session.
- Selected video exports requiring encoding use 30 fps to establish a valid filter frame rate before cloning trimmed final frames. Core reports observed encoding progress, limits FFmpeg threads and verifies both output stream lengths before publication.
