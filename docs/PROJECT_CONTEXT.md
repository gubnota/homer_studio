# Project context

## Identity
- Project: Homer Studio (prototype labels: Audiobook Studio / VoxEdit).
- Workspace/repository directory: `homer_studio`.
- Repository: `https://github.com/gubnota/homer_studio.git` (supplied by the user).
- Local Git uses `main` with the supplied GitHub URL as `origin`; implementation commits remain local until the user requests a push.
- Purpose: turn source chapters into local chapter audio, a combined audiobook, and YouTube timestamps.
- Users: authors and audiobook creators on Apple Silicon Macs.

## Current milestone
- v0.3.5: visible held-frame coverage, complete soundtrack scrolling, Option/Ctrl fragment bounds and video navigation fallback. Full-length MP4 checks both stream durations. Local macOS build/replacement instructions in both READMEs.
- v0.3.4: export failures/overlap recovery stay visible inside the dialog; preparation and queued/running/completed progress are explicit. Reproduced overlap rejection in the user’s native app; preserve their project placements.
- v0.3.3: selected soundtrack split/source trim, explicit video removal, full-length or shorter-track MP4 export. User requested new patch tag and push after verification.
- Approved audio/video editing follow-up: explicit separate soundtrack imports, multi-video placement without overlaps, held-frame preview and full-timeline MP4 export. Included in the requested v0.3.3 tag.
- Version 0.3.2: incremental `.wavehs` folders, linked video and video-length MP4 export; project-owned recordings, memo playback/keyboard fixes, compact takes, paired view sliders and stronger button contrast. Preserve manual CSS and waveform caches.
- Version 0.3.1: reversible per-passage voice versions using original audio, compressed backward-readable `.wavehs` documents, global cancellable opening and Linux process-group cleanup fix. Preserve manual CSS/icons and waveform caches.
- Version 0.3.0: approved prototype UI refresh, unified modal/toast behavior, live memo levels, reliable project restore/export/purge, bounded waveform reuse and refreshed bilingual screenshots. Preserve user CSS changes. Release DMG and Linux bundle.
- Version 0.2.9: user requested plain Mute and Normalize button labels after publication of preceding tags; package policy remains DMG only on macOS and Linux application bundle.
- Version 0.2.7: release policy is macOS Apple Silicon DMG only and Linux x86_64 application bundle; README shows the voice editor and separate platform requirements. Increment the patch number for every new release; never reuse published tags.
- Version 0.2.6: bounded imports/playback, cancellable generation, overlapping SFX rows, multiple selection, shortcut help, neutral effect sliders, portable `.wavehs` bundles and an authenticated Linux browser edition. User authorized the version increase and GitHub artifacts.
- Version 0.2.4 includes app-owned Chatterbox setup, narration recovery when a local worker stops, a two-job restart boundary for local Chatterbox memory, per-section narration and takes, batch chapter narration from Review, queue progress/cleanup, and recording-to-narrator conversion.
- The Tauri app, durable project storage, drag-and-drop import, optional local text providers, local neural voice library, narration/import, review, export, and packaging are implemented.
- The arm64 app bundle is verified for patch releases. Review, Exports, and clip libraries have bulk save/delete controls; Voice Lab converts standalone recordings; Review has zoomable waveform playback.
- Version 0.2.5: Wave voice completion tracking, styled editing controls, portable searchable projects, muted video references and explicit M4A export. User authorized pushing and publishing v0.2.5 after successful verification.

## Technology
- CPAL 0.17.0/CoreAudio input capture and Hound float-WAV persistence; FFmpeg renders immutable audio compositions. Device discovery runs off the UI thread and has an optimized-build regression check (0.16.0 crashed in CoreAudio enumeration).
- Tauri 2 desktop shell with a narrow Rust command boundary.
- React 18, TypeScript, Vite, and Vitest renderer reconstructed from prototype components.
- Prototype-derived styling maintained as the visual baseline.
- Shared Rust services in `crates/homer-core`; Tauri desktop adapters and optional Axum HTTP server use the same commands.
- JSON project/configuration files; no database.
- FFmpeg, FFprobe, llama.cpp, and Ollama executables are discovered in bounded system and user Homebrew locations or selected explicitly in Settings.
- Replaceable local text providers: llama.cpp / GGUF and Ollama.
- Local Chatterbox Python workers stop when the desktop app exits, releasing loaded model memory. Ollama is an external service and retains its own lifecycle.
- Packaged worker scripts are Tauri resources. Chatterbox's Python 3.10 environment, checkpoint files, logs, and cache live in app data; setup is explicit in Settings.
- English speech uses local Chatterbox Turbo or Original with a built-in or selected recorded/imported voice reference; Original also converts recorded delivery to the chosen narrator voice.
- Sound Studio uses Chatterbox Turbo for standalone English speech and inline vocal gestures through versioned loopback HTTP. Sound-effect generation is retired; previously generated effects remain in the clip library. Turbo and Original checkpoints can be explicitly installed from Settings.
- GitHub Actions builds and verifies macOS arm64 packages and a Linux x86_64 browser-server archive.

## Hard constraints
- Primary target is macOS arm64 / Apple Silicon.
- Preserve the prototype's layout and visual intent.
- Do not claim compiled assets are a maintainable source project.
- Do not invent a reference application that is not present.
- Remain local-first. Optional user-managed Linux server uses one owner token; no hosted account service or automatic model downloads.
- Models and executables are configurable, never tied to one machine's paths.
- Download Turbo/Original weights only after an explicit user action in Settings; show byte progress and disk use.
- No mandatory Apple Developer credentials for test packages.
- No Intel or universal-build requirement for the first milestone.
- Development packages use ad-hoc signing; public Developer ID signing/notarization is a later distribution concern.

