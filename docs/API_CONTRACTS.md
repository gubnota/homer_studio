# API contracts

## Current state
No stable application API, persistence schema, or native protocol exists.
The downloaded demo's in-memory objects are reference data, not supported contracts.

## Accepted baseline
- Versioned project manifest containing project identity, ordered chapters, segment records, audio references, and export history.
- Original chapter source in readable UTF-8 files; project-owned paths are relative.
- Stable IDs independent of displayed chapter numbers.
- Settings separate LLM provider selection from speech generation settings.
- Narrow Tauri command API for project operations, settings, jobs, audio import/playback, and export.
- Structured errors include a code, actionable message, and optional affected chapter/job ID.
- Jobs expose real state/progress and cancellation; interrupted jobs never become completed automatically.
- Precise measured durations are accumulated before timestamps are formatted.
- Exact payloads, filenames, state transitions, and error codes are specified in `docs/IMPLEMENTATION_PLAN.md`.

Record exact versioned payloads and errors here as each implementation stage lands.
