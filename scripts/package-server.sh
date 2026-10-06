#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
version=$(node -p 'require("./package.json").version')
platform=${HOMER_PACKAGE_PLATFORM:-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)}
stage="target/server-package/homer-studio-${version}-${platform}"
rm -rf "$stage"
mkdir -p "$stage/resources" "$stage/web"
cp target/release/homer-server "$stage/"
cp -R dist/. "$stage/web/"
cp -R workers "$stage/resources/"
cp -R resources/sfx "$stage/resources/"
find "$stage" -name __pycache__ -type d -prune -exec rm -rf '{}' +
find "$stage" -name '* (1).*' -delete
cp server/run.sh server/Dockerfile server/compose.yaml "$stage/"
cp docs/LINUX_SERVER.md "$stage/README.md"
chmod +x "$stage/run.sh"
mkdir -p release-artifacts
tar -czf "release-artifacts/homer-studio-${version}-${platform}.tar.gz" -C target/server-package "$(basename "$stage")"
