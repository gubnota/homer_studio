# ADR-0001: Preserve the supplied UI as the product reference

Date: 2026-09-07
Status: Accepted

Context: The user supplied a downloaded UI prototype and requires its visual structure to be preserved.
Decision: Use `audio_studio_ui` as the primary UI/UX reference; inspect only relevant assets and maintain compact project context.
Consequences: Preserve screens and styling; simulated functionality must be replaced with real local operations.
Related files: `audio_studio_ui/index-BeQ8ttIE.js`, `audio_studio_ui/index-D-2jFf5J.css`, `docs/PROJECT_CONTEXT.md`.

# ADR-0002: Tauri shell with prototype-derived React source

Date: 2026-09-07
Status: Accepted

Context: Only compiled React assets exist; the app will run beside memory-intensive local audio and language models.
Decision: Use Tauri 2, Rust, React, and TypeScript; recover screen structure into maintainable source and reuse the supplied CSS. Keep native capabilities behind explicit commands and remove hosted-platform dependencies.
Consequences: The packaged shell is small and uses macOS WebKit, while native filesystem/process work is implemented and tested in Rust. The compiled bundle remains a reference, not the production extension point.
Related files: `src-tauri/`, `src/renderer/`, `docs/IMPLEMENTATION_PLAN.md`.

# ADR-0003: Filesystem projects and replaceable local engines

Date: 2026-09-07
Status: Accepted

Context: The requested workflow is local-first and needs interchangeable text models, durable chapters, audio assembly, and measured timestamps.
Decision: Use versioned JSON/files, llama.cpp and Ollama text adapters, a separate speech interface, installed macOS voices, imported audio, and local FFmpeg/FFprobe executables.
Consequences: No database or hosted backend; external executable availability and schema validation need explicit handling. Speech engine choice was superseded by ADR-0008.
Related files: `docs/API_CONTRACTS.md`, `docs/IMPLEMENTATION_PLAN.md`.

# ADR-0004: Verified Apple Silicon development packages

Date: 2026-09-07
Status: Accepted

