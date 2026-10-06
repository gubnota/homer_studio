# Implementation Plan: Wave reliability and Linux browser edition

Status: Approved by user; implementation complete; local verification passed, GitHub release in progress. Date: 2026-10-06.

## Goal

Resolve the reported editing, playback, import, cancellation and resource problems; add portable `.wavehs` projects; then provide a Linux server build of Homer Studio accessible through a browser, with generation running on the server GPU. Preserve existing desktop projects, immutable source audio, voice provenance and WAV/M4A export.

## Existing behavior

- Desktop is Tauri/Rust with a React editor and Python model workers. Browser development mode does not provide functioning native services; a Linux browser edition requires a backend.
- SFX preview loads a source but requires a second Play action. Coincident SFX share a visual row and hide each other while both remain audible in the mix.
- View state stores one selected fragment. Shift-selection, double-click selection and playback-preserving keyboard navigation are absent.
- Gain validation and peak normalization cap boosts at +6 dB. Peak normalization can reduce volume to meet the peak target; it does not measure perceived loudness.
- Fade/speed/gain controls mix sliders with numeric steppers and presets. The bottom slider pans rather than zooms.
- Conversion already uses approximately 15-second chunks, and TTS uses text chunks. Worker cancellation does not interrupt active model inference; Original TTS and conversion models can remain resident together.
- Waveform requests have limited concurrency within each canvas, but obsolete requests continue and separate canvases can duplicate work. Playback uses bounded chunks but needs stronger cancellation/loading/error handling.
- Video import re-encodes the entire file, including already compatible video, with generic Working feedback.
- Portable `.homer-wave` folders exist. Single-file bundles, full external voice-reference packaging and OS file-open integration do not.
- The named MP3 and referenced screenshots were not available for inspection. Current app handle count was normal; the reported exhausted-file-handle failure is not yet reproduced.

## Proposed approach

### Desktop reliability and resource bounds

1. Add persistent job progress and Cancel controls across generation surfaces and import preparation. Cancellation interrupts the relevant managed inference process, waits for cleanup and leaves accepted project audio intact. Restart workers when needed; do not terminate unrelated services.
2. Make SFX preview a single action with visible playback/errors. Use a browser-compatible preview format if testing identifies float-WAV or RF64 decoding limitations.
3. Pack overlapping SFX into visible rows with row-aware hit testing, selection and dragging. Preserve intentional overlap and existing mix timing; deleting one instance removes that instance only.
4. Deduplicate waveform preparation by source, bound native work across canvases and discard/cancel stale work. Limit subprocesses and open inputs, close handles on every exit path and make playback loading cancellable with actionable failures.
5. Reproduce long MP3 import with synthetic fixtures and the user's file when available. Verify decoded media before publishing it into a project, support long/RF64 sources, and ensure removal cancels dependent work. Avoid promising playback when decoding has failed.
6. Fast-path compatible video through muted remux; use supported hardware encoding with a software fallback when required. Show measured preparation progress and cancellation; expose the reference only after verification.
7. Keep sequential bounded conversion/TTS chunks and disk staging. Unload inactive model modes, clear device caches after stopping work and recycle managed workers. Measure RAM/VRAM behavior rather than claiming a fixed model-independent memory ceiling.

### Editing and controls

- Add optional `selectedIds` to saved view state; migrate old `selectedId` automatically. Shift-click extends selection, Cmd-click toggles, and double-click selects a whole fragment. Selection highlighting and drag behavior use the same selected IDs.
- Join requires contiguous compatible fragments. If voice tags differ, ask for one resulting voice or Clear tags before committing. Produce one contiguous voice region and invalidate conversion completion when the chosen voice changes. Joining audio alone does not change its acoustic voice.
- Keyboard bindings: Left/Right choose previous/next fragment and seek its start; Ctrl/Cmd+Left/Right seek selected fragment start/end; Shift+Left/Right seek five seconds; Space toggles playback; Shift+Space plays the selected fragment/range. Preserve active playback on seeks. Ignore editing shortcuts inside text fields and show a shortcut-help popover.
- Provide a real zoom slider anchored at the playhead, retaining scroll/trackpad panning and Fit.
- Gain presets become Mute, −20, −10, 0, +10 and +20 dB. Extend native/schema limits to +20 dB, retain legacy negative gains and use a centered neutral gain slider with snapping.
- Replace gain/fade/speed steppers and speed preset buttons with styled sliders and readouts. Fade zero is its endpoint and means no fade. Speed's neutral center is 1×, displayed as 0% change; it must never become 0× playback.
- Remove the +6 dB normalization cap; show the measured peak, applied change and any +20 dB limit. Clearly explain attenuation when meeting the target requires reducing gain. Preserve source files and allow undo.
- Use consistent collapsed-sidebar icons, sizing, active states and top spacing; desktop window-control clearance and browser spacing are platform-specific.

