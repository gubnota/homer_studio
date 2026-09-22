# ADR-0001: Preserve the supplied UI as the product reference

Date: 2026-09-07
Status: Accepted

Context: The user supplied a downloaded UI prototype and requires its visual structure to be preserved.
Decision: Use `audio_studio_ui` as the primary UI/UX reference; inspect only relevant assets and maintain compact project context.
Consequences: Preserve screens and styling; simulated functionality must be replaced with real local operations.
Related files: `audio_studio_ui/index-BeQ8ttIE.js`, `audio_studio_ui/index-D-2jFf5J.css`, `docs/PROJECT_CONTEXT.md`.

# ADR-0002: Tauri shell with prototype-derived React source

Date: 2026-09-07
Status: Accepted

Context: Only compiled React assets exist; the app will run beside memory-intensive local audio and language models.
Decision: Use Tauri 2, Rust, React, and TypeScript; recover screen structure into maintainable source and reuse the supplied CSS. Keep native capabilities behind explicit commands and remove hosted-platform dependencies.
Consequences: The packaged shell is small and uses macOS WebKit, while native filesystem/process work is implemented and tested in Rust. The compiled bundle remains a reference, not the production extension point.
Related files: `src-tauri/`, `src/renderer/`, `docs/IMPLEMENTATION_PLAN.md`.

# ADR-0003: Filesystem projects and replaceable local engines

Date: 2026-09-07
Status: Accepted

Context: The requested workflow is local-first and needs interchangeable text models, durable chapters, audio assembly, and measured timestamps.
Decision: Use versioned JSON/files, llama.cpp and Ollama text adapters, a separate speech interface, installed macOS voices, imported audio, and local FFmpeg/FFprobe executables.
Consequences: No database or hosted backend; external executable availability and schema validation need explicit handling.
Related files: `docs/API_CONTRACTS.md`, `docs/IMPLEMENTATION_PLAN.md`.

# ADR-0004: Verified Apple Silicon development packages

Date: 2026-09-07
Status: Accepted

Context: The first release must run locally on Apple Silicon without requiring paid Apple distribution credentials.
Decision: Build arm64 app, DMG, and ZIP artifacts, apply Tauri's ad-hoc signature, and verify the executable architecture, signature, media integration, and packaged startup. Repeat these checks on GitHub's macOS arm64 runner and publish only version-matching tags.
Consequences: Local test packages are reproducible and reviewable. Public distribution can add Developer ID signing and notarization without changing the application architecture.
Related files: `scripts/verify-release.mjs`, `scripts/smoke-app.mjs`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`.

# ADR-0005: Bounded local tool discovery with explicit overrides

Date: 2026-09-07
Status: Accepted

Context: A packaged macOS app does not inherit the same shell PATH as Terminal, so correctly installed local tools can appear missing.
Decision: Search inherited PATH plus a fixed list of system, Homebrew, and user Homebrew bin directories. Keep explicit file paths in Settings and report configured, detected, invalid, and service-reachability states separately.
Consequences: Common Apple Silicon installations work without shell configuration, custom layouts remain selectable, and executable discovery never becomes an unbounded filesystem search.
Related files: `src-tauri/src/services/process_runner.rs`, `src-tauri/src/services/settings.rs`, `src/renderer/src/pages.tsx`.

# ADR-0006: Reusable macOS voice presets

Date: 2026-09-07
Status: Accepted

Context: A flat installed-voice list does not preserve useful narration speed choices and can imply broader voice creation than the local engine supports.
Decision: Store named presets containing an installed macOS voice ID and speaking rate. Seed curated presets only for installed voices, allow custom presets, and synthesize previews through the production speech pipeline.
Consequences: Existing speech settings remain the effective generation contract, old settings migrate without a schema-version break, and the product clearly represents presets rather than voice cloning.
Related files: `src-tauri/src/services/settings.rs`, `src-tauri/src/services/speech.rs`, `src/renderer/src/voice-presets.ts`, `src/renderer/src/pages.tsx`.

# ADR-0007: Independent prompt-to-audio library and local workers

Date: 2026-09-22
Status: Accepted

Context: Authors need short sounds from prompts without a book. Chatterbox handles speech and documented vocal tags but is not a general fabric/ambience engine; the future Linux GPU deployment should share a stable boundary.
Decision: Keep a separate versioned sound-asset manifest in app data. Use Chatterbox Turbo for speech/tags and AudioLDM 2 or Stable Audio Open for effects through a versioned loopback job API. Rust validates worker output and publishes a 48 kHz WAV master plus M4A preview. Model files and Python environments are supplied explicitly; no automatic downloads. Use a bounded standard-library HTTP client because the intended HTTP crate was unavailable in the offline Cargo cache.
Consequences: No manuscript dependency or new Rust dependency. Workers must be installed and started separately. AudioLDM 2 is publicly downloadable but noncommercial; Stable Audio Open requires checkpoint access. The Mac app accepts only loopback URLs; a future private Linux endpoint needs explicit authentication and transport design. Model quality still needs human listening checks.
Related files: `workers/`, `src-tauri/src/services/sound_workers.rs`, `src-tauri/src/services/sound_render.rs`, `src-tauri/src/services/sound_store.rs`, `docs/API_CONTRACTS.md`.
