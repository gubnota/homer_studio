# Project context

## Identity
- Project: Homer Studio (prototype labels: Audiobook Studio / VoxEdit).
- Workspace/repository directory: `homer_studio`.
- Repository: `https://github.com/gubnota/homer_studio.git` (supplied by the user).
- Local Git uses `main` with the supplied GitHub URL as `origin`; implementation commits remain local until the user requests a push.
- Purpose: turn source chapters into local chapter audio, a combined audiobook, and YouTube timestamps.
- Users: authors and audiobook creators on Apple Silicon Macs.

## Current milestone
- Version 0.2.0 now includes explicit English model installation, per-section narration and takes, queue progress/cleanup, and recording-to-narrator conversion; Apple Silicon packaging is the current release checkpoint.
- The Tauri app, durable project storage, drag-and-drop import, optional local text providers, local neural voice library, narration/import, review, export, and packaging are implemented.
- The current arm64 app bundle is being rebuilt and verified; no new release has been pushed or tagged.
- The user explicitly deferred pushing; keep this implementation and its commits local until asked.

## Technology
- Tauri 2 desktop shell with a narrow Rust command boundary.
- React 18, TypeScript, Vite, and Vitest renderer reconstructed from prototype components.
- Prototype-derived styling maintained as the visual baseline.
- Rust filesystem and subprocess services in the native backend.
- JSON project/configuration files; no database.
- FFmpeg, FFprobe, llama.cpp, and Ollama executables are discovered in bounded system and user Homebrew locations or selected explicitly in Settings.
- Replaceable local text providers: llama.cpp / GGUF and Ollama.
- English speech uses local Chatterbox Turbo or Original with a built-in or selected recorded/imported voice reference; Original also converts recorded delivery to the chosen narrator voice.
- Sound Studio uses Chatterbox Turbo for speech and Stable Audio Open for effects through versioned loopback HTTP. Turbo and Original checkpoints can be explicitly installed from Settings.
- GitHub Actions builds and verifies packages on macOS arm64.

## Hard constraints
- Primary target is macOS arm64 / Apple Silicon.
- Preserve the prototype's layout and visual intent.
- Do not claim compiled assets are a maintainable source project.
- Do not invent a reference application that is not present.
- Remain local-first; no application accounts or remote backend.
- Models and executables are configurable, never tied to one machine's paths.
- Download Turbo/Original weights only after an explicit user action in Settings; show byte progress and disk use.
- No mandatory Apple Developer credentials for test packages.
- No Intel or universal-build requirement for the first milestone.
- Development packages use ad-hoc signing; public Developer ID signing/notarization is a later distribution concern.

## Core workflow
1. Create/open a project in a user-selected directory.
2. Choose, drop, or paste a TXT/Markdown manuscript.
3. Inspect chapter boundaries and ordering.
4. Edit chapter/segment text.
5. Optionally process text using a selected local LLM.
6. Choose a built-in or custom voice, install/start the English worker, then narrate a chapter or individual section; record and convert a section as a preview take when needed.
7. Listen to takes, choose the preferred one, assemble the chapter, then flag or approve it.
8. Combine complete chapter audio using FFmpeg.
9. Save the final audio and measured chapter timestamps.
10. Independently, prompt speech with supported Turbo gesture tags or a Stable Audio Open effect in Sound Studio, then play, compare, retry, or export clips.

## UI reference
- Projects and Import screens.
- Project overview with ordered chapter rows.
- Editor with chapter navigation, segment list, and inspector.
- Review screen.
- Voices screen.
- Render Queue screen.
- Exports screen.
- Settings screen.
- Project-independent Sound Studio and clip library.
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
- `docs/MODULE_OWNERSHIP.md`: accepted boundaries and dependencies.
- `docs/API_CONTRACTS.md`: persisted, command, provider, media, and export contracts.
- `docs/DECISIONS.md`: accepted architecture decisions.
- `docs/TASK_LOG.md`: progress and verification status.
- `docs/IMPLEMENTATION_PLAN_NARRATION_EDITING_AND_MODEL_SETUP.md`: approved current feature plan.