### Portable projects

- Export `.wavehs` as a streamed single-file archive containing a versioned manifest, referenced audio/video, original conversion baselines and required voice metadata/reference recordings. Deduplicate assets and include integrity checks.
- Import validates schema, paths, entries, size bounds and checksums, stages atomically and remaps IDs without overwriting existing projects or voices. Reject traversal, symlinks and incomplete bundles.
- Support drag/drop, File → Open, startup command-line paths and macOS Open With/file association. Continue opening existing `.homer-wave` folders. Save stays a normal project save; Export Project creates the portable bundle.

### Linux browser edition

- Build a headless Linux server serving the same React UI and typed API. Extract reusable Rust services behind a runtime context for paths/resources/job lifecycle; retain microphone/device capture and native dialogs in the desktop adapter.
- Add an explicit transport/capability adapter: desktop uses Tauri, browser uses authenticated HTTP, uploads, downloads and streamed progress. Remove blanket desktop-only guards from workflows supported by the server.
- Support Wave Studio, project management, voices, Voice Memos, Sound Studio, manuscript narration and exports through the shared backend. Browser recording uses its microphone and uploads recordings to the server; it does not record the server microphone.
- The server manages loopback Python workers using CUDA when available. Ship Linux build/run instructions and a GPU container configuration; model installation remains an explicit action with progress.
- Provide configurable host/port and persistent storage, single-user authentication, request-origin checks, bounded uploads and authenticated ranged media/download routes. Browser microphone access requires HTTPS or localhost; document an HTTPS reverse-proxy setup.
- This provides a server build and browser application. Actual remote deployment and CUDA performance validation depend on access to a Linux GPU host.

## File changes

Desktop and shared editor files to modify:

- `src/shared/waveStudio.ts`: selected IDs, backward-compatible validation and updated gain limits.
- `src/shared/waveStudioEdits.ts`: multi-selection, contiguous join and voice/provenance consolidation.
- `src/renderer/src/WaveStudioPage.tsx`: keyboard navigation/help, selection and join confirmation.
- `src/renderer/src/WaveStudioProvider.tsx`: selection migration, job/open-file coordination and stable persistence.
- `src/renderer/src/useWaveStudioPlayback.ts`: cancellable loading, bounded requests, seek while playing and cleanup.
- `src/renderer/src/components/WaveStudioTimeline.tsx`: overlapping SFX rows, hit testing, double/Shift/Cmd clicks and zoom control.
- `src/renderer/src/components/WaveformCanvas.tsx`: shared request deduplication, cancellation and accurate errors.
- `src/renderer/src/components/WaveStudioInspector.tsx`: neutral/snapping sliders, gain presets and normalization feedback.
- `src/renderer/src/components/WaveStudioLibraryDrawer.tsx`: one-action preview and instance selection.
- `src/renderer/src/components/AudioTransport.tsx`: reliable preview start/error handling and styled speed control.
- `src/renderer/src/components/WaveProductionPanel.tsx`: persistent progress/cancel state.
- `src/renderer/src/components/WaveProjectsPanel.tsx`: bundle export/import.
- `src/renderer/src/components/StudioLayout.tsx`: collapsed sidebar and platform spacing.
- `src/renderer/src/pages.tsx`: generation cancellation and browser-capability branches for existing workspaces.
- Create `src/renderer/src/components/NeutralSlider.tsx`: shared styled snapping slider with accessible keyboard controls.
- Create `src/renderer/src/components/WaveShortcutHelp.tsx`: visible shortcut reference.

