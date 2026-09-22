# Local sound workers

Sound Studio can make clips without a book. Its two Python workers are separate processes on this Mac. Chatterbox Turbo handles English speech and its documented vocal tags. Stable Audio Open handles general sound effects, including fabric; panting is experimental and may be poor. The app reports each worker's status in Sound Studio and its address in Settings. FFmpeg and FFprobe are also required for finished clips.

The workers do not download checkpoints or install packages when the app starts or generates a clip. Use Python 3.10 for the currently pinned Chatterbox package; its NumPy dependency does not install reliably on Python 3.12. Install each worker in its own virtual environment. Run commands from the repository root.

## Chatterbox Turbo

1. Download the [complete Chatterbox Turbo checkpoint](https://huggingface.co/ResembleAI/chatterbox-turbo/tree/main) to a local folder. It must include `t3_turbo_v1.safetensors`, `s3gen_meanflow.safetensors`, `ve.safetensors`, tokenizer files, and `conds.pt`; the latter is its default voice conditioning, so no voice sample is required. Read the [Chatterbox license](https://github.com/resemble-ai/chatterbox) before use.
2. Install and start the worker:

```sh
python3.10 -m venv workers/chatterbox/.venv
workers/chatterbox/.venv/bin/python -m pip install -r workers/chatterbox/requirements.txt
HOMER_CHATTERBOX_MODEL_DIR=/absolute/path/to/chatterbox-turbo python3 workers/start_local.py chatterbox
```

The default address is `http://127.0.0.1:8765`. Use `HOMER_CHATTERBOX_PORT` to change it, and set the matching address in Homer Studio Settings. Speech takes exact English text. Documented Turbo tags are `[clear throat]`, `[sigh]`, `[shush]`, `[cough]`, `[groan]`, `[sniff]`, `[gasp]`, `[chuckle]`, and `[laugh]`; they can appear inline with spoken words. Chatterbox is not used for arbitrary fabric or ambience prompts.

## Original Chatterbox for expressive English speech

The original Chatterbox model is an alternative speech engine. It provides exaggeration and CFG controls and accepts a selected voice sample. It does not interpret Turbo's bracketed vocal-gesture tags; choose Turbo for those tags.

Download the [complete original checkpoint](https://huggingface.co/ResembleAI/chatterbox/tree/main) into a local folder containing `ve.safetensors`, `t3_cfg.safetensors`, `s3gen.safetensors`, `tokenizer.json`, and `conds.pt`. Install the Chatterbox requirements as above, then run:

```sh
HOMER_CHATTERBOX_ORIGINAL_MODEL_DIR=/absolute/path/to/chatterbox workers/chatterbox/.venv/bin/python workers/start_local.py original
```

The original worker listens at `http://127.0.0.1:8767` by default. Select **Original Chatterbox** in Settings and set that service address there. Its health check confirms the dependencies and files are present; the model loads when the first generation begins. A saved voice sample is copied into Homer Studio's local library when imported, so its original file can be moved afterward.

## Sound effects: Stable Audio Open

1. Request access to the [Stable Audio Open 1.0 checkpoint](https://huggingface.co/stabilityai/stable-audio-open-1.0), accept its terms, and download the complete Diffusers-format repository to a local folder. It must include `model_index.json` and the component weights. Check the model license and permitted uses yourself.
2. Install and start the worker:

```sh
python3.12 -m venv workers/sfx/.venv
workers/sfx/.venv/bin/python -m pip install -r workers/sfx/requirements.txt
HOMER_SFX_MODEL_DIR=/absolute/path/to/stable-audio-open-1.0 python3 workers/start_local.py sfx
```

Change the sound worker port with `HOMER_SFX_PORT` and the matching Settings field. This model was designed for sound effects; realistic vocals are a known limitation. A panting prompt is an experiment, not a guaranteed result.
The default address is `http://127.0.0.1:8766`. The worker uses Apple Silicon MPS when available and otherwise CPU; CPU generation can be slow. Make prompts concrete and describe the audible texture and setting. Try another seed if the result is strange.

## Check and use

```sh
curl http://127.0.0.1:8765/v2/health
curl http://127.0.0.1:8766/v2/health
curl http://127.0.0.1:8767/v2/health
```

Each response must report `protocolVersion: 2` and `ready: true`. This confirms checkpoint files and Python dependencies are present. The model is loaded on the first generation request; if that load fails, the job reports the error. Open **Sound Studio**, choose a category, enter a prompt, and generate. Finished clips appear in the independent Clip library, with a 48 kHz WAV master and M4A preview/export copy in the app data folder. Neither a manuscript nor a user voice sample is required.

To start both installed workers in the background from the repository root, run `python3 workers/start_local.py`. You may set `HOMER_CHATTERBOX_MODEL_DIR`, `HOMER_SFX_MODEL_DIR`, `HOMER_CHATTERBOX_PYTHON`, or `HOMER_SFX_PYTHON` for custom locations. The launcher reports missing model/environment paths and writes worker logs beside each worker. Launch it again after a restart. Settings URLs point to the **running service**, not to checkpoint folders.

The worker API is documented in `docs/API_CONTRACTS.md`. It is bound to loopback and the Mac app accepts loopback addresses only. A future Linux GPU worker can implement the same versioned API, but remote authentication and transport are not part of this release.

Automated tests use stub audio and cannot judge sound quality. Listen to a generated clip to assess whether a prompt and seed work for your purpose.

## Voice samples and sound variations

In **Voices**, create a voice, import a clear 6–20 second spoken sample or record one with the microphone, select the sample, and preview it. Homer Studio normalizes the sample to a bounded WAV in local app data. Chatterbox receives it through `POST /v2/references` and consumes the temporary reference during the next job; the worker does not need access to the voice library. The built-in model voice needs no sample. Only one selected sample conditions a generation; multiple speakers are not blended.

In **Sound Studio**, the optional **Sounds to avoid** field becomes `negativePrompt` on an effects request. **Generate 3 variations** queues three neighboring seeds for listening comparison. Different seeds and more specific prompts can help, but neither model guarantees realistic fabric or panting.
