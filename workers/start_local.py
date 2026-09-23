"""Start installed local sound workers from bundled code or a development checkout."""

import json
import os
import subprocess
import sys
import time
from pathlib import Path
from urllib.error import URLError
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parent


def app_data_dir():
    configured = os.environ.get("HOMER_APP_DATA_DIR")
    if configured:
        return Path(configured)
    if sys.platform == "darwin":
        return Path.home() / "Library/Application Support/com.gubnota.homerstudio"
    return Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local/share")) / "com.gubnota.homerstudio"


def health(port):
    try:
        with urlopen(f"http://127.0.0.1:{port}/v2/health", timeout=2) as response:
            return json.load(response)
    except (OSError, URLError, ValueError):
        return None


def start(name, port, model, python, script, data_dir):
    current = health(port)
    if current:
        expected = "chatterbox_turbo" if name == "chatterbox" else "chatterbox_original"
        if current.get("protocolVersion") == 2 and current.get("engine") == expected and current.get("ready"):
            print(f"{name}: already running on port {port}: {current.get('message', 'ready')}")
            return True
        print(f"{name}: port {port} is occupied by another or unready worker")
        return False
    if not python.is_file():
        print(f"{name}: Python environment missing: {python}")
        return False
    if not model.is_dir():
        print(f"{name}: model folder missing: {model}")
        return False
    logs = data_dir / "logs"
    logs.mkdir(parents=True, exist_ok=True)
    log = logs / f"{name}.log"
    environment = os.environ.copy()
    cache = data_dir / "cache" / name
    cache.mkdir(parents=True, exist_ok=True)
    environment["HF_HOME"] = str(cache)
    environment["TOKENIZERS_PARALLELISM"] = "false"
    environment[{"chatterbox": "HOMER_CHATTERBOX_MODEL_DIR", "original": "HOMER_CHATTERBOX_ORIGINAL_MODEL_DIR"}[name]] = str(model)
    with log.open("ab") as output:
        subprocess.Popen([str(python), str(script)], cwd=data_dir, env=environment,
                         stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT,
                         start_new_session=True)
    for _ in range(120):
        time.sleep(0.25)
        current = health(port)
        if current:
            print(f"{name}: {'ready' if current.get('ready') else 'unavailable'} on port {port}: {current.get('message', '')}")
            return bool(current.get("ready"))
    print(f"{name}: did not start; see {log}")
    return False


if __name__ == "__main__":
    requested = set(sys.argv[1:]) or {"chatterbox"}
    if not requested <= {"chatterbox", "original"}:
        raise SystemExit("Usage: python3 workers/start_local.py [chatterbox] [original]")
    data_dir = app_data_dir()
    app_models = data_dir / "models"
    def checkpoint(name: str, fallback: Path) -> Path:
        installed = app_models / name
        return installed if installed.is_dir() else fallback
    result = True
    if "chatterbox" in requested:
        result = start("chatterbox", int(os.environ.get("HOMER_CHATTERBOX_PORT", "8765")),
              Path(os.environ.get("HOMER_CHATTERBOX_MODEL_DIR", checkpoint("chatterbox-turbo", ROOT / "chatterbox/models/chatterbox-turbo"))),
              Path(os.environ.get("HOMER_CHATTERBOX_PYTHON", data_dir / "python/chatterbox/bin/python")),
              ROOT / "chatterbox/server.py", data_dir) and result
    if "original" in requested:
        result = start("original", int(os.environ.get("HOMER_CHATTERBOX_ORIGINAL_PORT", "8767")),
              Path(os.environ.get("HOMER_CHATTERBOX_ORIGINAL_MODEL_DIR", checkpoint("chatterbox-original", ROOT / "chatterbox/models/chatterbox-original"))),
              Path(os.environ.get("HOMER_CHATTERBOX_PYTHON", data_dir / "python/chatterbox/bin/python")),
              ROOT / "chatterbox/original.py", data_dir) and result
    if not result:
        raise SystemExit(1)
