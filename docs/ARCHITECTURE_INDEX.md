# Architecture index

## Reference prototype
- `.gitignore`: excludes `audio_studio_ui/` and `.DS_Store`.
- `audio_studio_ui/index.html`: downloaded SPA entry, including hosted auth/tracking.
- `audio_studio_ui/index-BeQ8ttIE.js`: compiled React application, libraries, sample data, and simulated interactions.
- `audio_studio_ui/index-D-2jFf5J.css`: compiled utility classes, theme variables, and typography.
- `audio_studio_ui/urls.txt`: intended route map.
- `audio_studio_ui/manifest.json`: hosted VoxEdit PWA metadata.
- Other HTML files: identical downloaded SPA entries, not separate screen source files.
- `audio_studio_ui/badge.js`: hosted-platform badge; no desktop reuse planned.
- `docs/`: persistent project context and implementation plan.

## Application
- `src-tauri/`: Tauri lifecycle, permissions, desktop command adapters, native capture/audio protocol, icons and packaging.
- `crates/homer-core/src/commands/project.rs`: native project and manuscript commands.
- `crates/homer-core/src/services/project_store.rs`: schema-v1 persistence, Markdown import, stable sections/takes, audio cues, atomic saves, and filesystem tests.
- `crates/homer-core/src/services/settings.rs`: validated app settings, legacy voice-setting migration, atomic persistence, and local tool/service diagnostics.
- `crates/homer-core/src/services/process_runner.rs`: argument-only child processes, timeout, cancellation, and bounded diagnostics.
- `crates/homer-core/src/services/jobs.rs`: serialized heavy-work queue, bounded event logs, progress, pause/resume/cancel, and terminal-row cleanup.
- `crates/homer-core/src/services/llm.rs`: bounded llama.cpp and loopback Ollama text processing with candidate-only results.
- `crates/homer-core/src/services/spoken_text.rs`: Markdown-to-spoken-text normalization.
- `crates/homer-core/src/services/model_install.rs`: explicit pinned Turbo/Original downloads, verification, progress, and disk use.
- `crates/homer-core/src/services/worker_runtime.rs`: bundled worker resource lookup, app-data Python environment status and installation.
- `crates/homer-core/src/services/speech.rs`: Chatterbox narration, audio import, FFmpeg normalization, duration probing, and waveform peaks.
- `src-tauri/src/services/audio_protocol.rs`: registry-backed project audio delivery with byte-range support.
- `crates/homer-core/src/services/exports.rs`: re-probed chapter assembly, copy/re-encode fallback, duration verification, and timestamp generation.
- `crates/homer-core/src/services/sound_workers.rs`: bounded loopback worker client, health, job polling, cancellation, WAV transfer, and verified shutdown on app exit.
- `crates/homer-core/src/services/sound_render.rs`: standalone request validation, segmented long effects, media verification, and conversion.
- `crates/homer-core/src/services/sound_store.rs`: independent versioned clip library and safe asset paths.
- `crates/homer-core/src/commands/sounds.rs`: generate/list/play/export/delete commands and segmented standalone voice conversion.
- `crates/homer-core/src/commands/system.rs`: settings, diagnostics, and job-control commands.
- `crates/homer-core/src/commands/production.rs`: text candidates, disposable voice previews, queued narration/import/export, per-section takes, recorded-delivery conversion, audio playback, waveforms, chapter review, and export retrieval.
- `src/shared/`: serializable contracts and pure chapter/time logic.
- `src/renderer/src/native.ts`: typed renderer bridge to native commands, validated dropped manuscripts, and file dialogs.
- `crates/homer-core/src/services/voice_store.rs`: app-data voice library, normalized reference samples, selected sample, and deletion.
- `src/renderer/src/VoicePicker.tsx`: searchable modal voice picker shared across production screens.
- `src/renderer/`: React project library, drop-enabled importer, source/candidate chapter editor, neural voices, sample recording, and previews, narration/review, verified export history/timestamps, live queue, settings/tool status, shared controls, and prototype-derived styles.
- `src/renderer/src/SoundStudioPage.tsx`: book-independent prompt, worker status, clip library, playback, retry, export, and deletion.
- `src/renderer/src/VoiceLabPage.tsx`: standalone recording/import, Original Chatterbox conversion, and converted-clip library.
- `src/renderer/public/`: bundled app icon for the sidebar and built-in Turbo voice preview.
- `workers/`: local Turbo/Original Chatterbox Python companions, shared versioned protocol with graceful shutdown, launcher, tests, and setup guide. Retired effects worker code remains for legacy reference and cannot be started by the launcher.
- `tests/standalone-audio.test.tsx`: no-project and sound request UI checks.
- `tests/manuscript-drop.test.ts` and `tests/standalone-audio.test.tsx`: focused renderer behavior tests.
- `tests/DESKTOP_SMOKE.md`: automated and manual packaged-app smoke procedure.
- `scripts/verify-release.mjs`: version, architecture, signature, microphone usage/entitlement, bundled worker resources, and DMG package verification.
- `scripts/smoke-app.mjs`: isolated packaged-app startup check.
- `.github/workflows/ci.yml`: push/PR checks and Apple Silicon app build.
- `.github/workflows/release.yml`: version-tagged arm64 DMG and Linux x86_64 bundle publishing.
- `README.md`: user setup, local engine configuration, workflow, checks, and packaging.
- Root npm/Vite/TypeScript configuration: renderer development, checks, and Tauri arm64 packaging.
- `docs/IMPLEMENTATION_PLAN.md`: exact planned filenames and verification commands.
- `docs/IMPLEMENTATION_PLAN_INPUTS_AND_VOICES.md`: completed local-tool, manuscript-drop, and voice-preset plan.

