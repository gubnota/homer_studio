# Module ownership

Status: accepted boundaries for the authorized implementation.

| Module | Owns | May depend on |
| --- | --- | --- |
| Renderer | Screens, editing state, playback controls, progress/errors | Shared contracts and preload API |
| Preload | Explicit typed IPC methods and event subscription cleanup | Electron bridge APIs and shared types |
| Main process | App lifecycle, native dialogs, IPC validation, service coordination | Domain services and shared contracts |
| Project/configuration services | Validated JSON, safe paths, atomic writes, recovery | Node filesystem, shared schemas |
| Job/process services | Sequential execution, cancellation, bounded logs | Node subprocess APIs and shared job types |
| LLM providers | Optional text transformation through GGUF or Ollama | Process runner or local HTTP; shared provider contract |
| Speech/audio services | Speech generation, import, probing, normalization, concat | Process runner, project paths, shared audio types |
| Shared domain | Serializable types, validation, ordering, timestamp formatting | Pure TypeScript and schema validation only |
| Build/release | Reproducible dependencies, arm64 packages, CI | Root build configuration and scripts |
| Tests | Domain invariants and service-boundary checks | Relevant modules and small generated fixtures |

Rules:
- Renderer never imports Node filesystem, subprocess APIs, or main-process services.
- Preload never exposes raw IPC, arbitrary paths, or command execution.
- Shared code never imports Electron, renderer code, or native implementations.
- Speech generation and text LLM processing have separate interfaces.
- Services report structured results; the renderer displays them without inventing progress.
- The main process serializes writes and rejects stale/concurrent project changes.
- No shared-framework extraction from the absent reference application.
