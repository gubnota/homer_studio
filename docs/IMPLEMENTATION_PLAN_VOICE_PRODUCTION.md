# Voice production implementation

Approved by the user on 2026-10-05 after the implementation plan in chat.

## Scope
Lossless native capture, immutable sources and reversible compositions, persistent
waveform peaks, Voice Memos, shared editors, Voice Profiles, optional Seed-VC/RVC
conversion and DeepFilterNet/Resemble enhancement, supervised isolated workers,
accepted-composition integration and measured exports. Preserve existing Chatterbox.

## Milestones
1. Audio domain/persistence and deterministic rendering tests.
2. Native recording, waveform cache and shared editor/controls.
3. Isolated processors, explicit setup and cancellation tests.
4. Profiles, Book/Sound/Voice Lab integration, composition exports.
5. Full checks, arm64 packaging, smoke, documentation.

## Verification
`npm run build`; `cargo test --manifest-path src-tauri/Cargo.toml`;
`npm run test:integration`; Python unittest discovery; `npm run pack:mac`;
`npm run test:smoke`. Real microphone and model listening checks are manual.

## Execution
Subagent CLI was attempted but its local daemon credentials are unavailable.
Implementation continues in the main workspace; no remote publishing authorized.

All five implementation milestones are complete. Automated verification passed:
12 renderer tests with typecheck/build, 49 native tests including the FFmpeg
integration fixture, 17 Python tests, arm64 app/resource/signature verification,
and packaged startup smoke. Real microphone and model listening checks remain
manual; see `docs/VOICE_PRODUCTION.md`. Changes and the app bundle remain local.
