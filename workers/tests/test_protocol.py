import io
import json
import socket
import tempfile
import threading
import time
import unittest
import wave
from pathlib import Path
from urllib.request import Request, urlopen

from workers.worker_protocol import Worker, serve
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
        self.assertEqual(self.worker.health()["protocolVersion"], 2)
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
        model_index.write_text('{"_class_name": "StableAudioPipeline"}')
        self.assertEqual(engine_for(Path(self.folder.name)), "stable_audio_open")
        model_index.write_text('{"_class_name": "AudioLDM2Pipeline"}')
        self.assertIsNone(engine_for(Path(self.folder.name)))
        model_index.write_text('{"_class_name": "UnrelatedPipeline"}')
        self.assertIsNone(engine_for(Path(self.folder.name)))

    def test_reference_is_transferred_once_and_cleaned_after_generation(self):
        paths = []
        def use_reference(request, _model_dir):
            path = Path(request["referencePath"])
            self.assertTrue(path.is_file())
            paths.append(path)
            return silence(request, _model_dir)
        voice = Worker("chatterbox_turbo", self.folder.name, ["speech"], use_reference)
        with self.assertRaises(ValueError):
            voice.add_reference(b"not a wav")
        reference_id = voice.add_reference(silence(None, None))
        job_id = voice.start({"prompt": "Hello.", "category": "speech", "durationSeconds": 2, "referenceId": reference_id})
        for _ in range(50):
            if voice.status(job_id)["status"] == "completed":
                break
            time.sleep(0.01)
        self.assertEqual(voice.status(job_id)["status"], "completed")
        self.assertFalse(paths[0].exists())
        with self.assertRaises(ValueError):
            voice.start({"prompt": "Hello.", "category": "speech", "durationSeconds": 2, "referenceId": reference_id})

    def test_negative_prompt_rejected_for_speech(self):
        voice = Worker("chatterbox_turbo", self.folder.name, ["speech"], silence)
        with self.assertRaises(ValueError):
            voice.start({"prompt": "Hello.", "category": "speech", "durationSeconds": 2, "negativePrompt": "music"})

    def test_original_speech_accepts_reference_but_not_turbo_gestures(self):
        voice = Worker("chatterbox_original", self.folder.name, ["speech"], silence)
        reference_id = voice.add_reference(silence(None, None))
        job_id = voice.start({"prompt": "Tomorrow.", "category": "speech", "durationSeconds": 2,
                              "referenceId": reference_id, "exaggeration": 0.7, "cfgWeight": 0.4})
        for _ in range(50):
            if voice.status(job_id)["status"] == "completed":
                break
            time.sleep(0.01)
        self.assertEqual(voice.status(job_id)["status"], "completed")
        with self.assertRaises(ValueError):
            voice.start({"prompt": "[sigh]", "category": "vocal_gesture", "durationSeconds": 2})

    def test_voice_conversion_uses_two_recordings_and_cleans_them(self):
        paths = []
        def convert(request, _model_dir):
            paths.extend((Path(request["sourcePath"]), Path(request["referencePath"])))
            self.assertTrue(all(path.is_file() for path in paths))
            return silence(request, _model_dir)
        voice = Worker("chatterbox_original", self.folder.name, ["speech", "voice_conversion"], convert)
        source_id = voice.add_reference(silence(None, None))
        reference_id = voice.add_reference(silence(None, None))
        with self.assertRaises(ValueError):
            voice.start({"prompt": "Convert", "category": "voice_conversion", "durationSeconds": 2, "referenceId": reference_id})
        job_id = voice.start({"prompt": "Convert", "category": "voice_conversion", "durationSeconds": 2,
                              "sourceId": source_id, "referenceId": reference_id})
        for _ in range(50):
            if voice.status(job_id)["status"] == "completed":
                break
            time.sleep(0.01)
        self.assertEqual(voice.status(job_id)["status"], "completed")
        self.assertTrue(all(not path.exists() for path in paths))
        self.assertNotEqual(paths[0], paths[1])

    def test_shutdown_stops_server_and_removes_unused_references(self):
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        voice = Worker("chatterbox_turbo", self.folder.name, ["speech"], silence)
        reference = voice.add_reference(silence(None, None))
        staged = Path(voice.references[reference])
        server_thread = threading.Thread(target=serve, args=(voice, port), daemon=True)
        server_thread.start()
        for _ in range(50):
            try:
                with urlopen(f"http://127.0.0.1:{port}/v2/health", timeout=0.2) as response:
                    self.assertEqual(json.load(response)["engine"], "chatterbox_turbo")
                break
            except OSError:
                time.sleep(0.01)
        else:
            self.fail("worker server did not start")
        request = Request(f"http://127.0.0.1:{port}/v2/shutdown", data=b"", method="POST")
        with urlopen(request, timeout=1) as response:
            self.assertEqual(json.load(response), {"status": "stopping"})
        server_thread.join(timeout=2)
        self.assertFalse(server_thread.is_alive())
        self.assertFalse(staged.exists())


if __name__ == "__main__":
    unittest.main()