Context: The first release must run locally on Apple Silicon without requiring paid Apple distribution credentials.
Decision: Build arm64 app, DMG, and ZIP artifacts, apply Tauri's ad-hoc signature, and verify the executable architecture, signature, media integration, and packaged startup. Repeat these checks on GitHub's macOS arm64 runner and publish only version-matching tags.
Consequences: Local test packages are reproducible and reviewable. Public distribution can add Developer ID signing and notarization without changing the application architecture.
Related files: `scripts/verify-release.mjs`, `scripts/smoke-app.mjs`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`.

# ADR-0005: Bounded local tool discovery with explicit overrides

Date: 2026-09-07
Status: Accepted

Context: A packaged macOS app does not inherit the same shell PATH as Terminal, so correctly installed local tools can appear missing.
Decision: Search inherited PATH plus a fixed list of system, Homebrew, and user Homebrew bin directories. Keep explicit file paths in Settings and report configured, detected, invalid, and service-reachability states separately.
Consequences: Common Apple Silicon installations work without shell configuration, custom layouts remain selectable, and executable discovery never becomes an unbounded filesystem search.
Related files: `src-tauri/src/services/process_runner.rs`, `src-tauri/src/services/settings.rs`, `src/renderer/src/pages.tsx`.

# ADR-0006: Reusable macOS voice presets

Date: 2026-09-07
Status: Deprecated (superseded by ADR-0008)

Context: A flat installed-voice list does not preserve useful narration speed choices and can imply broader voice creation than the local engine supports.
Decision: Store named presets containing an installed macOS voice ID and speaking rate. Seed curated presets only for installed voices, allow custom presets, and synthesize previews through the production speech pipeline.
Consequences: Existing speech settings remain the effective generation contract, old settings migrate without a schema-version break, and the product clearly represents presets rather than voice cloning.
Related files: `src-tauri/src/services/settings.rs`, `src-tauri/src/services/speech.rs`, `src/renderer/src/voice-presets.ts`, `src/renderer/src/pages.tsx`.

# ADR-0007: Independent prompt-to-audio library and local workers

Date: 2026-09-22
Status: Deprecated (effects engine and download policy superseded by ADR-0010)

Context: Authors need short sounds from prompts without a book. Chatterbox handles speech and documented vocal tags but is not a general fabric/ambience engine; the future Linux GPU deployment should share a stable boundary.
Decision: Keep a separate versioned sound-asset manifest in app data. Use Chatterbox Turbo for speech/tags and AudioLDM 2 or Stable Audio Open for effects through a versioned loopback job API. Rust validates worker output and publishes a 48 kHz WAV master plus M4A preview. Model files and Python environments are supplied explicitly; no automatic downloads. Use a bounded standard-library HTTP client because the intended HTTP crate was unavailable in the offline Cargo cache.
Consequences: No manuscript dependency or new Rust dependency. Workers must be installed and started separately. AudioLDM 2 is publicly downloadable but noncommercial; Stable Audio Open requires checkpoint access. The Mac app accepts only loopback URLs; a future private Linux endpoint needs explicit authentication and transport design. Model quality still needs human listening checks.
Related files: `workers/`, `src-tauri/src/services/sound_workers.rs`, `src-tauri/src/services/sound_render.rs`, `src-tauri/src/services/sound_store.rs`, `docs/API_CONTRACTS.md`.

# ADR-0008: Chatterbox voice references replace system narration

Date: 2026-09-22
Status: Accepted

Context: macOS preset narration sounded robotic and offered no reference samples. The user wants local natural voices with recording and import, and a searchable picker across screens.
Decision: Use Chatterbox Turbo for previews, Sound Studio speech, and chapter narration. Keep a local voice library with one selected normalized WAV sample per custom voice. Transfer the sample to the loopback worker with a bounded protocol-v2 upload and discard its temporary copy after generation. Preserve legacy settings fields for migration but stop offering macOS voice presets. Share one searchable voice picker in the renderer.
Consequences: Speech needs a running local Chatterbox worker. A custom voice cannot generate without a selected sample. Samples remain in app data; the worker receives only a transient copy. Future remote GPU workers require an authenticated transport design. Punctuation and supported gesture tags shape phrasing; exact word emphasis is unsupported.
Related files: `src-tauri/src/services/voice_store.rs`, `src-tauri/src/services/speech.rs`, `workers/worker_protocol.py`, `src/renderer/src/VoicePicker.tsx`.

# ADR-0009: Compare effects variations without a manuscript

Date: 2026-09-22
Status: Deprecated (superseded by ADR-0012)

Context: One fabric or panting generation can sound unrealistic and users need a way to compare alternatives.
Decision: Keep Sound Studio independent of projects. Add an effects-only negative prompt and queue three neighboring seeds for comparison in the standalone clip library.
Consequences: Variation quality remains model-dependent; the UI presents selectable output, not a quality guarantee. All generated assets use the existing validated WAV/M4A publishing path.
Related files: `src/renderer/src/SoundStudioPage.tsx`, `src-tauri/src/services/sound_render.rs`, `workers/sfx/server.py`.

# ADR-0010: English section takes and explicit model setup

Date: 2026-09-23
Status: Accepted

Context: Authors need visible chapter progress, manual section repair, natural voiced delivery, and no surprise model downloads. AudioLDM 2 output did not meet the user's quality bar.
Decision: Retain Turbo for its nine documented English gesture tags, add Original for expressive English speech and voice conversion from a recorded delivery, and use Stable Audio Open alone for effects. Pin allow-listed Turbo/Original checkpoint downloads behind explicit Settings actions. Store each narrated section as a project-owned take; conversion creates an unselected preview that must be chosen and assembled. A selected waveform passage can be spliced into a new take with short edge fades. Queue cleanup removes records, never project media.
Consequences: Voice conversion and model quality require real listening. Existing AudioLDM 2 clips remain readable; its generation path is retired. Chapter cues are section/line timed, not word aligned; selection is bounded within one section. Python environments and worker startup remain separate from checkpoint installation.
Related files: `src-tauri/src/services/model_install.rs`, `src-tauri/src/services/project_store.rs`, `src-tauri/src/commands/production.rs`, `workers/chatterbox/original.py`, `src/renderer/src/pages.tsx`.

# ADR-0011: Keep audio deletion within its owning library

Date: 2026-09-23
Status: Accepted

Context: Review, Exports, and standalone clips all store audio with different ownership and recovery rules.
Decision: Validate and delete within each owning store. Review only deletes generated chapter audio; Exports removes project-owned audio/timestamp pairs; the standalone sound store removes its own WAV/M4A pairs. Extend the standalone store to voice-converted clips. Longer effect and voice-conversion requests are split into worker-sized jobs and joined locally.
Consequences: Source manuscripts, imported chapter recordings, and copied voice samples are never deleted by library cleanup. Long renders can be cancelled between segments. Segment joins may have audible boundaries and should be reviewed before export.
Related files: `src-tauri/src/services/project_store.rs`, `src-tauri/src/services/sound_store.rs`, `src-tauri/src/services/sound_render.rs`, `src-tauri/src/commands/sounds.rs`.

# ADR-0012: Retire sound-effect generation

Date: 2026-09-23
Status: Accepted

Context: The sound-effect generator no longer works reliably and its output does not meet the user's quality bar. Sound Studio still displayed unavailable effect controls and worker status.
Decision: Remove effects controls and worker status from the app, reject new effect generation requests, and stop starting the effects worker by default. Keep previously generated effect clips in the independent library for playback, export, and deletion. Preserve the old settings field for migration; the worker source remains dormant.
Consequences: Sound Studio now focuses on English speech with inline vocal gestures. ADR-0009's effects variations and the effects portion of ADR-0004 and ADR-0010 are superseded. Longer speech is split into bounded worker requests and joined locally; the duration control is a maximum, not a way to stretch short text.
Related files: `src/renderer/src/SoundStudioPage.tsx`, `src/renderer/src/pages.tsx`, `src-tauri/src/services/sound_render.rs`, `src-tauri/src/commands/sounds.rs`, `workers/start_local.py`.

# ADR-0013: Narrate selected chapters in one revision-aware job

Date: 2026-09-23
Status: Accepted

Context: Review could narrate one chapter at a time. Independently queued chapter jobs would conflict after the first commit because each commit advances the project revision.
Decision: Queue a single native job for an ordered chapter selection. Snapshot text and voice settings at submission, update the expected revision after every successful chapter, and stop on an external edit, cancellation, or chapter error. Report aggregate progress and named chapter events in Render queue. Keep completed chapters when later chapters stop.
Consequences: Review can generate pending chapters or regenerate selected chapters, including imported audio after confirmation. Batch work continues across tab navigation; Review reloads project state when reopened or when the tracked job finishes. Chapter failures do not roll back earlier results.
Related files: `src-tauri/src/commands/production.rs`, `src-tauri/src/services/jobs.rs`, `src/renderer/src/pages.tsx`, `src/renderer/src/native.ts`.

# ADR-0014: Stop local speech workers when the app exits

Date: 2026-09-23
Status: Accepted

Context: The manual worker launcher detaches Python processes, so Chatterbox models can stay in memory after Homer Studio closes. A retired effects process was also found orphaned.
Decision: The shared worker protocol exposes graceful `POST /v2/shutdown`. On Tauri exit, the app checks protocol version and exact engine at each configured local Chatterbox URL, then requests shutdown with bounded timeouts. Remove the retired effects option from the launcher. Do not manage Ollama or unrelated services.
Consequences: Start local Chatterbox workers again for each app session. Older workers require a restart with the updated protocol before automatic shutdown works. This handles normal app exits, not forced termination or system crashes.
Related files: `src-tauri/src/lib.rs`, `src-tauri/src/services/sound_workers.rs`, `workers/worker_protocol.py`, `workers/start_local.py`.
# ADR-0015: Package Chatterbox workers with an app-owned Python environment

Date: 2026-09-23
Status: Accepted

Context: The previous release launched worker code and a virtual environment from the developer's checkout, so a copied app could not start speech on another Mac. Turbo waveform synthesis also ran outside an enclosing inference-mode scope and the MPS cache was retained after requests.
Decision: Bundle the minimal Python worker source as Tauri resources. Create and verify the pinned Python 3.10 environment under the app's Application Support directory only after the user starts setup in Settings. Keep checkpoints, cache, and logs there as well. Wrap complete synthesis in inference mode and release unused MPS cache after each request.
Consequences: A new Mac needs Python 3.10 and an explicit package/checkpoint install, but no repository checkout. Package setup uses disk and network access. Model weights remain resident while a worker is active; quitting the app stops it.
Related files: `src-tauri/src/services/worker_runtime.rs`, `src-tauri/tauri.conf.json`, `workers/start_local.py`, `workers/chatterbox/server.py`, `workers/chatterbox/original.py`.

# ADR-0016: Diagnose and recover local narration workers at request time

Date: 2026-09-23
Status: Accepted

Context: An installed 0.2.2 app contained bundled worker scripts, but an existing Mac still had its Python environment and checkpoint in the old checkout. Startup logged a missing runtime without showing it in the narration job, which later failed with a generic connection error.
Decision: Before narration, accept a healthy configured worker; otherwise, for the default local worker, report a missing app-owned runtime or checkpoint specifically. If both are installed, start the worker again and verify its health before generating audio. Keep runtime and checkpoint installation explicit in Settings.
Consequences: A stopped local worker can recover during a narration job, and absent setup yields an actionable queue error. Existing checkout installations still need a one-time migration or a Settings install.
Related files: `src-tauri/src/services/speech.rs`, `src-tauri/src/services/worker_runtime.rs`, `src-tauri/src/services/model_install.rs`.

# ADR-0017: Recycle local Chatterbox between short batches

Date: 2026-09-23
Status: Accepted

Context: A chapter batch reached an 18 GB physical footprint and a 30.6 GB peak despite inference mode and clearing the MPS cache. The worker held no growing job history, so model or GPU allocations were surviving completed requests.
Decision: Count successful generations in worker health. Before a new request uploads its voice reference, restart the packaged default-port worker after two completed jobs and verify the replacement. Shorten narration requests to 180 characters to reduce per-request peak allocation. Leave custom worker URLs under their operator's control.
Consequences: Long narration jobs reload the model more often, trading speed for bounded cross-request growth. An individual request can still have a high peak; long-run memory behavior requires an installed-app measurement.
Related files: `workers/worker_protocol.py`, `src-tauri/src/services/sound_workers.rs`, `src-tauri/src/services/speech.rs`, `src-tauri/src/services/sound_render.rs`.

# ADR-0018: Immutable voice production assets and isolated optional processors

Date: 2026-10-05
Status: Accepted

Context: The approved voice-production milestone needs lossless capture, retakes, reversible edits and optional local processors while preserving existing audiobook and Chatterbox workflows.
Decision: Store microphone originals as native-rate mono float WAV and derived takes as 48 kHz float WAV. Persist revisioned memo compositions and selected-take history independently of projects. Render immutable variants with FFmpeg, remap cues from measured spans and publish only accepted results through existing stores. Capture uses one native stream. Optional engines run in separate explicitly installed environments using bounded NDJSON and process-group cancellation.
Consequences: Failed jobs preserve previous media; edits and processing previews remain reviewable. Models/checkouts must be supplied locally, and adapter readiness does not substitute for live inference verification. Cue timing after voice conversion is approximate. Audio files referenced by compositions are retained after soft deletion.
Related files: `src-tauri/src/services/audio_*.rs`, `memo_store.rs`, `voice_profiles.rs`, `waveform.rs`; `workers/audio/`; `src/renderer/src/VoiceMemosPage.tsx`; `docs/VOICE_PRODUCTION.md`.

# ADR-0019: Independent narration timeline with bounded native audio

Date: 2026-10-05
Status: Accepted

Context: Wave Studio needs reversible long-recording edits and independent SFX without loading complete audiobooks in a web view or changing legacy projects.
Decision: Store immutable stereo sources and revisioned timeline metadata independently. Keep pure editing/history in the renderer provider; serve bounded native waveform windows and ten-second mixed audio chunks. Prepare disk-cached tempo variants so preview windows and exports use identical pitch-preserving samples. Voice assignments remain annotations.
Consequences: Originals survive edits, navigation preserves history, and renderer audio memory stays bounded. First playback of a changed-speed long clip can require native preparation; disk cache retains variants referenced by active projects.
Related files: `src/shared/waveStudio*.ts`, `src/renderer/src/WaveStudioProvider.tsx`, `src-tauri/src/services/wave_*.rs`, `docs/WAVE_STUDIO.md`.

# ADR-0020: Explicit Wave production and persisted editing state

Date: 2026-10-05
Status: Accepted

Context: Voice tags alone could not produce audible/exportable conversion, and empty projects required external recordings.
Decision: Use native memo capture, queued local speech/Original conversion with immutable result previews, explicit acceptance and an original timeline baseline. Persist optional view/baseline fields in schema 1. Narration insertion ripples later placements; Join renders only selected narration; normalization measures isolated fragment peaks.
Consequences: Accepted audio uses existing shared preview/export rendering. Large conversion timing mismatches fail rather than moving later audio. Undo history remains session-only.
Related files: `services/wave_processing.rs`, `WaveStudioProvider.tsx`, `WaveRecordingPanel.tsx`, `WaveProductionPanel.tsx`, `docs/WAVE_STUDIO.md`.

# ADR-0021: Persistent voice completion and silent video references

Date: 2026-10-05
Status: Accepted

Context: Reapplying voice conversion processed completed audio repeatedly; project switching and synchronization needed clearer controls.
Decision: Record completed voice production against the selected voice and audible source intervals, preserving it across splits and level changes. Skip complete passages unless regeneration is explicit. Store video as owned silent MP4 references outside audio export. Portable project copies include media and import with new owned IDs; deletion is recoverable.
Consequences: Existing schema-1 projects remain readable through optional fields. Video import may take time; references cannot accidentally contribute audio. History remains session-local while production and view state persist.
Related files: `src/shared/waveStudio.ts`, `WaveProductionPanel.tsx`, `WaveProjectsPanel.tsx`, `WaveVideoReference.tsx`, `services/wave_store.rs`, `services/wave_video.rs`.

# ADR-0022: Shared services and optional Linux browser server

Date: 2026-10-06
Status: Accepted

Context: User requested a Linux GPU machine running Homer Studio through a browser.
Decision: Extract platform-neutral Rust commands into homer-core, retain Tauri adapters, add a single-owner Axum server and renderer HTTP transport. Ship Linux x86_64 archive and optional CUDA container instructions. Require explicit model installation.
Consequences: Server owns media and jobs; browser streams uploads and recording chunks. Loopback is the default; secure tunneling/HTTPS is required for remote microphone access. GPU performance remains unverified without a host.
Related files: crates/homer-core, server, src/renderer/src/platform.ts, docs/LINUX_SERVER.md.

# ADR-0023: Bounded media work and portable Wave archives

Date: 2026-10-06
Status: Accepted

Context: Long imports and stale waveform/playback work exhausted process/file resources; projects needed transferable voice and media state.
Decision: Bound subprocess/decoder work, cancel obsolete requests, stream recording and `.wavehs` archives, and keep conversion chunks sequential. Package owned media and voice references with checked paths and checksums.
Consequences: Large work uses disk staging and may require worker restart after cancellation. Model baseline memory and exact user MP3 failures still require hardware/file-specific validation.
Related files: crates/homer-core/src/services, src/renderer/src/useWaveStudioPlayback.ts, workers/chatterbox/original.py.

# ADR-0024: Platform release artifacts and version increments

Date: 2026-10-06
Status: Accepted

Context: User requested Linux application artifacts, macOS Apple Silicon DMG only and increasing release numbers.
Decision: Publish a runnable Linux x86_64 tar bundle and macOS aarch64 DMG, starting v0.2.7; increment the patch version for each new release. Linux remains the browser-server edition.
Consequences: ZIP is no longer created or required for new macOS releases. Older release assets remain historical. README separates runtime and source-build requirements for each platform.
Related files: `.github/workflows/release.yml`, `scripts/verify-release.mjs`, `scripts/package-server.sh`, `README.md`, `docs/LINUX_SERVER.md`.

# ADR-0025: Recovery ordering and shared studio UI

Date: 2026-10-06
Status: Accepted

Context: Restored projects could export stale/empty state and custom dialogs/controls differed across screens.
Decision: Drain serialized saves before project changes, synchronously maintain current project refs, and load restored manifests before closing the project picker. Purge only validated trash manifests, retaining shared media. Use shared icon/modal/toast components and bounded completed waveform-window reuse over native disk caches. Preserve manual CSS edits.
Consequences: Dialog close waits for recording stop when necessary; errors remain retryable. Memory cache evicts old windows without loading full audio. Linux has no Finder action.
Related files: WaveStudioProvider.tsx, WaveProjectsPanel.tsx, StudioModal.tsx, StudioToast.tsx, waveformCache.ts, wave_store.rs.

# ADR-0026: Reversible voices and compressed project documents

Date: 2026-10-06
Status: Accepted

Context: Retagging converted audio lost earlier accepted results; large binary project files were slow to open and lacked visible progress.
Decision: Persist passage-local original clips and accepted versions by voice. Convert from original, restore cached versions when retagging, and invalidate incompatible snapshots after content edits. Export streaming gzip WAVEHS02 documents and continue reading WAVEHS01. Serialize opening, expose cancellation and adopt validated canonical WAV without decoding again.
Consequences: Projects retain original and alternate media; compression is lossless but size depends on media. Finder/CLI/menu/global drops share the opening workflow. Linux process-group termination uses an explicit option boundary so descendants release output pipes.
Related files: waveStudioEdits.ts, WaveStudioProvider.tsx, wave_studio.rs, wave_processing.rs, wave_bundle.rs, wave_store.rs, process_runner.rs, audio_processors.rs.

# ADR-0027: Incremental project folders and project-scoped audio

Date: 2026-10-06
Status: Accepted

Context: Binary documents were large and opaque; project recordings appeared in global memo lists, and synchronized video needed edited-audio export.
Decision: Default desktop saves use readable `.wavehs` packages and incremental copies. Keep video linked unless explicitly included; preserve legacy archive reading and explicit portable export. Use selected video length for MP4 export, with silent gaps. Scope Wave recordings to their owning project.
Consequences: Moving a linked-video project may require Locate video. Project-owned recordings do not clutter standalone Voice Memos. Manual CSS and waveform caches remain intact.
Related files: wave_store.rs, wave_bundle.rs, wave_video.rs, memo_store.rs, WaveProjectsPanel.tsx, WaveExportPanel.tsx, WaveRecordingPanel.tsx.

# ADR-0028: Explicit audio lanes and non-overlapping video timelines

Date: 2026-10-08
Status: Accepted

Context: Audio imports unexpectedly split narration; overlapping videos and gaps made playback/export ambiguous.
Decision: Default the audio dialog to an independent soundtrack, honor drop lane/time, place videos in the first available full interval, and reject overlapping saves. Preserve legacy reads with explicit Arrange. Preview and combined export hold the previous clip’s final frame in gaps/tail and stay black before the first clip. Import videos sequentially, consolidate input probes and copy compatible H.264.
Consequences: No persisted schema migration. Combined export normalizes video geometry/frame rate and uses edited audio; selected-video export remains supported. Incompatible inputs still require transcoding.
Related files: WaveAudioImportPanel.tsx, WaveStudioPage.tsx, WaveStudioProvider.tsx, waveStudio.ts, wave_video.rs, wave_studio.rs, scripts/test-server.py.

# ADR-0029: Source-bounded soundtrack editing and explicit MP4 length

Date: 2026-10-08
Status: Accepted

Context: Selected effects lacked split/trim controls; second videos were hard to remove, and video-length export cut longer audio.
Decision: Route Split to the selected SFX, expose source-bounded edge handles and explicit selected-video removal. Default new MP4 UI exports to full length, with shorter-track choice and duration preview. Extend video using its final frame and pad short audio; preserve legacy API defaults when lengthMode is absent.
Consequences: Other soundtracks/narration retain placement; audio edits remain undoable. Held-frame selected exports require encoding. A shortest export with no remaining audio is disabled/rejected. User requested patch version tag and push.
Related files: src/shared/waveStudioEdits.ts; src/renderer/src/components/WaveStudioTimeline.tsx; src/renderer/src/components/WaveExportPanel.tsx; crates/homer-core/src/services/wave_video.rs.

# ADR-0030: Visible export preflight and progress

Date: 2026-10-08
Status: Accepted

Context: Native export of a legacy overlapping-video project failed before enqueue; the error appeared behind the export dialog, making Save appear unresponsive.
Decision: Show overlap preflight with explicit Arrange inside the dialog, disable Save until resolved, retain preparation/command errors inline, and display preparation plus queued/job progress immediately.
Consequences: No implicit reordering of user videos; failed starts remain reviewable and retryable. Native media and API validation stay unchanged.
Related files: src/renderer/src/components/WaveExportPanel.tsx; src/renderer/src/WaveStudioPage.tsx; tests/wave-export.test.tsx.

# ADR-0031: Explicit held coverage and complete timeline navigation

Date: 2026-10-08
Status: Accepted

Context: Export extension was difficult to distinguish in the timeline; additional soundtrack rows were clipped and Option arrows were ignored.
Decision: Draw derived held-frame intervals and label held preview frames; scroll the timeline vertically inside a window-constrained grid. Route Option/Ctrl/Command bounds through a shared helper, falling back to videos when narration is empty. Verify both MP4 stream lengths and moving-source final frames.
Consequences: Source durations and project schema remain unchanged. Horizontal/Shift-wheel pans; vertical wheel scrolls tracks. User requested local app/DMG, clearer build instructions, patch tag and push.
Related files: src/shared/waveStudio.ts; WaveStudioPage.tsx; WaveStudioTimeline.tsx; wave-studio.css; wave_video.rs; tests/wave-navigation.test.ts; scripts/test-server.py; README.md.
