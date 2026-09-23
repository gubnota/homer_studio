"""English Chatterbox TTS with explicit local weights and expression controls."""

import io
import importlib.util
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from worker_protocol import Worker, serve

_model = None
_vc_model = None


def check_model(path):
    required = ("ve.safetensors", "t3_cfg.safetensors", "s3gen.safetensors", "tokenizer.json", "conds.pt")
    missing = [name for name in required if not (path / name).is_file()]
    if missing:
        return False, f"Original Chatterbox checkpoint missing {', '.join(missing)} in {path}"
    if any(importlib.util.find_spec(name) is None for name in ("torch", "torchaudio", "chatterbox")):
        return False, "Python packages missing. Install workers/chatterbox/requirements.txt."
    return True, "Checkpoint and packages found; ready to load on first request."


def generate(request, model_dir):
    global _model, _vc_model
    import torch
    import torchaudio
    from chatterbox.tts import ChatterboxTTS

    if request["category"] == "voice_conversion":
        from chatterbox.vc import ChatterboxVC
        device = "cuda" if torch.cuda.is_available() else "mps" if torch.backends.mps.is_available() else "cpu"
        if _vc_model is None:
            _vc_model = ChatterboxVC.from_local(str(model_dir), device=device)
        try:
            with torch.inference_mode():
                wave = _vc_model.generate(audio=request["sourcePath"], target_voice_path=request["referencePath"])
                result = io.BytesIO()
                torchaudio.save(result, wave.cpu(), _vc_model.sr, format="wav")
                return result.getvalue()
        finally:
            if device == "mps":
                torch.mps.empty_cache()
    if request["category"] != "speech":
        raise ValueError("Original Chatterbox supports speech, not Turbo vocal tags.")
    device = "cuda" if torch.cuda.is_available() else "mps" if torch.backends.mps.is_available() else "cpu"
    if _model is None:
        _model = ChatterboxTTS.from_local(str(model_dir), device=device)
    exaggeration = request.get("exaggeration", 0.5)
    cfg_weight = request.get("cfgWeight", 0.5)
    if type(exaggeration) not in (float, int) or not 0.25 <= exaggeration <= 2:
        raise ValueError("Exaggeration must be between 0.25 and 2.")
    if type(cfg_weight) not in (float, int) or not 0 <= cfg_weight <= 1:
        raise ValueError("Pace must be between 0 and 1.")
    try:
        with torch.inference_mode():
            wave = _model.generate(request["prompt"].strip(), audio_prompt_path=request.get("referencePath"),
                                   exaggeration=exaggeration, cfg_weight=cfg_weight)
            result = io.BytesIO()
            torchaudio.save(result, wave.cpu(), _model.sr, format="wav")
            return result.getvalue()
    finally:
        if device == "mps":
            torch.mps.empty_cache()


if __name__ == "__main__":
    model_dir = os.environ.get("HOMER_CHATTERBOX_ORIGINAL_MODEL_DIR", "")
    if not model_dir:
        raise SystemExit("Set HOMER_CHATTERBOX_ORIGINAL_MODEL_DIR to a local English Chatterbox checkpoint.")
    serve(Worker("chatterbox_original", model_dir, ["speech", "voice_conversion"], generate, check_model),
          int(os.environ.get("HOMER_CHATTERBOX_ORIGINAL_PORT", "8767")))
