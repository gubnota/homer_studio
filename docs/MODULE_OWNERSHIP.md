# Module ownership

Status: accepted boundaries for the authorized implementation.

| Module | Owns | May depend on |
| --- | --- | --- |
| Renderer | Screens, editing state, playback controls, progress/errors | Shared contracts and explicit Tauri commands |
| Tauri application | App lifecycle, capabilities, command validation, service coordination | Rust domain services and shared serialized contracts |
| Project/configuration services | Validated JSON, safe paths, atomic writes, recovery, tool diagnostics, and legacy voice-setting migration | Rust standard library, Serde schemas, bounded process discovery |
| Job/process services | Sequential execution, cancellation, bounded logs | Rust process APIs and shared job types |
| LLM providers | Optional text transformation through GGUF or Ollama | Process runner or local HTTP; shared provider contract |
| Speech/audio services | Neural voice reference storage and preview synthesis, Markdown speech normalization, per-section takes, recording conversion, import, probing, and chapter assembly | Process runner, project paths, shared audio types |
| Sound workers | Local model loading and prompt-to-WAV generation | Shared worker protocol; explicit local checkpoint folders only |
| Standalone sound services | Worker access, WAV validation/conversion, independent clip manifest and export | Job/process services, FFmpeg/FFprobe, loopback worker protocol; never project manifests |
| Shared domain | Serializable types, validation, ordering, timestamp formatting | Pure TypeScript and schema validation only |
| Build/release | Reproducible dependencies, arm64 packages, CI | Root build configuration and scripts |
| Tests | Domain invariants and service-boundary checks | Relevant modules and small generated fixtures |

Rules:
- Renderer never receives arbitrary filesystem or process access.
- Tauri capabilities expose only named commands needed by the product.
- Shared TypeScript code never imports renderer code or native implementations.
- Speech generation and text LLM processing have separate interfaces.
- The voice store owns app-data voice identities and reference samples. Speech generation reads the selected sample and sends it through the worker client. The renderer never passes worker filesystem paths.
- Standalone sounds never require or mutate a book project. The renderer uses narrow Tauri commands; only Rust talks to local workers.
- Tool discovery is bounded to inherited PATH and documented system/user installation prefixes; the renderer can only select explicit files.
- Services report structured results; the renderer displays them without inventing progress.
- The Rust application serializes writes and rejects stale/concurrent project changes.
- No shared-framework extraction from the absent reference application.

- `model_install.rs` owns pinned, allow-listed download paths. Workers load installed checkpoints and never initiate downloads.
- `project_store.rs` owns non-destructive take selection and revision checks; UI recording never writes project files directly.
