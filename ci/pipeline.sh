#!/bin/sh
# CI/CD for the Godot project — the single source of truth.
# Gate on a headless boot check, build the Web (wasm) export, deploy it if a
# writable deploy dir exists. Pure POSIX sh; the same script runs on a laptop,
# the droplet, or cloud CI. Env knobs: GODOT (binary), SKIP_EXPORT=1
# (boot-check only, for cloud CI without export templates), DEPLOY_DIR, CHECK_URL.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

GODOT="${GODOT:-}"
if [ -z "$GODOT" ]; then
  for g in godot "$HOME/bin/godot" /opt/homebrew/bin/godot; do
    if command -v "$g" >/dev/null 2>&1 || [ -x "$g" ]; then GODOT="$g"; break; fi
  done
fi
[ -n "$GODOT" ] || { echo "ci: godot not found; set GODOT=/path/to/godot"; exit 2; }

echo "ci: import + headless boot check"
"$GODOT" --headless --path "$DIR/game" --import >/dev/null 2>&1 || true
OUT="$("$GODOT" --headless --path "$DIR/game" --quit-after 30 2>&1)" || {
  printf '%s\n' "$OUT" | tail -15
  echo "ci: boot check FAILED (non-zero exit)"
  exit 1
}
if printf '%s' "$OUT" | grep -qiE "SCRIPT ERROR|Parse Error|ERROR: Failed to"; then
  printf '%s\n' "$OUT" | grep -iE "SCRIPT ERROR|Parse Error|ERROR: Failed to" | head -10
  echo "ci: boot check FAILED (script errors)"
  exit 1
fi
echo "ci: boot check OK"

if [ "${SKIP_EXPORT:-}" = "1" ]; then
  echo "ci: SKIP_EXPORT=1 — boot-check-only run"
  echo "ci: OK"
  exit 0
fi

echo "ci: exporting Web build"
mkdir -p "$DIR/game/build/web"
"$GODOT" --headless --path "$DIR/game" --export-release "Web" build/web/index.html >/dev/null 2>&1 || true
[ -s "$DIR/game/build/web/index.wasm" ] || { echo "ci: export FAILED (no index.wasm)"; exit 1; }
echo "ci: export OK ($(wc -c < "$DIR/game/build/web/index.wasm" | tr -d ' ') bytes of wasm)"

DEPLOY_DIR="${DEPLOY_DIR:-/var/www/unseeing}"
if [ -d "$DEPLOY_DIR" ] && [ -w "$DEPLOY_DIR" ]; then
  rm -f "$DEPLOY_DIR"/*
  cp "$DIR/game/build/web/"* "$DEPLOY_DIR/"
  echo "ci: deployed Web build -> $DEPLOY_DIR"
  URL="${CHECK_URL:-http://127.0.0.1/}"
  TMP="$(mktemp)"
  if curl -sL --max-time 10 "$URL" > "$TMP" && cmp -s "$TMP" "$DIR/game/build/web/index.html"; then
    echo "ci: served bytes verified at $URL"
  else
    echo "ci: WARNING — could not verify served bytes at $URL"
  fi
  rm -f "$TMP"
else
  echo "ci: no writable deploy dir at $DEPLOY_DIR — build-only run"
fi
echo "ci: OK"
