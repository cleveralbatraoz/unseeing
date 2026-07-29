#!/bin/sh
# CI/CD for the Godot project — the single source of truth.
# Stages: headless boot check -> unit tests -> clean Web export (strict) ->
# build stamping -> precompression -> browser smoke test -> deploy + verify.
# Pure POSIX sh; the same script runs on a laptop, the droplet, or cloud CI.
# Env knobs: GODOT (binary), SKIP_EXPORT=1 (checks only, for CI without
# export templates), SKIP_SMOKE=1, DEPLOY_DIR, CHECK_URL.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

GODOT="${GODOT:-}"
if [ -z "$GODOT" ]; then
  for g in godot "$HOME/bin/godot" /opt/homebrew/bin/godot; do
    if command -v "$g" >/dev/null 2>&1 || [ -x "$g" ]; then GODOT="$g"; break; fi
  done
fi
[ -n "$GODOT" ] || { echo "ci: godot not found; set GODOT=/path/to/godot"; exit 2; }

if [ -f "$DIR/.godot-version" ]; then
  WANT="$(cat "$DIR/.godot-version")"
  HAVE="$("$GODOT" --version 2>/dev/null | head -1)"
  case "$HAVE" in
    "$WANT"*) : ;;
    *) echo "ci: WARNING godot version '$HAVE' != pinned '$WANT'" ;;
  esac
fi

echo "ci: import + headless boot check"
"$GODOT" --headless --path "$DIR/game" --import >/dev/null 2>&1 || true
OUT="$("$GODOT" --headless --path "$DIR/game" --quit-after 30 2>&1)" || {
  printf '%s\n' "$OUT" | tail -15
  echo "ci: boot check FAILED (non-zero exit)"
  exit 1
}
if printf '%s' "$OUT" | grep -qiE "SCRIPT ERROR|SHADER ERROR|Parse Error|ERROR: Failed to"; then
  printf '%s\n' "$OUT" | grep -iE "SCRIPT ERROR|SHADER ERROR|Parse Error|ERROR: Failed to" | head -10
  echo "ci: boot check FAILED (script/shader errors)"
  exit 1
fi
echo "ci: boot check OK"

echo "ci: unit tests"
"$GODOT" --headless --path "$DIR/game" -s res://tests/run_tests.gd || {
  echo "ci: unit tests FAILED"
  exit 1
}

if [ "${SKIP_EXPORT:-}" = "1" ]; then
  echo "ci: SKIP_EXPORT=1 — checks-only run"
  echo "ci: OK"
  exit 0
fi

echo "ci: exporting Web build (clean)"
rm -rf "$DIR/game/build/web"
mkdir -p "$DIR/game/build/web"
touch "$DIR/game/build/.gdignore"
if ! "$GODOT" --headless --path "$DIR/game" --export-release "Web" build/web/index.html > /tmp/godot-export.log 2>&1; then
  tail -15 /tmp/godot-export.log
  echo "ci: export FAILED (non-zero exit)"
  exit 1
fi
for f in index.html index.js index.wasm index.pck; do
  [ -s "$DIR/game/build/web/$f" ] || { echo "ci: export FAILED (missing $f)"; exit 1; }
done
echo "ci: export OK ($(wc -c < "$DIR/game/build/web/index.wasm" | tr -d ' ') bytes of wasm)"

# stamp the build sha into the shell (head_include carries __BUILD__)
SHA="$(git -C "$DIR" rev-parse --short HEAD 2>/dev/null || echo unknown)"
sed -i.bak "s/__BUILD__/$SHA/g" "$DIR/game/build/web/index.html" 2>/dev/null || \
  sed -i "s/__BUILD__/$SHA/g" "$DIR/game/build/web/index.html"
rm -f "$DIR/game/build/web/index.html.bak"

# precompress: nginx gzip_static serves these, cutting the download ~4x
echo "ci: precompressing"
for f in "$DIR/game/build/web/index.wasm" "$DIR/game/build/web/index.pck" \
         "$DIR/game/build/web/index.js"; do
  gzip -9 -k -f "$f"
  if command -v brotli >/dev/null 2>&1; then brotli -q 11 -f -k "$f"; fi
done

if [ "${SKIP_SMOKE:-}" != "1" ] && [ -x "$DIR/test/web_smoke.sh" ]; then
  echo "ci: browser smoke test"
  "$DIR/test/web_smoke.sh" "$DIR/game/build/web" || { echo "ci: smoke test FAILED"; exit 1; }
fi

DEPLOY_DIR="${DEPLOY_DIR:-/var/www/unseeing}"
if [ -d "$DEPLOY_DIR" ] && [ -w "$DEPLOY_DIR" ]; then
  # near-atomic: copy to temp names, then rename per file (html last, so a
  # mid-deploy visitor never gets new html referencing missing assets)
  for f in "$DIR/game/build/web/"*; do
    b="$(basename "$f")"
    [ "$b" = "index.html" ] && continue
    cp "$f" "$DEPLOY_DIR/.$b.new" && mv -f "$DEPLOY_DIR/.$b.new" "$DEPLOY_DIR/$b"
  done
  cp "$DIR/game/build/web/index.html" "$DEPLOY_DIR/.index.html.new"
  mv -f "$DEPLOY_DIR/.index.html.new" "$DEPLOY_DIR/index.html"
  echo "ci: deployed Web build -> $DEPLOY_DIR"
  URL="${CHECK_URL:-https://dggrus.hlab.kz/}"
  TMP="$(mktemp)"
  if curl -sL --max-time 15 "$URL" > "$TMP" && cmp -s "$TMP" "$DIR/game/build/web/index.html"; then
    echo "ci: served bytes verified at $URL"
    rm -f "$TMP"
  else
    rm -f "$TMP"
    echo "ci: FAILED — served bytes do not match the build at $URL"
    exit 1
  fi
else
  echo "ci: no writable deploy dir at $DEPLOY_DIR — build-only run"
fi
echo "ci: OK"
