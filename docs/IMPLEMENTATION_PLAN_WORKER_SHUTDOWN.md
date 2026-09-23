# Local worker shutdown plan

Status: Approved by the user's auto-approval request (2026-09-23).

## Problem

`workers/start_local.py` detaches Python model workers. Closing the Tauri app does not stop them, so their models can remain resident in memory. The observed orphan on this machine is the retired effects worker. Ollama is a separate external service and must not be stopped.

## Changes

1. Add a graceful, loopback-only `/v2/shutdown` operation to the shared Python worker protocol. Complete the HTTP response, stop serving, and exit even if a generation thread is active.
2. On Tauri exit, read the configured Turbo and Original worker URLs, verify each worker's protocol and engine, then request shutdown with short timeouts. Ignore unavailable or older workers without blocking app exit. Do not touch Ollama or arbitrary services on those ports.
3. Remove the retired effects worker from the launcher choices so it cannot be accidentally restarted. Document that manually started Chatterbox workers run for the app session and stop when the app exits.
4. Add focused protocol and client tests. Update the compact architecture/contract/decision/task docs, run the Python and Rust tests, build the renderer, and build a local arm64 Mac bundle. Commit the work; do not push.

## Acceptance

- Closing Homer Studio releases the local Turbo and Original Python workers and their model memory.
- A different service or engine at a configured URL is left untouched.
- Ollama remains under the user's control.
- Shutdown is bounded and the app can still exit if a worker is unresponsive.
