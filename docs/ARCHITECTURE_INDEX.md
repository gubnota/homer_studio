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
- `src-tauri/`: Tauri lifecycle, permissions, Rust commands, icons, build configuration, and future native services.
- `src-tauri/src/commands/project.rs`: native project and manuscript commands.
- `src-tauri/src/services/project_store.rs`: schema-v1 persistence, Markdown import, stable sections/takes, audio cues, atomic saves, and filesystem tests.
- `src-tauri/src/services/settings.rs`: validated app settings, legacy voice-setting migration, atomic persistence, and local tool/service diagnostics.
- `src-tauri/src/services/process_runner.rs`: argument-only child processes, timeout, cancellation, and bounded diagnostics.
- `src-tauri/src/services/jobs.rs`: serialized heavy-work queue, bounded event logs, progress, pause/resume/cancel, and terminal-row cleanup.
- `src-tauri/src/services/llm.rs`: bounded llama.cpp and loopback Ollama text processing with candidate-only results.
- `src-tauri/src/services/spoken_text.rs`: Markdown-to-spoken-text normalization.
- `src-tauri/src/services/model_install.rs`: explicit pinned Turbo/Original downloads, verification, progress, and disk use.
- `src-tauri/src/services/speech.rs`: Chatterbox narration, audio import, FFmpeg normalization, duration probing, and waveform peaks.
- `src-tauri/src/services/audio_protocol.rs`: registry-backed project audio delivery with byte-range support.
- `src-tauri/src/services/exports.rs`: re-probed chapter assembly, copy/re-encode fallback, duration verification, and timestamp generation.
- `src-tauri/src/services/sound_workers.rs`: bounded loopback worker client, health, job polling, cancellation, and WAV transfer.
- `src-tauri/src/services/sound_render.rs`: standalone request validation, generation, media verification, and conversion.
- `src-tauri/src/services/sound_store.rs`: independent versioned clip library and safe asset paths.
- `src-tauri/src/commands/sounds.rs`: narrow generate/list/play/export commands.
- `src-tauri/src/commands/system.rs`: settings, diagnostics, and job-control commands.
- `src-tauri/src/commands/production.rs`: text candidates, disposable voice previews, queued narration/import/export, per-section takes, recorded-delivery conversion, audio playback, waveforms, chapter review, and export retrieval.
- `src/shared/`: serializable contracts and pure chapter/time logic.
- `src/renderer/src/native.ts`: typed renderer bridge to native commands, validated dropped manuscripts, and file dialogs.
- `src-tauri/src/services/voice_store.rs`: app-data voice library, normalized reference samples, selected sample, and deletion.
- `src/renderer/src/VoicePicker.tsx`: searchable modal voice picker shared across production screens.
- `src/renderer/`: React project library, drop-enabled importer, source/candidate chapter editor, neural voices, sample recording, and previews, narration/review, verified export history/timestamps, live queue, settings/tool status, shared controls, and prototype-derived styles.
- `src/renderer/src/SoundStudioPage.tsx`: book-independent prompt, worker status, clip library, playback, retry, and export.
- `workers/`: local Turbo/Original Chatterbox and Stable Audio Open Python companions, shared versioned protocol, launcher, tests, and setup guide.
- `tests/standalone-audio.test.tsx`: no-project and sound request UI checks.
- `tests/manuscript-drop.test.ts` and `tests/standalone-audio.test.tsx`: focused renderer behavior tests.
- `tests/DESKTOP_SMOKE.md`: automated and manual packaged-app smoke procedure.
- `scripts/verify-release.mjs`: version, architecture, signature, and package verification plus ZIP creation.
- `scripts/smoke-app.mjs`: isolated packaged-app startup check.
- `.github/workflows/ci.yml`: push/PR checks and Apple Silicon app build.
- `.github/workflows/release.yml`: version-tagged DMG/ZIP release publishing.
- `README.md`: user setup, local engine configuration, workflow, checks, and packaging.
- Root npm/Vite/TypeScript configuration: renderer development, checks, and Tauri arm64 packaging.
- `docs/IMPLEMENTATION_PLAN.md`: exact planned filenames and verification commands.
- `docs/IMPLEMENTATION_PLAN_INPUTS_AND_VOICES.md`: completed local-tool, manuscript-drop, and voice-preset plan.

## Entry points and build
- `src/renderer/main.tsx`: renderer entry.
- `src-tauri/src/main.rs` and `src-tauri/src/lib.rs`: native entry and command registration.
- `src-tauri/tauri.conf.json`: application identity, windows, CSP, bundle, and icons.
- `src-tauri/capabilities/default.json`: renderer permission allowlist.
- `npm run dev`: Tauri development app.
- `npm run build`: typecheck, renderer tests, and renderer production bundle.
- `npm run test:integration`: real FFmpeg export fixture.
- `npm run pack:mac`: release `.app` bundle plus arm64/signature verification.
- `npm run test:smoke`: packaged app startup check.
- `npm run package:mac`: verified Apple Silicon app, DMG, and ZIP.
