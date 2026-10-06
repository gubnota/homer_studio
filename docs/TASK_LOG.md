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

# 2026-09-23 — Download completed chapters during a batch
- Review now refreshes when each chapter is committed and keeps Save chapter audio available while later chapters render. Exports exposes per-chapter downloads without requiring full-audiobook approval.
- Renderer typecheck, nine tests, production web build, and local arm64 app/signature verification passed. The installed app still needs the updated bundle to show these controls.

# 2026-10-05 — Approved voice production implementation
- Implemented the approved voice production plan: native lossless microphone capture, immutable audio assets, revision-aware memos/takes, reversible selection edits, waveform caching, retake previews, crossfades, and shared playback/editor controls.
- Added Voice Memos and shared recording/editing integration in Review, Sound Studio, and Voice Lab; persisted take names, favorites, notes, context, undo/redo, voice profiles, processing history, and remapped sentence cues.
- Added explicit isolated runtime setup and supervised NDJSON adapters for Seed-VC, RVC, DeepFilterNet, and Resemble Enhance. Original Chatterbox remains supported; jobs serialize, report stages, and cancel child process groups.
- Accepted compositions can become chapter/segment audio, sound assets, or voice references. Exports support WAV, FLAC, AAC, and MP3 with measured duration checks; chapter imports retain lossless WAV sources.
- Validation: renderer typecheck/production build and 12 tests, 49 native tests (including real FFmpeg exports and waveform fixtures), seven processor tests, and ten existing worker tests passed. Local arm64 app packaging, bundled resource/signature verification, packaged startup smoke, and whitespace checks passed.
- Added `docs/VOICE_PRODUCTION.md` with engine setup, editing workflows, and manual verification steps. Microphone permission/device behavior, listening checks, actual model installation/inference, and CPU/MPS compatibility still require hands-on verification; automated checks do not establish model quality or availability.
- Saved in local commits only. No remote push or publishing was requested.

# 2026-10-05 — Repair voice workspace navigation crashes
- macOS crash reports traced Voice Lab, Voice Memos and Sound Studio failures to shared CoreAudio input-device discovery. CPAL 0.16.0 passed an immutable size output to CoreAudio, producing an invalid device buffer in optimized builds.
- Pinned CPAL 0.17.0 (with the upstream buffer fix), adapted capture device/sample-rate APIs, and moved discovery to a blocking worker instead of the UI thread. Added repeated macOS discovery coverage and a release-mode verification command.
- Startup verification also found Python bytecode being written into signed bundled resources. The local worker launcher now disables bytecode writes for its child workers.
- Validation: 50 optimized native tests, renderer typecheck/build and 12 tests, ten worker protocol/lifecycle tests, final arm64 package/signature verification and packaged startup smoke passed. GUI/loopback checks ran outside the restricted sandbox where required.
- Updated `/Applications/Homer Studio.app`, retaining the previous bundle at `/private/tmp/homer-studio-before-device-fix-ovs3tlzw/Homer Studio.app`. Verified all three screens in the installed app, normal Quit/restart, and Voice Memos after restart. The installed signature remains valid after launch. Saved locally only; no push.

# 2026-10-05 — Voice Memos padding and microphone permission
- Wrapped the standalone Voice Memos route in the existing page layout for consistent outer padding; embedded editors retain their existing spacing.
- The microphone purpose string already existed. Added the missing hardened-runtime `com.apple.security.device.audio-input` entitlement and wired it into Tauri signing. Release verification now checks both the bundled purpose string and signed entitlement, and rejected the previous bundle as expected.
- Renderer typecheck/build and 12 tests, entitlement plist validation, arm64 signed app verification and whitespace checks passed. Updated `/Applications/Homer Studio.app`, preserving the prior bundle at `/private/tmp/homer-studio-before-mic-fix-3olxu01k/Homer Studio.app`.
- Visually verified page padding, successful native capture with a live level meter, and discarded the brief test recording. System Settings → Privacy & Security → Microphone lists Homer Studio with access enabled. Local changes only; no publishing.

