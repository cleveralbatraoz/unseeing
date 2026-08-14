#!/bin/sh
# Browser smoke test: serve the exported web build locally, load it with the
# ?demo flag in headless Chrome/Chromium, and assert the engine actually
# renders (the loader overlay disappears AND the canvas shows lit pixels).
# This is the gate that catches "wasm builds fine, renders nothing" — the
# failure mode a pure boot check cannot see.
# Usage: web_smoke.sh <build-dir>
# Env knobs: CHROME (browser binary), SMOKE_PORT (pin the HTTP port),
#   SMOKE_WAIT (seconds the probe waits for first paint).
set -eu
BUILD="${1:?usage: web_smoke.sh <build-dir>}"
DIR="$(cd "$(dirname "$0")" && pwd)"

CHROME="${CHROME:-}"
# An explicitly named browser is checked, not trusted. Left unchecked, a stale
# CHROME= pointing at a moved binary sailed past the discovery guard below and
# surfaced minutes later as "the DevTools endpoint never answered" — which
# reads like a broken build rather than a bad path.
if [ -n "$CHROME" ] && ! [ -x "$CHROME" ] && ! command -v "$CHROME" >/dev/null 2>&1; then
  echo "smoke: FAILED CHROME names '$CHROME', which is not an executable"
  exit 2
fi
if [ -z "$CHROME" ]; then
  # Every naming convention this browser actually ships under. `chrome` and
  # `google-chrome-stable` are the Arch and Fedora/RPM names; the .app path is
  # macOS, where nothing lands on PATH at all.
  for c in "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
           "/Applications/Chromium.app/Contents/MacOS/Chromium" \
           google-chrome google-chrome-stable chromium-browser chromium chrome; do
    if [ -x "$c" ] || command -v "$c" >/dev/null 2>&1; then CHROME="$c"; break; fi
  done
fi
# A missing browser used to exit 0. This is the ONLY gate that can see "the
# wasm loads and paints nothing", it runs immediately before ci/pipeline.sh
# deploys, and skipping it silently meant a machine without Chrome shipped
# green while never rendering a single frame. An absent browser is a broken
# host, not a pass — and SKIP_SMOKE=1 already exists for a deliberate opt-out.
[ -n "$CHROME" ] || {
  echo "smoke: FAILED no chrome/chromium found, and this is the only gate that proves the build renders"
  echo "smoke:        fix: install Chrome or Chromium, set CHROME=/path/to/binary,"
  echo "smoke:        or run the pipeline with SKIP_SMOKE=1 to skip it deliberately"
  exit 2
}
command -v python3 >/dev/null 2>&1 || {
  echo "smoke: FAILED python3 not found (it serves the build and drives the browser)"
  echo "smoke:        fix: install Python 3, or run the pipeline with SKIP_SMOKE=1"
  exit 2
}

# Fast preflight, no Chrome, no network: web_probe.py's decode_png reverses
# real PNG scanline filtering to read the pixels every assertion below
# relies on, and nothing else in the repo (cargo, gdUnit) ever touches this
# file — so a regression here has no other net under it. Fail before paying
# for a browser boot if the decoder itself is wrong.
echo "smoke: PNG decoder self-test"
python3 "$DIR/web_probe.py" --selftest \
  || { echo "smoke: FAIL — PNG decoder self-test"; exit 1; }

# Ports come from the kernel, not from a constant. 8931/8932 were fixed, so two
# runs on one machine — a second worktree, another developer on a shared box,
# CI with two jobs — served each other's build and reported on it confidently.
# SMOKE_PORT still pins them when someone needs a known port.
if [ -n "${SMOKE_PORT:-}" ]; then
  PORT="$SMOKE_PORT"
  DBG="$((PORT + 1))"
else
  PORTS="$(python3 - <<'PY'
import socket
def free():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    return port
print(free(), free())
PY
)" || { echo "smoke: FAILED could not reserve local ports"; exit 2; }
  PORT="${PORTS% *}"
  DBG="${PORTS#* }"
fi

python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$BUILD" >/dev/null 2>&1 &
SRV=$!
PROFILE="$(mktemp -d)"
trap 'kill "${CHR:-}" "$SRV" 2>/dev/null || true; wait "${CHR:-}" "$SRV" 2>/dev/null || true; rm -rf "$PROFILE" 2>/dev/null || true' EXIT INT TERM HUP

# Poll, never guess. A server that has not finished binding produced a browser
# that loaded nothing, which the probe then reported as a shader or engine
# failure — the build blamed for the harness being early.
# One process owning the whole poll: the loop lives inside Python so a retry
# costs a socket attempt rather than an interpreter start-up, and the bound is
# a real deadline instead of an iteration count that means different wall-clock
# time on every machine.
wait_for() { # wait_for <url> <what>
  if python3 - "$1" "${SMOKE_READY_TIMEOUT:-30}" <<'PY'
import sys, time, urllib.error, urllib.request

url, budget = sys.argv[1], float(sys.argv[2])
deadline = time.monotonic() + budget
while time.monotonic() < deadline:
    try:
        urllib.request.urlopen(url, timeout=1).read(1)
        sys.exit(0)
    except (urllib.error.URLError, OSError):
        time.sleep(0.05)
sys.exit(1)
PY
  then
    return 0
  fi
  echo "smoke: FAILED $2 never answered at $1"
  return 1
}

wait_for "http://127.0.0.1:$PORT/index.html" "the local HTTP server" || exit 1

"$CHROME" --headless=new --disable-gpu --enable-unsafe-swiftshader \
  --window-size=640,360 --remote-debugging-port="$DBG" \
  --user-data-dir="$PROFILE" "http://127.0.0.1:$PORT/index.html?demo" >/dev/null 2>&1 &
CHR=$!

# Replaces the three-second sleep this file used to carry as acknowledged debt:
# the DevTools endpoint answers exactly when Chrome is ready to be driven, so
# there is nothing left to estimate.
wait_for "http://127.0.0.1:$DBG/json/version" "the browser's DevTools endpoint" || exit 1

python3 "$DIR/web_probe.py" "$DBG" "${SMOKE_WAIT:-22}"
