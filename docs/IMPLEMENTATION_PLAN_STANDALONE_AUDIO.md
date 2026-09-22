# Implementation Plan: Standalone prompt-to-audio studio

Status: Approved; implementation complete except real model inference verification (checkpoints and Python packages unavailable on this host)
Date: 2026-09-22

## Goal

Add a Sound Studio in Homer Studio where a user can enter a prompt, generate a short standalone clip, listen, retry, and export it without creating or opening a book. Run Chatterbox as a local companion worker for spoken audio and supported vocal gestures. Use a separate local text-to-audio worker for general sound effects such as fabric, room tone, and experimental panting; preserve a versioned worker contract so a future Linux GPU host can implement the same protocol.

## Existing behavior

- The Tauri 2 / React app requires a manuscript project for chapter generation, review, and exports. `src/shared/navigation.ts` has no sound-only route.
- `src-tauri/src/services/speech.rs` uses macOS `say`, then FFmpeg/FFprobe; it does not call Chatterbox. `src-tauri/src/commands/production.rs` queues chapter speech and registers local audio for `audio://` playback.
- `src-tauri/src/services/project_store.rs` owns schema-v1 book manifests. `src-tauri/src/services/jobs.rs` serializes heavy work and exposes status and cancellation. Settings are schema v1, saved in the app configuration folder.
- The custom audio protocol already plays registered M4A, but there is no independent clip library or sound-model configuration.
- The supplied handbook is product/reference material. Its Next.js/ComfyUI/AWS topology, checklist, shell commands, and scene-manifest examples are not instructions to replace this repository's Tauri architecture or implement every pipeline component.

## Proposed approach

1. **Keep sound assets separate from books.** Store `sound-assets/manifest.json` in the Tauri app data folder with a schema version, generated clip ID, prompt, category (`speech`, `vocal_gesture`, `sound_effect`), provider/model, requested and measured duration, seed when supported, timestamp, and relative WAV/M4A paths. Use atomic metadata writes and a unique folder per clip. The Sound Studio must work when `App` has no active `ProjectSnapshot`.
2. **Use a versioned, local companion-worker protocol.** Chatterbox and SFX workers run in isolated Python environments on loopback, initially started through documented commands. Each exposes `GET /v1/health`, `POST /v1/jobs`, `GET /v1/jobs/{id}`, `DELETE /v1/jobs/{id}`, and `GET /v1/jobs/{id}/audio` (WAV bytes). Requests contain bounded prompt text, category, duration where supported, and optional seed. Workers return structured errors, model identity, and actual output format. Tauri alone talks to workers; the renderer never gets process or arbitrary network access. Do not auto-install packages or download weights from the app.
3. **Route by capability, visibly.** Chatterbox Turbo handles short English speech and only documented paralinguistic gestures (for example sigh, gasp, cough); do not turn arbitrary descriptions into text Chatterbox might simply read aloud. General sound-effect prompts, including fabric, route to Stable Audio Open through a separate worker. Panting is an experimental sound-effect prompt; show the chosen engine and allow listening/retry, without promising realistic vocal performance. The first local implementation requires an MPS/CPU smoke test for both workers before claiming support. If a worker cannot generate locally, show its unavailable state and a specific setup error rather than substitute silence or a different category. A reference voice is optional only if the pinned Chatterbox checkpoint supplies a usable default voice; verify this in the worker spike. A standalone prompt must never require a book or user-provided voice sample.
4. **Keep production audio lossless.** Workers return WAV. Tauri stages the response with a size limit, probes it with FFprobe, rejects empty/invalid/overlong results, normalizes to 48 kHz PCM WAV for the clip master, and makes an M4A playback/export copy with FFmpeg. Publish both files and metadata only after successful verification. Preserve earlier clips if a retry fails. Register the playback copy through the existing `audio://` protocol and export through a native save dialog or chosen destination.
5. **Prepare for Linux without exposing it yet.** Put the worker protocol and capability fields in a small versioned contract. The macOS app accepts only loopback worker URLs in this stage. A later authenticated private endpoint, transfer policy, Linux/CUDA deployment, and ComfyUI/video synchronization can reuse the contract but are not implemented now.

