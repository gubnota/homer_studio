# Task log

## 2026-09-23 — English narration and model setup
- Approved plan: `docs/IMPLEMENTATION_PLAN_NARRATION_EDITING_AND_MODEL_SETUP.md`. Commits are local; pushing is deferred. The user's heading-font change in `src/renderer/styles/app.css` is deliberately untouched.
- Render Queue now exposes actual chunk progress, elapsed phase, bounded logs/errors, Select all/Deselect all/Clean, and cancel. Chapter export/delete and Sound Studio's Kind modal, selection controls, and improved spacing are in place. Save-dialog permission and explicit macOS icon verification were added.
- Markdown is normalized before narration. Chapters have stable editable sections, non-destructive saved takes, manual Generate remaining and Assemble, measured section cues, waveform navigation, and section-scoped recording. Original Chatterbox voice conversion creates a preview take; choosing a take is explicit.
- Turbo and Original English checkpoints have explicit install/retry/cancel actions with byte progress and disk use. AudioLDM 2 generation was removed; existing clips remain in the library.
- An additional waveform editor now selects a bounded range inside one assembled section; FFmpeg splices converted delivery into a new take with 10 ms edge fades. Earlier takes remain selectable. FFmpeg tests cover replacement at the beginning, middle, and end. All six renderer tests and 28 Rust tests pass. The rebuilt arm64 `.app` passed architecture/signature/icon checks and packaged-app startup smoke. Real voice conversion has not been listened to because local Original weights/dependencies are not installed.

## 2026-09-22 — Reliable custom voice previews
- Voice sample imports already normalize and copy source audio into app-owned storage; the UI now confirms users can move the original file.
- Align voice sample counts to the right edge of each card.
- Custom voice previews synthesize in a background thread when samples are added or selected, then cache a measured M4A beside the voice. Preview playback reads the cache and returns a pending message while synthesis is running.
- Renderer typecheck, six tests, production web build, Rust formatting for touched files, and `cargo check` passed. Model-side preview latency remains dependent on the local Chatterbox worker.

## 2026-09-22 — Neural voices and Sound Studio controls
- User approved `docs/IMPLEMENTATION_PLAN_NEURAL_VOICES_AND_CONTROLS.md`. Four local stage commits added app-owned voice samples, Chatterbox previews and narration, a searchable voice picker with microphone recording/import, and three seeded Sound Studio effects variations. No push was made.
- Voice references use one selected clear 6–20 second sample; the minimum was raised after a real Chatterbox job rejected a shorter clip. Voices screen and setup guides state the usable range. Legacy macOS preset settings still migrate, but no new speech path invokes system narration.
- Renderer typecheck, six tests across three files, production web build, 22 native tests, and seven worker protocol tests passed. A real local Chatterbox default-voice WAV, a 3.56-second WAV conditioned on a 12.84-second reference, and an AudioLDM 2 effects WAV completed and were probed. The final arm64 app, DMG, and ZIP passed package verification; the packaged app passed its startup smoke check.
- Sound realism, spoken delivery, microphone permission behavior, modal layout, and long chapter narration still need listening and interactive packaged-app review. AudioLDM 2 quality is model-dependent; three variations are a selection aid.

## 2026-09-22 — Local sound worker repair
- User reported both sound workers unavailable, Ollama falsely available, and odd effect output. Push remains deferred.
- Ollama is running locally but `/api/tags` reports no installed models. Diagnostics now query the model API and distinguish unreachable service, empty model list, missing selected model, and ready service.
- Installed Chatterbox Turbo in a repo-local Python 3.10 environment and its complete checkpoint; tested direct generation and the versioned worker API on port 8765.
- Stable Audio Open's checkpoint requires separate access, so added publicly downloadable AudioLDM 2 as an effects engine behind the same port 8766 contract. Installed its complete local checkpoint, fixed published-checkpoint compatibility with current Diffusers, and generated a measured four-second fabric WAV through both the model and worker API. AudioLDM 2 is noncommercial, and sound quality requires listening review.
- Added a launcher for both installed workers, actionable offline guidance, category-specific prompts, and checkpoint engine selection. Local workers are running for this verification; launch them again after a restart.
- Renderer typecheck, nine renderer tests, production web build, 24 Rust tests including FFmpeg integration, five Python worker tests, and worker API generation passed.
- Built and verified the ad-hoc signed arm64 `.app`; packaged startup passed with normal macOS GUI access. The packaged Sound Studio displayed both workers as ready and retained generated effects clips; Settings displayed Ollama as running with no models installed. The restricted sandbox cannot launch the app UI and exits with `SIGABRT`.

