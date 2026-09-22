# Implementation Plan: Neural voices, sample recording, and coherent controls

Status: Approved and implemented locally. Date: 2026-09-22.

## Goal

Replace macOS-generated narration and its Karen/Samantha/Daniel presets with local Chatterbox speech for both books and standalone clips. Let a creator choose the model's natural default voice or build a named voice from their own recorded/imported samples, preview it, and select it in a searchable modal. Explain and provide practical speech-delivery controls without pretending Chatterbox Turbo supports exact word emphasis. Improve the Sound Studio effect workflow and make form controls visually consistent throughout the app.

## Existing behavior

- `settings.rs` defaults to `macos_say`, seeds installed-voice presets, and checks that a saved voice is installed. `speech.rs` invokes `/usr/bin/say`; `production.rs` uses it for previews and chapter jobs.
- Sound Studio alone calls the Chatterbox Turbo worker. `workers/chatterbox/server.py` calls `generate(prompt)` with its bundled default conditionals; the version-1 JSON worker protocol has no reference-audio input. `sound_render.rs` routes speech/tags to port 8765 and effects to the AudioLDM 2 / Stable Audio Open worker on port 8766.
- Voice presets are settings records with a macOS voice ID and words-per-minute value. The Voices screen is a grid and an inline native `<select>` editor. Settings also exposes a macOS voice ID and rate.
- The effects worker currently produces one candidate at a time. This Mac uses AudioLDM 2 because Stable Audio Open weights require separate access. The user has listened to a fabric result and found it poor; a successful WAV validation does not prove perceptual quality.
- The Tauri app targets Apple Silicon; completed project audio and imports are stored separately from the speech provider. The worker API binds loopback, accepts bounded JSON, and returns bounded WAV.

## Proposed approach

### Neural voice model and samples

- Treat “LLM-generated voice” as **local neural text-to-speech**, distinct from the optional text LLM. Use the existing Chatterbox Turbo checkpoint for English speech. Keep its built-in model voice as the no-sample starter choice; remove macOS voices and macOS preset seeding from all new generation flows. Do not synthesize a replacement voice when the worker is unavailable: show setup status and preserve existing audio/import functionality.
- Add an app-data voice library with a versioned manifest, stable UUID voice/sample IDs, display names, sample metadata, and app-owned normalized reference WAVs. A voice may contain multiple recordings **from the same speaker**. Each synthesis uses a selected, explicitly previewable reference (or a validated short same-speaker compilation prepared in app data); do not claim that Turbo can blend distinct speaker identities. This interpretation is provisional pending the user's answer about “combine.”
- Accept imported audio through a native file chooser and in-app microphone recording. Normalize copies with FFmpeg, validate decodable speech, reasonable duration, non-silence, size, and sample rate; retain originals outside the library untouched. Show recording level, playback, retry, and a local-use/permission reminder. Mic capture uses `getUserMedia`/`MediaRecorder` in the Tauri webview, a macOS microphone usage description, and a bounded native save command. Handle denial, unsupported codec, interrupted recording, and cleanup. No sample is transmitted to a hosted service.
- Extend the loopback worker contract to version 2 with a bounded reference-audio upload by bytes, not a renderer-supplied filesystem path or untrusted worker-side path. Rust owns sample lookup and validation. The worker stages the bytes in a temporary file, calls `ChatterboxTurboTTS.generate(..., audio_prompt_path=...)`, and cleans up. Version mismatch yields an actionable upgrade message. The default voice remains available without a sample. Voice-library deletion is blocked for the active voice or requires choosing another voice; existing generated chapters and clips remain playable.
- Route previews, queued chapter narration, and Sound Studio speech through one speech service backed by the worker. Split long chapters at sentence/clause boundaries into bounded chunks, synthesize sequentially with one chosen voice, assemble with FFmpeg, probe actual duration, and publish only after all chunks succeed. Keep job cancellation and last-good-audio semantics. Preserve existing project manifests and previously generated/imported files. Migrate settings away from `macos_say` and old presets without deleting saved audio; retain old values only long enough to read and migrate, then expose the model default as the selected voice.
- Expose plain-language delivery guidance near text fields: punctuation and sentence breaks influence phrasing; supported paralinguistic tags can be previewed; select a voice reference with the delivery style desired. Offer a sentence-level preview/retry flow and optional pause breaks assembled between chunks for deliberate emphasis. Do **not** label uppercase, SSML, `*word*`, or an exaggeration slider as precise word emphasis: Turbo ignores CFG/exaggeration and has no documented word-level emphasis markup. Keep manuscript text unchanged unless the user accepts edited delivery text.

### Sound effects and UI

