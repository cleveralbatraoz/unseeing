#!/bin/sh
# Deploy = git push to the droplet: its server-side pipeline runs the full
# gauntlet (boot check, unit tests, export, smoke test) and ships only on
# green. A local checks-only run fails fast first, and origin is pushed too
# so GitHub, its CI, and the tags always describe what is live.
set -eu
DIR="$(cd "$(dirname "$0")" && pwd)"

echo "== local checks =="
SKIP_EXPORT=1 "$DIR/ci/pipeline.sh"

echo "== pushing to production (server-side CI takes over) =="
git -C "$DIR" push production main

echo "== pushing to origin =="
git -C "$DIR" push origin main --tags
