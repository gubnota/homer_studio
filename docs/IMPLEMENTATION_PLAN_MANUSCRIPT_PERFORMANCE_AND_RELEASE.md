# Implementation Plan: Long manuscripts, expressive narration, line cues, and arm64 release

Status: Proposed, awaiting approval. Date: 2026-09-22.

## Goal

Let an author import or paste Markdown and create a project without a hidden prerequisite, process long chapters through a local LLM in bounded pieces, edit or discard parts of the combined candidate, mark up expressive speech with supported vocal gestures and pauses, and follow the spoken line during playback. Let the author select all generated Sound Studio clips and export the selection together. Replace the generic app icon and publish a versioned Apple Silicon release with its DMG and ZIP.

## Existing behavior

- Import accepts UTF-8 TXT/MD/Markdown up to 20 MB, but `ImportPage` disables **Create project** until a separate parent folder is chosen. Choosing a file fills the manuscript and title only. The reason for the disabled button is not shown.
- `process_text` accepts at most 16,000 characters and calls llama.cpp or Ollama once. The chapter editor already has an editable candidate textarea and an explicit accept step; source text is preserved.
- Chatterbox narration splits text into roughly 280-character requests, then concatenates WAVs into one M4A. It does not interpret pause markup or save timing boundaries. Vocal tags are documented for standalone gestures, but delivery and exact word stress are model-dependent.
- The project manifest has no timing cues. Review playback is a plain audio control and waveform. Existing projects use schema v1; importing audio produces no text alignment.
- Sound Studio stores independent generated clips in one persistent library and repeats recent variation clips in a comparison panel. Each card has individual WAV/M4A export, with a separate save dialog for every clip; there is no multi-selection.
- `src-tauri/icons/icon.svg` and generated icons contain a simple “H”. The release workflow publishes DMG and ZIP on a `v*` tag; package, Tauri, and Cargo versions are 0.1.0. The local branch has five unpublished feature commits. A read-only remote tag query currently fails DNS resolution, so remote tags must be checked again before choosing/pushing a release tag.

## Proposed approach

- Keep the project manifest backward compatible by adding optional/defaulted line cues to chapters. Cues belong to the current generated audio only. A cue contains stable line order, display text, measured `startMs`/`endMs`, and word tokens with character offsets but **no invented word timing**. Editing accepted text, regenerating, or importing audio clears or replaces cues with the audio transaction.
- Use paragraph/sentence-aware bounded LLM chunks sized conservatively from the configured context and output limits. Preserve order and boundaries, process sequentially, reject empty/oversized chunk output, and assemble only after every chunk succeeds. Cap total work with an explicit large-text limit and chunk-count bound. Expose chunk progress and cancellation through the existing job system where practical; no partial candidate overwrites the user's draft.
- In the editor, show the returned pieces as editable fragments with remove/replace controls and a combined candidate preview. Accept writes the combined reviewed text through the existing revision-checked `accept_processed_text` path. Keep source text and accepted text separate.
- Parse a small, visible narration notation: normal punctuation/capitalization stays literal, documented Chatterbox vocal tags become separate vocal segments, and an explicit pause marker inserts measured silence. Reject unknown control markers with a useful error. Provide an editable dramatic-reading example using “Tomorrow, and tomorrow, and tomorrow” to demonstrate pacing; do not promise a particular actor's voice or exact per-word stress. Use the selected voice for speech and gestures when the worker supports it.
- Generate per-line/per-phrase WAV pieces, measure each piece and inserted silence, concatenate, and store line cues with the successful M4A commit. In Review, highlight the current line from the audio element's playhead and allow seeking by line. Cue display uses the accepted spoken text with markup removed. Old projects and imported audio show an honest “timing unavailable” state until a later alignment feature.
- Give Sound Studio one clip-ID selection shared by the library and variation cards. **Select all** selects every currently displayed library clip; **Clear selection** resets it. Bulk WAV/M4A export asks for one folder, validates all selected IDs natively, creates collision-safe filenames, and reports exported/skipped/failed counts. No bulk deletion is implied. A later library refresh keeps surviving selections and removes IDs no longer present.
- Replace the current vector “H” with a distinct book-and-waveform icon, generate the macOS icon set from that source, and verify it in the bundled app and Finder. Set version 0.2.0 across package/Tauri/Cargo files; after local checks and manual packaged-app review, verify remote tag availability, push the commits and `v0.2.0` tag, and confirm the GitHub release contains arm64 DMG and ZIP. If network or Actions fails, report the exact unpublished or incomplete state instead of claiming a release.

## File changes

