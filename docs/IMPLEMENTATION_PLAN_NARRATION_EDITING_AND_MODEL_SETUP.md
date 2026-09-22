# Implementation Plan: Reliable English narration and audio editing

## Goal

Make Homer Studio a usable local narration workspace: fix packaging and exports, provide truthful progress and errors with queue cleanup, narrate selected English chapter fragments, and let users listen, record, convert, preview, and replace a passage while preserving earlier audio. Improve Sound Studio's controls and supported gestures, remove AudioLDM 2, and show explicit model downloads.

## Existing behavior

- Tauri 2, React, Rust services, FFmpeg, and local Python workers are already present. Jobs are memory-only records with one message. Chapter narration reports 10% before the whole `speech::generate` call and 90% afterwards; the renderer has no job log or bulk queue controls.
- The save dialog is called by the renderer but `src-tauri/capabilities/default.json` permits open only, causing `plugin:dialog|save not allowed by ACL`. Sound Studio already has selection controls, but labels one `Clear selection` and uses a native category select.
- A custom icon source and `.icns` exist, yet the reported installed macOS icon is generic. The cause must be checked in the actual rebuilt bundle. The user has already fixed the heading font.
- Chatterbox Turbo is the English speech worker. The current worker loads on first generation; Settings checks availability but has no managed checkpoint download, byte progress, or disk-use view. AudioLDM 2 is the default effects worker, with Stable Audio Open as another option.
- Speech receives text split from raw chapter lines. It can speak Markdown punctuation and stores approximate line cues. There is no per-fragment generated state, precise word timing, waveform edit selection, conversion of a recorded delivery to the chosen voice, or non-destructive take history. Existing chapter audio must remain usable if a new render fails.
- Chatterbox Turbo's official demo lists nine vocal gestures: `[clear throat]`, `[sigh]`, `[shush]`, `[cough]`, `[groan]`, `[sniff]`, `[gasp]`, `[chuckle]`, and `[laugh]`. Descriptive strings such as `[soft exhale]`, `[heavy panting]`, and `[low groan]` are not in that supported list and must not be advertised as reliable Turbo controls. The original Chatterbox model is a separate English model with exaggeration and pacing controls; Chatterbox Voice Conversion is another distinct model.

## Proposed approach

Keep the current native/renderer/worker boundary and versioned loopback worker protocol. Extend job records with bounded timestamped events (phase, progress, warning/error text, elapsed time, output identity) and report chapter progress after each completed speech chunk plus preparation and finalization phases. Do not invent within-chunk percentages; show an active phase and elapsed time while the model is running. Cancellation must propagate to the worker and keep the last good chapter audio. Persist enough project/output metadata to offer export/delete independently of an in-memory job row. Queue cleanup means dismiss selected finished, failed, or cancelled jobs; it does **not** remove chapter audio. Active jobs cannot be dismissed until cancelled or finished. A separate, clearly labeled Delete generated audio action removes project-owned audio/takes and updates export eligibility after confirmation. Logs display actionable model/FFmpeg failures without manuscript text or local secrets.

Normalize Markdown for speech before chunking while retaining the original displayed source and a stable mapping to paragraph/sentence fragments. Preserve punctuation, inline Turbo gesture tags, and explicit pauses; skip Markdown syntax, code fences, links' URLs, and image markup. Store fragment IDs, narration status, selected take, and measured start/end cue data in a backward-compatible manifest change. Existing schema-v1 projects and line cues must open and migrate without discarding audio. Manual Generate on one or several selected fragments, plus Generate remaining, should assemble a chapter from approved takes in source order. Highlight only cues with measured fragment timing; word-level highlighting remains a later alignment feature. Imported audio without aligned cues remains playable without false highlights.

