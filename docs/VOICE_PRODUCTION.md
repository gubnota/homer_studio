# Voice production

Open **Voice Memos** to record and edit without a book. The same workspace is
available in Review, Sound Studio and Voice Lab. Existing Chatterbox narration
and recorded-delivery conversion remain available.

## Record and edit

The packaged macOS app includes a microphone usage description and a signed
audio-input entitlement. Click **Record** to trigger the first permission request
and choose **Allow**. macOS lists the app under System Settings → Privacy &
Security → Microphone after this request; enable it there if previously denied.

1. Create a memo, choose a microphone, and grant macOS microphone permission.
2. Record, pause/resume, then **Stop & save**. Watch the level and clipping meter.
   Capture is mono 32-bit float WAV at the device sample rate, with a one-hour limit.
3. Drag the waveform or enter start/end times. Zoom and pan for a smaller passage.
4. Trim, delete, silence, split, fade, change gain, insert silence, or shorten a
   selected silence. **Amount** sets milliseconds for duration/fade operations;
   **Gain multiplier** controls level. Silence shortening is a manual selection.
5. Save a selection as a separate memo, or record a replacement after a short
   lead-in and countdown. Listen to the new preview before accepting it.

Original WAVs stay intact. Edits create variants; Undo/Redo changes the selected
take, with up to 100 undo entries. Takes support names, notes and favorites.
Memo deletion is reversible through **Recently deleted**. Deleted/rejected take
files are retained because other compositions can reference them.

Playback offers play/pause, stop, beginning/end, seeking, speed and selection
looping. With the editor focused, Space toggles playback, Command/Ctrl+Z undoes,
Shift+Command/Ctrl+Z redoes, Delete removes the selection, and Escape clears it.
Typing in text/number fields retains ordinary editing shortcuts.

## Quality and optional engines

**Original** applies no processing. **Quick Clean** selects DeepFilterNet;
**Studio Quality** selects Resemble Enhance. **Custom** exposes the engine and
advanced JSON parameters. Processing always creates a preview. Conversion and
enhancement can optionally run DeepFilterNet first.

In **Local engine setup**, either enter an existing environment's Python path or
explicitly install pinned dependencies using a Python 3.10/3.11 interpreter.
Each engine gets a separate environment in app data. Installation uses the
network; inference expects local models and disables implicit Hugging Face
downloads. Model weights/checkouts are supplied separately. Select their folder
and click **Save & check readiness** after installation.

| Engine | Local folder requirements | Backend |
| --- | --- | --- |
| Seed-VC | Checkout with `inference.py`; `checkpoints/DiT_seed_v2_uvit_whisper_small_wavenet_bigvgan_pruned.pth`, `checkpoints/config_dit_mel_seed_uvit_whisper_small_wavenet.yml`, populated `checkpoints/hf_cache` for Whisper, BigVGAN and CAMPPlus | CPU or Apple GPU when available |
| RVC | Checkout with `infer/cli.py`, `assets/hubert/hubert_base.pt`, `assets/rmvpe/rmvpe.pt`; select the trained voice model and optional index in the profile | CPU |
| DeepFilterNet | Model folder with `config.ini` and populated `checkpoints` | CPU |
| Resemble Enhance | Model folder with `hparams.yaml` and `ds/G/default/mp_rank_00_model_states.pt` | CPU or Apple GPU when available |

Seed-VC parameters include `diffusionSteps`, `lengthAdjust` and `guidance`; RVC
supports `pitch`, `indexRate` and `protect`; DeepFilterNet supports `attenuation`;
Resemble supports `nfe`, `lambda` and `tau`. Parameters are validated by the
adapter. Package requirements are in `workers/audio/*.txt`. A readiness check
verifies imports and file presence; it cannot guarantee checkpoint compatibility
or model quality. These optional adapters still require a real-model test on
Apple Silicon, including dependency installation and CPU/Apple GPU behavior.

Existing voices keep their Chatterbox samples. Choose a voice profile and attach
an accepted memo as its clean single-speaker conversion reference. RVC profiles
also hold model/index paths. Chatterbox samples still require 6–20 seconds.

## Use and export

Accept a result, then send it to Sound Library, a book chapter/section, or a custom
Chatterbox voice. Import chapter or sound-library media into a memo for editing.
Standalone exports support 24-bit WAV, FLAC, M4A/AAC and MP3. Native FFmpeg
rendering and FFprobe measure the final audio before publication. Changes during
a queued operation produce a revision error instead of overwriting newer work.
Cancellation stops worker subprocesses and preserves the last saved take.

Chapter sentence cues follow cuts and insertions. Replacement cues are scaled
to the measured replacement duration, so highlighting remains approximate.
There is no word-level forced alignment. Publishing into a different chapter
does not carry unrelated sentence cues.

## Manual verification still required

- After each release build, open Voice Lab, Voice Memos and Sound Studio in the
  packaged app. Each mounts microphone discovery and must remain responsive.
  Run `cargo test --release input_device_discovery --manifest-path src-tauri/Cargo.toml`
  to exercise CoreAudio enumeration with production optimizations.
- Grant/deny microphone access; record through the intended device, pause/resume,
  stop, and listen for dropouts and clipping. Test device disconnection and normal
  Quit while recording. Capture errors report a recovery file when available.
- Compare a short replacement with the original, accept/reject, undo/redo, and
  listen across both crossfade boundaries. Repeat after reopening the app.
- Install each desired engine explicitly; run clean/noisy references and verify
  real output, model compatibility, cancellation and memory release.
- Publish an edited chapter/section, rebuild the audiobook, and check exported
  cues and duration by listening.

Automated checks cover composition rendering/duration, source preservation,
cue remapping, metadata validation, protocol bounds, process cancellation,
existing Chatterbox behavior, renderer contracts and arm64 packaging.