- `src/renderer/src/pages.tsx` — modify import guidance/location flow, fragment editor, expression help, and line-highlight playback.
- `src/renderer/src/SoundStudioPage.tsx` — modify shared clip selection, Select all/Clear selection, selected count, and batch export controls across library and variation views.
- `src/renderer/src/native.ts` — modify typed calls if LLM work becomes a job, for any import location default command, and for one-folder batch clip export.
- `src/renderer/styles/app.css` — modify fragment, markup-help, cue, active-line, and clip-selection styles.
- `src/shared/contracts.ts` — modify candidate-piece and optional chapter-cue types.
- `src-tauri/src/services/llm.rs` — modify bounded chunking, sequential provider calls, output assembly, progress/cancellation, and tests.
- `src-tauri/src/commands/production.rs` — modify text-processing job/response bridge and atomic narration cue commit.
- `src-tauri/src/lib.rs` — modify command registration/app state only if required by the candidate job result.
- `src-tauri/src/services/jobs.rs` — modify only if existing job results cannot expose a completed text candidate safely.
- `src-tauri/src/services/speech.rs` — modify narration notation parsing, speech/gesture/pause rendering, measured line cues, and tests.
- `src-tauri/src/services/project_store.rs` — modify optional cue persistence, invalidation, audio commit, Markdown import tests, and schema compatibility tests.
- `src-tauri/src/commands/sounds.rs` — modify native sound export to accept a bounded batch of selected clip IDs and a user-chosen folder, validate each asset, and report results without silently overwriting files.
- `src-tauri/src/commands/project.rs` — modify only if parent-location default belongs at the native import boundary.
- `tests/manuscript-drop.test.ts`, `tests/DESKTOP_SMOKE.md` — modify import and desktop verification cases.
- `tests/standalone-audio.test.tsx` — modify only if shared gesture validation changes.
- `src-tauri/icons/icon.svg`, `src-tauri/icons/icon.icns`, `src-tauri/icons/icon.png`, `src-tauri/icons/32x32.png`, `src-tauri/icons/128x128.png`, `src-tauri/icons/128x128@2x.png` — replace source artwork and regenerated desktop assets.
- `package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` — modify release version to 0.2.0.
- `README.md`, `docs/API_CONTRACTS.md`, `docs/ARCHITECTURE_INDEX.md`, `docs/MODULE_OWNERSHIP.md`, `docs/DECISIONS.md`, `docs/PROJECT_CONTEXT.md`, `docs/TASK_LOG.md` — modify setup, contracts, architecture, decisions, and verification status.

## Implementation steps

1. **Make manuscript creation obvious and functional.** Keep the native caller's project-parent safety check. Prefill a sensible local project parent or guide the user to choose one in the same import flow; show which requirement is missing beside the Create action. Verify choose, drop, and paste paths with Markdown headings and UTF-8. Commit: `Make manuscript import ready to create a project`.
2. **Process and review longer text.** Add deterministic chunking and bounded sequential LLM calls; preserve headings and paragraph order, report progress, and assemble only complete output. Return editable fragments to the renderer; allow delete, replace, and combined-preview editing before the existing accept operation. Test long input, long single paragraphs, Unicode, provider failure mid-run, and unchanged source on discard/failure. Commit: `Process long chapters in editable pieces`.
3. **Add expressive narration controls.** Parse documented vocal tags and a pause marker, validate their placement and bounds, keep visible text free of control markers, and synthesize ordered speech/gesture/silence pieces. Put practical examples and an audition path in the editor. Test parsing, escaping of literal brackets, chunk boundaries, unsupported tags, and cancellation. Commit: `Add pauses and vocal gestures to narration`.
4. **Record and use line cues.** Measure generated pieces, persist monotonic cues in the chapter only when audio commits, clear stale cues on text edits/import, and highlight/seek the current line during Review playback. Test legacy manifests, timing order, stale-audio handling, and line selection at exact boundaries. Commit: `Highlight spoken lines during playback`.
5. **Select and export generated clips together.** Add ID-based selection to `SoundStudioPage.tsx`; provide Select all, Clear selection, and batch export buttons. Add a native bounded batch export in `commands/sounds.rs` using the existing `sound_store` asset validation and one folder chooser in `native.ts`. Test duplicate IDs, missing source audio, filename collisions, a cancelled folder choice, and selection staying consistent when new clips appear. Commit: `Export selected sound clips together`.
6. **Finish the icon and release.** Replace the icon artwork and regenerate desktop assets, bump all versions, update docs, build/test/smoke the arm64 package, and inspect icon/playback/import in the packaged app. Verify remote state, push the implementation commits and `v0.2.0` tag, then verify the release page contains the expected DMG and ZIP. Commit: `Prepare Homer Studio 0.2.0 for Apple Silicon`.

## Verification

- Run `npm run build`, `cargo test --manifest-path src-tauri/Cargo.toml`, `python3 -m unittest discover -s workers/tests`, `npm run test:integration`, `npm run package:mac`, `npm run test:smoke`, and `node scripts/verify-release.mjs --tag v0.2.0 --package`.
- In the packaged app, import a Markdown book by chooser and by drop; paste a manuscript; confirm Create becomes actionable and the project opens with parsed chapters.
- Process a chapter over 16,000 characters with a configured local provider; remove/replace a fragment, inspect combined text, accept it, reopen the project, and verify source text remains intact.
- Audition a short dramatic passage with repeated words, a pause, and a supported gesture; check cue order and visible line highlighting while playing and seeking. Listen for quality without treating punctuation or tags as guaranteed emphasis.
- Generate three variations, select one, use Select all, clear and reselect, then export selected clips in WAV and M4A to one folder. Confirm unique filenames and verify that each exported file opens.
- Check the final `.app` architecture, signature, icon appearance, DMG/ZIP contents, version/tag match, and GitHub release assets after the tag workflow completes.

## Risks / edge cases

- LLM context sizes differ by provider/model; conservative chunks and bounded output are needed to avoid truncation. Many sequential requests can take a long time, so cancellation and honest progress matter.
- Chatterbox may pronounce punctuation and gestures inconsistently. Pauses are deterministic; exact stress and a specific actor's performance are not.
- Concatenated audio can have codec padding. Measure WAV pieces and reconcile the final cue end with probed M4A duration; never present word timestamps inferred from character counts as accurate alignment.
- Previously imported chapter audio has no line map. Existing schema-v1 projects must still open, play, and export.
- A clip appearing in both comparison and library views must have one selection state. Bulk export must preserve existing files and explain partial failures.
- A tag push triggers a public GitHub release. The current network cannot resolve GitHub; publication depends on restoring access and confirming CI succeeds.

## Out of scope

- Cloning Ian McKellen's voice or reproducing an exact performance; model training; automatic word-level forced alignment; karaoke video export; alignment of externally imported audio; bulk clip deletion; a Linux GPU deployment; public Developer ID signing/notarization.
