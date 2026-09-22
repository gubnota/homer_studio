"""Start installed local sound workers in the background from a repository checkout."""

import json
import os
import subprocess
import sys
import time
from pathlib import Path
from urllib.error import URLError
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parent


def health(port):
    try:
        with urlopen(f"http://127.0.0.1:{port}/v1/health", timeout=2) as response:
            return json.load(response)
    except (OSError, URLError, ValueError):
        return None


def start(name, port, model, python, script):
    current = health(port)
    if current:
        print(f"{name}: already running on port {port}: {current['message']}")
        return
    if not python.is_file():
        print(f"{name}: Python environment missing: {python}")
        return
    if not model.is_dir():
        print(f"{name}: model folder missing: {model}")
        return
    log = ROOT / ("chatterbox" if name == "original" else name) / ("original.log" if name == "original" else "worker.log")
    environment = os.environ.copy()
    environment["HF_HOME"] = str(ROOT / name / ".cache")
    environment["TOKENIZERS_PARALLELISM"] = "false"
    environment[{"chatterbox": "HOMER_CHATTERBOX_MODEL_DIR", "original": "HOMER_CHATTERBOX_ORIGINAL_MODEL_DIR", "sfx": "HOMER_SFX_MODEL_DIR"}[name]] = str(model)
    with log.open("ab") as output:
        subprocess.Popen([str(python), str(script)], cwd=ROOT.parent, env=environment,
                         stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT,
                         start_new_session=True)
    for _ in range(20):
        time.sleep(0.25)
        current = health(port)
        if current:
            print(f"{name}: {'ready' if current['ready'] else 'unavailable'} on port {port}: {current['message']}")
            return
    print(f"{name}: did not start; see {log}")


if __name__ == "__main__":
    requested = set(sys.argv[1:]) or {"chatterbox", "sfx"}
    if not requested <= {"chatterbox", "original", "sfx"}:
        raise SystemExit("Usage: python3 workers/start_local.py [chatterbox] [original] [sfx]")
    if "chatterbox" in requested:
        start("chatterbox", int(os.environ.get("HOMER_CHATTERBOX_PORT", "8765")),
              Path(os.environ.get("HOMER_CHATTERBOX_MODEL_DIR", ROOT / "chatterbox/models/chatterbox-turbo")),
              Path(os.environ.get("HOMER_CHATTERBOX_PYTHON", ROOT / "chatterbox/.venv/bin/python")),
              ROOT / "chatterbox/server.py")
    if "sfx" in requested:
        start("sfx", int(os.environ.get("HOMER_SFX_PORT", "8766")),
              Path(os.environ.get("HOMER_SFX_MODEL_DIR", ROOT / "sfx/models/stable-audio-open-1.0")),
              Path(os.environ.get("HOMER_SFX_PYTHON", ROOT / "sfx/.venv/bin/python")),
              ROOT / "sfx/server.py")
    if "original" in requested:
        start("original", int(os.environ.get("HOMER_CHATTERBOX_ORIGINAL_PORT", "8767")),
              Path(os.environ.get("HOMER_CHATTERBOX_ORIGINAL_MODEL_DIR", ROOT / "chatterbox/models/chatterbox-original")),
              Path(os.environ.get("HOMER_CHATTERBOX_PYTHON", ROOT / "chatterbox/.venv/bin/python")),
              ROOT / "chatterbox/original.py")
