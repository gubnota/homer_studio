"""Exercise the packaged HTTP backend with isolated data and real FFmpeg media."""
import json
import gzip
import os
from pathlib import Path
import secrets
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[1]


def main():
    with tempfile.TemporaryDirectory(prefix="homer-server-test-") as directory:
        temp = Path(directory)
        resources = temp / "resources"
        resources.mkdir()
        (resources / "workers").symlink_to(ROOT / "workers", target_is_directory=True)
        (resources / "sfx").symlink_to(ROOT / "resources/sfx", target_is_directory=True)
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            port = sock.getsockname()[1]
        token = secrets.token_hex(24)
        base = f"http://127.0.0.1:{port}"
        env = {**os.environ, "HOMER_TOKEN": token, "HOMER_BIND": f"127.0.0.1:{port}",
               "HOMER_DATA_DIR": str(temp / "data"), "HOMER_RESOURCES_DIR": str(resources),
               "HOMER_WEB_DIR": str(ROOT / "dist")}
        with (temp / "server.log").open("wb") as log:
            server = subprocess.Popen([str(ROOT / "target/release/homer-server")], env=env, stdout=log, stderr=log)
            try:
                def request(path, data=None, method=None, auth=True, headers=None, status=200):
                    body = json.dumps(data).encode() if isinstance(data, dict) else data
                    h = {"Authorization": f"Bearer {token}"} if auth else {}
                    if isinstance(data, dict):
                        h["Content-Type"] = "application/json"
                    h.update(headers or {})
                    req = urllib.request.Request(base + path, body, h, method=method)
                    try:
                        response = urllib.request.urlopen(req, timeout=90)
                    except urllib.error.HTTPError as error:
                        response = error
                    with response:
                        content = response.read()
                        assert response.status == status, (path, response.status, content[:1000])
                        return content, response.headers

                def command(operation, **args):
                    return json.loads(request("/api/command/" + operation, args)[0])

                for _ in range(200):
                    if server.poll() is not None:
                        raise RuntimeError((temp / "server.log").read_text())
                    try:
                        request("/")
                        break
                    except OSError:
                        time.sleep(.1)
                else:
                    raise RuntimeError("Server did not start")
                request("/api/command/wave_list", {}, auth=False, status=401)
                request("/api/command/wave_list", {}, headers={"Origin": "https://evil.example"}, status=403)
                request("/api/session", {"token": "wrong"}, auth=False, status=401)
                _, headers = request("/api/session", {"token": token}, auth=False)
                assert "HttpOnly" in headers["Set-Cookie"] and "SameSite=Strict" in headers["Set-Cookie"]
                request("/api/command/read_manuscript", {"path": "/etc/passwd"}, status=400)
                source = temp / "fixture.mp3"
                subprocess.run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i", "sine=frequency=440:duration=2", str(source)], check=True)
                upload = json.loads(request("/api/upload?name=fixture.mp3", source.read_bytes(), method="POST")[0])["path"]
                project = command("wave_create", name="HTTP integration fixture")
                audio = command("wave_import", kind="file", path=upload, requestId=str(uuid.uuid4()))
                assert 1900 < audio["durationMs"] < 2200
                clip = {"id": str(uuid.uuid4()), "sourceId": audio["id"], "name": audio["name"],
                        "startMs": 0, "sourceStartMs": 0, "sourceEndMs": 2000, "speed": 1,
                        "gainDb": 0, "fadeInMs": 0, "fadeOutMs": 0}
                project["sources"] = [audio]
                project["timeline"]["clips"] = [clip]
                project = command("wave_save", project=project, expectedRevision=0)
                assert command("wave_get", id=project["id"])["timeline"]["clips"] == [clip]
                peaks = command("wave_peaks", sourceId=audio["id"], startMs=0, endMs=2000, maxPeaks=100, requestId=str(uuid.uuid4()))
                assert peaks and any(peaks.values())
                url = command("wave_source_url", sourceId=audio["id"])
                content, _ = request(url, headers={"Range": "bytes=0-15"}, status=206)
                assert len(content) == 16 and content.startswith(b"RIFF")
                request(url, headers={"Range": "bytes=999999999-"}, status=416)
                preview = command("wave_preview", project=project, startMs=0, endMs=1000)
                assert bytes(preview["binary"][:4]) == b"RIFF"

                def wait_job(job_id):
                    for _ in range(600):
                        job = next((j for j in command("list_jobs") if j["id"] == job_id), None)
                        if job and job["status"] == "completed":
                            return
                        assert not job or job["status"] not in ("failed", "cancelled"), job
                        time.sleep(.1)
                    raise RuntimeError("Job timed out")

                for extension in ("wav", "m4a"):
                    output = json.loads(request("/api/destination", {"name": "mix." + extension})[0])["path"]
                    wait_job(command("wave_export", project=project, outputPath=output))
                    media = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-show_format", "-show_streams", "-of", "json", output]))
                    assert abs(float(media["format"]["duration"]) - 2) < .05
                    assert media["streams"][0]["channels"] == 2
                    assert media["streams"][0]["channel_layout"] == "stereo"
                    if extension == "m4a":
                        assert media["streams"][0]["codec_name"] == "aac"
                    downloaded, h = request("/api/download?path=" + urllib.parse.quote(output))
                    assert len(downloaded) > 1000 and "attachment" in h["Content-Disposition"]
                # Keep an inactive accepted voice too; every cached source/voice must remap.
                alex = command("create_voice", name="Alex cache fixture")
                john = command("create_voice", name="John active fixture")
                production = {"status": "converted", "voiceId": john["id"], "audioKey": json.dumps([audio["id"]])}
                region = {"id": str(uuid.uuid4()), "voiceId": john["id"], "name": "John", "color": "#112233", "startMs": 0, "endMs": 2000, "production": production,
                          "audio": {"original": [clip], "activeAudioKey": json.dumps([audio["id"]]), "versions": [
                              {"voiceId": alex["id"], "clips": [clip], "production": {**production, "voiceId": alex["id"]}},
                              {"voiceId": john["id"], "clips": [clip], "production": production}]}}
                project["timeline"]["voices"] = [region]
                project = command("wave_save", project=project, expectedRevision=project["revision"])
                bundle = json.loads(request("/api/destination", {"name": "project.wavehs"})[0])["path"]
                command("wave_export_bundle", project=project, path=bundle)
                restored = command("wave_import_bundle", path=bundle)
                assert restored["id"] != project["id"]
                assert restored["sources"][0]["id"] != audio["id"]
                assert restored["timeline"]["clips"][0]["sourceId"] == restored["sources"][0]["id"]
                document = Path(bundle).read_bytes()
                assert document.startswith(b"WAVEHS02")
                copied = restored["timeline"]["voices"][0]
                assert copied["voiceId"] != john["id"]
                assert copied["audio"]["versions"][0]["voiceId"] != alex["id"]
                assert copied["production"]["voiceId"] == copied["voiceId"]
                for snapshot in [copied["audio"]["original"], *[v["clips"] for v in copied["audio"]["versions"]]]:
                    assert snapshot[0]["sourceId"] == restored["sources"][0]["id"]
                copied_url = command("wave_source_url", sourceId=restored["sources"][0]["id"])
                assert request(url)[0] == request(copied_url)[0], "Bundle import must preserve exact WAV bytes"
                assert audio["id"] not in copied["audio"]["activeAudioKey"]
                # An old uncompressed binary document remains readable.
                legacy = temp / "data/uploads/legacy.wavehs"
                legacy.parent.mkdir(parents=True, exist_ok=True)
                legacy.write_bytes(b"WAVEHS01" + gzip.decompress(document[8:]))
                old = command("wave_import_bundle", path=str(legacy), requestId=str(uuid.uuid4()))
                assert len(old["timeline"]["voices"][0]["audio"]["versions"]) == 2
                assert len(command("wave_sfx_list")) == 11
                recording = temp / "recording.webm"
                subprocess.run(["ffmpeg", "-v", "error", "-i", str(source), "-c:a", "libopus", str(recording)], check=True)
                path = json.loads(request("/api/recording", {"name": "recording.webm"})[0])["path"]
                data = recording.read_bytes()
                for offset in range(0, len(data), 1024):
                    request("/api/recording?path=" + urllib.parse.quote(path), data[offset:offset+1024], method="PUT", status=204)
                assert Path(path).read_bytes() == data
                memo = command("create_memo", name="Browser recording fixture", context={})
                wait_job(command("import_memo_audio", memoId=memo["id"], expectedRevision=memo["revision"], sourcePath=path, name="Recording"))
                assert command("get_memo", memoId=memo["id"])["selectedTakeId"]
                # Restore/export uses the saved project, including its timeline and media.
                command("wave_delete", id=project["id"], restore=False)
                assert any(p["id"] == project["id"] for p in command("wave_deleted"))
                command("wave_delete", id=project["id"], restore=True)
                recovered = command("wave_get", id=project["id"])
                assert recovered["timeline"]["clips"] == [clip]
                destination = json.loads(request("/api/destination", {"name": "restored.wavehs"})[0])["path"]
                command("wave_export_bundle", project=recovered, path=destination)
                restored = command("wave_import_bundle", path=destination)
                assert len(restored["timeline"]["clips"]) == 1 and len(restored["sources"]) == 1
                command("wave_delete", id=project["id"], restore=False)
                assert command("wave_purge", id=project["id"]) == 1
                assert command("wave_get", id=restored["id"])["sources"]
                command("wave_delete", id=restored["id"], restore=False)
                assert command("wave_purge") == 1
                assert command("wave_deleted") == []
                assert request(url, headers={"Range": "bytes=0-3"}, status=206)[0] == b"RIFF"
                print("HTTP integration passed: authentication, paths, MP3/peaks, ranged playback, WAV/M4A, compressed/legacy bundles with voice versions, SFX and streamed recording.")
            finally:
                server.terminate()
                try:
                    server.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    server.kill()
                    server.wait()


if __name__ == "__main__":
    main()
