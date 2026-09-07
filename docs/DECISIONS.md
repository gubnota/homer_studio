# ADR-0001: Preserve the supplied UI as the product reference

Date: 2026-09-07
Status: Accepted

Context: The user supplied a downloaded UI prototype and requires its visual structure to be preserved.
Decision: Use `audio_studio_ui` as the primary UI/UX reference; inspect only relevant assets and maintain compact project context.
Consequences: Preserve screens and styling; simulated functionality must be replaced with real local operations.
Related files: `audio_studio_ui/index-BeQ8ttIE.js`, `audio_studio_ui/index-D-2jFf5J.css`, `docs/PROJECT_CONTEXT.md`.

# ADR-0002: Electron shell with prototype-derived React source

Date: 2026-09-07
Status: Accepted

Context: Only compiled React assets exist; there is no reusable native application or build infrastructure.
Decision: Use Electron, React, and TypeScript; recover screen structure into maintainable source and reuse the supplied CSS. Remove hosted-platform dependencies from the runtime.
Consequences: One application language and direct local filesystem/subprocess support; a larger desktop runtime than a system-webview shell. The compiled bundle is a reference, not the production extension point.
Related files: `docs/IMPLEMENTATION_PLAN.md`.

# ADR-0003: Filesystem projects and replaceable local engines

Date: 2026-09-07
Status: Accepted

Context: The requested workflow is local-first and needs interchangeable text models, durable chapters, audio assembly, and measured timestamps.
Decision: Use versioned JSON/files, llama.cpp and Ollama text adapters, a separate speech interface, installed macOS voices, imported audio, and local FFmpeg/FFprobe executables.
Consequences: No database or hosted backend; external executable availability and schema validation need explicit handling.
Related files: `docs/API_CONTRACTS.md`, `docs/IMPLEMENTATION_PLAN.md`.
