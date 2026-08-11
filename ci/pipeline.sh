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
    *)
      echo "ci: FAILED godot version '$HAVE' != pinned '$WANT' (set GODOT= to a matching binary)"
      exit 2
      ;;
  esac
fi

# Cheapest gate in the pipeline (no Godot, no network) — run it first so a
# stray export binary or an unignored worktree fails in milliseconds.
echo "ci: repository hygiene"
"$DIR/test/repo_hygiene.sh" || exit 1

# Self-tests for two gates further down (#21, #28) — pure shell, no
# Godot, so they belong up here with the other cheap invariant checks
# rather than after minutes of Rust/export work.
echo "ci: boot-error gate self-test"
"$DIR/test/ci_boot_error_gate.sh" || exit 1
echo "ci: gdscript lint scope self-test"
"$DIR/test/ci_gdscript_lint_scope.sh" || exit 1

# The test bench is vendored third-party code, so nothing else in this
# pipeline would notice if it drifted — the pre-commit hook deliberately
# skips game/addons/. Check it against its lock before trusting a green run.
echo "ci: vendored gdUnit4 integrity"
"$DIR/ci/vendor-gdunit4.sh" verify || exit 1

echo "ci: rust gates (fmt + clippy + test + release build)"
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
command -v cargo >/dev/null 2>&1 || {
  echo "ci: FAILED cargo not found (install rustup; rust-toolchain.toml pins the version)"
  exit 2
}
# Rust needs a C linker even for pure-Rust crates, and compiling godot-core
# needs more RAM than the droplet has. PREBUILT_RUST=1 (or a missing linker)
# switches to prebuilt artifacts, seeded by deploy.sh — loudly.
if [ "${PREBUILT_RUST:-}" != "1" ] \
  && { command -v cc >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1; }; then
  (
    cd "$DIR/rust"
    cargo fmt --check || { echo "ci: rust format FAILED (run cargo fmt)"; exit 1; }
    cargo clippy --all-targets -- -D warnings || { echo "ci: clippy FAILED"; exit 1; }
    cargo test || { echo "ci: cargo test FAILED"; exit 1; }
    cargo build --release || { echo "ci: rust build FAILED"; exit 1; }
  ) || exit 1
  echo "ci: rust gates OK"
else
  echo "ci: running on PREBUILT rust artifacts (PREBUILT_RUST=1 or no C linker)"
  NATIVE_LIB="$DIR/rust/target/release/libunseeing_core.so"
  [ "$(uname)" = "Darwin" ] && NATIVE_LIB="$DIR/rust/target/release/libunseeing_core.dylib"
  [ -f "$NATIVE_LIB" ] || {
    echo "ci: FAILED no prebuilt native core at $NATIVE_LIB"
    exit 2
  }
  # A core that merely EXISTS proves nothing: cargo-target outlives every
  # push, so a failed scp would leave the previous deploy's binaries in
  # place and this run would ship them under the new commit's name. When the
  # hook tells us which commit is being deployed, demand the cores say the
  # same. (No BUILD_SHA means a hand-run local build — nothing to compare.)
  if [ -n "${BUILD_SHA:-}" ]; then
    STAMP="$DIR/rust/target/core.commit"
    [ -f "$STAMP" ] || {
      echo "ci: FAILED prebuilt cores carry no commit stamp (deploy.sh seeds core.commit)"
      exit 2
    }
    BUILT="$(printf %.9s "$(cat "$STAMP")")"
    [ "$BUILT" = "$BUILD_SHA" ] || {
      echo "ci: FAILED prebuilt cores were built from $BUILT but $BUILD_SHA is being deployed"
      exit 2
    }
    echo "ci: prebuilt cores stamped $BUILT — matches the pushed commit"
  fi
  echo "ci: rust gates SKIPPED (prebuilt native core present)"
fi

echo "ci: gdscript format + lint"
GDFORMAT="$(command -v gdformat || echo "$HOME/.local/bin/gdformat")"
GDLINT="$(command -v gdlint || echo "$HOME/.local/bin/gdlint")"
[ -x "$GDFORMAT" ] && [ -x "$GDLINT" ] || {
  echo "ci: FAILED gdformat/gdlint not found (pipx install 'gdtoolkit==4.*')"
  exit 2
}
. "$DIR/ci/gdscript_files.sh"
GD_FILES="$(gdscript_files "$DIR")"
"$GDFORMAT" --check $GD_FILES || { echo "ci: format check FAILED (run gdformat on the files above)"; exit 1; }
"$GDLINT" $GD_FILES || { echo "ci: lint FAILED"; exit 1; }
echo "ci: format + lint OK"