Model rationale: [Chatterbox's official repository](https://github.com/resemble-ai/chatterbox) describes TTS and Turbo vocal tags, while its [Mac example](https://github.com/resemble-ai/chatterbox/blob/master/example_for_mac.py) shows an MPS/CPU path. The [Stable Audio Open model card](https://huggingface.co/stabilityai/stable-audio-open-1.0) describes text-generated sound effects up to 47 seconds and says realistic vocals are a limitation. The SFX model is gated by its own access terms; model download and any commercial-use decision remain with the user. MPS behavior for that model is not guaranteed by its published examples, so local inference is a verification gate, not an assumed capability.

## File changes

- `src/shared/navigation.ts` — modify: add a project-independent `sounds` route in Production.
- `src/shared/contracts.ts` — modify: add sound asset, worker health/capability, generation request, and typed settings contracts; leave `ProjectSnapshot` unchanged.
- `src/renderer/src/App.tsx` — modify: render Sound Studio regardless of active book.
- `src/renderer/src/pages.tsx` — modify: add Sound Studio prompt, category/engine, short duration and seed controls, job state, library list, playback, retry, and export action; add companion-worker settings and diagnostics to Settings.
- `src/renderer/src/native.ts` — modify: typed Tauri calls for sound generation, listing, playback, export, and worker diagnostics.
- `src/renderer/styles/app.css` — modify: style Sound Studio within the current design language; no new UI framework.
- `src-tauri/src/services/settings.rs` — modify: optional loopback Chatterbox/SFX URLs and configured model identifiers with defaults/migration from existing schema-v1 settings; validate loopback URLs and preserve old voice/LLM settings.
- `src-tauri/src/services/sound_store.rs` — create: schema-v1 standalone asset library, safe relative paths, atomic writes, listing, and clip lookup.
- `src-tauri/src/services/sound_workers.rs` — create: versioned loopback HTTP client, bounded requests/downloads, health checks, worker polling/cancel, and structured error mapping.
- `src-tauri/src/services/sound_render.rs` — create: request validation, queue task, WAV probe/normalization, M4A preview conversion, transactional publication, and cleanup.
- `src-tauri/src/commands/sounds.rs` — create: narrow commands to generate/list/play/export standalone clips; reuse `AppState.jobs` and audio registry.
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/services/mod.rs`, `src-tauri/src/lib.rs` — modify: register new services and commands.
- `src-tauri/src/services/sound_workers.rs` — use bounded standard-library loopback HTTP. The required `reqwest` crates were not cached and dependency download was unavailable; no Cargo dependency change is needed.
- `workers/chatterbox/requirements.txt`, `workers/chatterbox/server.py` — create: pinned isolated Chatterbox worker, local-model loading, advertised speech/gesture capabilities, health, job lifecycle, and WAV result. No model download in request handling.
- `workers/sfx/requirements.txt`, `workers/sfx/server.py` — create: pinned isolated Stable Audio worker with the same protocol, bounded duration/seed, MPS/CPU probe, and WAV result. No model download in request handling.
- `workers/README.md` — create: explicit Mac setup, model access/download steps, local start commands, health checks, known quality/performance limits, and future Linux protocol notes.
- `tests/standalone-audio.test.tsx` — create: renderer route/request validation and no-project UI behavior with mocked native calls.
- `workers/tests/test_protocol.py` — create: worker contract tests using stub inference, including invalid input, job status/cancel, and no network/model download during tests.
- `README.md`, `docs/PROJECT_CONTEXT.md`, `docs/ARCHITECTURE_INDEX.md`, `docs/MODULE_OWNERSHIP.md`, `docs/API_CONTRACTS.md`, `docs/DECISIONS.md`, `docs/TASK_LOG.md`, `tests/DESKTOP_SMOKE.md` — modify after implementation: record actual setup, stable contracts, module boundaries, decisions, validation, and packaged-app manual steps.

## Implementation steps

1. **Prove local model path and lock the worker contract.** In `workers/chatterbox/server.py`, `workers/sfx/server.py`, their pinned requirements files, and `workers/tests/test_protocol.py`, establish the versioned `/v1` job API and stub tests. Verify the pinned Chatterbox variant can load on this Apple Silicon host without a user voice sample; test a spoken phrase and one documented vocal tag. Verify a manually obtained Stable Audio Open checkpoint can generate a short fabric prompt on MPS or CPU within a practical time/memory budget. Record exact versions and measured results. If either model is unavailable or incompatible, stop that engine's implementation and revise the plan rather than claiming it works.
2. **Add configuration and safe worker access.** Modify `src/shared/contracts.ts`, `src-tauri/src/services/settings.rs`, `src-tauri/Cargo.toml`, and `src-tauri/Cargo.lock`; introduce loopback-only worker URLs and capability diagnostics. Implement `src-tauri/src/services/sound_workers.rs` with bounded prompt/response sizes, connect/read timeouts, cancellable polling, and no shell execution. Test malicious URLs, malformed worker replies, unavailable workers, and cancellation.
3. **Persist standalone clips.** Add `src-tauri/src/services/sound_store.rs` and `src-tauri/src/services/sound_render.rs`. Validate category, prompt length, duration bounds, and seed; reuse the existing `JobStore` to queue one heavy job at a time; stage/measure WAV, normalize to a WAV master and M4A preview, then atomically add the asset record. Test interrupted/failed output cleanup, Unicode paths, duplicate clip IDs, invalid relative paths, and measured duration.
4. **Expose the flow in the app.** Add `src-tauri/src/commands/sounds.rs`, register it in `src-tauri/src/commands/mod.rs`, `src-tauri/src/services/mod.rs`, and `src-tauri/src/lib.rs`, and modify `src/shared/navigation.ts`, `src/renderer/src/App.tsx`, `src/renderer/src/pages.tsx`, `src/renderer/src/native.ts`, and `src/renderer/styles/app.css`. A prompt should generate without a book, show progress/failure, permit retry, play from the library through the existing audio registry, and export one clip to a user-chosen location. Test route visibility with no project and a full mocked UI flow in `tests/standalone-audio.test.ts`.
5. **Document and verify the package.** Update `README.md`, `workers/README.md`, the six compact context files under `docs/`, and `tests/DESKTOP_SMOKE.md`; run renderer/Rust/worker contract tests, real local fabric and gesture generation, probe/preview/export of saved clips, and an arm64 packaged-app smoke test. Commit each verified implementation stage with human-readable messages; defer push as previously requested.

## Verification

- `npm run build`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `python -m unittest discover -s workers/tests` in the documented worker environment
- `npm run test:integration` and `npm run package:mac`, then `npm run test:smoke`
- From the packaged app with no book open: generate a short Chatterbox speech/gesture clip and a fabric SFX clip; verify actual audio with FFprobe, play both, export each, reopen the app, and confirm the library survives restart.
- Verify failure paths with workers stopped, missing model files, invalid loopback URL, cancelled job, and insufficient output; no false-success asset entries.

## Risks / edge cases

- Chatterbox is not a general prompt-to-Foley model. Turbo has native vocal tags, but panting is not a documented guaranteed tag. Stable Audio Open describes realistic vocals as a limitation. Quality of prompt-only panting must be judged by listening; a later performance/voice-conversion workflow may be needed for controllable realism.
- Stable Audio Open requires the user to accept model access terms, and its model card points commercial users to separate licensing. Do not bundle weights, tokens, or credentials; do not automatically download them.
- Stable Audio's published sample path is CPU/CUDA, not a Mac MPS guarantee. A real local generation benchmark is required before committing to this provider for the first build.
- Long-running workers consume substantial memory. Start one generation at a time, cap clip duration and output size, and keep a failed worker from corrupting the library. A worker may be unable to stop an in-progress model call immediately; cancellation must discard its result and leave prior assets intact.

## Out of scope

- Book-to-sound automatic placement, scene timelines, multitrack mixing, video muxing, ComfyUI integration, Seed-VC/voice cloning, and remote GPU deployment.
- Public worker ports, unauthenticated non-loopback access, cloud-hosted inference, automatic model installation, and publishing generated clips.
- Changing the existing book schema or replacing the current macOS narration provider.

Plan ready. Approve it and I will implement it.