- Keep fabric, ambience, and panting as sound-effect prompts, separate from speech/Chatterbox tags. Add prompt guidance with concrete acoustic examples, negative prompt and seed controls supported by the effects pipeline, and a small candidate batch (e.g. three seeds) with immediate A/B listening and one-click save/export. Clearly show the active model and any noncommercial/gated model restrictions. Do not falsely describe an upsampled 16 kHz source as high-fidelity. Evaluate fabric/rustle and breathing prompts by listening on the Mac; if AudioLDM 2 remains unsuitable, report that limitation and make model selection/future Linux GPU worker use straightforward. No fabricated quality guarantee.
- Build one reusable accessible modal voice picker with a search field, keyboard navigation, focus return/trap, voice previews, and clear default/custom voice badges. Reuse it for Voices, chapter narration selection, Settings, and Sound Studio speech. Keep selection in one effective settings field so previews and book rendering cannot silently diverge.
- Define shared form tokens/classes for text/number fields, textareas, choices, checkboxes, toggles, and disabled/focus/error states; replace page-specific unstyled controls across the renderer. Preserve labels, keyboard access, visible focus, and the existing neutral visual theme. Use a modal picker where searching a growing collection helps; leave simple short option lists as styled controls.

## File changes

- `src-tauri/src/services/settings.rs` — modify: migrate speech settings and old macOS presets, validate selected library voice and worker configuration, remove installed-voice requirement/diagnostic.
- `src-tauri/src/services/speech.rs` — modify: remove macOS voice discovery/synthesis, retain media import/probing helpers, add shared Chatterbox-backed speech generation and bounded chapter chunk assembly.
- `src-tauri/src/services/voice_store.rs` — create: app-data voice/sample manifest, safe paths, normalized sample import/recording, selection/deletion validation, migration helpers.
- `src-tauri/src/services/sound_workers.rs` — modify: worker protocol-v2 handshake and bounded binary sample upload; retain loopback-only transport, cancellation, and response limits.
- `src-tauri/src/services/sound_render.rs` — modify: selected voice in speech requests, effects candidate/negative-prompt metadata and verification.
- `src-tauri/src/services/sound_store.rs` — modify: backward-compatible optional voice/effect candidate metadata in the independent clip manifest.
- `src-tauri/src/commands/production.rs` — modify: voice-library commands, neural preview, chapter generation through the common speech service; keep audio import and previous completed audio.
- `src-tauri/src/commands/sounds.rs` — modify: pass chosen voice and effects options through narrow commands.
- `src-tauri/src/commands/system.rs` — modify: report neural worker/model readiness instead of macOS speech availability.
- `src-tauri/src/services/mod.rs`, `src-tauri/src/lib.rs` — modify: register the new voice store/commands.
- `workers/worker_protocol.py` — modify: versioned bounded reference-audio transfer and strict request validation, with temporary-file cleanup.
- `workers/chatterbox/server.py` — modify: optional reference clip conditioning, default voice fallback, explicit model capability/error reporting.
- `workers/sfx/server.py` — modify: exposed negative-prompt/seed options and accurate source quality metadata; preserve current model selection.
- `src/shared/contracts.ts` — modify: voice-library, settings, request/asset, worker-health, and new command types.
- `src/renderer/src/native.ts` — modify: typed voice library, sample chooser/upload, recording, preview, and effects-candidate APIs.
- `src/renderer/src/pages.tsx`, `src/renderer/src/voice-presets.ts` — modify/delete respectively: replace macOS preset views/helpers with voice-library controls, unify chapter/Settings selection, add delivery guidance.
- `src/renderer/src/SoundStudioPage.tsx` — modify: selected voice and guidance for speech, effect candidate generation/listening, honest engine/quality labels.
- `src/renderer/src/components/VoicePicker.tsx` — create: accessible searchable voice-selection modal.
- `src/renderer/src/components/VoiceRecorder.tsx` — create: mic recording, playback/retry, and bounded handoff to native storage.
- `src/renderer/styles/app.css`, `src/renderer/styles/prototype.css` — modify where controls are defined: shared form/modal styles and responsive states.
- `src-tauri/Info.plist` — create: macOS microphone usage description for packaged recording.
- `tests/voice-presets.test.ts`, `tests/standalone-audio.test.tsx`, `tests/navigation.test.ts`, `workers/tests/test_protocol.py`, `tests/DESKTOP_SMOKE.md` — modify: replace obsolete preset assumptions; cover search/selection, sample errors, worker transfer, effects candidates, and packaged recording.
- `docs/API_CONTRACTS.md`, `docs/ARCHITECTURE_INDEX.md`, `docs/MODULE_OWNERSHIP.md`, `docs/DECISIONS.md`, `docs/PROJECT_CONTEXT.md`, `docs/TASK_LOG.md`, `README.md`, `workers/README.md` — modify after implementation to reflect actual contracts, setup, limitations, and verification.

