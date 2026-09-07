# Project context

## Identity
- Project: Audio Studio (prototype labels: Audiobook Studio / VoxEdit).
- Workspace/repository directory: `homer_studio`.
- Repository: `https://github.com/gubnota/homer_studio.git` (supplied by the user).
- Local Git is initialized on `main` with no commits and the supplied GitHub URL as `origin`; remote inspection returned no HEAD, branches, or tags.
- Purpose: turn source chapters into local chapter audio, a combined audiobook, and YouTube timestamps.
- Users: authors and audiobook creators on Apple Silicon Macs.

## Current milestone
- Implementation authorized on 2026-09-07.
- Build the first working local arm64 desktop package, then complete the approved workflow in small verified commits.
- No runnable desktop source project is present.
- The only application material is a downloaded prototype in `audio_studio_ui/`.
- The six context documents were absent and have been initialized during planning.

## Observed technology
- Prototype JavaScript embeds React 18.3.1.
- Client routing and utility CSS are present in compiled assets.
- A Vite-style module preload wrapper is present; original build configuration is absent.
- No package manifest, lockfile, source map, tests, native shell, or CI files exist.
- All downloaded HTML pages are identical SPA entry documents.
- Base44 authentication, tracking, and badge scripts are present.
- Model labels and audio operations are demonstration state, not working integrations.

## Approved technology
- Tauri 2 desktop shell with a narrow Rust command boundary.
- React and TypeScript renderer reconstructed from prototype components.
- Existing compiled CSS reused as the visual baseline.
- Rust filesystem and subprocess services in the native backend.
- JSON project/configuration files; no database.
- FFmpeg and FFprobe installed locally and selected through settings.
- Replaceable local text providers: llama.cpp / GGUF and Ollama.
- Initial speech uses installed macOS voices plus imported audio; CI targets GitHub on macOS arm64.

## Hard constraints
- Primary target is macOS arm64 / Apple Silicon.
- Preserve the prototype's layout and visual intent.
- Do not claim compiled assets are a maintainable source project.
- Do not invent a reference application that is not present.
- Remain local-first; no application accounts or remote backend.
- Models and executables are configurable, never tied to one machine's paths.
- Do not download model weights automatically.
- No mandatory Apple Developer credentials for test packages.
- No Intel or universal-build requirement for the first milestone.

## Core workflow
1. Create/open a project in a user-selected directory.
2. Import TXT/Markdown or paste text.
3. Inspect chapter boundaries and ordering.
4. Edit chapter/segment text.
5. Optionally process text using a selected local LLM.
6. Generate speech or import chapter audio.
7. Listen, flag, approve, and retry items.
8. Combine complete chapter audio using FFmpeg.
9. Save the final audio and measured chapter timestamps.

## UI reference
- Projects and Import screens.
- Project overview with ordered chapter rows.
- Editor with chapter navigation, segment list, and inspector.
- Review screen.
- Voices screen.
- Render Queue screen.
- Exports screen.
- Settings screen.
- Persistent project playback bar.
- Neutral backgrounds, white panels, compact typography, and narrow borders.

## Data principles
- Human-readable, versioned project manifest.
- Project content paths are relative to the chosen project root.
- Stable chapter IDs survive reordering.
- Source text is retained when processed text changes.
- Duration comes from actual audio, never source-text estimates.
- Failed chapters must not silently disappear from exports.
- Completed outputs are published only after successful verification.
- Unsupported schema versions must produce actionable errors.

## Privacy and security
- No Base44 authentication, badge, analytics, or hosted assets in the desktop runtime.
- Renderer has no arbitrary filesystem or process access.
- Typed, validated Tauri commands own the UI/native boundary.
- Child processes receive argument arrays with shell execution disabled.
- Local model requests default to loopback endpoints.
- Logs contain operational metadata and errors, not full manuscripts.
- Certificates, tokens, and signing passwords never belong in source control.

## Performance and reliability
- Process long jobs asynchronously with progress and cancellation.
- Run one heavy generation/export operation at a time initially.
- Avoid loading an entire audiobook into renderer memory for playback.
- Use actual cumulative duration values before formatting timestamps.
- Preserve the last good audio when a replacement job fails.
- Recover interrupted jobs as interrupted, never as successful.

## Agent workflow
- Read the six compact context files before targeted code inspection.
- Use `docs/ARCHITECTURE_INDEX.md` to locate relevant modules.
- Keep summaries implementation-aligned and separate proposed from existing behavior.
- Follow the approved implementation plan for source/configuration changes.
- Keep implementation steps small and dependency-ordered.
- Update API contracts when persisted or IPC interfaces change.
- Record accepted architectural changes in `docs/DECISIONS.md`.
- Record completed work and validation in `docs/TASK_LOG.md`.
- Do not scan or paste the whole repository by default.
- Commit each verified implementation stage; do not push until the user asks.

## Navigation
- `docs/ARCHITECTURE_INDEX.md`: repository map.
- `docs/MODULE_OWNERSHIP.md`: proposed boundaries and dependencies.
- `docs/API_CONTRACTS.md`: current contract status and proposed baseline.
- `docs/DECISIONS.md`: decisions and proposals.
- `docs/TASK_LOG.md`: progress and verification status.
- `docs/IMPLEMENTATION_PLAN.md`: implementation handoff once written.