Native and worker files to modify:

- `src-tauri/src/services/wave_studio.rs`: compatible selection schema and gain validation.
- `src-tauri/src/services/wave_store.rs`: verified/cancellable import and bundle orchestration.
- `src-tauri/src/services/wave_render.rs`: bounded preview preparation/input handling and normalization reporting.
- `src-tauri/src/services/wave_video.rs`: muted remux, encode fallback and progress/cancellation.
- `src-tauri/src/services/waveform.rs`: deduplicated bounded peak work and cancellation.
- `src-tauri/src/services/audio_protocol.rs`: tested bounded ranged preview delivery.
- `src-tauri/src/services/jobs.rs`: job status/progress and cancellation lifecycle.
- `src-tauri/src/services/process_runner.rs`: bounded progress reporting and cleanup on all exits.
- `src-tauri/src/services/wave_processing.rs`, `speech.rs`, `sound_workers.rs`, `worker_runtime.rs`: supervised inference cancellation and memory lifecycle.
- `src-tauri/src/commands/wave_studio.rs`: typed import/cancel/bundle/normalization commands.
- Create `src-tauri/src/services/wave_bundle.rs`: streamed archive validation and atomic import/export.
- `src-tauri/src/lib.rs`, `main.rs`, `Cargo.toml`, `tauri.conf.json`: file menu, open events/arguments, file association and archive/core dependencies.
- `workers/chatterbox/original.py`, `workers/chatterbox/server.py`, `workers/worker_protocol.py`: model-mode cleanup and interruptible supervised job execution.
- `workers/start_local.py`, `workers/chatterbox/requirements.txt`: explicit platform/device setup for Linux CUDA.

Linux/server files to create or modify:

- Create root `Cargo.toml` and `crates/homer-core/Cargo.toml`, `crates/homer-core/src/lib.rs`, `crates/homer-core/src/runtime_context.rs`: shared workspace/service boundary and platform-independent runtime.
- Move reusable modules from `src-tauri/src/services/` to `crates/homer-core/src/services/`, adapting runtime arguments; retain desktop capture and Tauri media responses in the shell. Update existing native command imports to the shared core. Preserve module ownership and existing service tests during the move.
- Create `server/Cargo.toml`, `server/src/main.rs`, `api.rs`, `auth.rs`, `media.rs`, `config.rs`: headless runtime, allowlisted commands, authentication, uploads/progress and bounded media serving.
- Create `src/renderer/src/platform.ts`, `serverTransport.ts`, `useBrowserRecording.ts`: typed transport, capabilities and browser recording.
- Modify `src/renderer/src/native.ts`, `audioNative.ts`, `waveStudioNative.ts`, `useNativeRecording.ts`: route operations through platform adapters and browser recording where appropriate.
- Create `server/Dockerfile`, `server/compose.gpu.yml`, `docs/LINUX_SERVER.md`: reproducible Linux/GPU build, persistent storage and startup/HTTPS instructions.
- Modify `package.json` and the canonical lockfile for server build/check scripts and required dependencies. Preserve unrelated untracked files.

Verification and documentation:

- Extend `tests/wave-studio.test.ts`, `tests/audio-studio.test.tsx`, `tests/standalone-audio.test.tsx`, `tests/navigation.test.ts` and native service tests for changed behaviors.
- Extend `workers/tests/test_protocol.py`; create `server/tests/api_contracts.rs` and `tests/server-transport.test.ts` for authentication, capabilities, job cancellation and uploads.
- Update `tests/DESKTOP_SMOKE.md`, `docs/WAVE_STUDIO.md` and the six compact context documents with verified behavior, contracts, platform boundaries and an ADR accepting the optional Linux server edition.

## Implementation steps