Keep both Turbo and original Chatterbox available for English: Turbo for its nine documented inline gesture tags; original Chatterbox for creative expression using its exaggeration and pacing controls, including attempts at softer or heavier delivery. Do not feed unsupported descriptive bracket tags to either model as if they were guaranteed commands. Surface each model's real capabilities in UI; do not promise Turbo-only gestures in original Chatterbox. Keep user voice samples in app-owned storage, independent of their import path. For vocal repairs, record a selected passage with the user's intonation, run the distinct Chatterbox VC model with the selected narrator's reference, then preview the converted take. Offer original/converted A/B, waveform range selection tied to one text fragment, and explicit Accept. Splice only on valid cue/range boundaries with short fades, remeasure duration, shift later cues, and atomically publish the result while retaining the previous take for revert. Voice conversion may change timing or words; require listening before acceptance, and make no claim of exact prosody or phoneme preservation.

Add an explicit Install/Retry/Cancel model action in Settings. A constrained helper downloads pinned, allow-listed model snapshots into app-managed model storage, reports completed/total bytes, verifies required files, and measures actual disk use. Never trigger a multi-GB download merely by opening Settings. Show installed versus missing models and keep support for an existing local checkpoint path. Do not mistake HTTP health for loaded-model readiness. Worker startup and download/setup failures appear as readable diagnostics. Continue to allow a separate configurable GPU-hosted worker later through the existing endpoint boundary, but implement and test local macOS use now.

Keep Stable Audio Open as the only effects engine and remove AudioLDM 2 execution, configuration, download/setup instructions, and dependency requirements. Do not automatically delete already generated clips or model files from the user's disk. In Sound Studio, combine speech and supported Turbo gesture tags in one prompt flow, provide an accessible searchable Kind of audio modal styled like the voice picker, rename the selection control Deselect all, and space job status and right-aligned Cancel. Explicitly set and verify macOS icon bundle resources in a fresh arm64 app package; resolve any stale-icon issue found in the actual bundle rather than replacing the existing artwork blindly. Leave the user's heading-font fix untouched.

## File changes

