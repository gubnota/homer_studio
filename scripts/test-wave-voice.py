"""Exercise 20-second conversions through HTTP using a deterministic local worker.

Requires a debug homer-server build and FFmpeg; never reads real projects or models.
"""
import hashlib
import io
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request
import uuid
import wave

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "workers"))
from worker_protocol import Worker, serve


def port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def main():
    ffmpeg = os.environ.get("FFMPEG", "ffmpeg")
    ffprobe = os.environ.get("FFPROBE", "ffprobe")
    worker_port, server_port = port(), port()
    calls = []

    def generate(request, _):
        source = Path(request["sourcePath"])
        info = json.loads(subprocess.check_output([ffprobe, "-v", "error", "-show_streams", "-of", "json", str(source)]))["streams"][0]
        assert info["sample_rate"] == "16000" and info["channels"] == 1
        assert info["codec_name"] == "pcm_f32le" and source.stat().st_size < 2 * 1024 * 1024
        reference = Path(request["referencePath"]).read_bytes()
        calls.append((hashlib.sha256(source.read_bytes()).hexdigest(), hashlib.sha256(reference).hexdigest()))
        # Simulate quiet native model output, retaining delivery and exact duration.
        with tempfile.TemporaryDirectory(prefix="homer-model-output-") as generated:
            output = Path(generated) / "native.wav"
            subprocess.run([ffmpeg, "-v", "error", "-i", str(source), "-af", "volume=0.1", "-ar", "24000", "-c:a", "pcm_f32le", str(output)], check=True)
            return output.read_bytes()

    worker = Worker("chatterbox_original", ROOT, ["speech", "voice_conversion"], generate, lambda _: True)
    threading.Thread(target=serve, args=(worker, worker_port), daemon=True).start()
    with tempfile.TemporaryDirectory(prefix="homer-voice-http-") as directory:
        temp = Path(directory)
        token = secrets.token_hex(24)
        env = {**os.environ, "HOMER_TOKEN": token, "HOMER_BIND": f"127.0.0.1:{server_port}",
               "HOMER_DATA_DIR": str(temp / "data"), "HOMER_RESOURCES_DIR": str(ROOT / "resources"), "HOMER_WEB_DIR": str(ROOT / "dist")}
        with (temp / "server.log").open("wb") as log:
            server = subprocess.Popen([str(ROOT / "target/debug/homer-server")], env=env, stdout=log, stderr=log)
            try:
                def request(path, data, status=200):
                    req = urllib.request.Request(f"http://127.0.0.1:{server_port}" + path,
                          json.dumps(data).encode() if isinstance(data, dict) else data,
                          {"Authorization": f"Bearer {token}", "Content-Type": "application/json" if isinstance(data, dict) else "application/octet-stream"})
                    try:
                        response = urllib.request.urlopen(req, timeout=90)
                    except urllib.error.HTTPError as error:
                        response = error
                    with response:
                        body = response.read()
                        assert response.status == status, (response.status, body[:1000])
                        return json.loads(body)

                def command(command_name, **args):
                    return request("/api/command/" + command_name, args)

                def wait(job_id):
                    for _ in range(600):
                        job = next((j for j in command("list_jobs") if j["id"] == job_id), None)
                        assert not job or job["status"] not in ("failed", "cancelled"), job
                        if job and job["status"] == "completed":
                            return command("wave_processing_result", jobId=job_id)
                        time.sleep(.05)
                    raise AssertionError("Audio job timed out")

                for _ in range(200):
                    assert server.poll() is None, (temp / "server.log").read_text()
                    try:
                        command("wave_list")
                        break
                    except OSError:
                        time.sleep(.05)
                settings = command("get_settings")
                settings["sounds"]["originalUrl"] = f"http://127.0.0.1:{worker_port}"
                command("save_settings", settings=settings)
                project = command("wave_create", name="Voice HTTP regression")
                recording = temp / "original.wav"
                subprocess.run([ffmpeg, "-v", "error", "-f", "lavfi", "-i", "sine=frequency=220:duration=20", str(recording)], check=True)

                def upload(path):
                    return request("/api/upload?name=" + path.name, path.read_bytes())["path"]

                source = command("wave_import", projectId=project["id"], kind="file", path=upload(recording))
                original = {"id": str(uuid.uuid4()), "sourceId": source["id"], "name": "Original", "startMs": 0,
                            "sourceStartMs": 0, "sourceEndMs": 20000, "speed": 1, "gainDb": 0, "fadeInMs": 0, "fadeOutMs": 0}
                project["sources"] = [source]
                project["timeline"]["clips"] = [original]
                region = {"id": str(uuid.uuid4()), "voiceId": "", "name": "Speaker", "color": "#123456", "startMs": 0, "endMs": 20000}
                project["timeline"]["voices"] = [region]
                for index, tone in enumerate((700, 1100)):
                    voice = command("create_voice", name=f"Reference {index}")
                    sample = temp / f"reference-{index}.wav"
                    subprocess.run([ffmpeg, "-v", "error", "-f", "lavfi", "-i", f"sine=frequency={tone}:duration=7", str(sample)], check=True)
                    command("add_voice_sample", voiceId=voice["id"], name=sample.stem, sourcePath=upload(sample))
                    region["voiceId"] = voice["id"]
                    region.pop("production", None)
                    result = wait(command("wave_convert_regions", project=project, regionIds=[region["id"]], regenerate=True, reduceNoise=index == 0))
                    assert len(result["sources"]) == len(result["replacements"]) == 2
                    assert ("withoutCleanup" if index == 0 else "withCleanup") in result
                    project["sources"].extend(result["sources"])
                    project["timeline"]["clips"] = [{**original, "id": str(uuid.uuid4()), "sourceId": r["sourceId"], "startMs": r["startMs"], "sourceEndMs": r["endMs"] - r["startMs"]} for r in result["replacements"]]
                    region["production"] = {"status": "converted", "voiceId": voice["id"], "audioKey": "[]"}
                    region["audio"] = {"original": [original], "versions": [], "activeAudioKey": "[]"}
                    for start in (0, 9500, 10000):
                        data = command("wave_preview", project=project, startMs=start, endMs=start + 10000, requestId=str(uuid.uuid4()))
                        with wave.open(io.BytesIO(bytes(data["binary"]))) as wav:
                            assert wav.getnframes() == 480000 and wav.getsampwidth() == 2
                            assert any(wav.readframes(2000))
                assert len(calls) == 4 and calls[0][0] == calls[2][0] and calls[1][0] == calls[3][0], "Reconversion must use the original recording"
                assert calls[0][1] != calls[2][1], "Different speakers must send different reference samples"
                old = str(uuid.uuid4())
                command("wave_cancel_preview", requestId=old)
                request("/api/command/wave_preview", {"project": project, "startMs": 0, "endMs": 1000, "requestId": old}, status=400)
                command("wave_preview", project=project, startMs=0, endMs=1000, requestId=str(uuid.uuid4()))
                normalized = command("wave_normalize_selection", project=project, clipIds=[], startMs=500, endMs=19500)
                assert len(normalized["baked"]) == 2 and normalized["gainDb"] > 0
                settings["sounds"]["originalUrl"] = "http://127.0.0.1:1"
                command("save_settings", settings=settings)
                repaired = wait(command("wave_match_voice_volume", project=project, regionId=region["id"]))
                assert len(repaired["sources"]) == 2 and len(calls) == 4, "Repair must not call AI"
                print("PASS: HTTP 20s/two-chunk conversions, native float input, distinct references, original reuse, audible previews, scoped cancellation, normalization, and repair without AI")
            finally:
                server.terminate()
                server.wait(timeout=10)


if __name__ == "__main__":
    main()
