# Implementation Plan: Portable Chatterbox runtime

## Goal

Make a packaged Homer Studio install usable on another Apple Silicon Mac without any path into the developer's checkout. Bundle the worker source, create its Python environment under the app's Application Support directory, and give the user visible setup and failure feedback.

## Existing behavior

`sound_workers.rs` falls back to a compile-time repository path; `start_local.py` assumes `workers/chatterbox/.venv`; the macOS bundle contains neither worker scripts nor the requirements file. Model checkpoints already live in Tauri app data and require explicit installation. Python 3.10 is required by the pinned Chatterbox package. The release smoke test does not inspect bundled workers.

## Proposed approach

Bundle the minimal Python worker files as Tauri resources. Resolve those resources and the app-data directory through Tauri at runtime. Keep the virtual environment, logs, and cache under app data. Add a cancellable runtime installation job in Settings that finds or accepts a Python 3.10 interpreter, creates the venv, and installs the pinned requirements. Launch Turbo (and Original when selected) from the packaged resources with the app-data venv. Do not copy or move the old checkout venv, because venvs contain absolute paths. Do not silently download packages at app startup. Show an actionable missing-runtime message and allow installation from Settings. Keep environment overrides for development and diagnostics only.

## File changes

- `src-tauri/tauri.conf.json` — modify: bundle worker launcher, protocol, Chatterbox servers, and requirements.
- `src-tauri/src/services/sound_workers.rs` — modify: resolve bundled resources, app-data runtime paths, interpreter discovery and installation, health/startup errors; replace checkout-only test.
- `src-tauri/src/lib.rs` — modify: pass app handle to startup and wire runtime commands.
- `src-tauri/src/commands/system.rs` — modify: runtime status and install commands using the existing job queue.
- `workers/start_local.py` — modify: derive venv, logs, and cache from app data; run from bundled source with dev fallback.
- `src/renderer/src/native.ts` and `src/renderer/src/pages.tsx` — modify: runtime status and setup controls in Settings.
- `scripts/verify-release.mjs` and `scripts/smoke-app.mjs` — modify: verify worker resources and test packaged startup without a checkout fallback.
- `workers/README.md`, `README.md`, and compact `docs/*` memory files — modify: describe portable setup, Python prerequisite, and current contracts.
- `package.json` and `src-tauri/tauri.conf.json` — modify: bump patch release version after validation.

## Implementation steps

1. Add resource mappings and app-data paths. Verify bundle content directly.
2. Add runtime setup/status job. Verify it creates a new venv outside the checkout, handles missing Python 3.10 and pip failure clearly, and can be cancelled.
3. Start workers using the installed app-data Python and bundled scripts. Verify restart and shutdown, including Original when selected.
4. Show status/progress in Settings, update setup documentation, and add targeted tests.
5. Build and smoke-test arm64 app/DMG outside the checkout, then commit, tag, and push the patch release.

## Verification

Run Python worker tests, TypeScript checks, Rust tests, web build, arm64 package validation, and packaged-app smoke test. Inspect the app bundle for all required scripts and requirements. Verify the environment path resolves under Application Support on a fresh home profile.

## Risks / edge cases

Python 3.10 is still a system prerequisite and can live in a nonstandard location; setup must accept an explicit executable path. Pip requires network access and can install several gigabytes; setup must be user-initiated, cancellable, and report errors. A moved venv is invalid and must be recreated. Existing user-installed checkpoints remain in place.

## Out of scope

Bundling an entire Python interpreter/Torch distribution, Linux packaging, remote GPU workers, and changing model checkpoints or speech behavior.