- `src-tauri/capabilities/default.json` — modify: grant only the save-dialog permission needed by export.
- `src-tauri/tauri.conf.json` — modify: explicitly reference the existing macOS icon resources and keep package metadata consistent.
- `scripts/verify-release.mjs` — modify: inspect packaged app icon declaration/resource in addition to existing architecture/signature checks.
- `src/renderer/styles/app.css` — modify: queue selection/log layout, fragment state, waveform editor, modal and Sound Studio spacing; preserve existing heading fonts.
- `src/renderer/src/pages.tsx` — modify: Render Queue selection, Select all/Deselect all/Clean, logs/errors, completed output actions, chapter fragment controls, listen/edit UI.
- `src/renderer/src/SoundStudioPage.tsx` — modify: combined speech/gesture prompting, kind modal, Deselect all, readable job layout, remove AudioLDM 2 choice.
- `src/renderer/src/VoicePicker.tsx` — modify only where needed to reuse modal accessibility/style patterns and expose compatible English model/voice choices.
- `src/renderer/src/native.ts` — modify: typed wrappers for new queue, model, fragment, conversion, and audio actions; keep save-dialog export path.
- `src/shared/contracts.ts` — modify: mirror persisted fragment/take/cue, job event, model status, and command response contracts.
- `src-tauri/src/services/jobs.rs` — modify: bounded event log, stage and chunk progress, errors, safe bulk dismissal of terminal jobs, cancellation state.
- `src-tauri/src/commands/production.rs` — modify: per-fragment generation/conversion/assembly, per-chunk job updates, output export/delete, validation and cancellation.
- `src-tauri/src/services/speech.rs` — modify: normalized speech chunks, English model routing, timing/progress callback, measured assembly and fades.
- `src-tauri/src/services/project_store.rs` — modify: backward-compatible fragment/take/cue schema migration, audio revisions, atomic publish/revert/delete operations.
- `src-tauri/src/services/settings.rs` — modify: original Chatterbox/VC model configuration, install state, disk size, worker diagnostics; retire AudioLDM 2 setting safely.
- `src-tauri/src/services/sound_workers.rs` — modify: truthful worker capabilities/readiness and request routing for original Chatterbox speech and voice conversion.
- `src-tauri/src/services/sound_render.rs` — modify: retain Stable Audio Open effects path, remove AudioLDM 2 branch, maintain playback of existing assets.
- `src-tauri/src/services/spoken_text.rs` — create: Markdown-to-spoken-text normalization and source-to-fragment mapping with focused tests.
- `src-tauri/src/services/model_install.rs` — create: allow-listed explicit model install, byte progress/cancel, required-file verification, and disk-use accounting.
- `src-tauri/src/services/audio_edit.rs` — create: bounded range extraction, recording conversion input, de-clicked splice, cue adjustment, temporary preview and verified publication.
- `src-tauri/src/lib.rs` — modify: register the new services and narrow Tauri commands.
- `src-tauri/Cargo.toml` — modify: add a Markdown parser only if needed for robust normalization; avoid HTML/Markdown regex stripping.
- `workers/chatterbox/server.py` — modify: surface actual Turbo readiness and full supported tag set; route the original English Chatterbox model with its distinct expression controls.
- `workers/chatterbox/requirements.txt` — modify only if required by pinned original Chatterbox/VC dependencies.
- `workers/sfx/server.py` — modify: Stable Audio Open only.
- `workers/sfx/requirements.txt` — modify: remove AudioLDM 2-specific dependencies while retaining those needed by Stable Audio Open.
- `workers/start_local.py` — modify: launch configured Turbo/original/VC services and remove AudioLDM 2 default.
- `workers/worker_protocol.py` — modify: additive capability/readiness and conversion messages, with compatibility tests.
- `workers/download_models.py` — create: pinned allow-list downloads with machine-readable byte progress for the native installer.
- `workers/README.md` and `README.md` — modify: English model/setup requirements, explicit model size/download flow, gestures, editor, and troubleshooting.
- `docs/PROJECT_CONTEXT.md`, `docs/ARCHITECTURE_INDEX.md`, `docs/MODULE_OWNERSHIP.md`, `docs/API_CONTRACTS.md`, `docs/DECISIONS.md`, `docs/TASK_LOG.md` — modify during implementation to reflect final interfaces, revised explicit-download rule, migration, and verification; keep concise.
- `src-tauri/src/services/sound_render.rs` and `workers/sfx/server.py` tests, existing renderer tests, and native module tests — modify/add focused cases for changed contracts rather than broad snapshot tests.

## Implementation steps

1. **Desktop and visible controls.** In capability/config/release script, fix save-dialog ACL, set/check macOS icon, build a fresh package and inspect `Info.plist`/resources. In `app.css`, `pages.tsx`, and `SoundStudioPage.tsx`, add spaced status/actions, Deselect all, a Kind modal, and queue Select all/Deselect all/Clean controls while preserving the existing heading-font fix. Verify actual save/export and installed app icon. Commit this verified stage with a human-readable name.
2. **Truthful jobs and cleanup.** In `jobs.rs`, `production.rs`, renderer contracts/UI, emit bounded structured progress and timestamped error events for each chunk and finalization; show elapsed active phase, logs, Cancel, and completed output actions. Bulk Clean removes only terminal job records; Delete generated audio is a separate project operation. Test cancellation, empty/mixed selection, no source deletion, failure log, and preservation of the previous chapter audio. Commit this stage.
3. **Speech text and fragment persistence.** Add `spoken_text.rs`, evolve `project_store.rs`/contracts with stable fragments and non-destructive takes, normalize Markdown and split by speakable sentence/paragraph within model limits. Retain schema-v1 reading and previously generated/imported audio. Test Markdown headings/emphasis/links/lists/code, long English chapters, migration, and invalid/stale cues. Commit this stage.
4. **English models and worker readiness.** Add explicit pinned model installer and disk-use UI, original Chatterbox/VC worker routes, English model selection, truthful loaded/missing status, and cancel/retryable downloads. Keep local endpoints configurable for a later GPU server. Remove AudioLDM 2 engine, setup, and dependencies while retaining its saved clips and Stable Audio Open generation. Test protocol, model-missing and interrupted-download states, allow-list paths, and both English model routes. Commit this stage.
5. **Manual fragment narration and gestures.** Connect selected-fragment generation, Generate remaining, visible narrated/needs-render state, measured cues, click-to-seek/highlight, and chapter assembly. Offer all nine documented Turbo gestures inline with speech and original-model exaggeration/pacing controls. Label soft exhale, heavy panting, and low groan as creative goals requiring preview or a recorded vocal take, not supported bracket tags. Test fragment retry, out-of-order generation, edits invalidating only affected audio, cancellation, and export eligibility. Commit this stage.
6. **Listen and edit.** Add text/waveform synchronized selection, microphone capture, chosen-voice VC preview, A/B listening, explicit Accept/Revert, bounded splice with fades, cue recalculation, and atomic publish. Verify a real recording/conversion and a replacement near beginning/end; ensure failure/cancel leaves original audio and export untouched. Commit this stage.
7. **Documentation and final release check.** Update the six compact context files, README/worker guides, test coverage, and arm64 package verification. Run the existing smoke/release checks on a fresh app, inspect the icon and download/export paths in the packaged GUI, then make the final local commit. Do not push; the earlier user instruction explicitly deferred pushing.

