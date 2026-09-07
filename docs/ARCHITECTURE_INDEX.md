# Architecture index

## Existing workspace
- `.gitignore`: excludes `audio_studio_ui/` and `.DS_Store`.
- `audio_studio_ui/index.html`: downloaded SPA entry, including hosted auth/tracking.
- `audio_studio_ui/index-BeQ8ttIE.js`: compiled React application, libraries, sample data, and simulated interactions.
- `audio_studio_ui/index-D-2jFf5J.css`: compiled utility classes, theme variables, and typography.
- `audio_studio_ui/urls.txt`: intended route map.
- `audio_studio_ui/manifest.json`: hosted VoxEdit PWA metadata.
- Other HTML files: identical downloaded SPA entries, not separate screen source files.
- `audio_studio_ui/badge.js`: hosted-platform badge; no desktop reuse planned.
- `docs/`: persistent project context and implementation plan.

## Absent
- No separate reference `homer_studio` application exists inside this workspace.
- No application source tree, package manifest, lockfile, native entry, tests, or CI configuration.
- Local Git: `main`, no commits, `origin` points to GitHub (`gubnota/homer_studio`); remote inspection returned no HEAD, branches, or tags.

## Proposed navigation (not implemented)
- `src/main/`: Electron lifecycle, validated IPC, project/configuration I/O, subprocess jobs, models, and audio.
- `src/preload/`: narrow renderer API.
- `src/shared/`: serializable contracts and pure chapter/time logic.
- `src/renderer/`: React screens, shared controls, and prototype-derived styles.
- `tests/`: focused unit, filesystem, media-integration, and desktop smoke tests.
- Root package/build configuration: development, checks, and arm64 packaging.
- `docs/IMPLEMENTATION_PLAN.md`: exact planned filenames and verification commands.
