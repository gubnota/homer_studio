# Module ownership

Status: accepted boundaries for the authorized implementation.

| Module | Owns | May depend on |
| --- | --- | --- |
| Renderer | Screens, editing state, playback controls, progress/errors | Shared contracts and explicit platform transport |
| Tauri application | App lifecycle, capabilities, command validation, service coordination | Shared Rust core and serialized contracts |
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
- Voice Lab recordings and converted clips belong to the standalone sound store; chapter recordings and replacement takes belong to the project store.
- Review deletes generated chapter audio only; Exports deletes project-owned export pairs; standalone clip deletion affects app-owned sound assets only.
- Standalone sounds never require or mutate a book project. The renderer uses narrow Tauri commands; only Rust talks to local workers.
- The Tauri app owns the session lifecycle for configured local Chatterbox workers. It verifies the engine before shutdown; Ollama and unrelated loopback services remain externally managed.
- Tool discovery is bounded to inherited PATH and documented system/user installation prefixes; the renderer can only select explicit files.
- Services report structured results; the renderer displays them without inventing progress.
- The Rust application serializes writes and rejects stale/concurrent project changes.
- No shared-framework extraction from the absent reference application.

- `model_install.rs` owns pinned, allow-listed download paths. Workers load installed checkpoints and never initiate downloads.
- `worker_runtime.rs` owns the app-data Python environment and bundled worker paths; the launcher must not depend on a checkout or mutate its source directory.
- `project_store.rs` owns non-destructive take selection and revision checks; UI recording never writes project files directly.

## Voice production boundaries
- Audio asset/store services own immutable source files and revisioned session metadata. Renderer supplies IDs and intents, never asset paths for playback.
- `audio_edits` owns selection math, crossfades, measured rendering and cue remapping; originals are never rewritten.
- Native capture owns microphone permission, stream, bounded disk queue and meters. One active capture is shared across views; normal app quit finalizes capture.
- Waveform service owns streamed decoding and persistent min/max peaks. Renderer requests bounded time windows.
- Processor service owns per-engine environments, bounded protocol parsing, deadlines and process-group cancellation. Python owns inference only; it never updates project or memo manifests.
- Audio commands serialize publication through the shared queue and recheck revisions under the project write lock. Existing project/sound/voice stores retain final publication ownership.
- Profile sidecars reference existing voice IDs and accepted memos; Chatterbox sample storage and limits remain authoritative.
- Shared editor/transport components own interaction and presentation; they depend on typed bridges and shared contracts, not native filesystem/process APIs.

## Wave Studio boundaries
- Shared pure edits own timeline transformations and timing remaps; they have no filesystem, process or model access.
- Renderer provider owns navigation-safe history and save scheduling; canvas owns bounded viewport visualization, playback owns bounded preview scheduling.
- Native stores validate revisions and owned source identities; native render owns FFmpeg processing, cancellation, tempo variants and verified exports.
- Voice library owns canonical names, colors, notes and provider metadata. Timeline annotations refer to voice IDs and never trigger conversion or model installation.
- SFX library owns reusable assets; projects own gain/fade/trim placements. Removing a placement never alters the original. Legacy project and memo schemas remain independent.

- Wave processing owns isolated peak measurement, lossless Join, owned generated sources and Original conversion with duration fitting. Renderer owns explicit acceptance, original timeline restoration and project/view save draining. Capture retains memo assets before Wave import.

- Wave native storage owns portable media validation and recoverable manifest deletion. Video service owns silent reference transcoding; renderer synchronizes frames to the audio clock. Shared timeline helpers own voice completion identity; acceptance records completion after replacement.

## Server and shared core
- `homer-core` owns domain commands/services and resource bounds. Desktop and HTTP adapters depend on it; core server builds must not require Tauri or CoreAudio.
- Tauri owns native microphone/device access, menus, OS file-open handling and desktop audio URLs.
- Axum owns single-owner authentication, same-origin checks, scoped paths, streamed transfers and browser recording staging. It exposes an explicit command allowlist.
- Browser renderer owns microphone permission and capture; chunks go directly to server storage. Only core services execute media tools or workers.
- Release tooling owns macOS arm64 and Linux x86_64 artifacts. Real GPU validation requires a supplied Linux CUDA host.
