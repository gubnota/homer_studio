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
    if not (path / "conds.pt").is_file() or not (path / "t3_turbo_v1.safetensors").is_file():
        return False, f"Chatterbox Turbo checkpoint missing in {path}"
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
    if category == "vocal_gesture" and prompt.lower() not in ("[sigh]", "[gasp]", "[cough]", "[laugh]", "[chuckle]", "[groan]"):
        raise ValueError("Use a supported vocal tag: [sigh], [gasp], [cough], [laugh], [chuckle], or [groan].")
    device = "mps" if torch.backends.mps.is_available() else "cpu"
    if _model is None:
        _model = ChatterboxTurboTTS.from_local(str(model_dir), device=device)
        if _model.conds is None:
            raise ValueError("This checkpoint has no default voice. Add conds.pt to the model folder.")
    wave = _model.generate(prompt)
    result = io.BytesIO()
    torchaudio.save(result, wave.cpu(), _model.sr, format="wav")
    return result.getvalue()


if __name__ == "__main__":
    model_dir = os.environ.get("HOMER_CHATTERBOX_MODEL_DIR", "")
    if not model_dir:
        raise SystemExit("Set HOMER_CHATTERBOX_MODEL_DIR to a local Chatterbox Turbo checkpoint folder.")
    serve(Worker("chatterbox_turbo", model_dir, ["speech", "vocal_gesture"], generate, check_model), int(os.environ.get("HOMER_CHATTERBOX_PORT", "8765")))