# 2026-10-05 — Approved Wave Studio implementation
- Added independent Wave Studio projects as the default workspace, collapsed navigation, selection editing, silence insertion/resizing, non-overwriting fragment moves/swap, undo/redo, pitch-preserving speed changes, voice annotation regions, and canonical voice metadata editing. Existing manuscript and voice production workflows remain accessible; memos and generated clips can open in Wave Studio.
- Added categorized/recent SFX, import/copy integration, independent sound-effect editing with gain/mute/fades, and the real bundled immutable `sitcom_laugh01.m4a` at a default −10 dB. Voice annotations do not invoke AI or download models; future selection processing contracts are documented.
- Native app-data projects use revisioned atomic saves and owned immutable PCM sources. Viewport waveform peaks and bounded preview chunks support long recordings without decoding the whole file in the renderer. Playback/export share native sample-aligned mixing and pitch-preserving tempo preparation; first preparation of a long speed-adjusted clip can take time.
- Validation: 25 renderer tests and 53 optimized native tests passed, including timeline operations, 5/30/60/120-minute viewport bounds, real FFmpeg speed/pitch/duration/fades, preview/export sample agreement, and SFX gain/mute/source preservation. Production typecheck/build, arm64 app/resource/signature verification, packaged startup smoke, and whitespace checks passed.
- GUI verification covered project import, voice assignment, speed change, bundled SFX insertion, mixed playback, navigation persistence, and a two-hour FLAC import with zoom/pan and playback near the one-hour position. The installed app restored the saved project/view/sidebar after restart and exported an 18.181813-second, 48 kHz stereo float WAV matching the edited duration. This is functional verification, not a full hardware performance benchmark or exhaustive listening assessment.
- Updated `/Applications/Homer Studio.app`; retained the preceding app at `/private/tmp/homer-studio-before-wave-szu_sh7c/Homer Studio.app`. Added `docs/WAVE_STUDIO.md` and updated compact context/contracts/ownership/decision documents. Changes are local only; no publishing or model downloads.

# 2026-10-05 — Approved Wave production and usability improvements
- Added direct Wave recording, one-action Voice Memos recording, local queued speech generation and preview/acceptance of Original Chatterbox voice conversions. Accepted conversions replace audible/exported narration and retain a restorable original timeline.
- Added adjacent-fragment Join, isolated peak normalization, gain controls, chosen import placement, narration insertion and cross-lane dragging. Bundled all ten supplied sound assets alongside the existing laugh; Sound Studio can insert them into narration.
- Persisted each project's view, selection, playhead, Loop and original voice baseline. Project switches and normal Quit drain pending saves. Fixed a save-revision timing race; history remains session-local. Added collapsed-sidebar top spacing and aligned Loop in the top transport row.
- Validation: 29 renderer tests, typecheck/build, 55 optimized native tests, real FFmpeg sample fitting, arm64 package/resource/signature checks and packaged startup smoke passed. GUI checks confirmed microphone recording in both workspaces, saved takes, recording insertion, Join, normalization, the eleven-sound library, dragging a laugh into narration, and restoration after Quit/restart.
- GUI speech validation caught an AAC/WAV container mismatch in the new workflow; corrected staging to M4A before lossless timeline import and rebuilt the installed app. Retest produced a 2.560-second take that previewed and inserted correctly. GUI M4A export completed and probing confirmed stereo AAC at 48 kHz with the exact 15.875-second mixed timeline duration. Reopened the preserved `homer-wave-check` project. Full voice-conversion listening/export quality remains a manual check with real speech and chosen references.
- Installed `/Applications/Homer Studio.app`; retained the preceding bundle at `/private/tmp/homer-studio-before-production-1st0khu6/Homer Studio.app`. Verification used a separate `Wave production verification` project; the existing narration project was preserved. Updated compact context and Wave workflow documentation. No model downloads or remote publishing.

