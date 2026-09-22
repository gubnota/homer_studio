"""Local text-to-sound worker for AudioLDM 2 or Stable Audio Open."""

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
    return {"AudioLDM2Pipeline": "audioldm2", "StableAudioPipeline": "stable_audio_open"}.get(model.get("_class_name"))


def check_model(path):
    if engine_for(path) is None:
        return False, f"AudioLDM 2 or Stable Audio Open Diffusers checkpoint missing in {path}"
    if any(importlib.util.find_spec(name) is None for name in ("torch", "diffusers", "soundfile")):
        return False, "Python packages missing. Install workers/sfx/requirements.txt."
    return True, "Checkpoint and packages found; ready to load on first request."


def generate(request, model_dir):
    global _pipeline
    import numpy as np
    import soundfile as sf
    import torch
    from diffusers import AudioLDM2Pipeline, StableAudioPipeline

    device = "mps" if torch.backends.mps.is_available() else "cpu"
    engine = engine_for(model_dir)
    if _pipeline is None:
        pipeline_type = AudioLDM2Pipeline if engine == "audioldm2" else StableAudioPipeline
        _pipeline = pipeline_type.from_pretrained(str(model_dir), torch_dtype=torch.float32, local_files_only=True)
        if engine == "audioldm2" and not hasattr(_pipeline.language_model, "_get_initial_cache_position"):
            # The published checkpoint names GPT2Model, but current Diffusers expects
            # GPT2LMHeadModel's generation helpers. Its transformer weights are identical.
            from transformers import GPT2LMHeadModel
            language_model = GPT2LMHeadModel(_pipeline.language_model.config)
            language_model.transformer.load_state_dict(_pipeline.language_model.state_dict())
            _pipeline.language_model = language_model
        _pipeline = _pipeline.to(device)
    generator = torch.Generator(device="cpu")
    if request.get("seed") is not None:
        generator.manual_seed(request["seed"])
    options = {"negative_prompt": request.get("negativePrompt") or "Low quality, distorted, music.", "num_inference_steps": 100, "generator": generator}
    if engine == "audioldm2":
        options["audio_length_in_s"] = float(request["durationSeconds"])
    else:
        options["audio_end_in_s"] = float(request["durationSeconds"])
    audio = _pipeline(request["prompt"], **options).audios[0]
    sampling_rate = _pipeline.vocoder.config.sampling_rate if engine == "audioldm2" else _pipeline.vae.sampling_rate
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
        raise SystemExit("Set HOMER_SFX_MODEL_DIR to a local AudioLDM 2 or Stable Audio Open Diffusers checkpoint folder.")
    engine = engine_for(Path(model_dir)) or "sound_effect"
    serve(Worker(engine, model_dir, ["sound_effect"], generate, check_model), int(os.environ.get("HOMER_SFX_PORT", "8766")))