## Implementation steps

1. **Voice-library foundation** (`voice_store.rs`, `settings.rs`, contracts, Rust tests): introduce versioned manifest and safe app-owned sample files; migrate old settings to a Chatterbox default with no macOS generation path; verify existing projects/audio remain intact. Commit: `Introduce a local voice library`.
2. **Reference-conditioned speech** (`worker_protocol.py`, `chatterbox/server.py`, `sound_workers.rs`, `speech.rs`, `production.rs`, worker/Rust tests): transfer a validated sample over protocol v2, render preview and chunked chapter jobs through Chatterbox, preserve cancellation and atomic publish. Confirm both bundled default and a real short reference sample produce playable WAV/M4A. Commit: `Use Chatterbox voices for narration`.
3. **Voice creation and delivery UX** (`VoicePicker.tsx`, `VoiceRecorder.tsx`, `pages.tsx`, `native.ts`, `Info.plist`, UI tests): import/record several same-speaker samples, preview and select a reference, show searchable modal, add truthful emphasis/pause guidance and sentence retries. Commit: `Add recorded voices and searchable selection`.
4. **Sound Studio quality workflow** (`SoundStudioPage.tsx`, `sound_render.rs`, `sound_store.rs`, `sfx/server.py`, tests): selectable speech voice; effect prompt/negative prompt/seed candidates and compare; display actual model capability. Listen to fabric and breathing examples, record whether results are usable. Commit: `Make sound effects easier to compare`.
5. **Unified controls and documentation** (`app.css`, `prototype.css`, affected page controls, docs/README, UI tests): apply one visual/focus system to all renderer inputs/selects/checkboxes and update persistent context/contracts/ADR. Commit: `Unify studio controls and document local voices`.

## Verification

- Run `npm run typecheck`, `npm test`, `npm run build`, `cargo test --manifest-path src-tauri/Cargo.toml`, `python3 -m unittest discover -s workers/tests`, `npm run test:integration`, `npm run package:mac`, and `npm run test:smoke` after the relevant stages.
- On Apple Silicon, launch the arm64 `.app`; import a sample and record one via microphone, deny permission once, create/select/preview a voice, narrate a short chapter and a longer multi-chunk chapter, cancel/retry a job, reopen an older project, and confirm old audio still plays.
- Generate several speech and effect clips through the actual local workers, play them, and document perceptual results for fabric, breathing, and a spoken sentence. Verify all voice options use Chatterbox and no UI can initiate macOS `say` speech.
- Check modal keyboard behavior, search, control alignment, focus, errors, and narrow-window layout; inspect the packaged app rather than relying only on browser preview.

## Risks / edge cases

- Chatterbox Turbo is English-focused and has no reliable exact word-emphasis API; chunk-level pauses and punctuation can change delivery but do not guarantee emphasis. Its reference clip should be clean, single-speaker speech around ten seconds; combining clips may reduce quality. Test both selected-clip and same-speaker compilation, and keep the better method explicit in the UI.
- The current AudioLDM 2 checkpoint's perceptual quality may be insufficient for close fabric sounds. Seed/prompt/candidate controls improve selection, not model fidelity. Stable Audio Open remains optional and access-controlled; a stronger future Linux GPU model can use the versioned worker boundary.
- Microphone availability differs between browser preview and packaged WebKit. Recording must be verified in the packaged arm64 app; import remains a complete fallback.
- Migrating a macOS preset must not re-render or delete existing projects. A missing worker/sample must fail visibly without replacing the last good chapter audio.
- Reference recordings are personal biometric-like data. Keep them local, avoid logging waveform contents or user paths, and require a deliberate action to add/delete them.

## Out of scope

- Training a new model, blending distinct speakers into a synthetic identity, hosted voice APIs, remote Linux access/authentication, automatic model-weight downloads, and changing previously generated chapter audio in place.
- Claiming SSML or a word-highlight button can force exact per-word stress with Chatterbox Turbo.

## Sources checked

- Chatterbox official [README](https://github.com/resemble-ai/chatterbox/blob/master/README.md) and [Turbo implementation](https://github.com/resemble-ai/chatterbox/blob/master/src/chatterbox/tts_turbo.py): reference-audio API and unsupported Turbo exaggeration/CFG.
- AudioLDM 2 official [repository](https://github.com/haoheliu/AudioLDM2): model variants and seed sensitivity.
- Tauri official [macOS bundling guide](https://v2.tauri.app/distribute/macos-application-bundle/): microphone usage description in `Info.plist`.
