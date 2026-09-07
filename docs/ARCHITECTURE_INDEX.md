# Architecture index

## Reference prototype
- `.gitignore`: excludes `audio_studio_ui/` and `.DS_Store`.
- `audio_studio_ui/index.html`: downloaded SPA entry, including hosted auth/tracking.
- `audio_studio_ui/index-BeQ8ttIE.js`: compiled React application, libraries, sample data, and simulated interactions.
- `audio_studio_ui/index-D-2jFf5J.css`: compiled utility classes, theme variables, and typography.
- `audio_studio_ui/urls.txt`: intended route map.
- `audio_studio_ui/manifest.json`: hosted VoxEdit PWA metadata.
- Other HTML files: identical downloaded SPA entries, not separate screen source files.
- `audio_studio_ui/badge.js`: hosted-platform badge; no desktop reuse planned.
- `docs/`: persistent project context and implementation plan.

## Application
- `src-tauri/`: Tauri lifecycle, permissions, Rust commands, icons, build configuration, and future native services.
- `src-tauri/src/commands/project.rs`: native project and manuscript commands.
- `src-tauri/src/services/project_store.rs`: schema v1 persistence, parsing, segmentation, atomic saves, and filesystem tests.
- `src/shared/`: serializable contracts and pure chapter/time logic.
- `src/renderer/src/native.ts`: typed renderer bridge to native commands and file dialogs.
- `src/renderer/`: React project library, importer, chapter editor, shared controls, and prototype-derived styles.
- `tests/`: focused unit, filesystem, media-integration, and desktop smoke tests.
- Root npm/Vite/TypeScript configuration: renderer development, checks, and Tauri arm64 packaging.
- `docs/IMPLEMENTATION_PLAN.md`: exact planned filenames and verification commands.

## Entry points and build
- `src/renderer/main.tsx`: renderer entry.
- `src-tauri/src/main.rs` and `src-tauri/src/lib.rs`: native entry and command registration.
- `src-tauri/tauri.conf.json`: application identity, windows, CSP, bundle, and icons.
- `src-tauri/capabilities/default.json`: renderer permission allowlist.
- `npm run dev`: Tauri development app.
- `npm run build`: typecheck, renderer tests, and renderer production bundle.
- `npm run pack:mac`: release `.app` bundle for the current macOS architecture.
