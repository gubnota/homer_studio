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
