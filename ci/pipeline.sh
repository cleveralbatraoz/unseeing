#!/bin/sh
# CI/CD for the Godot project — the single source of truth.
# Stages: headless boot check -> unit tests -> clean Web export (strict) ->
# build stamping -> precompression -> browser smoke test. Deployment itself
# is not this script's job: .github/workflows/test.yml's deploy job ships
# this stage's verified game/build/web/ output to GitHub Pages.
# Pure POSIX sh; the same script runs on a laptop or GitHub-hosted CI.
# Env knobs: GODOT (binary), SKIP_EXPORT=1 (checks only, for CI without
# export templates), SKIP_SMOKE=1, BUILD_SHA.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

# One owner decides which engine is the pinned one, and refuses anything
# else — including an explicitly supplied mismatch. tools/lib/engine.sh.
# shellcheck source=tools/lib/engine.sh
. "$DIR/tools/lib/engine.sh"
GODOT="$(unseeing_engine_select "$DIR" "${GODOT:-}")" || {
  echo "ci: FAILED no Godot matching .godot-version; set GODOT=/path/to/godot"
  exit 2
}

# Resolved here rather than beside the format stage it feeds: the check costs
# nothing and its absence is fatal, so discovering it after the full Rust gate
# meant paying cargo fmt + clippy + test + a release build to be told a two-
# millisecond precondition was missing. It also mis-reads further down: the
# pre-commit hook the hygiene suite drives refuses without these, and the suite
# then blames whichever guard it happened to be exercising.
# ~/.local/bin is pipx's default and is not on every login PATH.
GDFORMAT="$(command -v gdformat || echo "$HOME/.local/bin/gdformat")"
GDLINT="$(command -v gdlint || echo "$HOME/.local/bin/gdlint")"
[ -x "$GDFORMAT" ] && [ -x "$GDLINT" ] || {
  echo "ci: FAILED gdformat/gdlint not found (pipx install 'gdtoolkit==4.*')"
  exit 2
}

# Cheapest gate in the pipeline (no Godot, no network) — run it first so a
# stray export binary or an unignored worktree fails in milliseconds.
echo "ci: repository hygiene"
"$DIR/test/repo_hygiene.sh" || exit 1
echo "ci: pinned Superpowers metadata"
"$DIR/ci/verify-superpowers.sh" metadata || exit 1
echo "ci: Superpowers verifier self-test"
"$DIR/test/verify_superpowers_test.sh" || exit 1

# Self-tests for gates further down (#21, #28, and the exact gdUnit summary)
# are pure shell, so they belong up here with the other cheap invariant checks
# rather than after minutes of Rust/export work.
echo "ci: boot-error gate self-test"
"$DIR/test/ci_boot_error_gate.sh" || exit 1
echo "ci: gdscript lint scope self-test"
"$DIR/test/ci_gdscript_lint_scope.sh" || exit 1
echo "ci: GDScript tests/probes-only placement"
"$DIR/ci/check_gdscript_policy.sh" || exit 1
echo "ci: gdUnit source/summary gate self-test"
"$DIR/test/ci_gdunit_gate.sh" || exit 1
echo "ci: engine-selection self-test (discovery + the pinned-version predicate)"
"$DIR/test/engine_select_test.sh" || exit 1
echo "ci: engine-caller self-test (every script that runs Godot applies the pin)"
"$DIR/test/engine_callers_test.sh" || exit 1
echo "ci: content-digest self-test (a missing hasher must refuse, not agree)"
"$DIR/test/digest_test.sh" || exit 1
echo "ci: agent-tooling checkout/archive gate self-test"
"$DIR/test/ci_agent_tooling_gate_test.sh" || exit 1
echo "ci: agent-plugin scope self-test (another project's plugin is not ours to remove)"
"$DIR/ci/run_agent_tooling_self_test.sh" "$DIR" || exit 1
echo "ci: run-the-game self-test (it plays the world, never the editor)"
"$DIR/test/run_game_test.sh" || exit 1
# Nothing ran this suite. It was written, committed, and then never invoked by
# any script or workflow — so the gate guarding every macOS release had no gate
# of its own. It reports its own SKIP on hosts without lipo, loudly, rather than
# passing as though it had checked something.
echo "ci: macOS universal-gate self-test"
"$DIR/test/macos_universal_test.sh" || exit 1
echo "ci: POSIX designer-bootstrap self-test"
"$DIR/test/bootstrap_posix_test.sh" || exit 1
echo "ci: designer engine-bundle packaging self-test"
"$DIR/test/package_engine_bundle_test.sh" || exit 1
if command -v pwsh >/dev/null 2>&1; then
  echo "ci: Windows designer-bootstrap self-test (PowerShell boundary fakes)"
  pwsh -NoProfile -File "$DIR/test/bootstrap_windows_test.ps1" || exit 1
  echo "ci: Windows run-the-game self-test"
  pwsh -NoProfile -File "$DIR/test/run_game_windows_test.ps1" || exit 1
