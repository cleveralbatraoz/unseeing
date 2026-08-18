#!/bin/sh
# Measure the WEB target's platform facts — the last two the renderer's
# derivations rest on that nobody had measured.
#
# rust/src/render/channel.rs pins CHANNEL_LEVELS from a DESKTOP measurement,
# and the B-channel reconstruction guard turns on it: at 8 bits that guard is
# broken four times over. game/shaders/hearing_post.gdshader ORs its exact
# depth-based layer test with the older wall-table inference for the same
# reason — the depth texture is measured live on desktop GL and was unknown
# on WebGL2, so the OR was the only shape that could not regress there. Both
# notes said "unmeasured on the web". This is that measurement.
#
# It reuses the machinery test/web_smoke.sh already relies on: serve the
# export over plain HTTP, drive headless Chrome through the DevTools
# protocol, read what the page reports. The probe scene is built so one frame
# carries every verdict — see game/tests/probe/platform_probe.gd for why an
# in-game readback LOOP is the wrong shape to carry across to the web.
#
# The export must already have been built with the probe as its main scene;
# tools/probe_platform_web_export.sh does that and puts project.godot back.
#
# Usage: tools/platform_probe_web.sh <build-dir>
# Env knobs: CHROME (browser binary), PROBE_PORT (pin the HTTP port).
set -eu
BUILD="${1:?usage: platform_probe_web.sh <build-dir>}"
DIR="$(cd "$(dirname "$0")/.." && pwd)"

[ -s "$BUILD/index.html" ] || {
  echo "platform-web: FAILED no export at $BUILD (build the Web export first)"
  exit 2
}
command -v python3 >/dev/null 2>&1 || {
  echo "platform-web: FAILED python3 not found (it serves the build and drives the browser)"
  exit 2
}

CHROME="${CHROME:-}"
if [ -z "$CHROME" ]; then
  for c in chromium chromium-browser google-chrome google-chrome-stable; do
    command -v "$c" >/dev/null 2>&1 && { CHROME="$c"; break; }
  done
fi
[ -n "$CHROME" ] || {
  echo "platform-web: FAILED no Chrome/Chromium found; set CHROME=/path/to/browser"
  exit 2
}

PORT="${PROBE_PORT:-7811}"
DBG=$((PORT + 1))
PROFILE="$(mktemp -d "${TMPDIR:-/tmp}/unseeing-web-probe.XXXXXX")"
SRV=""
CHR=""
cleanup() {
  [ -z "$CHR" ] || kill "$CHR" 2>/dev/null || true
  [ -z "$SRV" ] || kill "$SRV" 2>/dev/null || true
  [ -z "$CHR" ] || wait "$CHR" 2>/dev/null || true
  [ -z "$SRV" ] || wait "$SRV" 2>/dev/null || true
  rm -rf "$PROFILE" 2>/dev/null || true
}
trap cleanup EXIT INT TERM HUP

# Plain http.server, exactly as test/web_smoke.sh does it: the export is
# single-threaded (the preset pins thread_support=false), so it needs no
# cross-origin isolation headers.
python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$BUILD" >/dev/null 2>&1 &
SRV=$!

# Poll, never guess — a server that has not finished binding produces a
# browser that loaded nothing, which then reads as a broken build.
python3 "$DIR/tools/wait_for_url.py" "http://127.0.0.1:$PORT/index.html" 30 || {
  echo "platform-web: FAILED the local HTTP server never answered"
  exit 1
}

# --enable-unsafe-swiftshader matches test/web_smoke.sh: a software
# rasteriser that executes the real GLSL. That is a caveat the Python half
# prints rather than hides.
"$CHROME" --headless=new --disable-gpu --enable-unsafe-swiftshader \
  --window-size=1280,720 --remote-debugging-port="$DBG" \
  --no-sandbox --user-data-dir="$PROFILE" \
  "http://127.0.0.1:$PORT/index.html" >/dev/null 2>&1 &
CHR=$!

python3 "$DIR/tools/platform_probe_web.py" "$DBG"
