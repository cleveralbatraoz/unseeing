#!/bin/sh
# Runs the Unseeing test harness in headless Chrome; exits 0 only if all pass.
set -u
DIR="$(cd "$(dirname "$0")" && pwd)"
CHROME="${CHROME:-}"
if [ -z "$CHROME" ]; then
  for c in "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" google-chrome chromium-browser chromium; do
    if [ -x "$c" ] || command -v "$c" >/dev/null 2>&1; then CHROME="$c"; break; fi
  done
fi
if [ -z "$CHROME" ]; then echo "Chrome not found; set CHROME=/path/to/chrome"; exit 2; fi

PROFILE="$(mktemp -d)"
PORT="${PORT:-9333}"
"$CHROME" --headless=new --disable-gpu --enable-unsafe-swiftshader \
  --allow-file-access-from-files --remote-debugging-port="$PORT" \
  --user-data-dir="$PROFILE" "file://$DIR/harness.html" >/dev/null 2>&1 &
CPID=$!
trap 'kill "$CPID" 2>/dev/null; rm -rf "$PROFILE"' EXIT

TITLE=""
i=0
while [ "$i" -lt 45 ]; do
  sleep 2
  TITLE="$(curl -s "http://127.0.0.1:$PORT/json/list" 2>/dev/null | grep -o '"title": *"T [^"]*"' | head -1)"
  case "$TITLE" in *done*) break ;; esac
  i=$((i + 1))
done

echo "harness: ${TITLE:-no report}"
case "$TITLE" in
  *done*'fail=0'*) echo "TESTS PASSED"; exit 0 ;;
  *) echo "TESTS FAILED"; exit 1 ;;
esac
