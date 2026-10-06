#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
export HOMER_WEB_DIR="${HOMER_WEB_DIR:-$PWD/web}"
export HOMER_RESOURCES_DIR="${HOMER_RESOURCES_DIR:-$PWD/resources}"
export HOMER_DATA_DIR="${HOMER_DATA_DIR:-$PWD/data}"
: "${HOMER_TOKEN:?Set HOMER_TOKEN to at least 24 random ASCII letters/digits}"
exec ./homer-server
