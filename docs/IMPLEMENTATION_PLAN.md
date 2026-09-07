# Implementation Plan: Apple Silicon Audio Studio

Status: proposed; implementation is not authorized.
Date: 2026-09-07
Repository: https://github.com/gubnota/homer_studio.git

## Goal

Turn the supplied Audio Studio prototype into a practical local-first macOS arm64 desktop application: import text, organize and edit chapters, optionally transform text with a local LLM, generate or import audio, review playback, combine chapters, and export measured YouTube timestamps. Preserve the prototype's visual structure. Deliver reproducible development commands, focused tests, and tag-based GitHub packages.

## Existing behavior

- The supplied workspace contained `.gitignore` and `audio_studio_ui/`; the six project-context documents were missing and have now been created. Final verification found local Git initialized on `main` with no commits and the supplied GitHub URL as `origin`.
- The user supplied `gubnota/homer_studio` as the GitHub repository. Read-only `git ls-remote` succeeded but returned no HEAD, branches, or tags. No reference implementation is available there as inspected on this date.
- `audio_studio_ui/index-BeQ8ttIE.js` embeds React 18.3.1, routing, component markup, sample project/segment data, and simulated operations. Import and regeneration use timers; playback moves a synthetic progress bar; queue/settings changes are in memory.
- `audio_studio_ui/index-D-2jFf5J.css` contains usable compiled utility styles and theme variables. The interface uses a 240px sidebar, white panels, neutral borders, compact text, project cards, chapter navigation, segment rows, a 320px inspector, and bottom playback controls.
- `index.html`, `editor.html`, `settings.html`, and the other HTML files are identical SPA entry documents. They are not original screen source files. No package manifest, lockfile, source map, tests, native shell, or build configuration is supplied.
- HTML and JavaScript include Base44 authentication, analytics, badge, and remote branding dependencies. The downloaded app cannot simply become a reliable offline desktop app by wrapping its entry page.
- The prototype advertises Qwen3-TTS, CosyVoice, voice cloning, emotion controls, automatic transcription, and several export formats. These are demonstrations, not established backend contracts.
- Local diagnostics found an arm64 host and Node/npm, FFmpeg/FFprobe, llama-cli, and Ollama on PATH. Availability does not establish model readiness or portability to another Mac.

### Reuse map

| Supplied reference | Concrete reuse |
| --- | --- |
| `audio_studio_ui/index-D-2jFf5J.css` | Copy to tracked `src/renderer/styles/prototype.css`; preserve its visual baseline. |
| `index-BeQ8ttIE.js`: `Qm`, `th`, `eh` | Recover sidebar, application layout, and player markup into named React components. Replace simulated playback. |
| Bundle: `rh`, `ah`, `ch` | Recover Projects, Import, and project overview structure and labels. Replace sample data. |
| Bundle: `hh`, `dh`, `ph`, `vh` | Recover editor, segment rows, inspector, and review layout. Replace timer-based actions with project/job operations. |
| Bundle: `xh`, `Sh`, `Ch`, `Th` | Recover Voices, Queue, Exports, and Settings layouts; show actual engine capabilities. |
| `audio_studio_ui/urls.txt` | Preserve screen destinations and project-scoped navigation. |
| `homer_studio` reference application | No code or patterns available to reuse; do not claim otherwise. |

Keep `audio_studio_ui/` untouched and excluded as today. The tracked application must build without this ignored directory after recovery. Do not ship the compiled JS bundle, hosted HTML wrappers, badge script, authentication, analytics, sample manuscripts, or remote image dependencies. Reconstruct the application-specific React markup into maintainable source; this is source recovery using the supplied design, not a new visual design.

## Proposed approach

### 1. Framework and first-version scope

Use **Electron + React + TypeScript**, with electron-vite for main/preload/renderer builds and electron-builder for macOS packages. This fits the observed React prototype and lets filesystem, subprocess, and orchestration code use the same language. Tauri would add a Rust toolchain without existing Rust code to reuse; SwiftUI would require rebuilding the interface in a different UI framework. Accept Electron's larger runtime for this first version. electron-vite supports a single configuration for the three application entry points. [Build documentation](https://electron-vite.org/guide/)

Use npm with a committed lockfile, a supported Node 22 release at least 22.12, strict TypeScript, Vitest, and Playwright's Electron support for a narrow smoke test. Choose mutually compatible maintained dependency versions when implementing and lock them; the prototype's bundled React version is evidence of its origin, not a requirement to retain an obsolete dependency.