else
  echo "ci: Windows self-tests SKIP (pwsh unavailable; Windows CI runs them)"
fi

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
# Rust needs a C linker even for pure-Rust crates. The prebuilt-artifact
# fallback this used to have existed only for the 1.8 GB droplet's own
# limited toolchain; now that the droplet is retired, every remaining
# caller (this dev host, every GitHub-hosted runner) has a real one, so a
# missing linker is a genuine environment defect, not a case to route
# around silently.
command -v cc >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1 || {
  echo "ci: FAILED no C linker found (cc/gcc) -- Rust needs one even for pure-Rust crates"
  exit 2
}
(
  cd "$DIR/rust"
  cargo fmt --check || { echo "ci: rust format FAILED (run cargo fmt)"; exit 1; }
  cargo clippy --all-targets -- -D warnings || { echo "ci: clippy FAILED"; exit 1; }
  cargo test || { echo "ci: cargo test FAILED"; exit 1; }
  # editor-docs is non-default (its register-docs docs would bloat a
  # shipped binary), so nothing above compiles it. The focused feature test
  # proves designer-facing class overviews actually enter Godot's XML; a
  # compile-only check would accept an empty description.
  cargo check --features editor-docs || { echo "ci: editor-docs feature build FAILED"; exit 1; }
  cargo test --features editor-docs editor_docs || {
    echo "ci: editor-docs descriptions FAILED"
    exit 1
  }
  cargo build --release || { echo "ci: rust build FAILED"; exit 1; }
) || exit 1
echo "ci: rust gates OK"

echo "ci: gdscript format + lint"
# GDFORMAT/GDLINT were resolved and gated at the top of this file.
# The placement checker runs in its own process; source the shared functions in
# this shell for the independent format/lint stage too.
. "$DIR/ci/gdscript_files.sh"
gdscript_files "$DIR" | while IFS= read -r gd_file; do
  "$GDFORMAT" --check "$gd_file" || {
    echo "ci: format check FAILED (run gdformat on $gd_file)"
    exit 1
  }
  "$GDLINT" "$gd_file" || { echo "ci: lint FAILED ($gd_file)"; exit 1; }
done
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
"$DIR/ci/run_gdunit.sh" "$DIR/game" \
  "$GODOT" --headless --path "$DIR/game" -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
  --ignoreHeadlessMode -c -a tests || {
  echo "ci: unit tests FAILED"
  exit 1
}

echo "ci: determinism probe (two seeded fixed-fps boots must agree)"
GODOT="$GODOT" "$DIR/tools/determinism_probe.sh"

echo "ci: restore probe (a restored world must live the same future)"
GODOT="$GODOT" "$DIR/tools/restore_probe.sh"