## Core workflow
1. Create/open a project in a user-selected directory.
2. Choose, drop, or paste a TXT/Markdown manuscript.
3. Inspect chapter boundaries and ordering.
4. Edit chapter/segment text.
5. Optionally process text using a selected local LLM.
6. Choose a built-in or custom voice, install/start the English worker, then narrate pending chapters in one batch, regenerate selected chapters, or narrate an individual section; record and convert a section as a preview take when needed.
7. Listen to takes, choose the preferred one, assemble the chapter, then flag or approve it.
8. Combine complete chapter audio using FFmpeg.
9. Save the final audio and measured chapter timestamps.
10. Independently, prompt English speech with supported Turbo gesture tags in Sound Studio, then play, retry, or export clips. Longer speech is split into worker-sized segments and joined locally.
11. In Voice Lab, record or import up to two minutes, convert delivery to a chosen narrator with Original Chatterbox, and save or delete the result.

## UI reference
- Projects and Import screens.
- Project overview with ordered chapter rows.
- Editor with chapter navigation, segment list, and inspector.
- Review screen.
- Voices screen.
- Render Queue screen.
- Exports screen.
- Settings screen.
- Project-independent Sound Studio and clip library.
- Shared custom audio transport in clip, voice and export views; chapter Review retains its waveform transport.
- Neutral backgrounds, white panels, compact typography, and narrow borders.

## Data principles
- Human-readable, versioned project manifest.
- Project content paths are relative to the chosen project root.
- Stable chapter IDs survive reordering.
- Source text is retained when processed text changes.
- Duration comes from actual audio, never source-text estimates.
- Failed chapters must not silently disappear from exports.
- Completed outputs are published only after successful verification.
- Unsupported schema versions must produce actionable errors.

## Privacy and security
- No Base44 authentication, badge, analytics, or hosted assets in the desktop runtime.
- Renderer has no arbitrary filesystem or process access.
- Typed, validated Tauri commands own the UI/native boundary.
- Child processes receive argument arrays with shell execution disabled.
- Local model requests default to loopback endpoints.
- Logs contain operational metadata and errors, not full manuscripts.
- Certificates, tokens, and signing passwords never belong in source control.

## Performance and reliability
- Process long jobs asynchronously with progress and cancellation.
- Run one heavy generation/export operation at a time initially.
- Avoid loading an entire audiobook into renderer memory for playback.
- Use actual cumulative duration values before formatting timestamps.
- Preserve the last good audio when a replacement job fails.
- Recover interrupted jobs as interrupted, never as successful.

## Agent workflow
- Read the six compact context files before targeted code inspection.
- Use `docs/ARCHITECTURE_INDEX.md` to locate relevant modules.
- Keep summaries implementation-aligned and separate proposed from existing behavior.
- Follow the approved implementation plan for source/configuration changes.
- Keep implementation steps small and dependency-ordered.
- Update API contracts when persisted or IPC interfaces change.
- Record accepted architectural changes in `docs/DECISIONS.md`.
- Record completed work and validation in `docs/TASK_LOG.md`.
- Do not scan or paste the whole repository by default.
- Commit each verified implementation stage; do not push until the user asks.

## Voice production
- Voice Memos is project independent; the same editor is embedded in Review, Sound Studio and Voice Lab.
- Sources remain immutable; selected takes and up to 100 undo steps persist in schema-v1 memo metadata.
- Native microphone capture is mono float WAV at the device rate; rendered variants are mono 48 kHz float WAV.
- Optional Seed-VC, RVC, DeepFilterNet and Resemble Enhance use explicit per-engine environments and local models.
- Processor installation/readiness is implemented; real model inference and microphone listening still require manual verification.
- Sentence cues follow edits using measured durations; replacement timing is approximate, with no forced alignment.
- See `docs/VOICE_PRODUCTION.md` for operation, local setup and limitations.

## Navigation
- `docs/ARCHITECTURE_INDEX.md`: repository map.
- `docs/MODULE_OWNERSHIP.md`: accepted boundaries and dependencies.
- `docs/API_CONTRACTS.md`: persisted, command, provider, media, and export contracts.
- `docs/DECISIONS.md`: accepted architecture decisions.
- `docs/TASK_LOG.md`: progress and verification status.
- `docs/IMPLEMENTATION_PLAN_NARRATION_EDITING_AND_MODEL_SETUP.md`: approved current feature plan.

## Wave Studio
- Default workspace: independent persistent narration timeline with voice annotations and a separate SFX lane. Existing chapter, memo and conversion workflows remain available.
- Immutable app-owned 48 kHz stereo sources; non-destructive splits, ripple edits, silence, moves, gain/fades and pitch-preserving speed. Voice assignments label passages without invoking AI.
- App-level undo/redo survives navigation; revisioned schema-v1 projects and viewport settings reopen after restart.
- Renderer requests viewport peaks and bounded ten-second mixed previews. Native FFmpeg renders playback and exports using the same sample-accurate processing.
- Changed-speed clips use disk-cached native tempo variants; first playback/export may wait for preparation. No full audiobook is decoded in the renderer.
- See `docs/WAVE_STUDIO.md` for controls, storage and limits.

## Wave production milestone
- Wave Studio supports empty projects, native recording, queued local TTS, explicit tagged-voice conversion preview/acceptance, Join, ripple insertion, cross-lane moves and peak normalization.
- Project/view and preconversion timeline persist in backward-compatible schema 1. Ten additional user-supplied SFX are bundled. Models still require explicit installation.
