# Inputs, local tools, and voice presets plan

Status: Implemented; final verification recorded in `docs/TASK_LOG.md`
Date: 2026-09-07

## Goal

Make first-run setup understandable in the packaged macOS app, accept manuscripts by drag and drop, and provide useful built-in voice presets plus user-created presets.

## Clarified current behavior

- The packaged app has a smaller environment than an interactive shell, so executables in a custom Homebrew prefix can be reported as missing.
- The backend already persists optional FFmpeg and FFprobe paths, but the Settings screen does not expose them.
- The llama.cpp executable and GGUF model fields appear only after selecting the llama.cpp provider.
- Ollama inference connects to its loopback HTTP server; the current diagnostic only searches for the `ollama` executable and can therefore be misleading.
- Manuscripts can be selected or pasted but cannot be dropped onto the import screen.
- Homer Studio lists all installed macOS voices and defaults to Samantha, but has no curated presets, preview action, or preset creation flow.

## Product interpretation

“Create a voice” will mean creating a reusable local voice preset with a name, installed macOS voice, and speaking rate. It will not clone a person’s recorded voice. Voice cloning would require a separate speech engine, model distribution, consent flow, and performance evaluation.

The empty item 4 is outside this plan until its requirement is supplied.

## Stage 1 — Local-tool setup that works in packaged apps

### Behavior

- Add visible path rows for FFmpeg, FFprobe, llama-cli, the GGUF model, and the optional Ollama CLI.
- Give every filesystem path a **Choose** button using the existing native file dialog.
- Keep text entry available for copying a path directly.
- Search the inherited `PATH` plus bounded common Apple Silicon locations: `/opt/homebrew/bin`, `/usr/local/bin`, `~/.local/bin`, `~/homebrew/bin`, and `~/local/homebrew/bin`.
- Show the detected full path beside each tool and add a **Use detected path** action when a saved override is absent or invalid.
- Diagnose Ollama inference by checking the configured loopback service, separately from whether the command-line executable is installed.
- Display an actionable state: found automatically, configured path works, configured path is invalid, server is reachable, or server is not running.
- Preserve existing settings through a backward-compatible settings migration.

### Files

- `src-tauri/src/services/settings.rs`: new Ollama executable override, bounded search locations, richer diagnostics, settings migration, and tests.
- `src-tauri/src/services/llm.rs`: reuse the loopback health check and keep inference independent of the CLI path.
- `src-tauri/src/commands/system.rs`: expose refreshed diagnostics if the richer shape requires it.
- `src/shared/contracts.ts`: settings and diagnostic contract updates.
- `src/renderer/src/native.ts`: generic executable/model chooser helpers.
- `src/renderer/src/pages.tsx`: path controls, choose/use-detected actions, and clearer Ollama status.
- `src/renderer/styles/app.css`: compact path-row and status styling.
- `docs/API_CONTRACTS.md`, `README.md`: document saved paths and Ollama server behavior.

### Verification

- Rust tests cover explicit overrides, custom Homebrew discovery, invalid paths, migration, and loopback-only Ollama checks.
- Renderer tests cover provider-dependent fields and applying detected paths.
- Launch the packaged app and confirm this Mac resolves all four executables under `/Users/vm/local/homebrew/bin`.

### Commit

`Make local tool setup reliable`

## Stage 2 — Manuscript drag and drop

### Behavior

- Turn the manuscript area on the Import screen into a clear drop zone while keeping paste and **Choose file**.
- Subscribe to Tauri 2 webview drag/drop events while the Import screen is mounted and remove the listener when it unmounts.
- Highlight the drop zone during hover.
- Accept one `.txt`, `.md`, or `.markdown` file and load it through the existing validated native `read_manuscript` command.
- Reject folders, multiple files, and unsupported extensions with an inline actionable message.
- Use the filename as the initial project title under the same rules as file selection.
- Browser preview keeps paste/file controls and does not pretend desktop paths are available.

### Files

- `src/renderer/src/native.ts`: typed dropped-manuscript loader reusing `read_manuscript`.
- `src/renderer/src/pages.tsx`: scoped drag/drop listener and import state.
- `src/renderer/styles/app.css`: accessible hover, active, and error states.
- `src/renderer/src/navigation.test.tsx` or a focused import test: supported and rejected drop payloads.
- `tests/DESKTOP_SMOKE.md`: manual packaged drag/drop check.

### Verification

- Renderer tests cover one valid file, wrong extension, multiple files, and listener cleanup.
- Packaged smoke check confirms a dropped Markdown manuscript populates the title and text without creating a project prematurely.

### Commit

`Add manuscript drag and drop`

## Stage 3 — Built-in and custom voice presets

### Data contract

- Add a settings-owned `voicePresets` list with stable ID, display name, macOS voice ID, speaking rate, and built-in/custom marker.
- Add `selectedVoicePresetId`; continue storing the effective `voiceId` and rate so existing generation remains compatible.
- Seed curated presets only when their underlying voices are installed. Initial candidates are Samantha, Daniel, and Karen with conservative narration rates.
- Migrate existing settings by creating a custom-compatible preset from the saved voice and rate when needed.

### Behavior

- Replace the unfiltered voice grid with a **Presets** section first and an **Installed voices** browser second.
- Let the user create a preset by naming it, selecting an installed voice, and setting words per minute.
- Let the user edit or delete custom presets; built-in presets remain resettable and cannot be deleted.
- Selecting a preset makes it the active narration configuration.
- Add a short **Preview** action that synthesizes disposable local audio using the selected voice/rate, exposes it through the existing registered audio protocol, and cleans old previews.
- Clearly label this as a macOS voice preset so the interface does not imply voice cloning.

### Files

- `src-tauri/src/services/settings.rs`: preset schema, defaults, migration, validation, and tests.
- `src-tauri/src/services/speech.rs`: preview generation and installed-voice validation.
- `src-tauri/src/commands/production.rs`, `src-tauri/src/lib.rs`: narrow preview command registration.
- `src/shared/contracts.ts`, `src/renderer/src/native.ts`: preset and preview contracts.
- `src/renderer/src/pages.tsx`: preset gallery, create/edit form, installed-voice browser, and preview controls.
- `src/renderer/styles/app.css`: preset editor and selected/disabled states.
- `docs/API_CONTRACTS.md`, `docs/DECISIONS.md`, `README.md`: persisted preset and preview behavior.

### Verification

- Rust tests cover preset seeding, migration, validation, unavailable installed voices, and preview cleanup.
- Renderer tests cover create, edit, select, and delete behavior.
- Generate a real preview with an installed macOS voice, then generate one chapter with the selected custom preset and probe the resulting M4A.

### Commit

`Add reusable voice presets`

## Final verification and package

Run:

```sh
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:integration
npm run package:mac
npm run test:smoke
```

Then manually verify tool selection, one dragged manuscript, preset creation, preview playback, chapter narration, and export in the arm64 packaged app. Update the compact architecture, contracts, decisions, and task log to the implemented state.

Final documentation/package commit: `Document the improved setup workflow`.
