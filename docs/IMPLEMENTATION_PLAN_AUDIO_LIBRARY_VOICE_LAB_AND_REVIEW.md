# Implementation Plan: Audio library controls, voice lab, and Review player

## Goal

Make generated audio manageable from Review, Exports, and Sound Studio; add a project-independent tab to record a performance and convert it to a chosen narrator voice; make voice previews immediately useful and visibly recoverable; and replace Review's small static waveform and browser controls with a precise, scrollable player.

## Existing behavior

- `ReviewPage` already exports and deletes one generated chapter audio file, records up to 20 seconds for a manuscript section, converts it with the Original Chatterbox worker, shows spoken cues, and renders a 120-bin waveform. It has no chapter selection or bulk actions. The player uses `<audio controls>`.
- `ExportsPage` creates and lists project-owned M4A/timestamp pairs but only offers playback and timestamp copying. There is no Save As or removal command for an export record.
- Sound Studio's clip library persists WAV/M4A pairs, supports single/bulk export, Select all, and Deselect all, but has no delete command.
- The Voices page copies samples into the app. On sample add/select, a detached thread silently tries to synthesize `preview.m4a`. Failure is discarded, so Preview can report “still being prepared” forever. The built-in voice generates a preview on demand. No standalone voice-conversion tab exists.
- Chapter waveform selection is mapped to full chapter time, capped to a single spoken cue and 20 seconds. The backend decodes the whole file at low rate, then aggregates into 120 peaks. Existing project manifest schema is version 1.

## Proposed approach

Keep three distinct deletion scopes: Review removes generated chapter audio only; Exports removes selected project-owned M4A/timestamp pairs and history records; Sound Studio removes selected app-owned clip pairs and metadata. Give each surface a row selection control, Select all, Deselect all, Delete selected, and Delete all. Keep per-item Export/Save As and Delete. Confirm destructive actions with item counts, skip imported chapter audio, validate IDs and owned paths in Rust, update manifest atomically, and report partial filesystem failures without claiming success. Bulk export/Save As copies to a chosen folder with collision-safe names. For Review, bulk export operates on available chapter audio; bulk deletion operates on generated audio only. “Delete all” means all deletable items in the current surface, not unrelated source recordings, voice samples, or files previously saved elsewhere.

Add a **Voice Lab** production tab that works without opening a book. Capture a short microphone recording or import one, choose a narrator through the existing voice modal, preview the original, submit a cancellable Original Chatterbox voice-conversion job, compare the result, and save it to the app's clip library (and export it). Reuse the worker request and FFmpeg conversion flow from section replacement through a shared service, while keeping chapter splice logic in the Review command. Extend clip metadata with an optional conversion category/source label in a backward-compatible way; existing clip manifests remain readable. Explicitly show that conversion requires a voice sample, the Original worker, and FFmpeg.

Make previews deterministic: play the app-owned selected sample immediately for a custom voice, while a fixed spoken preview is generated in a tracked job. Expose ready/running/failed status and the actual error; retry generation from Preview, including after a restart or stale marker. For the built-in model voice, bundle an actual short pre-rendered preview in app resources if a locally generated/licensed asset can be produced during implementation; otherwise display a clear “generate preview” action with job progress instead of claiming it is pre-recorded. Do not silently use the user's own sample as though it were converted narrator output.

For Review, retain native `<audio>` as the playback engine but use custom Play/Pause, Stop, seek, time display, speed-independent controls, and a playhead. Request bounded waveform peaks for the current visible time range from Rust, with zoom controls and horizontal scroll/pan; preserve full-chapter overview and cue highlighting. Pointer drag selects a passage within one spoken cue; clicking a cue seeks there. Keyboard controls and focus states must support the same actions. Debounce/abort superseded viewport requests and avoid decoding an entire long chapter on every pan.

## File changes

- `src/shared/navigation.ts` — modify: add the Voice Lab route.
- `src/renderer/src/App.tsx` — modify: render Voice Lab in the existing layout.
- `src/renderer/src/VoiceLabPage.tsx` — create: standalone capture/import, narrator selection, job state, A/B preview, Save/Export flow.
- `src/renderer/src/pages.tsx` — modify: Review bulk toolbar and item selection; Exports Save As, Delete, bulk toolbar; truthful voice preview states; replace Review's current `ChapterAudio` internals with the shared waveform player.
- `src/renderer/src/components/WaveformPlayer.tsx` — create: accessible playback controls, playhead, range selection, zoom, horizontal scroll/pan, cue synchronization.
- `src/renderer/src/SoundStudioPage.tsx` — modify: per-clip Delete and bulk Delete selected/Delete all; keep selection and playing state in sync after deletion; show saved voice-conversion clips.
- `src/renderer/src/native.ts` — modify: typed commands and Save As/folder dialogs for chapter/export bulk copies, export deletion, clip deletion, preview status/retry, conversion job, and waveform windows.
- `src/shared/contracts.ts` — modify: additive preview status, conversion clip metadata, and waveform-window response types; no project schema bump unless implementation proves necessary.
- `src/renderer/styles/app.css` — modify: selection toolbars, preview/job status, Voice Lab, and scrollable waveform/player styling; preserve the user's current uncommitted heading-font changes.
- `src-tauri/src/commands/production.rs` — modify: validate and expose bulk chapter/export actions, preview status/retry, conversion job, and bounded waveform windows; delegate shared conversion mechanics.
- `src-tauri/src/commands/sounds.rs` — modify: single/bulk clip deletion and safe publication of converted voice clips.
- `src-tauri/src/services/project_store.rs` — modify: project-owned export deletion and bulk generated-audio mutation with revision checks and manifest consistency.
- `src-tauri/src/services/sound_store.rs` — modify: safe clip deletion, metadata update, and converted-clip publication while reading existing manifests.
- `src-tauri/src/services/voice_store.rs` — modify: preview paths/status for custom and built-in voices and selected-sample fallback; recover stale/incomplete previews.
- `src-tauri/src/services/voice_conversion.rs` — create: shared validation, FFmpeg preparation/encoding, Original worker invocation, cancellation and staging for standalone and section workflows.
- `src-tauri/src/services/speech.rs` — modify: seeked, duration-limited waveform extraction with bounded peak count and validation.
- `src-tauri/src/services/mod.rs` and `src-tauri/src/lib.rs` — modify: register shared conversion service and new Tauri commands.
- `tests/navigation.test.ts` and `tests/standalone-audio.test.tsx` — modify: cover the added route, bulk selection, clip deletion state, and Voice Lab's no-project behavior.
- `docs/PROJECT_CONTEXT.md`, `docs/ARCHITECTURE_INDEX.md`, `docs/MODULE_OWNERSHIP.md`, `docs/API_CONTRACTS.md`, `docs/DECISIONS.md`, and `docs/TASK_LOG.md` — modify after implementation to record final ownership, command contracts, deletion rules, and task status.
- `tests/DESKTOP_SMOKE.md` — modify: packaged-app checks for Preview, conversion, waveform navigation, and Save As/Delete controls.