## Entry points and build
- `src/renderer/main.tsx`: renderer entry.
- `src-tauri/src/main.rs` and `src-tauri/src/lib.rs`: native entry and command registration.
- `src-tauri/tauri.conf.json`: application identity, windows, CSP, bundle, and icons.
- `src-tauri/Info.plist` and `src-tauri/Entitlements.plist`: microphone purpose text and hardened-runtime audio-input access; bundle signing includes the entitlement.
- `src-tauri/capabilities/default.json`: renderer permission allowlist.
- `npm run dev`: Tauri development app.
- `npm run build`: typecheck, renderer tests, and renderer production bundle.
- `npm run test:integration`: real FFmpeg export fixture.
- `npm run pack:mac`: release `.app` bundle plus arm64/signature verification.
- `npm run test:smoke`: packaged app startup check.
- `npm run package:mac`: verified Apple Silicon app, DMG, and ZIP.

## Shared voice production
- `src/shared/audio.ts`: memo, composition, capture, processing and voice-profile contracts.
- `src/renderer/src/audioNative.ts`: typed audio/capture/processor bridge.
- `src/renderer/src/VoiceMemosPage.tsx`: standalone/embedded memo library, recording, retakes, exports and publication.
- `src/renderer/src/components/AudioEditor.tsx`: selections, waveform, edits, take previews and sentence cues.
- `AudioTransport.tsx`, `AudioProcessingPanel.tsx`, `StudioControls.tsx` in the same folder: playback, optional processors and styled controls.
- `src/renderer/src/useNativeRecording.ts`: lossless capture shared by legacy recording entry points.
- `crates/homer-core/src/commands/audio.rs`, `audio_capture.rs`, `audio_processing.rs`: validated IPC and queued operations.
- `crates/homer-core/src/services/audio_assets.rs`, `audio_edits.rs`, `memo_store.rs`: immutable WAV assets, composition rendering and revisioned memo persistence.
- `audio_capture.rs`, `waveform.rs` in services: CoreAudio capture lifecycle and bounded multilevel peak cache.
- `audio_processors.rs`, `voice_profiles.rs` in services: isolated NDJSON supervisors and backward-compatible voice sidecars.
- `workers/audio/processor.py`, `inference_driver.py`: lazy local inference adapters; per-engine `.txt` files pin environment dependencies.
- `workers/audio/test_processor.py`, `tests/audio-studio.test.tsx`: protocol validation and shared editor regression coverage; native tests remain colocated with services.
- `docs/VOICE_PRODUCTION.md`: setup and manual verification guide.

