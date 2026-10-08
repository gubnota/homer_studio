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
                    progress_seen = set()
                    for _ in range(3000):
                        job = next((j for j in command("list_jobs") if j["id"] == job_id), None)
                        if job:
                            progress_seen.add(job.get("progress", 0))
                        if job and job["status"] == "completed":
                            return progress_seen
                        assert not job or job["status"] not in ("failed", "cancelled"), job
                        time.sleep(.1)
                    raise RuntimeError("Job timed out")

                for extension in ("wav", "m4a"):
                    output = json.loads(request("/api/destination", {"name": "mix." + extension})[0])["path"]
                    wait_job(command("wave_export", project=project, outputPath=output))
                    media = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-show_format", "-show_streams", "-of", "json", output]))
                    assert abs(float(media["format"]["duration"]) - 2) < .05
                    assert media["streams"][0]["channels"] == 2
                    if extension == "m4a":
                        assert media["streams"][0]["codec_name"] == "aac"
                        assert media["streams"][0]["channel_layout"] == "stereo"
                    downloaded, h = request("/api/download?path=" + urllib.parse.quote(output))
                    assert len(downloaded) > 1000 and "attachment" in h["Content-Disposition"]
                # Contrasting clips verify combined timeline frame selection and held gaps.
                montage = command("wave_create", name="Video timeline fixture")
                montage["sources"] = [audio]
                montage["timeline"]["clips"] = [clip]
                montage["timeline"]["sfx"] = [{**clip, "id": str(uuid.uuid4()), "startMs": 6000}]
                fragments = []
                for color, codec, at in [("red", "libx264", 500), ("blue", "mpeg4", 3000)]:
                    fixture = temp / (color + ".mp4")
                    subprocess.run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i", f"color=c={color}:s=320x240:r=30:d=1", "-c:v", codec, "-pix_fmt", "yuv420p", str(fixture)], check=True)
                    uploaded = json.loads(request("/api/upload?name=" + fixture.name, fixture.read_bytes(), method="POST")[0])["path"]
                    started = time.monotonic()
                    fragment = command("wave_import_video", path=uploaded, requestId=str(uuid.uuid4()))
                    print(f"Video import {codec}: {time.monotonic()-started:.3f}s (one-second fixture)")
                    fragment["startMs"] = at
                    fragments.append(fragment)
                montage["videos"] = fragments
                bad = {**montage, "videos": [fragments[0], {**fragments[1], "startMs": 600}]}
                request("/api/command/wave_save", {"project": bad, "expectedRevision": 0}, status=400)
                request("/api/command/wave_export", {"project": bad, "outputPath": str(temp / "data/exports/bad.mp4"), "videoTimeline": True}, status=400)
                montage = command("wave_save", project=montage, expectedRevision=0)
                combined = json.loads(request("/api/destination", {"name": "timeline.mp4"})[0])["path"]
                wait_job(command("wave_export", project=montage, outputPath=combined, videoTimeline=True))
                measured = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-show_format", "-show_streams", "-of", "json", combined]))
                assert abs(float(measured["format"]["duration"]) - 8) < .15
                assert any(stream["codec_type"] == "audio" and stream["channels"] == 2 for stream in measured["streams"])
                def pixel(at):
                    return subprocess.check_output(["ffmpeg", "-v", "error", "-ss", str(at), "-i", combined, "-frames:v", "1", "-vf", "scale=1:1", "-pix_fmt", "rgb24", "-f", "rawvideo", "-"])
                assert max(pixel(.1)) < 10, "Leading gap must be black"
                for at in (.7, 2.5):
                    r,g,b = pixel(at); assert r > 180 and b < 30, (at,r,g,b)
                for at in (3.2, 7.5):
                    r,g,b = pixel(at); assert b > 180 and r < 30, (at,r,g,b)
                import struct
                gap_audio = subprocess.check_output(["ffmpeg", "-v", "error", "-ss", "4.5", "-i", combined, "-t", "0.2", "-f", "f32le", "-acodec", "pcm_f32le", "-"])
                assert max(abs(v[0]) for v in struct.iter_unpack("<f", gap_audio)) < .001
                # Explicit length choices: whole timeline and offset selected clip.
                request("/api/command/wave_export", {"project": montage, "outputPath": combined, "lengthMode": "invalid", "videoTimeline": True}, status=400)
                for selection, fixture_project, mode, expected, frame_at, color in [
                    (None,montage,"longest",8,7.5,"blue"),
                    (None,montage,"shortest",4,3.8,"blue"),
                    (fragments[0]["id"],montage,"longest",7.5,7,"red"),
                    (fragments[0]["id"],montage,"shortest",1,.8,"red"),
                    (None,{**montage,"timeline":{**montage["timeline"],"sfx":[]}},"shortest",2,1.8,"red"),
                    (None,{**montage,"timeline":{**montage["timeline"],"sfx":[]}},"longest",4,3.8,"blue"),
                    (fragments[1]["id"],{**montage,"timeline":{**montage["timeline"],"sfx":[]}},"longest",1,.8,"blue"),
                ]:
                    output=json.loads(request("/api/destination",{"name":str(uuid.uuid4())+".mp4"})[0])["path"]
                    wait_job(command("wave_export",project=fixture_project,outputPath=output,videoId=selection,videoTimeline=selection is None,lengthMode=mode))
                    media=json.loads(subprocess.check_output(["ffprobe","-v","error","-show_format","-show_streams","-of","json",output]))
                    assert abs(float(media["format"]["duration"])-expected)<.15,media
                    assert all(abs(float(stream["duration"])-expected)<.15 for stream in media["streams"]),media
                    r,g,b=subprocess.check_output(["ffmpeg","-v","error","-ss",str(frame_at),"-i",output,"-frames:v","1","-vf","scale=1:1","-pix_fmt","rgb24","-f","rawvideo","-"])
                    assert (r>180 and b<30) if color=="red" else (b>180 and r<30),(r,g,b)
                    if fixture_project["timeline"]["sfx"]==[] and mode=="longest":
                        raw=subprocess.check_output(["ffmpeg","-v","error","-ss",str(expected-.3),"-i",output,"-t","0.2","-f","f32le","-acodec","pcm_f32le","-"])
                        assert raw and max(abs(v[0]) for v in struct.iter_unpack("<f",raw))<.001
                print("Export length choices passed: full/shorter timeline and selected video, held frames, padded silence and per-stream durations.")
                # Moving footage proves the tail holds its actual final image, not an arbitrary frame.
                moving=temp/"moving.mp4"
                subprocess.run(["ffmpeg","-v","error","-f","lavfi","-i","testsrc2=s=320x180:r=60:d=1.35","-an","-c:v","libx264","-pix_fmt","yuv420p",str(moving)],check=True)
                moving_uploaded=json.loads(request("/api/upload?name=moving.mp4",moving.read_bytes(),method="POST")[0])["path"]
                moving_video=command("wave_import_video",path=moving_uploaded,requestId=str(uuid.uuid4()))
                moving_video["startMs"]=0
                moving_project={**montage,"videos":[moving_video]}
                def image_bytes(path,at):
                    return subprocess.check_output(["ffmpeg","-v","error","-ss",str(at),"-i",str(path),"-frames:v","1","-vf","scale=160:90","-pix_fmt","rgb24","-f","rawvideo","-"])
                final=image_bytes(moving,1.333)
                earlier=image_bytes(moving,.2)
                for selected in [None,moving_video["id"]]:
                    output=json.loads(request("/api/destination",{"name":str(uuid.uuid4())+".mp4"})[0])["path"]
                    wait_job(command("wave_export",project=moving_project,outputPath=output,videoId=selected,videoTimeline=selected is None,lengthMode="longest"))
                    media=json.loads(subprocess.check_output(["ffprobe","-v","error","-show_streams","-of","json",output]))
                    assert all(abs(float(stream["duration"])-8)<.15 for stream in media["streams"]),media
                    first_tail=image_bytes(output,2)
                    last_tail=image_bytes(output,7.8)
                    # Allow small lossy H.264 quantization differences across held frames.
                    difference=lambda a,b:sum(abs(x-y) for x,y in zip(a,b))/len(a)
                    assert len(last_tail)==len(final)==43200
                    assert difference(first_tail,last_tail)<2, ("Held tail changed its image",selected,difference(first_tail,last_tail),difference(final,last_tail))
                    assert difference(final,last_tail)<5, "Tail did not preserve the last source frame"
                    assert difference(earlier,last_tail)>10, "Moving fixture must distinguish earlier frames"
                print("Moving 60fps footage passed final-frame hold and both-stream length checks for selected/combined exports.")
                # Split/copy placements share the original asset and preserve source offsets across folder saves.
                cut={**moving_video,"id":str(uuid.uuid4()),"assetId":moving_video["id"],"sourceStartMs":500,"sourceDurationMs":moving_video["durationMs"],"durationMs":500,"startMs":0}
                copied={**cut,"id":str(uuid.uuid4()),"startMs":1000}
                cut_project=command("wave_create",name="Trimmed video copy")
                cut_project.update(sources=[audio],videos=[cut,copied],timeline={**cut_project["timeline"],"clips":[clip,{**clip,"id":str(uuid.uuid4()),"startMs":500}]})
                cut_project=command("wave_save",project=cut_project,expectedRevision=0)
                assert len(cut_project["timeline"]["clips"])==2, "Overlapping narration must save without ripple"
                cut_folder=str(temp/"data/exports/cut.wavehs")
                command("wave_save_copy",project=cut_project,path=cut_folder,includeVideo=True)
                reopened=command("wave_import_bundle",path=cut_folder)
                assert [(v["id"],v["sourceStartMs"],v["durationMs"],v["startMs"]) for v in reopened["videos"]]==[(v["id"],500,500,v["startMs"]) for v in cut_project["videos"]]
                for selected in [None,reopened["videos"][0]["id"]]:
                    output=str(temp/"data/exports"/(str(uuid.uuid4())+".mp4"))
                    wait_job(command("wave_export",project=reopened,outputPath=output,videoId=selected,videoTimeline=selected is None,lengthMode="longest"))
                    assert difference(image_bytes(moving,.983),image_bytes(output,2.3))<6,"Trimmed video tail must use the trimmed final frame"
                print("Overlapping narration and split/copied video passed folder round-trip and selected/combined exports.")
                # Exercise the reported 4K HEVC file without touching the user's saved project.
                actual=Path("/Users/vm/Downloads/2026-10-08 14.26.46_apo8_thf4.mp4")
                if actual.is_file():
                    uploaded=json.loads(request("/api/upload?name=actual.mp4",actual.read_bytes(),method="POST")[0])["path"]
                    actual_video=command("wave_import_video",path=uploaded,requestId=str(uuid.uuid4()))
                    actual_project=command("wave_create",name="4K export regression")
                    actual_project.update(sources=[audio],videos=[actual_video],timeline={**actual_project["timeline"],"clips":[clip],"sfx":[{**clip,"id":str(uuid.uuid4()),"startMs":28660.609}]})
                    for selected in [None,actual_video["id"]]:
                        output=str(temp/"data/exports"/(str(uuid.uuid4())+".mp4"))
                        observed=wait_job(command("wave_export",project=actual_project,outputPath=output,videoId=selected,videoTimeline=selected is None,lengthMode="longest"))
                        media=json.loads(subprocess.check_output(["ffprobe","-v","error","-show_streams","-of","json",output]))
                        assert all(abs(float(stream["duration"])-30.660609)<.15 for stream in media["streams"]),media
                        assert any(30<p<95 for p in observed),observed
                        assert difference(image_bytes(output,29.6),image_bytes(output,30.5))<2
                        print("Actual 4K HEVC keep-full-length export passed:","selected" if selected else "combined", "progress",sorted(observed))
                long_project = {**montage, "timeline": {**montage["timeline"], "sfx": [{**clip, "id": str(uuid.uuid4()), "startMs": 118000}]}}
                cancelled_output = str(temp / "data/exports/cancelled.mp4")
                cancelled_job = command("wave_export", project=long_project, outputPath=cancelled_output, videoTimeline=True)
                command("control_job", jobId=cancelled_job, action="cancel")
                for _ in range(100):
                    status = next(j for j in command("list_jobs") if j["id"] == cancelled_job)["status"]
                    if status == "cancelled": break
                    time.sleep(.1)
                assert status == "cancelled" and not Path(cancelled_output).exists()
                user_audio = Path("/Users/vm/Downloads/2026-10-08 14.26.46.m4a")
                if user_audio.is_file():
                    uploaded = json.loads(request("/api/upload?name=user.m4a", user_audio.read_bytes(), method="POST")[0])["path"]
                    imported = command("wave_import", kind="file", path=uploaded, requestId=str(uuid.uuid4()))
                    assert abs(imported["durationMs"] - 29312) < 150
                    print("User M4A import passed: 29.312 seconds")
                print("Combined video integration passed: overlap rejection, black lead, held gaps/tail, edited stereo audio and cancellation.")
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
                memo = command("get_memo", memoId=memo["id"])
                take = next(t for t in memo["takes"] if t["id"] == memo["selectedTakeId"])
                removed = command("update_memo_take", memoId=memo["id"], expectedRevision=memo["revision"],
                                  takeId=take["id"], name=take["name"], notes="", favorite=False, deleted=True)
                assert removed["selectedTakeId"] is None and removed["takes"][0]["state"] == "rejected"
                scoped = command("create_memo", name="Project recording", context={"projectId": project["id"], "contextType": "wave-project"})
                assert scoped["projectId"] == project["id"]
                assert all(m["id"] != scoped["id"] for m in command("list_memos", includeDeleted=True))
                # Folder saves reuse media; opening the same folder does not multiply project audio.
                folder = temp / "data/exports/folder.wavehs"
                command("wave_save_copy", project=project, path=str(folder), includeVideo=False)
                stored = folder / "media" / (audio["id"] + ".wav")
                before = stored.stat().st_mtime_ns
                project["name"] = "Incrementally saved"
                project = command("wave_save", project=project, expectedRevision=project["revision"])
                assert stored.stat().st_mtime_ns == before
                assert json.loads((folder / "project.json").read_text())["name"] == project["name"]
                opened = command("wave_import_bundle", path=str(folder))
                assert opened["id"] == project["id"]
                assert command("wave_import_bundle", path=str(folder))["id"] == opened["id"]
                # Video owns duration: two seconds of audio plus three seconds of silence.
                video_file = temp / "video.mp4"
                subprocess.run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i", "color=c=blue:s=160x90:r=10:d=5",
                                "-f", "lavfi", "-i", "sine=frequency=900:duration=5", "-c:v", "libx264", "-c:a", "aac", str(video_file)], check=True)
                video_path = json.loads(request("/api/upload?name=video.mp4", video_file.read_bytes(), method="POST")[0])["path"]
                video = command("wave_import_video", path=video_path, requestId=str(uuid.uuid4()))
                project["videos"] = [video]
                project = command("wave_save", project=project, expectedRevision=project["revision"])
                assert not list((folder / "media").glob("*.mp4")), "Default folder save must link video"
                output = json.loads(request("/api/destination", {"name": "synced.mp4"})[0])["path"]
                wait_job(command("wave_export", project=project, outputPath=output, videoId=video["id"]))
                media = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-show_format", "-show_streams", "-of", "json", output]))
                assert abs(float(media["format"]["duration"]) - 5) < .15
                silence = subprocess.check_output(["ffmpeg", "-v", "error", "-ss", "3", "-i", output, "-t", "1", "-f", "f32le", "-ac", "1", "-"])
                import array
                samples = array.array("f"); samples.frombytes(silence)
                assert samples and max(abs(v) for v in samples) < .001, "Original video audio must be replaced with silence in gaps"
                bundled_folder = temp / "data/exports/with-video.wavehs"
                command("wave_save_copy", project=project, path=str(bundled_folder), includeVideo=True)
                assert (bundled_folder / "media" / (video["id"] + ".mp4")).read_bytes() == video_file.read_bytes()

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