echo "ci: import + headless boot check"
. "$DIR/ci/boot_error_pattern.sh"
"$GODOT" --headless --path "$DIR/game" --import >/dev/null 2>&1 || true
OUT="$("$GODOT" --headless --path "$DIR/game" --quit-after 30 2>&1)" || {
  printf '%s\n' "$OUT" | tail -15
  echo "ci: boot check FAILED (non-zero exit)"
  exit 1
}
if printf '%s' "$OUT" | grep -qiE "$BOOT_ERROR_PATTERN"; then
  printf '%s\n' "$OUT" | grep -iE "$BOOT_ERROR_PATTERN" | head -10
  echo "ci: boot check FAILED (script/shader/engine-class errors)"
  exit 1
fi
echo "ci: boot check OK"

echo "ci: unit tests (gdUnit4)"
"$GODOT" --headless --path "$DIR/game" -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
  --ignoreHeadlessMode -c -a tests || {
  echo "ci: unit tests FAILED"
  exit 1
}

echo "ci: determinism probe (two seeded fixed-fps boots must agree)"
GODOT="$GODOT" "$DIR/tools/determinism_probe.sh"

if [ "${SKIP_EXPORT:-}" = "1" ]; then
  echo "ci: SKIP_EXPORT=1 — checks-only run"
  echo "ci: OK"
  exit 0
fi

echo "ci: rust wasm build (the web export loads it)"
if [ "${PREBUILT_RUST:-}" != "1" ] \
  && { command -v cc >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1; }; then
  "$DIR/rust/build-wasm.sh" || { echo "ci: wasm build FAILED"; exit 1; }
else
  WASM_LIB="$DIR/rust/target/wasm32-unknown-emscripten/release/unseeing_core.wasm"
  [ -s "$WASM_LIB" ] || { echo "ci: FAILED no prebuilt wasm core at $WASM_LIB"; exit 2; }
  echo "ci: wasm build SKIPPED (prebuilt core present)"
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
# index.side.wasm is the Rust GDExtension: without it the game boots into a
# world with no engine nodes at all, so it belongs in the same guard
for f in index.html index.js index.wasm index.side.wasm index.pck; do
  [ -s "$DIR/game/build/web/$f" ] || { echo "ci: export FAILED (missing $f)"; exit 1; }
done
echo "ci: export OK ($(wc -c < "$DIR/game/build/web/index.wasm" | tr -d ' ') bytes of wasm)"

# stamp the build sha into the shell (head_include carries __BUILD__);
# BUILD_SHA comes from the post-receive hook — its work tree is a tar
# extract of the pushed commit, not a git repo, so rev-parse can't know
SHA="${BUILD_SHA:-$(git -C "$DIR" rev-parse --short HEAD 2>/dev/null || echo unknown)}"
sed -i.bak "s/__BUILD__/$SHA/g" "$DIR/game/build/web/index.html" 2>/dev/null || \
  sed -i "s/__BUILD__/$SHA/g" "$DIR/game/build/web/index.html"
rm -f "$DIR/game/build/web/index.html.bak"

# precompress: nginx gzip_static serves these, cutting the download ~4x.
# Found BY EXTENSION, never by a hand-written list: the old list named three
# files and so quietly missed index.side.wasm, the largest artifact in the
# export by a factor of thirty, which shipped raw on every cold load.
echo "ci: precompressing"
for f in "$DIR/game/build/web/"*.wasm "$DIR/game/build/web/"*.pck \
         "$DIR/game/build/web/"*.js; do
  [ -f "$f" ] || continue
  gzip -9 -k -f "$f"
  if command -v brotli >/dev/null 2>&1; then brotli -q 11 -f -k "$f"; fi
done
command -v brotli >/dev/null 2>&1 \
  || echo "ci: NOTE brotli absent — gzip only (brotli would take ~15% more off the wasm)"
RAW="$(du -ck "$DIR/game/build/web/"*.wasm "$DIR/game/build/web/"*.pck \
                "$DIR/game/build/web/"*.js 2>/dev/null | tail -1 | cut -f1)"
GZ="$(du -ck "$DIR/game/build/web/"*.gz 2>/dev/null | tail -1 | cut -f1)"
BR="$(du -ck "$DIR/game/build/web/"*.br 2>/dev/null | tail -1 | cut -f1)"
if [ -n "$BR" ]; then
  echo "ci: precompressed ${RAW} KB -> ${GZ} KB gzip, ${BR} KB brotli"
else
  echo "ci: precompressed ${RAW} KB -> ${GZ} KB gzip"
fi

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
  # verify against the server's own IP; -k because the TLS cert is issued
  # for the site's hostname, which a bare-IP request does not present
  URL="${CHECK_URL:-https://206.223.241.165/}"
  TMP="$(mktemp)"
  if curl -skL --max-time 15 "$URL" > "$TMP" && cmp -s "$TMP" "$DIR/game/build/web/index.html"; then
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