# The suite above is blind to one branch by construction: it runs in the
# GAME, and Godot exposes no way to set Engine.is_editor_hint() from a
# script. The probe launches a second engine with `-e` instead, which is the
# only way to reach the editor half of the slab law — and it is headless, so
# it belongs here rather than with the windowed probes a human runs.
#
# GODOT is handed down for the same reason its sibling above gets it: this
# script would otherwise re-discover a binary on its own, skipping the
# .godot-version pin check performed at the top of this file.
echo "ci: editor-mode probe (the slab law's other half)"
GODOT="$GODOT" "$DIR/tools/probe_editor_slabs.sh" || { echo "ci: editor-mode probe FAILED"; exit 1; }

echo "ci: editor-source probe (blueprint limbs vs the runtime guard)"
GODOT="$GODOT" "$DIR/tools/probe_editor_sources.sh" || { echo "ci: editor-source probe FAILED"; exit 1; }

echo "ci: editor-level probe (the level derives while the designer watches)"
GODOT="$GODOT" "$DIR/tools/probe_editor_level.sh" || { echo "ci: editor-level probe FAILED"; exit 1; }

echo "ci: editor-prefab probe (reusable scenes stay composition-only)"
GODOT="$GODOT" "$DIR/tools/probe_editor_prefabs.sh" || { echo "ci: editor-prefab probe FAILED"; exit 1; }

# Pure ClassDB census, one mode, no editor/run duality to prove — so unlike
# its three siblings above it needs no tools/probe_*.sh wrapper, just the
# same import-then-invoke shape the boot check already uses. It exists so
# tools/bootstrap.sh's own final step (a fresh designer's whole reason to
# run this) is exercised here too, and a class-roster drift never reaches
# a clone before CI catches it.
echo "ci: engine census (every registered class the source declares is present)"
"$GODOT" --headless --path "$DIR/game" -s res://tests/probe/engine_census_probe.gd \
  || { echo "ci: engine census FAILED"; exit 1; }

if [ "${SKIP_EXPORT:-}" = "1" ]; then
  echo "ci: SKIP_EXPORT=1 — checks-only run"
  echo "ci: OK"
  exit 0
fi

echo "ci: rust wasm build (the web export loads it)"
"$DIR/rust/build-wasm.sh" || { echo "ci: wasm build FAILED"; exit 1; }

echo "ci: exporting Web build (clean)"
rm -rf "$DIR/game/build/web"
mkdir -p "$DIR/game/build/web"
touch "$DIR/game/build/.gdignore"
# Not /tmp/godot-export.log: that is one fixed name shared by every worktree,
# every concurrent run and every user on the box, and TMPDIR exists precisely
# so a sandboxed or multi-user host can put it somewhere private.
EXPORT_LOG="$(mktemp "${TMPDIR:-/tmp}/unseeing-export.XXXXXX")"
if ! "$GODOT" --headless --path "$DIR/game" --export-release "Web" build/web/index.html > "$EXPORT_LOG" 2>&1; then
  tail -15 "$EXPORT_LOG"
  rm -f "$EXPORT_LOG"
  echo "ci: export FAILED (non-zero exit)"
  exit 1
fi
# index.side.wasm is the Rust GDExtension: without it the game boots into a
# world with no engine nodes at all, so it belongs in the same guard
rm -f "$EXPORT_LOG"
for f in index.html index.js index.wasm index.side.wasm index.pck; do
  [ -s "$DIR/game/build/web/$f" ] || { echo "ci: export FAILED (missing $f)"; exit 1; }
done
echo "ci: export OK ($(wc -c < "$DIR/game/build/web/index.wasm" | tr -d ' ') bytes of wasm)"

# stamp the build sha into the shell (head_include carries __BUILD__);
# test.yml's checks job passes BUILD_SHA explicitly as printf %.9s of the
# pushed commit, matching what the deploy job's own live-verify step
# compares against. The git-rev-parse fallback below is for a hand-run
# local build, where nothing needs to match anything.
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

echo "ci: OK"
