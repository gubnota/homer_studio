# Local sound workers

Sound Studio can make English speech clips without a book. Chatterbox Turbo handles speech and its documented vocal tags; the optional Original Chatterbox worker supports expressive chapter narration and voice conversion. Sound-effect generation has been retired because its output did not meet the quality bar. Existing effect clips remain available in the library. FFmpeg and FFprobe are also required for finished clips.

The workers do not download checkpoints or install packages when the app starts or generates a clip. In the packaged app, use **Settings → Chatterbox Python runtime → Install runtime** to create the shared Python environment under `~/Library/Application Support/com.gubnota.homerstudio/python/chatterbox`. Install the Turbo or Original checkpoint from Settings too; the app then starts the selected worker automatically. Python 3.10 must be available on the Mac. If it is outside the usual paths, set its executable in Settings first. Packages, checkpoints, logs, and cache stay in Application Support; the app does not use paths from a development checkout. Initial package installation requires network access and several gigabytes of free disk space.

For development without the packaged app, run commands from the repository root. The same app-data environment can be prepared manually:

```sh
python3.10 -m venv "$HOME/Library/Application Support/com.gubnota.homerstudio/python/chatterbox"
"$HOME/Library/Application Support/com.gubnota.homerstudio/python/chatterbox/bin/python" -m pip install -r workers/chatterbox/requirements.txt
python3 workers/start_local.py chatterbox
```

## Chatterbox Turbo

1. Use **Install** in Settings or download the [complete Chatterbox Turbo checkpoint](https://huggingface.co/ResembleAI/chatterbox-turbo/tree/main) to a local folder. It must include `t3_turbo_v1.safetensors`, `s3gen_meanflow.safetensors`, `ve.safetensors`, tokenizer files, and `conds.pt`; the latter is its default voice conditioning, so no voice sample is required. Read the [Chatterbox license](https://github.com/resemble-ai/chatterbox) before use.
2. In the packaged app, install the Python runtime in Settings. Homer Studio starts the worker after setup; the manual launcher above is for development.

The default address is `http://127.0.0.1:8765`. Use `HOMER_CHATTERBOX_PORT` to change it, and set the matching address in Homer Studio Settings. Speech takes exact English text. Documented Turbo tags are `[clear throat]`, `[sigh]`, `[shush]`, `[cough]`, `[groan]`, `[sniff]`, `[gasp]`, `[chuckle]`, and `[laugh]`; they can appear inline with spoken words. Chatterbox is not used for arbitrary fabric or ambience prompts.

## Original Chatterbox for expressive English speech

The original Chatterbox model is an alternative speech engine. It provides exaggeration and CFG controls and accepts a selected voice sample. It does not interpret Turbo's bracketed vocal-gesture tags; choose Turbo for those tags.

Use **Install** in Settings or download the [complete original checkpoint](https://huggingface.co/ResembleAI/chatterbox/tree/main) into a local folder containing `ve.safetensors`, `t3_cfg.safetensors`, `s3gen.safetensors`, `tokenizer.json`, and `conds.pt`. The packaged app uses the shared Chatterbox runtime and starts Original when selected. For development, run:

```sh
python3 workers/start_local.py original
```

The original worker listens at `http://127.0.0.1:8767` by default. Select **Original Chatterbox** in Settings and set that service address there. Its health check confirms the dependencies and files are present; the model loads when the first generation begins. A saved voice sample is copied into Homer Studio's local library when imported, so its original file can be moved afterward.

## Check and use

```sh
curl http://127.0.0.1:8765/v2/health
curl http://127.0.0.1:8767/v2/health
```

Each response must report `protocolVersion: 2` and `ready: true`. This confirms checkpoint files and Python dependencies are present. The model is loaded on the first generation request; if that load fails, the job reports the error. Open **Sound Studio**, enter speech text, and generate. Finished clips appear in the independent Clip library, with a 48 kHz WAV master and M4A preview/export copy in the app data folder. Neither a manuscript nor a user voice sample is required.

Install Turbo and/or Original checkpoints explicitly from Settings first. Settings shows download progress in bytes and the actual disk use. The launcher picks up app-managed checkpoints automatically; `HOMER_CHATTERBOX_MODEL_DIR` and `HOMER_CHATTERBOX_ORIGINAL_MODEL_DIR` override those locations for development. To start installed workers manually, run `python3 workers/start_local.py chatterbox original` (or name one). You may set `HOMER_CHATTERBOX_PYTHON` for a custom location. The launcher reports missing model/environment paths and writes logs in the app-data `logs` folder. Homer Studio stops configured local Chatterbox workers when the app exits, releasing their model memory. Settings URLs point to the **running service**, not to checkpoint folders. Ollama is managed separately.

The worker API is documented in `docs/API_CONTRACTS.md`. It is bound to loopback and the Mac app accepts loopback addresses only. A future Linux GPU worker can implement the same versioned API, but remote authentication and transport are not part of this release.

Automated tests use stub audio and cannot judge sound quality. Listen to a generated clip to assess whether a prompt and seed work for your purpose.

## Voice samples

In **Voices**, create a voice, import a clear 6–20 second spoken sample or record one with the microphone, select the sample, and preview it. Homer Studio normalizes the sample to a bounded WAV in local app data. Chatterbox receives it through `POST /v2/references` and consumes the temporary reference during the next job; the worker does not need access to the voice library. The built-in model voice needs no sample. Only one selected sample conditions a generation; multiple speakers are not blended.

## Correcting a narrated section with your delivery

Create a custom narrator voice and select one of its saved samples. Start the Original worker, open a chapter in **Review**, and expand **Manual sections**. After generating and assembling section takes, drag on the waveform to select up to 20 seconds inside one section, choose **Record replacement**, and speak that passage with your preferred pace and emphasis. You can also choose **Record delivery** on a section row to replace the whole take. The recorder stops after 20 seconds. **Convert to narrator voice** sends the recorded WAV and selected narrator sample to Original's `voice_conversion` route. For a partial replacement, the result is spliced into the existing take with short fades. The result appears as another saved take; play both and choose **Use take** only when it sounds right. **Assemble chapter** rebuilds the chapter and measured section cues. Previous takes are retained. Voice conversion can alter timing or words, so listening matters.

Sound Studio accepts a maximum duration up to two minutes for speech and gestures. Longer speech is split by words into worker requests of at most 20 seconds and joined locally; actual speech duration depends on the text. Voice Lab accepts a 1–120 second recording and sends 15-second-or-shorter chunks to Original Chatterbox, then joins the converted audio. The Original worker, FFmpeg, and a saved narrator sample must be available for conversion.
