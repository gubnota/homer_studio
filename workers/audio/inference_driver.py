"""Run a local checkout while enforcing the backend selected by Homer Studio."""
import os
from pathlib import Path
import runpy
import sys

backend, script, *arguments = sys.argv[1:]
import torch
# The local CLIs select their device from these availability methods.
# Scope these overrides to this one inference subprocess.
torch.cuda.is_available = lambda: False
if backend == 'cpu':
    torch.backends.mps.is_available = lambda: False
    torch.backends.mps.is_built = lambda: False
elif backend != 'mps' or not torch.backends.mps.is_available():
    raise RuntimeError('Requested Apple GPU backend is unavailable')
sys.path.insert(0, str(Path(script).resolve().parent))
sys.path.insert(0, os.getcwd())
sys.argv = [script, *arguments]
runpy.run_path(script, run_name='__main__')