**Speech scope proposal:** use installed macOS voices through `/usr/bin/say`, plus chapter-audio import. This provides real local speech without adding a Python/model-serving runtime. It is separate from text LLM processing. The user has been asked whether Qwen3-TTS must instead be included; this document proposes the macOS baseline and does not record that as an answered preference. Approval of this version accepts this baseline. If Qwen3-TTS is requested, revise its engine/setup/testing sections before implementation.

Preserve all primary screens. Make chapter import/edit/order, segment regeneration, manual review, installed-voice selection, queue cancellation, settings, playback, M4A export, and timestamp copying functional. Retain unsupported feature labels in their existing locations with a clear unavailable explanation: cloning, synthetic voice design, emotion conditioning, automatic speech recognition, advanced timing/crossfades, and additional export formats. Do not display fabricated success, issues, waveforms, disk usage, or model availability.

### 2. Native boundary and filesystem

Use one application window and one active project. The main process owns native dialogs, files, settings, jobs, and subprocesses. The renderer accesses a narrow typed `window.audioStudio` preload API; never expose raw IPC or an arbitrary shell/filesystem API. Enable context isolation and renderer sandboxing, disable Node integration, validate IPC sender and payloads, and load packaged UI through a restricted `app://` protocol with a CSP. In development, allow only the configured loopback Vite origin. [Electron isolation](https://www.electronjs.org/docs/latest/tutorial/context-isolation) and [security guidance](https://www.electronjs.org/docs/latest/tutorial/security)

Native file dialogs return selected inputs to main-process services. Renderer requests address project/chapter/segment IDs rather than arbitrary output paths. Canonicalize paths and reject traversal and symlink escapes. Serve playback through an `audio://` protocol accepting project-owned asset IDs, supporting MIME types and byte ranges; do not read whole audiobooks into the renderer or expose arbitrary local files.

Store settings and the recent-project list beneath `app.getPath('userData')`, and operational logs beneath `app.getPath('logs')`. Projects live in user-selected writable directories, not inside the installed app. Write settings/manifests using same-directory temporary files and rename; serialize writes, retain a last-good manifest, and use a small recovery journal for multi-file edits/exports. Reject concurrent edits during generation/export and verify input revision before committing a job result. Use Electron's single-instance lock.

Proposed readable project layout:

```text
MyAudioProject/
  project.json
  project.json.bak
  chapters/
    <stable-chapter-id>/
      source.txt
      processed.txt                 # only after explicitly accepted LLM processing
      audio.m4a                     # currently selected chapter audio
      segments/<segment-id>/
        <take-id>.wav               # retained generated takes
  output/
    full-audio.m4a
    youtube-chapters.txt
  .work/<job-id>/                   # explicit temporary job files, cleaned after completion
```

IDs stay stable on reorder; the manifest's chapter array is the canonical order and the UI derives displayed numbers. Audio imported from outside the project is copied into it, so later movement of the source file cannot break playback.

### 3. Data and IPC contracts

Define runtime-validated schemas in `src/shared/contracts.ts`, with one schema library rather than parallel handwritten validators.

| Contract | Required content / behavior |
| --- | --- |
| `ProjectV1` | `schemaVersion: 1`, ID, title, language, timestamps, monotonic revision, ordered chapters, export history. |
| `Chapter` | Stable ID, title, source/processed relative paths, ordered segments, selected audio metadata or null, content revision, `draft/ready/stale/failed` audio status, structured error or null. Source text is hydrated from its file for UI responses. |
| `Segment` | ID, editable text, narrator/voice ID and rate, literal pronunciation overrides, selected take ID, takes, manual review state. Segmentation revisions invalidate affected takes/chapter audio. |
| `AudioAsset` | Relative path, generation/import origin, precise duration in integer microseconds, codec, sample rate, channels/layout, time base, and relevant probe metadata. Use integer ticks/rational time bases internally when deriving durations. |
| `SettingsV1` | LLM discriminated union: `none`, `llama_cpp {executablePath, modelPath, contextSize, maxTokens, gpuLayers}`, or `ollama {baseUrl, model, contextSize, maxTokens}`; optional model directory; speech `{provider: macos_say, voiceId, rate}`; FFmpeg/FFprobe overrides; export settings. |
| `Job` | ID, kind, project/chapter/segment IDs, input revision, `queued/running/paused/completed/failed/cancelled/interrupted`, completed/total units, current stage, error. No invented percentage for indeterminate work. |
| `Result<T>` | `{ok:true,value:T}` or `{ok:false,error:{code,message,chapterId?,jobId?}}`; diagnostic details go to bounded logs. |

Preload methods and main-process handlers:

- `projects.create({title,language,chapters})`, `projects.open()`, `projects.recent()`, `projects.read()`: creation/opening uses native directory/manifest dialogs; read returns the active hydrated snapshot.
- `projects.update({expectedRevision,change})`: validated changes for title, chapter title/order, segment text/voice/pronunciation/take, and manual review. No opaque arbitrary patch object.
- `imports.readText()`: native TXT/Markdown picker returning decoded text; pasted text uses the same pure parser. `audio.import({chapterId})`: native audio picker and copy/probe job.
- `settings.read()`, `settings.update({settings})`, `settings.pickPath({kind})`, `settings.check()`, `models.discover()`, `voices.list()`: constrained picker kinds, configuration validation, executable/model diagnostics, bounded optional GGUF discovery, installed voice enumeration.
- `jobs.start({kind,targetIds,options})`, `jobs.list()`, `jobs.control({jobId,action})`: kinds are text processing, segment/chapter generation, and export; actions are cancel, retry, pause, resume. Text processing returns a candidate preview; only a separate validated accept change updates project text.
- `audio.url({assetId})`, `exports.readTimestamps({exportId})`, `exports.reveal({exportId})`, `clipboard.copyTimestamps({exportId})`: validated project-owned playback/output access.
- `onJobUpdated(handler)` and `onProjectUpdated(handler)` return unsubscribe functions; events contain serializable snapshots/revisions, not Electron event objects. This is direct IPC progress reporting, not an application-wide event-bus abstraction.

Error codes: `INVALID_PROJECT`, `UNSUPPORTED_SCHEMA`, `PERMISSION_DENIED`, `WRITE_FAILED`, `REVISION_CONFLICT`, `BUSY`, `MISSING_FILE`, `MODEL_NOT_FOUND`, `MODEL_LOAD_FAILED`, `MODEL_UNAVAILABLE`, `CONTEXT_LIMIT`, `TOOL_NOT_FOUND`, `UNSUPPORTED_TOOL_VERSION`, `INVALID_AUDIO`, `GENERATION_FAILED`, `CONCAT_FAILED`, `CANCELLED`, and `INTERRUPTED`. No automatic skipping of failed or stale chapters.

No previous supported project format exists to migrate. Reject future schema versions rather than overwriting them; validate malformed JSON, duplicate IDs, broken paths, nonfinite durations, and missing assets on open. Do not import prototype demo objects as user projects.

### 4. Chapters, local text models, and speech

Import UTF-8 TXT/Markdown or pasted text. Let the user preview chapter boundaries and choose the Markdown heading level; plain text defaults to one chapter with optional explicit chapter markers. Preserve preambles, Unicode, and original text. Segment chapter text deterministically by paragraphs/sentence boundaries with a configurable size; store stable segment IDs and avoid silent resegmentation of reviewed content.

Define a small `TextProvider` interface: `validate(config)` and `process({instruction,text,signal,onProgress}) -> {text,provider,model}`. Providers never mutate project files. LLM processing is opt-in and produces an editable preview; source text and approved audio survive cancellation, empty output, context overflow, or rejected suggestions.

- **llama.cpp/GGUF:** select an existing `.gguf` and executable independently. Run a bounded, noninteractive `llama-cli` child process with a UTF-8 prompt file, model path, token/context limits, and configurable GPU layers. Check supported CLI flags before execution; parse answer output without exposing prompt echoes or diagnostic text as manuscript content. Exercise noninteractive termination in a real optional smoke test. Apple Silicon/Metal acceleration is supported by the upstream runtime; show actual load errors rather than promising acceleration. [llama.cpp](https://github.com/ggml-org/llama.cpp)
- **Ollama:** call the user's existing loopback installation from the main process using `POST /api/generate`; configure model and base URL, handle HTTP errors and incremental responses, and abort the request on cancel. Do not start/stop the user's Ollama daemon. Validate loopback URLs, reject off-host redirects, and bound request/output sizes. [Ollama API](https://docs.ollama.com/api/generate)
- Process bounded segments sequentially; reject over-budget inputs with an actionable split/reduce-context message instead of silent truncation. Preserve ordered candidate outputs and do not apply partial processing automatically.
- Optional model discovery scans a configured directory to a bounded depth/count for GGUF files; it does not follow symlinks or download weights. Ollama model discovery uses its installed-model endpoint.
- **MLX evaluation:** defer. It is an additional Python package/runtime and model format here, while GGUF and Ollama already satisfy the text-provider requirement. There is no measured repository-specific gain to justify a third engine in v1. [MLX LM](https://github.com/ml-explore/mlx-lm)

Define a separate `SpeechProvider` with `listVoices()` and `generate({text,voiceId,rate,outputPath,signal})`. Initial implementation invokes `/usr/bin/say` using a text file and argument array, then FFmpeg converts the resulting audio to canonical PCM segment takes and chapter M4A. Only installed voices are selectable; absent language/voice resources produce a useful message. Literal pronunciation overrides affect spoken text without changing displayed source. No emotion/clone quality claims.

Use one global heavy-job queue. Pause stops after the current segment/chapter boundary; resume continues remaining units. Cancellation aborts the active process group/request, escalates termination after a short grace period, clears uncommitted output, and preserves last-good takes/audio. Never terminate unrelated model daemons. Persist job state after each completed unit; after restart classify unfinished work as interrupted and offer explicit retry.

### 5. Audio, concatenation, and timestamps

Resolve FFmpeg/FFprobe and llama-cli in this order: explicit validated setting, process PATH, standard Homebrew locations (`/opt/homebrew/bin`, `/usr/local/bin`). Never run a login shell to discover tools. Store user-selected locations rather than assuming this development machine's custom Homebrew path exists elsewhere.

**Dependency policy:** use system-installed FFmpeg/FFprobe in both development and initial releases. Document `brew install ffmpeg`, native executable pickers, and diagnostics. This avoids binary redistribution, linked-library, and signing work; it means the packaged app requires this additional installation. Bundle neither models nor inference/FFmpeg binaries in v1.

Run FFprobe with JSON output and inspect the actual selected audio stream. Reject zero-duration, unreadable, missing, encrypted, or unsupported audio; choose a single supported audio stream and drop cover/video streams. All media commands use `spawn(executable,args,{shell:false})`, bounded stderr/progress, timeouts, and cancellation.

For export:

1. Snapshot the ordered chapter list and revision; require every chapter's selected audio to exist, be readable, ready, and nonstale. Re-probe at export time and do not rely solely on cached durations.
2. Use concat-demuxer stream copy only when the selected codecs, codec configuration, sample rates, channel layouts, time bases, and output container are compatible and no requested transform requires encoding. Generate the concat manifest using safe internal staged filenames; spaces/quotes/Unicode in user-selected paths must not alter syntax. FFmpeg requires compatible streams and accurate durations for concat. [Concat documentation](https://ffmpeg.org/ffmpeg-formats.html#concat)
3. On incompatible inputs, or a failed/invalid copy result, decode/resample each chapter into canonical 48 kHz stereo PCM in `.work/<job-id>/`, measure it, concatenate PCM, and encode the final AAC/M4A once. Avoid separately encoding every normalized chapter to AAC and accumulating encoder padding. If the user explicitly enables loudness normalization, take this encoding path; default it off to retain stream-copy eligibility. Crossfades remain zero in v1.
4. Derive boundaries from the actual inputs used in the chosen path, including packet start/skip-padding metadata for compressed copy. Sum exact durations before converting each cumulative start to whole seconds. If copy timing cannot be established reliably or fails the final duration/boundary validation, retry through PCM; never repair chapter drift by proportionally scaling timestamps.
5. Probe the final file and compare the expected timeline to its audible duration using an explicit codec-frame tolerance. Test boundary markers as well as total duration. Do not publish an unverifiable result as successful.
6. Write `full-audio.m4a` and `youtube-chapters.txt` to staging. Commit the pair and export manifest together using the recovery journal; preserve the previous export on any failure. Record the source revision so edits/reorders make old exports visibly out of date.

Timestamp formatting floors only the cumulative boundary, not each chapter's duration. Start with `00:00`; use `MM:SS` below one hour and `H:MM:SS` thereafter. Example durations 272s, 378s, 235s produce starts `00:00`, `04:32`, `10:50`. Replace title newlines with spaces and use `Chapter N` only for a blank title. Copy the saved text from Exports.

Show a nonblocking eligibility message if there are fewer than three chapters, duplicate formatted starts, or chapters shorter than ten seconds. Keep correct timestamps and audio export available, but do not claim YouTube will activate chapters for those inputs. [YouTube requirements](https://support.google.com/youtube/answer/9884579?hl=en)

### 6. Packaging and GitHub CI

Package a macOS arm64 `.app`, `.dmg`, and `.zip` using electron-builder. Default initial version is `0.1.0`; configure development artifacts for ad-hoc signing without Developer ID credentials. Build a local, self-contained renderer; package only built code/runtime dependencies, not prototype files, manuscripts, models, or test fixtures. [macOS packaging](https://www.electron.build/mac/)

Use `macos-15` and assert `uname -m` is `arm64` in both workflows. GitHub currently documents this label as Apple Silicon; the assertion catches runner changes. Pin actions to reviewed revisions during implementation. [GitHub runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)

- Normal pushes and pull requests: checkout, supported Node setup/cache, `npm ci`, type checks, unit tests, FFmpeg integration tests, production build, desktop smoke test, and unpacked arm64 packaging check. Do not upload large packages on every commit.
- Tags `v*`: require a valid `vMAJOR.MINOR.PATCH` matching `package.json`, run the same checks, build arm64 DMG/ZIP, verify architecture and ad-hoc signature, and upload CI artifacts. Attach packages to a GitHub Release for the tag with minimal `contents: write` permission limited to the release job.
- Developers release approximately with `git tag v0.1.0` and `git push origin v0.1.0` after updating the package version/lockfile and pushing the approved commit. No tags, remote writes, or releases occur during planning.
- Put reusable version/architecture/package validation in one script; npm scripts remain the public commands. No parallel GitLab workflow is needed for this GitHub-only repository.
- README distinguishes local/ad-hoc testing from future production distribution: Developer ID, hardened runtime, signing, notarization, stapling, and CI secrets. Do not promise notarized-install behavior for initial artifacts.

## File changes

Paths below are relative to the workspace root. These are planned source/configuration changes, not files created by this planning task. No implementation files will be deleted.

| File | Action | Responsibility and concrete changes |
| --- | --- | --- |
| `package.json` | Create | Runtime/dev dependencies; `dev`, `typecheck`, `test`, `test:integration`, `test:smoke`, `build`, `pack:mac`, `package:mac` scripts; version and main entry. |
| `package-lock.json` | Create | Reproducible npm dependency resolution. |
| `tsconfig.json` | Create | Strict shared TypeScript settings and references to native/renderer configs. |
| `tsconfig.node.json` | Create | Main/preload/shared/build/test type-check scope. |
| `tsconfig.web.json` | Create | Renderer/shared DOM and JSX type-check scope. |
| `electron.vite.config.ts` | Create | Main, sandbox-compatible bundled CommonJS preload, React renderer, and local assets. |
| `electron-builder.yml` | Create | Product metadata, arm64 DMG/ZIP, ad-hoc signing, output and packaged-file allowlist. |
| `vitest.config.ts` | Create | Unit/integration grouping and timeouts without model downloads. |
| `playwright.config.ts` | Create | One-worker Electron smoke test configuration. |
| `.gitignore` | Modify | Preserve existing prototype exclusion; ignore dependencies, builds, packages, logs, test results, and temporary files. |
| `src/shared/contracts.ts` | Create | Versioned schemas, IPC request/result types, capabilities, structured errors, project/job/settings data. |
| `src/shared/chapters.ts` | Create | Import parsing, stable ordering, segmentation, and invalidation logic. |
| `src/shared/timestamps.ts` | Create | Exact cumulative timing, format/title cleanup, and YouTube eligibility checks. |
| `src/main/index.ts` | Create | App/window lifecycle, single-instance lock, shutdown cancellation, safe UI protocol, service composition. |
| `src/main/ipc.ts` | Create | Explicit handlers, trusted-sender validation, native pickers, payload validation, event delivery. |
| `src/main/project-store.ts` | Create | Project creation/open/read/edit, asset ownership, revision checks, atomic writes/recovery, interrupted-job persistence. |
| `src/main/settings.ts` | Create | Settings/recent-project persistence, bounded model discovery, tool diagnostics and configuration validation. |
| `src/main/process-runner.ts` | Create | Executable lookup, safe asynchronous spawn, progress, bounded diagnostics, timeouts, process-group cancellation. |
| `src/main/jobs.ts` | Create | Single heavy-job queue, progress/state transitions, pause boundaries, retry, revision-safe result commits. |
| `src/main/llm.ts` | Create | Minimal text-provider interface, llama-cli and Ollama adapters, request limits, response cleanup and cancellation. |
| `src/main/speech.ts` | Create | Installed macOS voice enumeration and `say` generation adapter. |
| `src/main/audio.ts` | Create | Audio import/probing, take/chapter assembly, compatibility checks, safe concat/fallback, waveform peak generation, verified export commit. |
| `src/main/audio-protocol.ts` | Create | Validated asset-ID playback, byte-range requests, MIME types, closed-project rejection. |
| `src/main/logger.ts` | Create | Bounded operational logs without manuscript text or model prompts. |
| `src/preload/index.ts` | Create | Typed minimal bridge and listener cleanup, without generic raw IPC exposure. |
| `src/renderer/index.html` | Create | Clean local entry document and CSP; no hosted scripts/assets. |
| `src/renderer/src/main.tsx` | Create | React mount and stylesheet imports. |
| `src/renderer/src/App.tsx` | Create | Hash-router equivalents of prototype routes, application state provider, error boundary. |
| `src/renderer/src/studio-state.tsx` | Create | Active/recent projects, settings/jobs, revision/event refresh and bridge typing. |
| `src/renderer/src/components/StudioLayout.tsx` | Create | Recovered sidebar, active-project summary, viewport layout, shared error/status presentation. |
| `src/renderer/src/components/AudioPlayer.tsx` | Create | Real HTML audio transport, seek/rate controls, asset-backed duration and measured waveform peaks. |
| `src/renderer/src/components/SegmentInspector.tsx` | Create | Recovered tabs, voice/rate/pronunciation/takes, supported-capability states. |
| `src/renderer/src/pages/Projects.tsx` | Create | Recovered project cards with native create/open/recent behavior and empty states. |
| `src/renderer/src/pages/Import.tsx` | Create | TXT/Markdown/paste import, boundary preview, validated project creation. |
| `src/renderer/src/pages/Project.tsx` | Create | Chapter overview/order/title, audio import, generation controls and stale/failed state. |
| `src/renderer/src/pages/Editor.tsx` | Create | Recovered chapter/segment panes, persistent edits, candidate text preview/accept, takes and regeneration. |
| `src/renderer/src/pages/Review.tsx` | Create | Actual manual flags, failures, approval and retry; no synthetic transcription issues. |
| `src/renderer/src/pages/Voices.tsx` | Create | Installed voice selection/preview using the recovered cards. |
| `src/renderer/src/pages/Queue.tsx` | Create | Real queue progress, pause/resume/cancel/retry, interrupted jobs. |
| `src/renderer/src/pages/Exports.tsx` | Create | Active-project M4A export, history, stale indicators, timestamp copy and output reveal. |
| `src/renderer/src/pages/Settings.tsx` | Create | Separate text/speech settings, pickers, tool/model checks, audio defaults and actual storage paths. |
| `src/renderer/styles/prototype.css` | Create | Tracked copy of supplied CSS with provenance comment. |
| `src/renderer/styles/app.css` | Create | Small explicit supplemental rules for functional states and desktop sizing; no redesign or dependency on newly invented uncompiled utility classes. |
| `tests/chapters.test.ts` | Create | Parsing, ordering, stable IDs, Unicode, empty text and invalidation cases. |
| `tests/timestamps.test.ts` | Create | Fractional accumulation, hour boundaries, title cleanup and eligibility cases. |
| `tests/project-store.test.ts` | Create | Round trips, invalid versions/paths, missing files, revisions, interrupted writes/recovery. |
| `tests/providers.test.ts` | Create | Config validation, CLI construction, local HTTP responses, context limits, output failure and cancellation using fakes. |
| `tests/jobs.test.ts` | Create | Queue transitions, boundary pause, retries, cancellation cleanup and restart recovery. |
| `tests/audio.test.ts` | Create | Probe parsing, command arguments, concat compatibility/escaping, protocol range/path checks and timestamp-source selection. |
| `tests/audio.integration.test.ts` | Create | Generated tiny audio fixtures, copy/fallback exports, measured boundaries, missing input and cancellation. |
| `tests/desktop.smoke.spec.ts` | Create | Electron offline launch, persisted project flow, IPC isolation, real asset playback and export visibility. |
| `scripts/verify-release.mjs` | Create | Shared tag/version validation and architecture/signature/package checks invoked by npm/CI. |
| `.github/workflows/ci.yml` | Create | Push/PR checks on actual macOS arm64. |
| `.github/workflows/release-macos.yml` | Create | Validated `v*` tag packaging, artifact upload and GitHub Release attachment. |
| `README.md` | Create | Setup, commands, supported features, models, FFmpeg, output layout, local builds, tags, signing path and limitations. |
| `docs/PROJECT_CONTEXT.md` | Modify during implementation | Replace proposals with verified milestone, stack, constraints and commands. |
| `docs/ARCHITECTURE_INDEX.md` | Modify during implementation | Record landed files/entry points without duplicating source. |
| `docs/MODULE_OWNERSHIP.md` | Modify during implementation | Confirm boundaries against actual imports/services. |
| `docs/API_CONTRACTS.md` | Modify during implementation | Publish implemented schemas, IPC contracts and error semantics. |
| `docs/DECISIONS.md` | Modify after approval | Accept the approved architecture/provider/dependency choices and record material deviations. |
| `docs/TASK_LOG.md` | Modify per step | Record completed scope, validation and outstanding issues. |

## Implementation steps

Implement in dependency order and keep each group suitable for a small focused commit. Do not automatically commit/push without the user's workflow authorization.

1. **Establish the shell and recover the visual foundation.** Create root package/build/type configs, `src/main/index.ts`, preload, renderer entry/App/layout, and both stylesheets. Recover the supplied sidebar/screen structure before adding new controls. Verify an offline arm64 Electron launch, isolated renderer, routing, and side-by-side layout comparison with the supplied markup/reference. Keep all screens explicit about unconnected functionality until subsequent steps land.
2. **Define contracts and durable chapters.** Implement `contracts.ts`, `chapters.ts`, `project-store.ts`, settings storage, IPC, state provider, Projects/Import/Project pages, and their unit/filesystem tests. Verify create/open/edit/reorder/reopen, empty/malformed input, safe paths, revisions, and recovery. Publish the version-1 contract in the compact docs.
3. **Add bounded process execution and job control.** Implement process runner, logger, jobs, Queue page, events, and tests. Verify real cancellation against a disposable test child process, pause-at-boundary behavior, restart recovery, and preservation of old outputs. Display operational errors immediately.
4. **Connect local text models and configuration.** Implement `llm.ts`, tool/model checks, Settings and editor candidate preview. Test both providers using fakes and a local test HTTP server; test bad model paths, unavailable Ollama, empty/oversized output, cancellation and CLI version differences. Run a real selected-model smoke test only if suitable installed weights are available; do not download one implicitly.
5. **Implement speech, import, and manual editing/review.** Add speech adapter, probing/take assembly, player/protocol, inspector, Voices/Editor/Review. Verify an installed voice generates audible audio, selected takes persist, pronunciation affects only spoken text, imports survive external-file movement, and changing text marks old audio stale. Derive waveform peaks from actual decoded samples with bounded streaming memory.
6. **Implement verified exports and chapter marks.** Complete audio concat/fallback, timestamps, Exports, and focused media tests. Generate tiny fixtures on demand; exercise copy, incompatible-format fallback, mixed rates/channels, fractional timing, Unicode paths, stale/missing chapters, failed concat and cancellation. Verify final audible boundaries and duration before declaring the output pair complete.
7. **Complete desktop acceptance and packaging.** Add smoke tests, packaging config and release verification script. Run typecheck/unit/integration/build checks, packaged offline launch and basic project playback/export, verify arm64 executable/signature, and produce DMG/ZIP locally. Test Finder launch so shell-specific PATH assumptions cannot hide missing-tool errors.
8. **Add GitHub automation and finish documentation.** Create the two workflows using the same npm commands; validate tag/version mismatch handling and artifact paths. Write README setup/model profiles/output/release/signing sections, and update all six memory files. Report what was actually run locally versus what requires the first GitHub push/tag or signing credentials.

## Verification

Planned commands after implementation (they do not exist yet):

```bash
npm ci
npm run typecheck
npm test
npm run test:integration
npm run build
npm run test:smoke
npm run pack:mac
npm run package:mac
```

- `npm run dev`: electron-vite development mode; `build`: production main/preload/renderer compilation.
- `test`: focused Vitest tests without real models; `test:integration`: required FFmpeg/FFprobe media tests, failing clearly when prerequisites are absent.
- `test:smoke`: Playwright Electron launch against the built app with isolated temporary user/project directories; no mutation of the user's actual settings.
- `pack:mac`: unpacked arm64 application plus architecture/signature validation; `package:mac`: DMG/ZIP plus the same checks.
- Timestamp acceptance: 272/378/235-second chapters give `00:00`, `04:32`, `10:50`; many fractional chapters demonstrate that no per-chapter rounding drift occurs; hour-long inputs format correctly.
- Media acceptance: derive distinguishable tiny tone markers; verify their order and boundary locations in decoded output, plus FFprobe duration within codec-frame tolerance. Include enough short encoded chapters to expose accumulated encoder-padding errors. Store no large binary fixtures.
- Native acceptance: launch without internet; create/import/edit/reorder, use an installed voice, import external audio, listen/seek, cancel/retry, export/copy/reveal, restart/reopen, and confirm persisted state.
- Negative acceptance: missing GGUF/executable/audio, corrupt project, unavailable Ollama, unwritable directory, context overflow, failed generation/concat, invalid media range/path, interrupted job, and closing the app during processing.
- Compare all primary screens at desktop sizes against the prototype's recovered layout; demonstrate actual empty/loading/error/unsupported states without simulated completion.
- CI acceptance requires a real pushed commit/tag later. Review generated workflow syntax locally; do not claim GitHub Actions or public distribution succeeded until observed.

README model guidance will give configurable profiles: 3B–4B quantized instruct for lighter tasks, 7B–9B around 4-bit as a balanced starting class, and 14B+ only with adequate free unified memory. Context/cache size and concurrent workloads matter; avoid fixed RAM guarantees. Explain `mkdir -p ~/Models`, selecting a compatible downloaded GGUF, installing llama.cpp, and `brew install ollama`, `ollama serve`, `ollama pull <model>`. Link to current model-owner cards when choosing concrete examples during implementation. Text LLM setup must not be presented as installing a speech model.

## Risks / edge cases

- **No original frontend source:** recovery from compiled components is real work and may miss subtle interactions. Use the explicit reuse map and visual comparison; do not patch minified production code as the long-term solution.
- **Speech capability gap:** macOS voices do not implement the prototype's neural emotion/cloning controls. This is an explicit first-version scope proposal, not a promise of Qwen3-TTS/CosyVoice quality.
- **Empty repository history:** GitHub is confirmed and local `main` is initialized, but no commits, previous implementation, or CI conventions exist. Preserve the current Git configuration; commits and remote writes belong to subsequent authorized work.
- **External tooling:** Finder-launched apps have a different PATH; diagnose missing executables and invalid versions through settings. Optional inference smoke tests depend on locally available weights and memory.
- **Compressed audio timing:** matching file extensions is insufficient; encoder delay, padding, stream metadata and mixed formats can shift boundaries. Use measured timelines and a verified PCM fallback.
- **Durability and disk use:** retained takes and fallback PCM can consume space; handle write/disk-full failures, clean job temporaries, and recover partial multi-file commits without losing previous results.
- **Untrusted imported content:** safe paths, strict schemas, IPC validation, plain text rendering, argument arrays and restricted local protocols are required at the relevant boundaries.
- **Testing distribution:** initial artifacts are ad-hoc signed and require separately installed FFmpeg. Developer ID/notarization is a separate future distribution milestone.

## Out of scope

Cloud accounts/backends, databases, Docker, automatic model downloads, remote inference endpoints, MLX runtime integration, Qwen3-TTS/CosyVoice integration under the proposed speech baseline, voice cloning, emotion-conditioned synthesis, automatic transcription/pronunciation QA, arbitrary plugins, multi-user or simultaneous multi-project editing, GPU job concurrency, crossfades, MP3/WAV/M4B export options, embedded audiobook chapters, YouTube upload/video creation, auto-updates, mandatory notarization, Intel/universal releases, and GitLab CI.

No source/configuration implementation, dependency installation, build, Git initialization, commit, push, or release is authorized by this planning task. Implementation starts only after the user's separate approval/prompt.