## 2026-09-22 — Standalone prompt-to-audio studio
- User approved `docs/IMPLEMENTATION_PLAN_STANDALONE_AUDIO.md`; work remains local and pushes are deferred.
- Added versioned loopback Python workers for Chatterbox Turbo speech/tags and Stable Audio Open effects, with stub protocol tests and manual checkpoint setup instructions.
- Added validated Rust worker client, isolated sound library, WAV/M4A media pipeline, and narrow Tauri commands. Added project-independent Sound Studio, model status/settings, clip playback, retry, and export.
- Renderer typecheck, four test files (nine tests), production web build, 23 Rust tests including FFmpeg integration, and four Python worker tests passed.
- FFmpeg/FFprobe are present. Python worker packages and checkpoints are absent, and downloads were unavailable; actual model loading, Apple Silicon MPS inference, and generated-audio quality remain unverified. Workers remain visibly unavailable until installed.
- Built and verified the ad-hoc signed arm64 `.app`, `.dmg`, and `.zip`; the isolated packaged app remained healthy through its startup smoke window. DMG assembly and GUI launch required normal macOS access outside the restricted command sandbox. The real FFmpeg integration fixture also passed.

## 2026-09-07 — Tool setup, manuscript drop, and voice presets
- User approved `docs/IMPLEMENTATION_PLAN_INPUTS_AND_VOICES.md`; implementation remained on local `main`, with pushes deferred.
- Tool setup checkpoint (`a8a7be1`): added packaged-app discovery for bounded system and user Homebrew locations, visible FFmpeg/FFprobe/llama.cpp/GGUF/Ollama path controls, detected-path actions, and separate Ollama CLI/server diagnostics.
- Manuscript checkpoint (`878d6d3`): added Tauri drag/drop handling, scoped listener cleanup, supported-file validation, title population, and focused renderer tests.
- Voice checkpoint (`8d0b86d`): added installed-only built-in presets, migration of saved speech settings, custom preset create/edit/delete/select behavior, real local preview synthesis, and preview cleanup.
- Final validation passed with three renderer test files containing seven tests, 20 Rust tests, and the real FFmpeg integration fixture.
- The packaged app found FFmpeg, FFprobe, llama.cpp, and Ollama in the user's local Homebrew prefix; Ollama CLI detection remained separate from the stopped loopback service.
- Packaged-app checks created, selected, and removed a custom Samantha preset, produced a four-second preview, and generated chapter 7 as a valid 60.251-second M4A. The built-in Warm narrator preset was restored afterward.
- Built, ad-hoc signed, and verified the arm64 app, DMG, and ZIP. The isolated packaged-app startup smoke test remained healthy for four seconds.

## 2026-09-07 — Implementation authorized
- User approved the saved plan and requested a human-readable commit after each verified stage.
- Remote pushes are deferred until the user asks.
- First delivery target is a locally tested macOS arm64 build.
- User selected Tauri after reviewing the initial Electron choice; the uncommitted shell was migrated before its first source commit.
- First Tauri checkpoint: renderer typecheck and tests pass, Rust check passes, and an ad-hoc signed 3.9 MB arm64 app launches at `tauri://localhost` with working navigation and native runtime information.
- Durable-project checkpoint: implemented versioned local projects, TXT/Markdown import, heading detection, bounded segmentation, chapter editing/reordering, revision conflict checks, safe relative paths, manifest backup, and atomic saves.
- Verified the project layer with four Rust tests, renderer typecheck/tests, a production bundle, and a reopened arm64 packaged app showing the native project library.
- Process/settings checkpoint: added schema-v1 settings in the native app configuration folder, loopback validation, real executable discovery, bounded subprocess execution, serialized job state, pause/resume/cancel controls, and live Queue/Settings screens.
- Seven Rust tests now pass, including cancellation of a real disposable child and pause-at-boundary behavior; renderer typecheck, tests, and production build also pass.
- Local-text checkpoint: connected llama.cpp and loopback Ollama through bounded native calls, added candidate review/accept/discard controls, and kept accepted narration text separate from the manuscript source.
- Nine Rust tests and two renderer tests pass; renderer typecheck and production build pass.
- Narration/review checkpoint: added installed macOS voice selection, queued speech generation and audio import, canonical M4A conversion, measured durations, registered range-capable playback, waveform peaks, stale-audio invalidation, and approve/changes-requested state.
- Twelve Rust tests and two renderer tests pass. Renderer typecheck/build pass, and a 1.25-second synthetic import produced a measured M4A and 5,015 decoded waveform samples.
- Export checkpoint: added current/approved eligibility, export history, re-probing, verified concat-copy with canonical AAC fallback, cumulative timestamps, range playback, copyable saved chapter marks, and transactional publication of the output pair.
- Fifteen Rust tests and two renderer tests pass. The media test covers copy and mixed-format fallback using real FFmpeg fixtures in a Unicode/quoted path; renderer typecheck and production build pass.
- Release checkpoint: added architecture/signature/package verification, isolated packaged-app smoke testing, Apple Silicon CI and tagged-release workflows, setup documentation, and the release checklist.
- Full local verification passed: renderer typecheck/build and two tests, all 15 Rust tests, the real FFmpeg integration fixture, arm64/signature checks, and a four-second packaged startup smoke test.
- Installed-voice verification also passed with normal macOS speech-service access: Samantha generated a non-empty 90,968-byte AIFF measured at 1.970 seconds.
- Created the ad-hoc signed `Homer Studio.app`, `Homer Studio_0.1.0_aarch64.dmg`, and `Homer Studio_0.1.0_aarch64.zip`. DMG assembly required normal macOS mount/Finder access outside the restricted command sandbox.

