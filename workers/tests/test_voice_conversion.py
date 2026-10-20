"""Verify speaker conditioning independently of optional AI packages/weights."""
import contextlib
import importlib.util
import sys
import types
import unittest
from pathlib import Path
from unittest.mock import Mock, patch


class VoiceConversionTests(unittest.TestCase):
    def test_each_reference_is_selected_and_builtin_does_not_reuse_previous_speaker(self):
        spec = importlib.util.spec_from_file_location(
            "original_voice_test", Path(__file__).parents[1] / "chatterbox/original.py")
        original = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(original)
        builtin = object()
        calls = []
        model = types.SimpleNamespace(ref_dict=builtin, sr=24000)

        def generate(**args):
            calls.append((args, model.ref_dict))
            model.ref_dict = args["target_voice_path"] or builtin
            if args["target_voice_path"] == "broken.wav":
                raise RuntimeError("bad reference")
            return types.SimpleNamespace(cpu=lambda: "native-wave")

        model.generate = generate
        loader = Mock(return_value=model)
        torch = types.SimpleNamespace(cuda=types.SimpleNamespace(is_available=lambda: False),
                                      backends=types.SimpleNamespace(mps=types.SimpleNamespace(is_available=lambda: False)),
                                      inference_mode=contextlib.nullcontext)
        saves = []

        def save(stream, wave, rate, **options):
            saves.append((wave, rate, options))
            stream.write(b"RIFF-test-WAVE")

        modules = {"torch": torch, "torchaudio": types.SimpleNamespace(save=save),
                   "chatterbox": types.ModuleType("chatterbox"),
                   "chatterbox.tts": types.SimpleNamespace(ChatterboxTTS=object),
                   "chatterbox.vc": types.SimpleNamespace(ChatterboxVC=types.SimpleNamespace(from_local=loader))}
        with patch.dict(sys.modules, modules):
            for reference in ("woman-a.wav", "woman-b.wav", "woman-c.wav", "broken.wav", None):
                request = {"category": "voice_conversion", "sourcePath": "original.wav", "referencePath": reference}
                if reference == "broken.wav":
                    with self.assertRaisesRegex(RuntimeError, "bad reference"):
                        original.generate(request, Path("weights"))
                else:
                    self.assertEqual(original.generate(request, Path("weights")), b"RIFF-test-WAVE")
        loader.assert_called_once_with("weights", device="cpu")
        self.assertEqual([c[0]["target_voice_path"] for c in calls],
                         ["woman-a.wav", "woman-b.wav", "woman-c.wav", "broken.wav", None])
        self.assertTrue(all(c[1] is builtin for c in calls))
        self.assertTrue(all(c[0]["audio"] == "original.wav" for c in calls))
        self.assertEqual(len(saves), 4)
        for wave, rate, options in saves:
            self.assertEqual((wave, rate), ("native-wave", 24000))
            self.assertEqual(options, {"format": "wav", "encoding": "PCM_F", "bits_per_sample": 32})


if __name__ == "__main__":
    unittest.main()
