# API contracts

## Project manifest v1
- File: `project.json` at the project root, with `project.json.bak` holding the previous revision.
- Required fields: `schemaVersion`, UUID `id`, `title`, monotonic `revision`, millisecond timestamps, and ordered `chapters`.
- Chapter fields: stable UUID, title, zero-based order, relative source/processed/audio paths, segments, and `audioStale`.
- Segment fields: stable UUID, zero-based order, text, and optional selected take path.
- Project-owned paths must be relative normal path components. Absolute paths and parent traversal are rejected.
- Writes use a same-folder temporary file followed by rename. Mutations require the caller's expected revision.

## Implemented Tauri commands
- `read_manuscript(path) -> { name, text }`: UTF-8 `.txt`, `.md`, or `.markdown`, at most 20 MB.
- `create_project(parentPath, title, manuscript) -> ProjectSnapshot`.
- `open_project(rootPath) -> ProjectSnapshot`.
- `update_chapter(rootPath, expectedRevision, chapterId, title, sourceText) -> ProjectSnapshot`.
- `reorder_chapters(rootPath, expectedRevision, chapterIds) -> ProjectSnapshot`.
- `desktop_info() -> { platform, architecture, runtime }`.

`ProjectSnapshot` adds the absolute `rootPath` and chapter source/processed text to the persisted manifest fields.

## Implemented error codes
- `INVALID_PATH`, `UNSAFE_PROJECT_PATH`, `PROJECT_EXISTS`, `PROJECT_NOT_FOUND`.
- `INVALID_TITLE`, `EMPTY_MANUSCRIPT`, `MANUSCRIPT_TOO_LARGE`, `UNSUPPORTED_FILE`.
- `INVALID_PROJECT`, `UNSUPPORTED_PROJECT_VERSION`, `CHAPTER_NOT_FOUND`, `INVALID_CHAPTER_ORDER`.
- `REVISION_CONFLICT`, `IO_ERROR`, `INTERNAL`.

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

Add exact payloads, state transitions, and errors here as each later stage lands.