## Verification

- Run `npm run typecheck`, `npm test -- --run`, and `npm run build` (or the repository's equivalent scripts if names differ), focused Rust tests and `cargo test --manifest-path src-tauri/Cargo.toml`, worker protocol tests, and `git diff --check`.
- Run a fresh macOS arm64 Tauri build, `scripts/verify-release.mjs`, and the packaged-app smoke test. Inspect the built `.app` icon in `Info.plist` and Finder/Dock; use a fresh install path to distinguish package defect from icon cache.
- With real local models where installed, synthesize and listen to English fragments using Turbo and the original Chatterbox model; confirm no Markdown markup is spoken, Turbo gestures work inline, progress/logs advance, cancellation stops generation, and previous output survives a failed retry.
- Start from a clean model cache to observe explicit size/progress/cancel/retry and measured disk use; simulate network interruption without treating the model as ready. Test an existing local checkpoint as an alternative.
- Exercise single and bulk queue cleanup, Sound Studio Select all/Deselect all, modal keyboard navigation, Save dialog exports, generated-audio deletion, and real microphone-to-VC preview/accept/revert in the packaged app. Confirm legacy AudioLDM 2 clips remain playable/exportable.

## Risks / edge cases

- Chatterbox Turbo, original Chatterbox, and VC require different weights and substantial disk space; downloads must be explicit, resumable or safely restartable, and verified before readiness. Apple Silicon inference latency is model-dependent; no synthetic percentages within one request.
- VC may alter pronunciation, length, or perceived likeness. A converted take is a preview until accepted; cue timing is remeasured and later cues shifted. Exact word alignment is unavailable without a separate aligner.
- Manifest evolution must preserve schema-v1 projects and old line cues. Existing imported audio cannot acquire truthful text timing automatically.
- Deleting a queue row must never delete a completed chapter's audio. Deleting generated audio changes approval/export eligibility and must not remove exports already saved elsewhere.
- Stable Audio Open may require separately authorized model access. Removing AudioLDM 2 must not silently erase its existing local library clips.
- The generic icon may reflect an old installed bundle or OS cache; verify the actual new bundle before changing art.

## Out of scope

- Word-by-word karaoke timing or automatic forced alignment; this stage guarantees measured sentence/block or selected-fragment cues.
- A managed Linux GPU service, remote deployment, account system, or automatic model downloads at startup. The endpoint contract should permit a later companion worker.
- Multilingual narration or Chatterbox Multilingual installation/integration; this version is English-only.
- Heading typography changes; the user has already fixed those.
- A guarantee that voice conversion exactly reproduces the recorded performance or that descriptive bracket tags work in Turbo or original Chatterbox.
- Replacing Stable Audio Open with another effects model, redesigning the whole visual system, or pushing/tagging a release before a later explicit user request.
