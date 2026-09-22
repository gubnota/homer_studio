# Task log

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