## Wave Studio
- `src/shared/waveStudio.ts`, `waveStudioEdits.ts`: schema, timing and immutable editing operations.
- `src/renderer/src/WaveStudioProvider.tsx`: session history, revisioned saves, restore and import handoffs.
- `WaveStudioPage.tsx`, `components/WaveStudio*.tsx`, `WaveformCanvas.tsx`: project toolbar, selection, inspector, libraries and canvas lanes.
- `useWaveStudioPlayback.ts`, `waveStudioNative.ts`: bounded Web Audio scheduling and typed native calls.
- `crates/homer-core/src/commands/wave_studio.rs`: import, persistence, peaks, preview cancellation, exports and SFX commands.
- `services/wave_studio.rs`, `wave_store.rs`, `wave_render.rs`, `wave_processing.rs`, `wave_video.rs`, `sfx_store.rs`: validation, owned media, shared preview/export processing and reusable effects.
- `resources/sfx/sitcom_laugh01.m4a`: immutable bundled Audience effect; release verification requires it.
- `src/renderer/styles/wave-studio.css`; `tests/wave-studio.test.ts`; native validation/render tests in the services above.

- `components/WaveRecordingPanel.tsx` and `WaveProductionPanel.tsx`: microphone capture and queued TTS/tagged-voice preview/acceptance.

- `components/WaveProjectsPanel.tsx`, `WaveExportPanel.tsx`, `WaveVideoReference.tsx`: searchable portable project manager, explicit WAV/M4A format choice and synchronized muted video frame.

## Optional Linux browser edition (0.2.6)
- Root `Cargo.toml` / `Cargo.lock`: shared core and server workspace; desktop keeps its platform-specific lockfile.
- `crates/homer-core/src/{commands,services}`: shared persistence, jobs, model workers, media and Wave editor logic.
- `crates/homer-core/src/services/wave_bundle.rs`: streamed, checked `.wavehs` archives and voice/media remapping.
- `server/src/main.rs`, `server/src/api.rs`: owner authentication, allowed command dispatch, scoped files, uploads, recording append and range streaming.
- `src/renderer/src/platform.ts`, `browserCapture.ts`: desktop/HTTP transport, downloads and bounded browser microphone uploads.
- `server/{run.sh,Dockerfile,compose.yaml}`, `scripts/package-server.sh`: server operation, optional GPU container and release archive.
- `scripts/test-server.py`, `tests/server-transport.test.ts`: isolated real-media HTTP flows and transport regressions.
- `docs/LINUX_SERVER.md`: secure access, deployment, explicit model installation and GPU requirements.

## Release documentation
- `docs/images/wave-editor-voices.jpg`: actual isolated editor screenshot embedded in README.
- Versioned releases publish macOS aarch64 DMG only and a runnable Linux x86_64 tar bundle.

- `README.ru.md`: Russian setup and workflows, kept aligned with the English README.

- Shared UI: `StudioIcon.tsx`, `StudioModal.tsx`, `StudioToast.tsx`; waveform memory LRU: `src/renderer/src/waveformCache.ts` (120 windows per editor).

## v0.3.2 storage and export
- `wave_store.rs` / `wave_bundle.rs`: folder binding, incremental media copies, legacy archive import and explicit portable archive export.
- `wave_video.rs`: one-probe import classification, compatible H.264 copy, relinking, selected-video mux and cancellable normalized full-timeline MP4 with held gap/tail frames.
- `AudioTransport.tsx`, `AudioEditor.tsx`, `VoiceMemosPage.tsx`: playback lifecycle, compact takes and recording shortcuts.

- `components/WaveAudioImportPanel.tsx`: explicit soundtrack/narration import modes and position. `tests/wave-import.test.tsx` checks default intent; shared Wave tests cover collision placement and held frames.

- Soundtrack source edits: `src/shared/waveStudioEdits.ts` (`splitSoundtrack`, `trimSoundtrack`, `soundtrackRange`); UI handles in `WaveStudioTimeline.tsx`, numeric controls in `WaveStudioInspector.tsx`. `WaveExportPanel.tsx` and shared `wave_video.rs` own length selection and held-frame rendering.

- `tests/wave-export.test.tsx`: export dialog overlap blocking/recovery visibility. `WaveExportPanel.tsx` owns preparation and inline errors; `WaveStudioPage.tsx` owns queued/job progress.

- Shared `waveStudio.ts` owns fragment navigation/video hold intervals; `tests/wave-navigation.test.ts` verifies narration precedence, video fallback and hold coverage. `wave-studio.css` constrains the editor grid and owns vertical timeline scrolling.
