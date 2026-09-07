# Module ownership

Status: accepted boundaries for the authorized implementation.

| Module | Owns | May depend on |
| --- | --- | --- |
| Renderer | Screens, editing state, playback controls, progress/errors | Shared contracts and explicit Tauri commands |
| Tauri application | App lifecycle, capabilities, command validation, service coordination | Rust domain services and shared serialized contracts |
| Project/configuration services | Validated JSON, safe paths, atomic writes, recovery | Rust standard library, Serde schemas |
| Job/process services | Sequential execution, cancellation, bounded logs | Rust process APIs and shared job types |
| LLM providers | Optional text transformation through GGUF or Ollama | Process runner or local HTTP; shared provider contract |
| Speech/audio services | Speech generation, import, probing, normalization, concat | Process runner, project paths, shared audio types |
| Shared domain | Serializable types, validation, ordering, timestamp formatting | Pure TypeScript and schema validation only |
| Build/release | Reproducible dependencies, arm64 packages, CI | Root build configuration and scripts |
| Tests | Domain invariants and service-boundary checks | Relevant modules and small generated fixtures |

Rules:
- Renderer never receives arbitrary filesystem or process access.
- Tauri capabilities expose only named commands needed by the product.
- Shared TypeScript code never imports renderer code or native implementations.
- Speech generation and text LLM processing have separate interfaces.
- Services report structured results; the renderer displays them without inventing progress.
- The Rust application serializes writes and rejects stale/concurrent project changes.
- No shared-framework extraction from the absent reference application.
