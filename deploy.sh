#!/bin/sh
# Deploy = git push to the droplet: its server-side pipeline tests and ships.
# Runs the suite locally first to fail fast before the push.
set -eu
DIR="$(cd "$(dirname "$0")" && pwd)"

echo "== local tests =="
"$DIR/test/run.sh"

echo "== pushing to production (server-side CI takes over) =="
git -C "$DIR" push production main
