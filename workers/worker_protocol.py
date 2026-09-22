"""Small loopback-only, versioned audio worker protocol shared by both engines."""

import json
import os
import threading
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

MAX_PROMPT = 500
MAX_BODY = 4096
MAX_WAV = 32 * 1024 * 1024


class Worker:
    def __init__(self, engine, model_dir, categories, generate, check_model=None):
        self.engine = engine
        self.model_dir = Path(model_dir)
        self.categories = categories
        self.generate = generate
        self.check_model = check_model
        self.jobs = {}
        self.lock = threading.Lock()
        self.busy = threading.Lock()

    def health(self):
        checked = self.check_model(self.model_dir) if self.check_model else (self.model_dir.is_dir() and any(self.model_dir.iterdir()))
        ready, message = checked if isinstance(checked, tuple) else (checked, "Ready" if checked else f"Model files missing: {self.model_dir}")
        return {
            "protocolVersion": 1,
            "engine": self.engine,
            "model": self.model_dir.name,
            "ready": ready,
            "categories": self.categories,
            "maxDurationSeconds": 20,
            "message": message,
        }

    def start(self, request):
        if not self.health()["ready"]:
            raise ValueError(self.health()["message"])
        prompt = request.get("prompt")
        category = request.get("category")
        duration = request.get("durationSeconds")
        seed = request.get("seed")
        if not isinstance(prompt, str) or not prompt.strip() or len(prompt) > MAX_PROMPT:
            raise ValueError("Prompt must contain 1–500 characters.")
        if category not in self.categories:
            raise ValueError("This engine does not support the selected category.")
        if type(duration) not in (int, float) or not 1 <= duration <= 20:
            raise ValueError("Duration must be between 1 and 20 seconds.")
        if seed is not None and (type(seed) is not int or not 0 <= seed <= 2147483647):
            raise ValueError("Seed must be a non-negative 32-bit integer.")
        with self.lock:
            if any(job["status"] in ("queued", "running") for job in self.jobs.values()):
                raise RuntimeError("Worker is busy.")
            self.jobs = {key: job for key, job in self.jobs.items() if job["status"] in ("queued", "running")}
            job_id = str(uuid.uuid4())
            self.jobs[job_id] = {"status": "queued", "audio": None, "error": None, "cancelled": False}
        threading.Thread(target=self._run, args=(job_id, request), daemon=True).start()
        return job_id

    def _run(self, job_id, request):
        with self.busy:
            with self.lock:
                job = self.jobs[job_id]
                if job["cancelled"]:
                    return
                job["status"] = "running"
            try:
                audio = self.generate(request, self.model_dir)
                if not isinstance(audio, bytes) or not audio.startswith(b"RIFF") or len(audio) > MAX_WAV:
                    raise ValueError("Engine did not return a bounded WAV file.")
                with self.lock:
                    if not job["cancelled"]:
                        job.update(status="completed", audio=audio)
            except Exception as error:
                with self.lock:
                    if not job["cancelled"]:
                        job.update(status="failed", error=str(error)[:500])

    def status(self, job_id):
        with self.lock:
            job = self.jobs.get(job_id)
            if job is None:
                return None
            return {"id": job_id, "status": job["status"], "error": job["error"], "format": "wav"}

    def cancel(self, job_id):
        with self.lock:
            job = self.jobs.get(job_id)
            if job is None:
                return False
            job.update(status="cancelled", cancelled=True, audio=None)
            return True


def serve(worker, port):
    class Handler(BaseHTTPRequestHandler):
        def respond(self, status, payload):
            data = json.dumps(payload).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self):
            path = urlsplit(self.path).path
            if path == "/v1/health":
                return self.respond(200, worker.health())
            parts = path.split("/")
            if len(parts) in (4, 5) and parts[:3] == ["", "v1", "jobs"]:
                job = worker.status(parts[3])
                if job is None:
                    return self.respond(404, {"error": "Unknown job"})
                if len(parts) == 4:
                    return self.respond(200, job)
                if parts[4] == "audio" and job["status"] == "completed":
                    audio = worker.jobs[parts[3]]["audio"]
                    self.send_response(200)
                    self.send_header("Content-Type", "audio/wav")
                    self.send_header("Content-Length", str(len(audio)))
                    self.end_headers()
                    return self.wfile.write(audio)
            self.respond(404, {"error": "Not found"})

        def do_POST(self):
            if urlsplit(self.path).path != "/v1/jobs":
                return self.respond(404, {"error": "Not found"})
            try:
                length = int(self.headers.get("Content-Length", "0"))
                if not 0 < length <= MAX_BODY:
                    raise ValueError("Request is too large or empty.")
                request = json.loads(self.rfile.read(length))
                if not isinstance(request, dict):
                    raise ValueError("Request must be a JSON object.")
                self.respond(202, {"id": worker.start(request)})
            except (ValueError, json.JSONDecodeError) as error:
                self.respond(400, {"error": str(error)})
            except RuntimeError as error:
                self.respond(409, {"error": str(error)})

        def do_DELETE(self):
            parts = urlsplit(self.path).path.split("/")
            if len(parts) == 4 and parts[:3] == ["", "v1", "jobs"] and worker.cancel(parts[3]):
                return self.respond(200, {"status": "cancelled"})
            self.respond(404, {"error": "Unknown job"})

        def log_message(self, fmt, *args):
            print(fmt % args, flush=True)

    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["TRANSFORMERS_OFFLINE"] = "1"
    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