1. Fix import/playback/resource lifecycle in the native services, playback hook and waveform component. Test long synthetic MP3, cancellation/removal, repeated playback and handle-count stability before changing editor interactions.
2. Add supervised cancellation and worker memory-mode cleanup. Test stopping TTS and conversion mid-job, restart/retry and accepted-audio preservation.
3. Implement fast video preparation with verified muted output. Compare compatible remux and fallback transcode; cancel each path and check temporary-file cleanup.
4. Implement shared selection state, visible SFX rows, keyboard/help/zoom and voice-aware Join. Extend pure editor tests and targeted interaction tests.
5. Replace effect controls, update gain contracts and normalization feedback; fix sidebar spacing. Verify neutral snapping, undo, actual output gain/fades/speed and old project loading.
6. Implement bundles and desktop file integration. Round-trip complete projects and reject malformed archives; verify drag/Open/CLI behavior and WAV/M4A output after reopening.
7. Extract shared Rust runtime/services without changing desktop behavior. Run the existing optimized native tests and package smoke before adding HTTP transport.
8. Build the authenticated Linux backend, renderer transport and browser recording/file flows. Test every existing server-supported workspace and retain native desktop adapters.
9. Package the Linux server/GPU configuration and document operation. Run Linux build/API/browser checks in a Linux environment; report unavailable real-CUDA checks explicitly.
10. Update compact documentation and report tested results, limitations and local artifacts. Use small coherent commits. Publishing v0.2.5 is already complete; do not replace that tag.

## Verification

- `npm run build` for type checking, renderer tests and production assets.
- `cargo test --release --manifest-path src-tauri/Cargo.toml` for existing and new native regression tests.
- `python3 -m unittest discover -s workers/tests` for worker protocol/cancellation tests.
- `npm run pack:mac` and `npm run test:smoke` for packaging and packaged startup.
- Add and run workspace Linux server build/API tests and browser transport tests once the workspace exists.
- Real FFmpeg fixtures: long MP3/RF64, coincident SFX, gain/fades, normalization attenuation/boost, tempo, video remux/fallback, bundle round-trip and exact-duration WAV/M4A exports.
- Stress repeated import/cancel/remove/play and source switching; compare open handles, child processes, memory and cleanup after completion/cancellation. Long conversion and TTS tests must distinguish model baseline memory from growth across chunks/jobs.
- Manual shortcut, join-conflict, overlapping-fragment, preview and collapsed-sidebar checks in a separate verification project. Do not interfere with the user's active app session.
- Browser tests: upload/download, microphone recording over a secure origin, project restore, job cancel and authenticated media ranges. CUDA throughput/VRAM checks require a GPU host.

## Risks / edge cases

- The exact MP3 failure requires reproduction; synthetic stress coverage cannot prove that particular file is fixed.
- Legacy negative gains, voice references and conversion baselines must survive schema/bundle migration.
- Interrupted inference may require restarting and reloading a model; cancellation must not block the interface or corrupt accepted results.
- Large bundle/video import must stream and have disk-space/progress checks.
- Shared-service extraction spans the backend and requires desktop regressions before server work proceeds.
- Browser media compatibility and microphone access differ from desktop; use supported preview/recording formats and capability checks.
- Referenced screenshots were absent; implement the identified spacing/control inconsistencies and compare with screenshots if supplied.

## Out of scope

Multi-user collaboration, billing, automatic model downloads, provisioning a cloud GPU account, and publishing a new release without a requested version. Actual server deployment requires a supplied host. Existing 0.2.5 release assets/tag remain intact.

## Implemented verification and remaining manual checks
- HTTP contract coverage is colocated in `server/src/main.rs` and `scripts/test-server.py` instead of the proposed standalone Rust test path. Renderer selection/row/transport tests cover the new pure behaviors.
- Local optimized checks: 55 shared-core tests, 2 HTTP-unit tests and 3 desktop tests passed; 38 renderer tests and 18 Python worker/processor tests passed.
- Isolated real-media server integration passed authentication/origin/path checks, MP3 import, peaks/preview/ranges, WAV/M4A export, project bundle restoration, all eleven SFX and streamed recording import.
- Exact reported MP3, screenshots, comprehensive browser listening/microphone interaction and CUDA memory/performance measurements remain unavailable. Synthetic and boundary tests do not establish those hardware-specific outcomes.
- User subsequently authorized increasing the release to v0.2.6 and publishing artifacts. v0.2.5 remains intact.
