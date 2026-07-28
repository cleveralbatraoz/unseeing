#!/bin/sh
# Deploy = git push to the droplet: its server-side pipeline boot-checks the
# Godot project, builds the Web export, and ships it. A local boot check runs
# first to fail fast before the push.
set -eu
DIR="$(cd "$(dirname "$0")" && pwd)"

echo "== local check (build-only) =="
SKIP_EXPORT=1 "$DIR/ci/pipeline.sh"

echo "== pushing to production (server-side CI takes over) =="
git -C "$DIR" push production main
