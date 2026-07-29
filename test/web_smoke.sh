#!/bin/sh
# Browser smoke test: serve the exported web build locally, load it with the
# ?demo flag in headless Chrome/Chromium, and assert the engine actually
# renders (the loader overlay disappears AND the canvas shows lit pixels).
# This is the gate that catches "wasm builds fine, renders nothing" — the
# failure mode a pure boot check cannot see.
# Usage: web_smoke.sh <build-dir>
set -eu
BUILD="${1:?usage: web_smoke.sh <build-dir>}"
DIR="$(cd "$(dirname "$0")" && pwd)"
PORT="${SMOKE_PORT:-8931}"

CHROME="${CHROME:-}"
if [ -z "$CHROME" ]; then
  for c in "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
           google-chrome chromium-browser chromium; do
    if [ -x "$c" ] || command -v "$c" >/dev/null 2>&1; then CHROME="$c"; break; fi
  done
fi
[ -n "$CHROME" ] || { echo "smoke: no chrome/chromium found — skipping"; exit 0; }
command -v python3 >/dev/null 2>&1 || { echo "smoke: no python3 — skipping"; exit 0; }

python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$BUILD" >/dev/null 2>&1 &
SRV=$!
PROFILE="$(mktemp -d)"
DBG="$((PORT + 1))"
"$CHROME" --headless=new --disable-gpu --enable-unsafe-swiftshader \
  --window-size=640,360 --remote-debugging-port="$DBG" \
  --user-data-dir="$PROFILE" "http://127.0.0.1:$PORT/index.html?demo" >/dev/null 2>&1 &
CHR=$!
trap 'kill "$CHR" "$SRV" 2>/dev/null || true; wait "$CHR" "$SRV" 2>/dev/null || true; sleep 1; rm -rf "$PROFILE" 2>/dev/null || true' EXIT

sleep 3
python3 "$DIR/web_probe.py" "$DBG" "${SMOKE_WAIT:-22}"
