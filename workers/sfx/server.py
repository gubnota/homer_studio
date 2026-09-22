"""Local Stable Audio Open text-to-sound worker."""

import io
import importlib.util
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from worker_protocol import Worker, serve

_pipeline = None


def check_model(path):
    if not (path / "model_index.json").is_file():
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
    audio = _pipeline(
        request["prompt"],
        negative_prompt="Low quality, distorted.",
        num_inference_steps=100,
        audio_end_in_s=float(request["durationSeconds"]),
        generator=generator,
    ).audios[0]
    output = io.BytesIO()
    sf.write(output, np.asarray(audio.T.cpu(), dtype="float32"), _pipeline.vae.sampling_rate, format="WAV")
    return output.getvalue()


if __name__ == "__main__":
    model_dir = os.environ.get("HOMER_SFX_MODEL_DIR", "")
    if not model_dir:
        raise SystemExit("Set HOMER_SFX_MODEL_DIR to a local Stable Audio Open Diffusers checkpoint folder.")
    serve(Worker("stable_audio_open", model_dir, ["sound_effect"], generate, check_model), int(os.environ.get("HOMER_SFX_PORT", "8766")))