## Implementation steps

1. **Persistence and safe deletion.** In `project_store.rs`, `sound_store.rs`, `production.rs`, and `sounds.rs`, add ID-validated single/bulk delete operations, revision checks for project records, collision-safe Save As copies, and manifest/file consistency handling. Rust tests cover selected/all, missing IDs, imported audio protection, repeated deletion, and preserved external exports. Commit as “Add safe deletion and export operations for audio libraries”.
2. **Review, Exports, and Clip Library controls.** In `pages.tsx`, `SoundStudioPage.tsx`, `native.ts`, `contracts.ts`, and `app.css`, wire item/bulk selection, single/bulk Save As, Delete selected/Delete all with count confirmation, and error feedback. Ensure selection reconciles after refresh and a deleted playing clip stops. UI tests cover selection and disabled/empty states. Commit as “Add bulk controls to Review, Exports, and Clip Library”.
3. **Standalone conversion.** Extract shared Original Chatterbox conversion into `voice_conversion.rs`; add a job command and persist converted audio through `sound_store.rs`. Add `VoiceLabPage.tsx` and route, with mic recording/import, source playback, voice choice, worker readiness, progress/cancel, converted playback, and export. Test invalid/oversized recordings, missing sample/worker, cancellation, and that a failed conversion does not create a library entry. Commit as “Add standalone narrator voice conversion”.
4. **Reliable voice previews.** In `voice_store.rs`, `production.rs`, `pages.tsx`, and contracts, replace detached silent preview generation with tracked status and retry. Provide immediate selected-sample listening while custom synthesized preview is pending; package a built-in rendered preview only after checking provenance and actual audio quality. Test crash/stale marker recovery, worker failure, sample change invalidation, and replay after restart. Commit as “Make narrator previews ready and recoverable”.
5. **Review waveform/player.** Add the bounded waveform window interface in `speech.rs`/`production.rs` and `WaveformPlayer.tsx`; connect it in `pages.tsx` and style it in `app.css`. Test time-coordinate mapping under zoom/scroll, cue seek/highlight, play/pause/stop, passage cap, empty/missing waveform, and long-audio bounded processing. Commit as “Add a zoomable Review waveform player”.
6. **Documentation and local release verification.** Update six context files and smoke guide; run automated checks and build the macOS arm64 `.app` locally. Manually exercise Preview, Voice Lab conversion if the worker/model is available, Save As/Delete in all three surfaces, and waveform navigation. Commit documentation/verification with a human-readable title. Do not push; pushing was deferred by the user.

## Verification

- `npm run typecheck`, `npm test`, `npm run build`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `git diff --check`.
- `npm run pack:mac` and `npm run test:smoke` on the local arm64 app; inspect the built app rather than only a browser preview.
- In the packaged app, check single and bulk Save As/Delete for Review, Exports, and clips; cancel dialogs; verify files saved outside app/project storage survive deletion; test a failed operation and its visible error.
- Preview a built-in and custom voice before and after restart. Confirm a missing worker reports a concrete failure and offers Retry; confirm a ready sample plays immediately.
- Record/import a short passage with no project open, convert to a selected sampled narrator if Original Chatterbox is available, then listen and export; test unavailable-worker behavior otherwise.
- Play, stop, click a line, zoom, pan, select a passage, and record a replacement in Review, including with a long chapter.

## Risks / edge cases

- Export records include both audio and timestamps. Deletion must never remove arbitrary paths from a tampered manifest or a user's separate Save As copy. On filesystem failure, show which items remain and reload canonical state.
- Voice conversion depends on the Original Chatterbox voice-conversion worker and a selected reference sample. The tab should remain usable for recording/import and explain missing prerequisites without producing a misleading conversion.
- A custom sample is a listening fallback, not proof of the synthesized narrator's sound. A built-in pre-rendered asset must be generated with the actual model and distributable provenance; if unavailable, the UI must say generation is needed.
- Long recordings require range-bounded decoding and careful timestamp mapping; waveform zoom must not change audio time or the existing 20-second/single-cue replacement limit.
- Existing uncommitted `src/renderer/styles/app.css` heading-font changes belong to the user and must be preserved.

## Out of scope

- Additional languages/models, text-to-speech model quality changes, model downloads, cloud conversion, DAW-style multitrack editing, automatic word-level alignment, and Git push/tag/publication.