# 2026-10-06 — Approved 0.2.5 Wave editing release
- Added per-passage Generate/Listen controls, distinct voice tints and generated/converted badges. Completion follows voice/audio provenance, survives splits and gain/fade edits, and skips completed passages unless regeneration is explicitly selected. Generated speech inserts with the chosen voice tag.
- Added styled speed, gain, fade and playback controls; exposed Join beside Split. Added searchable vertical projects, explicit save, portable media folders, recoverable deletion/restoration, and explicit WAV/M4A export with M4A selected initially.
- Added owned, muted video references with a separate timeline lane, frame preview and draggable start time. Video contributes to viewing/playback bounds and is excluded from audio export. Portable copies remap media and selected reference IDs.
- Replaced intermittently truncated FFmpeg chained crossfades with sample-aligned fades/mixing; repeated retake rendering now verifies duration across eight runs.
- Validation: 33 renderer tests, production typecheck/build, 56 optimized native tests, 18 Python tests, arm64 package/signature/resource checks, packaged startup smoke and whitespace checks passed. Installed-app checks confirmed portable video save/reopen, muted frame preview and the full-width timeline. The prior production verification confirmed exact-duration M4A/AAC export; the final export dialog now explicitly selects M4A or WAV. Full model conversion listening/quality and exhaustive hardware checks remain manual.
- Installed the verified final build in `/Applications/Homer Studio.app`; previous bundle retained at `/private/tmp/homer-studio-before-025/Homer Studio.app`. The user resumed interacting with the app during final UI checks, so automated UI interaction stopped. GitHub publishing is explicitly authorized; CI publishing now tolerates an existing release.

- Published verified DMG and ZIP packages at https://github.com/gubnota/homer_studio/releases/tag/v0.2.5; pushed main and annotated v0.2.5 (implementation commit 3d25b64). Subsequent user feedback: large video imports spend too long in the generic Working state; investigating a fast import path and cancellable progress as a follow-up.

# 2026-10-06 — Approved 0.2.6 reliability and Linux browser edition
- Added cancellable media preparation and generation, bounded decoder/subprocess/playback lifecycles, fast muted video remux, visible overlapping SFX rows, multiple selection, voice-aware Join, keyboard help, real zoom and neutral styled effect controls through +20 dB.
- Added checked streaming `.wavehs` bundles with media/custom voices, remapped identity and desktop drag/menu/CLI opening. Browser transport shares extracted Rust core services with Tauri; server provides owner authentication, scoped file streaming, recording uploads and persistent projects.
- Added Linux x86_64 packaging, GPU container configuration and operational documentation. Original worker unloads inactive model modes and retains bounded conversion chunks. No automatic model downloads.
- Local verification: 38 renderer tests, typecheck/production assets, 55 optimized core tests, 2 HTTP unit tests, 3 desktop tests and 18 Python tests passed. Isolated real-FFmpeg HTTP integration covered auth/path/ranges, MP3 peaks/preview, exact-duration WAV/M4A exports, bundle restore, eleven SFX and streamed recording import.
- Release is 0.2.6 as requested, with main/tag and macOS/Linux artifacts authorized for GitHub publication. Final arm64 DMG/ZIP, resource/signature verification and isolated packaged startup smoke passed; main was pushed as f001692. Linux CI/release status will be recorded after publication. The running installed app and user projects were preserved.
- Exact user MP3, real CUDA inference/memory and comprehensive live browser microphone/listening checks remain manual; automated success is not a quality/performance guarantee.

## 2026-10-06 · v0.2.7 release follow-up
- User requested increasing release numbers, Linux application bundle, macOS aarch64 DMG only, platform-specific requirements and an audio-editor screenshot.
- Updated owned version manifests/locks, removed ZIP creation/verification/publication, documented Linux bundle contents and dependencies.
- README screenshot captured from the actual browser editor using disposable spoken fixtures and Narrator/Maya/Leo assignments; installed app and user projects were untouched.
- v0.2.6 macOS release and CI succeeded; Linux CI was still running when this follow-up began. v0.2.7 renderer checks passed (38 tests), Apple Silicon DMG build/signature/resources passed and isolated packaged startup smoke passed. Publication and Linux CI follow.

## 2026-10-06 · v0.2.8 label follow-up
- Renamed the Wave fragment button to Mute; Unmute and behavior remain intact.
- v0.2.7 tag and DMG were already published, so incremented to v0.2.8 rather than modifying a published tag.
- Refreshed the real editor screenshot with the plain Mute label. Production web build, verified arm64 DMG/signature/resources and isolated packaged startup smoke passed. Main/tag publication follows; Linux bundle is built by the release workflow.

