# Homer Studio Linux server

The server runs the same React editor and Rust audio/project services as the macOS app. Run it on a Linux x86_64 server and use a browser on your Mac. Project files, processing, models and generated audio live on the server; browser microphone recordings are uploaded in short chunks. This is a single-owner edition; tabs share projects and revision checks reject conflicting saves.

## Linux application bundle

The application bundle includes the native server, browser editor, workers, sound assets, launcher and container configuration. Extract `homer-studio-0.2.9-linux-x86_64.tar.gz` on Ubuntu 22.04 or newer. Install FFmpeg, Python 3.10 and its venv support (`sudo apt install ffmpeg python3.10 python3.10-venv`). From the extracted directory:

```sh
export HOMER_TOKEN="$(python3 -c 'import secrets; print(secrets.token_hex(32))')"
./run.sh
```

Keep the token and enter it in the browser sign-in dialog. Defaults: `127.0.0.1:8080`, persistent `./data`, packaged `./resources` and `./web`. Restart with the same token and data directory. Back up the data directory while the server is stopped; individual Wave projects can also be exported as `.wavehs` bundles containing media and custom voice references.

For a remote server, forward its localhost port:

```sh
ssh -L 8080:127.0.0.1:8080 your-server
```

Open `http://localhost:8080`. Browser microphone access requires localhost or HTTPS. For a reverse proxy, preserve the original Host header, terminate HTTPS, set `HOMER_SECURE_COOKIE=1`, and protect the server port from direct access. Set `HOMER_BIND=0.0.0.0:8080` only when your network/proxy arrangement requires it. No CORS wildcard or worker port exposure is needed.

## GPU container

Install the NVIDIA driver, NVIDIA Container Toolkit, Docker and Compose on your GPU server. From the extracted archive, set `HOMER_TOKEN`, then run `docker compose up --build -d`. The supplied Compose file reserves a GPU and exposes only the browser port on localhost. Persistent data is stored in the `homer-data` volume. For a CPU container, remove the `deploy.resources.reservations.devices` entry. The container runs as UID 1000; bind-mounted data directories must be writable by that user.

In Settings, install the Chatterbox runtime, then explicitly install Turbo and/or Original model checkpoints. No model weights are included or automatically downloaded. Start the selected worker in Settings. PyTorch chooses CUDA when available, then MPS/CPU. Check worker status before generation. First setup needs internet access and substantial disk space; inference remains on your server.

## Limits and operation

- `HOMER_UPLOAD_LIMIT_BYTES`: maximum uploaded file/recording size, default 10 GiB. Uploads, media ranges, exports and bundles stream from disk.
- Browser recordings enqueue short chunks, with at most 8 MiB waiting for upload; a slow or failed connection stops recording with an error. Incomplete uploads remain in `data/uploads` for recovery/administrative cleanup; never delete that directory during active imports.
- Native desktop recording remains available on macOS. The browser uses its own microphone, not the server's input devices.
- Imports and waveform decoders are bounded; import progress and Cancel appear in Wave Studio. Cancelling a managed local model job may restart its worker and require model reload on the next request.
- The access token controls the entire single-owner server. Settings and tool paths are administration features, not a sandbox for untrusted tenants. HTTP file operations are restricted to the server's data/resources roots.
- File dialogs upload local files. Wave projects restore from the server project list or `.wavehs`; desktop filesystem folder copies remain a macOS option.
- Real CUDA inference throughput, VRAM and microphone hardware behavior require verification on your own host. The Linux release job checks compilation, audio rendering, HTTP authentication, ranges, upload, export and bundle restore without downloading model weights.

## Source development

```sh
npm ci
npm run build
cargo test -p homer-core -p homer-server --features homer-core/server
cargo build --release -p homer-server
HOMER_PACKAGE_PLATFORM=linux-x86_64 scripts/package-server.sh
```

For a checkout run, set `HOMER_WEB_DIR=$PWD/dist`, `HOMER_RESOURCES_DIR` to a directory containing `workers` and `sfx`, `HOMER_DATA_DIR` to an owned data directory and `HOMER_TOKEN` before starting `target/release/homer-server`. See `scripts/test-server.py` for isolated HTTP/media verification.
