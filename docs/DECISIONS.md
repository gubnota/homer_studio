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