## 2026-10-06 · v0.2.9 Normalize label follow-up
- Shortened the peak-normalization button to Normalize as requested; the measurement progress and applied-gain feedback remain available.
- Incremented version after v0.2.8 tag publication; refreshing screenshot and verifying the new package.
- Included user-requested video sync layout fix: the placement field occupies a fixed-width right column; frame timestamps use tabular digits, with actions on a separate row.
- Added linked Russian README with matching platform requirements and workflows; demo screenshot will include an imported muted video.
- Browser verification confirmed Mute/Normalize labels, visible imported video and video lane, and a 176-pixel right-aligned timeline-start field. Updated shared README screenshot; 38 renderer tests and typecheck/production build passed.
- Final v0.2.9 arm64 DMG build, bundled resources and ad-hoc signature checks passed. README language links and local documentation/image links passed validation. Linux release validation remains in CI.

## 2026-10-06 - Approved v0.3.0 UI and recovery release
- Updated existing screens to the supplied Echoline prototype style: route SVG icons, neutral buttons, black switches, Voice Memos search/bulk actions, aligned edit/conversion fields and live recording levels. Added common close/Escape/focus behavior and expiring dismissible toasts. Preserved the user's app/sidebar/switch CSS adjustments.
- Fixed save/restore ordering and loaded recovered project content before export. Added validated trash cleanup and desktop Finder reveal; shared media remains intact. HTTP integration verified populated restored `.wavehs` export/import and cleanup retention.
- Added bounded LRU reuse of completed immutable waveform windows in both editors while retaining native disk caching/invalidation. Failures remain retryable; no full audio retained in visual cache.
- Version set to 0.3.0 on explicit user request. Refreshed actual Wave/video/voice and Voice Memos screenshots and described workflows in English/Russian READMEs. Split Linux test compilation from execution after prior release compilation timeout.
- Validation: 40 renderer tests, typecheck/web build, 55 optimized core tests, 2 server tests, 3 desktop tests, real-FFmpeg HTTP recovery/media integration and 18 Python checks passed. Browser confirmed project dialog Escape/focus restoration, real memo waveform, editing buttons, and muted video with voice tags. Final packaging, startup smoke and publication follow.
- Final arm64 DMG, resources/ad-hoc signature verification and packaged startup smoke passed. Release v0.3.0 publication is authorized; Linux artifacts are validated and produced by GitHub Actions. Real microphone hardware and CUDA inference remain manual checks.
- Published main commit 7eb9a0e and annotated v0.3.0; verified DMG available at https://github.com/gubnota/homer_studio/releases/tag/v0.3.0. GitHub macOS release and general CI checks passed. Linux shared-service tests remained in progress after compilation; Linux bundle was not yet published, and detailed job logs were unavailable (401/404 from the log host). Run: https://github.com/gubnota/homer_studio/actions/runs/37467872056.

## 2026-10-06 - Approved v0.3.1 project and voice recovery
- Added per-passage original snapshots and accepted voice versions; Alex/John/Alex restores the accepted Alex clips/settings, new conversions use original audio, and Restore original participates in Undo/Redo. Portable import remaps inactive voices and cached source/provenance identities. Legacy pre-conversion state can populate the new snapshots.
- New exports are losslessly compressed WAVEHS02 documents; WAVEHS01 remains readable. Finder events, file menu, CLI and global drops open through one serialized progress/Cancel workflow. Validated canonical bundled WAV is adopted without redundant decoding. Preserved user CSS/icons and existing waveform visual caches.
- Reproduced Linux descendant-output cleanup failure: GNU kill requires an explicit -- before a negative process-group ID. Fixed both runner and processor cancellation paths; standalone Linux regression fell from a failed 30-second wait to a passing 0.05-second suite. Replaced deprecated fetch_update with compatible compare_exchange. Added visible test output in release CI.
- Full Linux ARM container suite passed (58 core, 2 server tests). Renderer build/46 tests and HTTP integration passed, including compressed/legacy imports, exact audio bytes and cached voice remapping. The user's earlier 2.77 GB archive is no longer at its provided Downloads path, so that exact file remains unverified. GitHub x86_64 verification and final packaging/publication follow.
- GitHub x86_64 core/server and Python suites passed after the process-group fix. HTTP integration exposed Ubuntu 22.04 FFmpeg's missing output channel layout after aeval; explicitly retained stereo layout and output channels, and strengthened WAV/M4A integration assertions. Refreshed the actual Wave editor screenshot while retaining user styling.
