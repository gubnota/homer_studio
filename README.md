# Homer Studio

Homer Studio is a local audiobook and sound-clip production app for Apple Silicon Macs. It imports TXT and Markdown manuscripts, lets you edit and process chapters, generates English narration with local Chatterbox Turbo or Original voices or imported audio, reviews each chapter, and exports one M4A audiobook with measured chapter timestamps. Sound Studio creates independent prompt-based clips without a book.

Manuscripts, model requests, narration, and exports stay on the Mac. The app has no account, analytics, or hosted backend.

## Requirements

- Apple Silicon Mac running macOS 13 or later
- Node.js 22 and npm
- Stable Rust toolchain with the Apple Silicon target
- FFmpeg and FFprobe, for example `brew install ffmpeg`

## Run from source

```sh
npm ci
npm run dev
```

Create a project in a folder you control, then choose, drop, or paste a TXT or Markdown manuscript. Homer Studio stores a readable `project.json` manifest and generated files inside that project folder.

## Local tool locations

Open **Settings** to see each tool's detected and configured location. Homer Studio checks the app environment and common Apple Silicon locations including `/opt/homebrew/bin`, `/usr/local/bin`, `~/.local/bin`, `~/homebrew/bin`, and `~/local/homebrew/bin`.

If a tool still shows **Not found**, enter its full path or use **Choose**. When automatic discovery succeeds, **Use detected path** saves that location. FFmpeg and FFprobe are required for narration and export. llama.cpp and Ollama are optional text-processing providers.

Ollama has two separate checks: the CLI location and the local server at the configured loopback URL. A found CLI does not mean the server is running.

## Local text processing

Text processing is optional. Open Settings and choose either:

- **llama.cpp:** install `llama-cli`, download a compatible GGUF model yourself, and select the executable and model paths.
- **Ollama:** install and start Ollama, pull a model, and enter its name. Homer Studio accepts loopback Ollama endpoints only.

Generated text is always presented as a candidate. Accepting it changes narration text while preserving the imported source.

## Narration and export

The Voices screen offers the built-in Chatterbox model voice and custom voices from local reference samples. Create a voice, import a clear 6–20 second spoken recording or record through the microphone, then choose one sample to condition generation. You can keep multiple samples and switch between them; the app does not blend different speakers. Preview before a long narration run. Turbo accepts documented inline tags such as `[sigh]` and `[laugh]`; Original has expression controls instead. Exact word emphasis and SSML are unavailable. Install the Turbo or Original checkpoint explicitly in **Settings**, install its Python environment, and start the worker as described in [workers/README.md](workers/README.md).

For a long chapter, open **Manual sections** in Review. Generate individual sections or the remaining ones, preview saved takes, choose a take, and assemble the chapter. Click a spoken cue or its waveform area to seek to that section. For a delivery correction, record up to 20 seconds and convert it to the selected custom narrator voice with Original Chatterbox; listen to the new take before choosing and assembling it. Generated chapter audio can be exported or deleted from Review. The Render Queue shows progress, errors, and controls for cancelling or clearing finished jobs.

You can also import existing chapter audio. FFmpeg converts chapter media to 48 kHz stereo AAC/M4A and FFprobe measures its real duration.

Approve every current chapter recording before export. Homer Studio rechecks the media, joins it, verifies the final duration, and writes both the M4A and timestamp text file to the project.

## Standalone sounds

Open **Sound Studio** without a project. Choose **Sound effect** for fabric, ambience, or experimental panting prompts; choose **Speech** or a supported **Vocal gesture** for Chatterbox Turbo. For effects, add sounds to avoid and generate three variations to compare. Speech uses the selected voice from the searchable voice picker. Each finished clip stays in a separate library and can be played, retried, or exported as WAV or M4A. Sound generation needs local model checkpoints and companion Python workers. Settings can download Turbo and Original weights on request; Python packages and workers still require separate setup. See [workers/README.md](workers/README.md) for setup and limitations. Worker addresses are set in **Settings**.

## Checks and packages

```sh
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:integration
npm run pack:mac
npm run test:smoke
npm run package:mac
```

The final command attempts an Apple Silicon `.app`, `.dmg`, and `.zip` below `src-tauri/target/release/bundle/`. Local packages are ad-hoc signed for development and direct testing. A public distribution can add an Apple Developer ID signature and notarization later.

GitHub CI repeats the checks on an Apple Silicon macOS runner. Tags matching the package version, such as `v0.2.0`, run the release workflow and publish the DMG and ZIP.