## 2026-09-07 — Desktop implementation planning
- Request: inspect the attached specification and prototype; save a plan; wait for a later implementation prompt.
- Read the plan-before-implementation skill and all supplied requirements.
- The six context files were absent; initialized them as documentation only.
- Inspected the top-level workspace, prototype entry/manifest/routes, targeted compiled screen functions, styling, and mock data.
- Found React 18.3.1, compiled utility CSS, hosted authentication/tracking, and simulated generation/playback/export.
- Found no original source, package configuration, source maps, desktop application, tests, or CI. Git metadata was absent on initial inspection; final verification found an initialized `main` branch with no commits and the supplied URL as `origin`.
- User supplied `https://github.com/gubnota/homer_studio.git`; read-only remote inspection succeeded but returned no HEAD, branches, or tags.
- Local diagnostics: arm64 host; Node/npm, FFmpeg/FFprobe, llama-cli, and Ollama are on PATH; inference has not been tested.
- Asked for first-version speech scope and CI host; user identified GitHub. The speech baseline remains a proposal.
- Verified relevant framework, runner, model, and media documentation using primary sources.
- Completed `docs/IMPLEMENTATION_PLAN.md`: reuse map, Electron architecture, exact file inventory, contracts, ordered steps, tests, arm64 packaging, and GitHub release plan.
- Speech baseline is explicitly proposed as macOS installed voices plus import, pending approval of the plan.
- Planning-only validation: checked document presence, internal file references, headings, and absence of source/config changes.
- This planning task performed no implementation, dependency installation, builds, tests, Git initialization, commits, or deployment.
# 2026-09-22 — Expressive narration and line playback
- Added supported vocal gesture tags and bounded `[pause:ms]` cues to Chatterbox narration, with parser coverage.
- Generated chapter manifests now persist line cues; review playback can seek to and highlight each cue. Imported files correctly have no generated line timings.
- Documented expressive markup, example dramatic pacing, cue behavior, and current line timing limitations in the UI and API contract.
- Validation/build/release status: in progress.

## 2026-09-22 — Apple Silicon 0.2.0 release checkpoint
- Refreshed the app icon set and aligned package/app versions at 0.2.0; saved the approved implementation/release plan.
- Renderer build (6 tests), full native suite (24 tests), arm64 signature verification, packaged startup smoke test, ZIP creation, and `git diff --check` passed.
- Apple Silicon DMG bundling failed in `bundle_dmg.sh` both within and outside the restricted sandbox; app bundle and ZIP were produced and verified instead.
- GitHub remote publishing remains to be attempted after the local release commit and tag are ready.

# 2026-09-23 — Audio libraries, Voice Lab, and review player
- Added Review, Exports, and Sound Studio save/delete selection controls with store-specific ownership checks. First stage committed as `0ecc6cb Add save and delete controls to audio libraries`.
- Added standalone Voice Lab recording/import and Original Chatterbox voice conversion; up to two minutes are divided into 15-second worker requests and joined into one saved clip. Sound effects now allow up to two minutes via joined 20-second worker renders.
- Review playback now uses custom play/pause/stop controls, bounded waveform-window loading, zoom, panning, seeking, and passage selection. Added a bundled built-in Turbo narrator preview and immediate selected-sample fallback for custom voices.
- Replaced the sidebar H badge with the exact macOS app icon asset. Renderer build (six tests), 28 Rust tests, nine worker tests, arm64 package/signature verification, and packaged-app startup smoke passed. Changes remain local until the user requests a push.

