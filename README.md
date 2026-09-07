# Homer Studio

Homer Studio is a local audiobook production app for Apple Silicon Macs. It imports TXT and Markdown manuscripts, lets you edit and process chapters, generates narration with installed macOS voices or imported audio, reviews each chapter, and exports one M4A audiobook with measured chapter timestamps.

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

Create a project in a folder you control, then import a TXT or Markdown manuscript. Homer Studio stores a readable `homer-project.json` manifest and generated files inside that project folder.

## Local text processing

Text processing is optional. Open Settings and choose either:

- **llama.cpp:** install `llama-cli`, download a compatible GGUF model yourself, and select the executable and model paths.
- **Ollama:** install and start Ollama, pull a model, and enter its name. Homer Studio accepts loopback Ollama endpoints only.

Generated text is always presented as a candidate. Accepting it changes narration text while preserving the imported source.

## Narration and export

The Voices screen lists installed macOS voices. Additional voices can be installed in macOS System Settings. You can also import existing chapter audio. FFmpeg converts chapter media to 48 kHz stereo AAC/M4A and FFprobe measures its real duration.

Approve every current chapter recording before export. Homer Studio rechecks the media, joins it, verifies the final duration, and writes both the M4A and timestamp text file to the project.

## Checks and packages

```sh
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:integration
npm run pack:mac
npm run test:smoke
npm run package:mac
```

The final command creates an Apple Silicon `.app`, `.dmg`, and `.zip` below `src-tauri/target/release/bundle/`. Local packages are ad-hoc signed for development and direct testing. A public distribution can add an Apple Developer ID signature and notarization later.

GitHub CI repeats the checks on an Apple Silicon macOS runner. Tags matching the package version, such as `v0.1.0`, run the release workflow and publish the DMG and ZIP.
