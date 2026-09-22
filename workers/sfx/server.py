"""Local text-to-sound worker for Stable Audio Open."""

import io
import importlib.util
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from worker_protocol import Worker, serve

_pipeline = None


def engine_for(path):
    try:
        model = json.loads((path / "model_index.json").read_text())
    except (OSError, ValueError):
        return None
    return "stable_audio_open" if model.get("_class_name") == "StableAudioPipeline" else None


def check_model(path):
    if engine_for(path) is None:
        return False, f"Stable Audio Open Diffusers checkpoint missing in {path}"
    if any(importlib.util.find_spec(name) is None for name in ("torch", "diffusers", "soundfile")):
        return False, "Python packages missing. Install workers/sfx/requirements.txt."
    return True, "Checkpoint and packages found; ready to load on first request."


def generate(request, model_dir):
    global _pipeline
    import numpy as np
    import soundfile as sf
    import torch
    from diffusers import StableAudioPipeline

    device = "mps" if torch.backends.mps.is_available() else "cpu"
    if _pipeline is None:
        _pipeline = StableAudioPipeline.from_pretrained(str(model_dir), torch_dtype=torch.float32, local_files_only=True)
        _pipeline = _pipeline.to(device)
    generator = torch.Generator(device="cpu")
    if request.get("seed") is not None:
        generator.manual_seed(request["seed"])
    options = {"negative_prompt": request.get("negativePrompt") or "Low quality, distorted, music.", "num_inference_steps": 100, "generator": generator}
    options["audio_end_in_s"] = float(request["durationSeconds"])
    audio = _pipeline(request["prompt"], **options).audios[0]
    sampling_rate = _pipeline.vae.sampling_rate
    output = io.BytesIO()
    if hasattr(audio, "cpu"):
        audio = audio.cpu().numpy()
    audio = np.asarray(audio, dtype="float32").squeeze()
    if audio.ndim == 2 and audio.shape[0] <= 2:
        audio = audio.T
    sf.write(output, audio, sampling_rate, format="WAV")
    return output.getvalue()


if __name__ == "__main__":
    model_dir = os.environ.get("HOMER_SFX_MODEL_DIR", "")
    if not model_dir:
        raise SystemExit("Set HOMER_SFX_MODEL_DIR to a local Stable Audio Open Diffusers checkpoint folder.")
    engine = engine_for(Path(model_dir)) or "sound_effect"
    serve(Worker(engine, model_dir, ["sound_effect"], generate, check_model), int(os.environ.get("HOMER_SFX_PORT", "8766")))