# 2026-09-23 — Retire effects and extend standalone speech
- Removed sound-effect generation controls and availability reporting; existing clips remain playable, exportable, and deletable. The launcher defaults to Chatterbox only.
- Sound Studio accepts maximum duration up to 120 seconds for speech. Longer text is split into bounded Turbo requests and joined in source order. Dedicated vocal-gesture requests retain the 120-second native limit; inline gesture tags remain available in speech.
- Renderer typecheck, six tests, production web build, 29 native tests, nine worker tests, arm64 app/signature verification, and packaged-app startup smoke passed. The build remains local; no push was requested.

# 2026-09-23 — Batch chapter narration from Review
- Approved `docs/IMPLEMENTATION_PLAN_REVIEW_BATCH_NARRATION.md` and added one revision-aware queue job to narrate chapters in project order. Each committed chapter remains available after a later failure or cancellation; queue events show the current/completed chapter and errors.
- Review now selects chapters with or without audio, offers **Generate pending chapters** and **(Re)Generate selected**, confirms replacement of existing audio, and tracks the batch across tab navigation. Existing bulk save/delete filtering remains audio-aware.
- Frontend typecheck/build and nine tests, all 33 native tests, `git diff --check`, arm64 app/signature verification, and packaged-app startup smoke passed. The packaged app is local; a live multi-chapter Chatterbox listening test remains manual.

# 2026-09-23 — Release local model memory on app exit
- Added graceful shutdown to the loopback Chatterbox worker protocol and verified engine identity before requesting it on normal Tauri exit. Removed the retired effects worker from the launcher; Ollama remains separately managed.
- Stopped the previously orphaned `workers/sfx/server.py` process (PID 10451) after confirming its command path; it is no longer running.
- Python protocol tests (10), native tests (35), renderer build/tests (9), arm64 bundle/signature verification, and packaged startup smoke passed. The smoke check sends SIGTERM, so the normal GUI Quit plus a real loaded model remains a manual lifecycle check.

# 2026-09-23 — Apple Silicon 0.2.1 release
- Prepared the patch release because the existing `v0.2.0` tag already identifies an earlier published commit.
- Release metadata now uses version 0.2.1 across the npm package, Tauri configuration, and native package manifest. The Apple Silicon package is rebuilt before the new annotated tag is published.
- Fixed the Ollama readiness fixture so it consumes all request headers before closing the loopback socket; the targeted test passed ten consecutive times after the repair, and the complete 36-test native suite passed.
- Homer Studio now starts the repository-installed Chatterbox Turbo launcher in the background during app startup. The launcher remains idempotent, so a normal restart restores a stopped worker without duplicating a healthy one.
# 2026-09-23 — Portable Chatterbox runtime and memory control
- Added an implementation plan for a bundled worker and app-data Python environment. Settings can install pinned packages using a discovered or selected Python 3.10 interpreter; worker startup uses bundled resources and app-data paths.
- Wrapped complete Turbo and Original generation in PyTorch inference mode and release unused MPS cache after each request. The old running worker was measured at roughly 15.5–16.6 GiB resident; an isolated updated Turbo worker completed ten short requests and remained near 2.7 GiB resident. Long chapter memory behavior still needs interactive review.
- Extended first-start readiness polling to 30 seconds and made Settings status use the selected Python path. Release verification now requires all bundled worker files. Native tests (35), renderer tests (9), Python protocol tests (10), arm64 app/DMG/ZIP verification, and packaged startup smoke passed.

# 2026-09-23 — Recover narration after portable runtime migration
- Found the installed 0.2.2 app had bundled worker code but no Python runtime or Turbo checkpoint in Application Support; the old 1.3 GB environment and 2.8 GB checkpoint remained in the checkout. Migrated those files into app data on this Mac and verified imports.
- The installed worker started from its bundled launcher and completed a real short speech request. Resident memory after model load was about 2 GB.
- Narration now checks the local runtime and checkpoint when health fails, gives a specific Settings action for missing setup, and restarts an installed local worker before retrying health. Custom worker URLs keep their existing behavior.
- Version 0.2.3 passed 35 native tests, nine renderer tests, typecheck/build, arm64 app/DMG/ZIP verification, and packaged startup smoke. A full multi-chapter listening run remains manual.

# 2026-09-23 — Bound Chatterbox memory across chapter requests
- A user process sample confirmed an 18 GB worker footprint and 30.6 GB peak during a batch; cache clearing alone was insufficient.
- Local Chatterbox workers now report successful generation count and restart after every two completed jobs before receiving the next voice reference. Narration chunks are limited to 180 characters. Existing staged audio and chapter commits survive a restart at the chunk boundary.
- Native suite (36 tests), Python protocol tests (10), renderer build (9 tests), and arm64 app/DMG/ZIP verification passed. The installed-app long-running memory test remains pending; the user's currently running 0.2.3 batch was not interrupted.
