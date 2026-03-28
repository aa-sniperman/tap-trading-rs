#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
RUNTIME_DIR="$ROOT_DIR/runtime"
OUTPUT_ZIP="$ROOT_DIR/pipeline.zip"

cd "$RUNTIME_DIR"
zip -rq "$OUTPUT_ZIP" config.yaml jobs
printf 'Created %s\n' "$OUTPUT_ZIP"
