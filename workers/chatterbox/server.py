"""Local Chatterbox Turbo speech and documented vocal-gesture worker."""

import io
import importlib.util
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from worker_protocol import Worker, serve

_model = None


def check_model(path):
    required = ("conds.pt", "t3_turbo_v1.safetensors", "s3gen_meanflow.safetensors", "ve.safetensors", "tokenizer_config.json", "vocab.json", "merges.txt")
    missing = [name for name in required if not (path / name).is_file()]
    if missing:
        return False, f"Chatterbox Turbo checkpoint missing {', '.join(missing)} in {path}"
    if any(importlib.util.find_spec(name) is None for name in ("torch", "torchaudio", "chatterbox")):
        return False, "Python packages missing. Install workers/chatterbox/requirements.txt."
    return True, "Checkpoint and packages found; ready to load on first request."


def generate(request, model_dir):
    global _model
    import torch
    import torchaudio
    from chatterbox.tts_turbo import ChatterboxTurboTTS

    category = request["category"]
    prompt = request["prompt"].strip()
    if category == "vocal_gesture" and prompt.lower() not in ("[clear throat]", "[sigh]", "[shush]", "[cough]", "[groan]", "[sniff]", "[gasp]", "[chuckle]", "[laugh]"):
        raise ValueError("Use one documented Chatterbox Turbo vocal tag.")
    device = "mps" if torch.backends.mps.is_available() else "cpu"
    if _model is None:
        _model = ChatterboxTurboTTS.from_local(str(model_dir), device=device)
        if _model.conds is None:
            raise ValueError("This checkpoint has no default voice. Add conds.pt to the model folder.")
    reference = request.get("referencePath")
    try:
        with torch.inference_mode():
            wave = _model.generate(prompt, audio_prompt_path=reference) if reference else _model.generate(prompt)
            result = io.BytesIO()
            torchaudio.save(result, wave.cpu(), _model.sr, format="wav")
            return result.getvalue()
    finally:
        if device == "mps":
            torch.mps.empty_cache()


if __name__ == "__main__":
    model_dir = os.environ.get("HOMER_CHATTERBOX_MODEL_DIR", "")
    if not model_dir:
        raise SystemExit("Set HOMER_CHATTERBOX_MODEL_DIR to a local Chatterbox Turbo checkpoint folder.")
    serve(Worker("chatterbox_turbo", model_dir, ["speech", "vocal_gesture"], generate, check_model), int(os.environ.get("HOMER_CHATTERBOX_PORT", "8765")))
