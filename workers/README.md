# Local sound workers

Sound Studio can make clips without a book. Its two Python workers are separate processes on this Mac. Chatterbox Turbo handles spoken text and its documented vocal tags. Stable Audio Open handles general sound effects, including fabric; panting is experimental and may be poor. The app reports each worker's status in Sound Studio and its address in Settings. FFmpeg and FFprobe are also required for finished clips.

The workers do not download checkpoints or install packages when the app starts or generates a clip. Python 3.10–3.12 is recommended. Run each worker in its own terminal and virtual environment. Replace paths below with absolute paths on your machine.

## Chatterbox Turbo

1. Download the [Chatterbox Turbo checkpoint](https://huggingface.co/ResembleAI/chatterbox-turbo/tree/main) to a local folder. It must contain `t3_turbo_v1.safetensors` and `conds.pt`; the latter is its default voice conditioning, so no voice sample is required. Read the [Chatterbox license](https://github.com/resemble-ai/chatterbox) before use.
2. Install and start the worker:

```sh
python3.12 -m venv workers/chatterbox/.venv
workers/chatterbox/.venv/bin/python -m pip install -r workers/chatterbox/requirements.txt
HOMER_CHATTERBOX_MODEL_DIR=/absolute/path/to/chatterbox-turbo workers/chatterbox/.venv/bin/python workers/chatterbox/server.py
```

The default address is `http://127.0.0.1:8765`. Use `HOMER_CHATTERBOX_PORT` to change it, and set the matching address in Homer Studio Settings. Speech takes exact English text. Vocal gestures accept one tag: `[sigh]`, `[gasp]`, `[cough]`, `[laugh]`, `[chuckle]`, or `[groan]`. Chatterbox is not used for arbitrary fabric or ambience prompts.

## Stable Audio Open

1. Request access to the [Stable Audio Open 1.0 checkpoint](https://huggingface.co/stabilityai/stable-audio-open-1.0), accept its terms, and download the complete Diffusers-format repository to a local folder. It must include `model_index.json` and the component weights. Check the model license and permitted uses yourself.
2. Install and start the worker:

```sh
python3.12 -m venv workers/sfx/.venv
workers/sfx/.venv/bin/python -m pip install -r workers/sfx/requirements.txt
HOMER_SFX_MODEL_DIR=/absolute/path/to/stable-audio-open-1.0 workers/sfx/.venv/bin/python workers/sfx/server.py
```

The default address is `http://127.0.0.1:8766`; change it with `HOMER_SFX_PORT` and the Settings field. The worker tries Apple Silicon MPS and otherwise CPU. CPU generation can be slow. This model was designed for sound effects; realistic vocals are a known limitation. A panting prompt is an experiment, not a guaranteed result.

## Check and use

```sh
curl http://127.0.0.1:8765/v1/health
curl http://127.0.0.1:8766/v1/health
```

Each response must report `protocolVersion: 1` and `ready: true`. This confirms checkpoint files and Python dependencies are present. The model is loaded on the first generation request; if that load fails, the job reports the error. Open **Sound Studio**, choose a category, enter a prompt, and generate. Finished clips appear in the independent Clip library, with a 48 kHz WAV master and M4A preview/export copy in the app data folder. Neither a manuscript nor a user voice sample is required.

The worker API is documented in `docs/API_CONTRACTS.md`. It is bound to loopback and the Mac app accepts loopback addresses only. A future Linux GPU worker can implement the same versioned API, but remote authentication and transport are not part of this release.

This repository's automated tests use stub audio and do not validate model quality. The development machine used for this implementation had no worker packages or checkpoints and could not download them, so actual Chatterbox and Stable Audio inference remains to be tested after setup.
