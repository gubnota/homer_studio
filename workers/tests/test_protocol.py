import io
import tempfile
import time
import unittest
import wave
from pathlib import Path

from workers.worker_protocol import Worker
from workers.sfx.server import engine_for


def silence(_request, _model_dir):
    output = io.BytesIO()
    with wave.open(output, "wb") as writer:
        writer.setnchannels(1)
        writer.setsampwidth(2)
        writer.setframerate(48000)
        writer.writeframes(b"\0\0" * 4800)
    return output.getvalue()


class WorkerTests(unittest.TestCase):
    def setUp(self):
        self.folder = tempfile.TemporaryDirectory()
        Path(self.folder.name, "model_index.json").touch()
        self.worker = Worker("stub", self.folder.name, ["sound_effect"], silence)

    def tearDown(self):
        self.folder.cleanup()

    def test_health_and_completed_job(self):
        self.assertTrue(self.worker.health()["ready"])
        job_id = self.worker.start({"prompt": "soft fabric rustle", "category": "sound_effect", "durationSeconds": 2})
        for _ in range(50):
            if self.worker.status(job_id)["status"] == "completed":
                break
            time.sleep(0.01)
        self.assertEqual(self.worker.status(job_id)["status"], "completed")
        self.assertTrue(self.worker.jobs[job_id]["audio"].startswith(b"RIFF"))

    def test_invalid_requests(self):
        for bad in ("", "x" * 501):
            with self.assertRaises(ValueError):
                self.worker.start({"prompt": bad, "category": "sound_effect", "durationSeconds": 2})
        with self.assertRaises(ValueError):
            self.worker.start({"prompt": "fabric", "category": "speech", "durationSeconds": 2})
        with self.assertRaises(ValueError):
            self.worker.start({"prompt": "fabric", "category": "sound_effect", "durationSeconds": 21})

    def test_cancel_discards_output(self):
        def slow(_request, _model_dir):
            time.sleep(0.08)
            return silence(None, None)
        self.worker.generate = slow
        job_id = self.worker.start({"prompt": "fabric", "category": "sound_effect", "durationSeconds": 2})
        self.assertTrue(self.worker.cancel(job_id))
        time.sleep(0.12)
        self.assertEqual(self.worker.status(job_id)["status"], "cancelled")
        self.assertIsNone(self.worker.jobs[job_id]["audio"])

    def test_missing_model_is_unavailable(self):
        missing = Worker("stub", Path(self.folder.name) / "missing", ["sound_effect"], silence)
        self.assertFalse(missing.health()["ready"])
        with self.assertRaises(ValueError):
            missing.start({"prompt": "fabric", "category": "sound_effect", "durationSeconds": 2})

    def test_effect_checkpoint_selects_supported_engine(self):
        model_index = Path(self.folder.name, "model_index.json")
        for checkpoint, engine in (("AudioLDM2Pipeline", "audioldm2"), ("StableAudioPipeline", "stable_audio_open")):
            model_index.write_text('{"_class_name": "' + checkpoint + '"}')
            self.assertEqual(engine_for(Path(self.folder.name)), engine)
        model_index.write_text('{"_class_name": "UnrelatedPipeline"}')
        self.assertIsNone(engine_for(Path(self.folder.name)))


if __name__ == "__main__":
    unittest.main()
