#!/bin/sh
# Classic CI/CD pipeline: test, then deploy if a writable deploy target exists.
# Pure POSIX sh — the same script runs on a laptop, the droplet, or cloud CI.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

echo "ci: running test suite"
"$DIR/test/run.sh"

DEPLOY_DIR="${DEPLOY_DIR:-/var/www/unseeing}"
if [ -d "$DEPLOY_DIR" ] && [ -w "$DEPLOY_DIR" ]; then
  cp "$DIR/index.html" "$DEPLOY_DIR/index.html"
  echo "ci: deployed index.html -> $DEPLOY_DIR"
  URL="${CHECK_URL:-http://127.0.0.1/}"
  TMP="$(mktemp)"
  if curl -s --max-time 10 "$URL" > "$TMP" && cmp -s "$TMP" "$DIR/index.html"; then
    echo "ci: served bytes verified at $URL"
  else
    echo "ci: WARNING — could not verify served bytes at $URL"
  fi
  rm -f "$TMP"
else
  echo "ci: no writable deploy dir at $DEPLOY_DIR — test-only run"
fi
echo "ci: OK"
