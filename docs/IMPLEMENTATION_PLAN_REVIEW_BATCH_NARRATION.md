# Implementation Plan: Batch narration from Review

## Goal

Let an author start narration for multiple chapters from Review in one action, leave the tab while it runs, and see overall progress, the current chapter, errors, and cancellation in Render queue. Completed chapters remain available for listening and manual correction.

## Existing behavior

- Review's **Generate audio** button enqueues one chapter job and polls until it finishes. **Generate remaining** does the same sequentially for sections within one chapter, so the author must start each chapter separately.
- The Review checkboxes only allow chapters that already have audio, because they currently serve bulk save/delete. The job store serializes work and exposes queue progress, pause, resume, cancel, and an activity log.
- `generate_chapter_audio` captures text and speech settings when queued, renders through `speech::generate_with_progress`, then commits against an expected project revision. Each commit increments that revision. A batch of independently queued chapter commands would therefore conflict after its first successful chapter.

## Proposed approach

Add a native `generate_chapters_audio` command that accepts an ordered list of chapter IDs and enqueues **one** batch job. The command validates a nonempty, duplicate-free list against the current project and requires usable spoken text. It snapshots the selected chapters' spoken text and the chosen voice/settings at submission. At run start and before each commit, it checks the project revision under the project write lock; its own successful commits advance the expected revision. An unrelated edit stops the batch with a clear conflict instead of publishing audio from stale text.

Render chapters in project order. Map each chapter's existing fine-grained speech progress into one overall 0–100% job value, and append chapter-start/completion messages to that job's activity log. Use the existing pause/cancel boundaries; cancellation stops before the next chapter and during worker processing where supported. A failure stops the batch and names the failed chapter; earlier committed chapters remain. The job does not approve chapters or assemble manual takes.

In Review, allow selection of every chapter while keeping bulk Save limited to chapters with audio and Delete limited to generated audio. Add **Generate pending chapters** and **(Re)Generate selected** actions. Pending means no chapter audio or stale generated audio; the first action excludes current and imported audio. The selected action processes every selected chapter with usable text, including current or imported audio; confirm the number of chapters whose existing audio will be replaced before enqueueing. Show the number eligible before starting. Starting returns immediately with the job ID and a link/action to Render queue; Review refreshes when the batch finishes or when reopened. Keep existing per-chapter and manual-section controls.

## File changes

- `src-tauri/src/commands/production.rs` — modify: validate and enqueue ordered batch narration; reuse the current speech renderer and commit path; perform revision checks per chapter; map progress and chapter failures.
- `src-tauri/src/services/jobs.rs` — modify: add a bounded job status/event reporting callback for batch chapter names and progress, without changing existing job callers' behavior.
- `src-tauri/src/lib.rs` — modify: register the new Tauri command.
- `src/renderer/src/native.ts` — modify: expose `generateChapters(project, chapterIds)` returning a job ID.
- `src/renderer/src/pages.tsx` — modify: Review selection, eligible counts, batch buttons, immediate queue handoff/feedback, and filtering of Save/Delete actions.
- `tests/review-batch.test.tsx` — create: verify pending-chapter eligibility, selected regeneration, and Review bulk controls with missing, stale, current, and imported audio.
- `tests/DESKTOP_SMOKE.md` — modify: packaged-app batch and cancellation checks.
- `docs/PROJECT_CONTEXT.md`, `docs/API_CONTRACTS.md`, `docs/DECISIONS.md`, `docs/TASK_LOG.md` — modify after implementation: record the new workflow, command semantics, revision/failure behavior, and verification result.

## Implementation steps

1. **Native batch job.** Add and register `generate_chapters_audio(rootPath, expectedRevision, chapterIds) -> jobId`. Validate IDs, usable text, order, and revision before enqueueing. In the single job, call the existing speech renderer once per chapter, commit each output with the updated expected revision, and keep earlier commits if a later item fails or is cancelled. Add Rust tests for ID validation, ordering, progress mapping, and revision advancement; cover cancellation and replacement behavior where practical.
2. **Useful queue feedback.** Extend the job store with a bounded way for a running batch to record “Chapter X of Y: title” and chapter completion/failure in `events` and `message`; retain the existing queue schema and controls. Test that events are ordered and bounded, including a failure message.
3. **Review controls.** Update the native API wrapper and Review component. Make every chapter selectable; filter the pending action to missing/stale generated audio and the selected action to selected chapters with usable text. Confirm replacement of existing audio for selected chapters; keep audio-only filtering for Save and generated-only filtering for Delete. Start batches without waiting in Review, show queued count and a route to Render queue, and reload the project after completion or when revisiting Review. Test selection and disabled states for mixed chapter statuses.
4. **Documentation and local build.** Update the named context/contract files and smoke guide. Commit verified stages with human-readable names. Build and check the Apple Silicon app locally; do not push until asked.

## Verification

- `npm run build`; `cargo test --manifest-path src-tauri/Cargo.toml`; `git diff --check`.
- `npm run pack:mac`; `npm run test:smoke` with normal macOS app-launch access.
- In the packaged app, start a batch containing several missing chapters, navigate away from Review, observe overall progress and chapter messages in Render queue, cancel partway through, and confirm finished chapters remain playable while unstarted chapters remain pending. Re-run pending chapters and verify ready/imported chapters are not overwritten by that action. Select a ready and an imported chapter, confirm **(Re)Generate selected**, and verify new generated audio replaces each. Verify an intervening edit produces a visible revision conflict.

## Risks / edge cases

- Each chapter commit increments the project revision; the batch must advance its expected revision exactly once per successful chapter and reject unrelated changes. A cancelled or failed batch is intentionally partial.
- Long chapters can exceed the existing speech worker's per-job request limit; report the chapter-specific error and leave subsequent chapters pending.
- Chatterbox availability, generated audio quality, and true multi-chapter timing need a local worker and listening; automated tests can verify orchestration and files but cannot judge voice quality.

## Out of scope

- Automatically approving chapters, building the final book export, and batch voice-converting manually recorded sections.
