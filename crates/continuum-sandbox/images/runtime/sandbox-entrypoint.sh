#!/usr/bin/env bash
set -euo pipefail

WORKDIR="${1:-/workspace}"
cd "$WORKDIR"

if [ -f "Cargo.toml" ]; then
    cargo check --workspace 2>&1 || true
elif [ -f "package.json" ]; then
    npm install --prefix "$WORKDIR" 2>&1 || true
elif [ -f "requirements.txt" ]; then
    pip install -r requirements.txt 2>&1 || true
elif [ -f "pyproject.toml" ]; then
    pip install -e . 2>&1 || true
fi

exec "$@"
