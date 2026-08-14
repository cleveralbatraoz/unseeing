#!/bin/sh
# Every script that runs Godot must select it through tools/lib/engine.sh, and
# therefore must refuse an engine that fails the pin.
#
# Before this suite, eight probes had no version gate at all: they took the
# first binary named `godot` and printed `probe: … OK` from an engine
# ci/pipeline.sh would have rejected outright. Two more skipped discovery
# entirely and reported a missing engine as "no hash — the probe crashed".
#
# Each caller runs against a COPY of the checkout, never the real one: a
# regression here would otherwise let a probe write override.cfg, or let
# `vendor update` rewrite the vendored addon, inside the developer's tree.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FAIL=0

ok() { echo "engine-callers: OK   $1"; }
bad() { echo "engine-callers: FAIL $1"; FAIL=1; }
note() { echo "engine-callers:      $1"; }

T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM HUP
REPO="$T/repo with spaces"
LOG="$T/calls.log"
mkdir -p "$REPO/game" "$T/engines" "$T/home"
cp -R "$ROOT/tools" "$REPO/tools"
cp -R "$ROOT/ci" "$REPO/ci"
cp "$ROOT/.godot-version" "$REPO/.godot-version"

# Two fixture engines, identical but for the version they report. Everything
# below is decided by that one string.
make_engine() {
  cat >"$1" <<EOF
#!/bin/sh
printf 'engine %s\n' "\$*" >>"\$ENGINE_CALL_LOG"
[ "\$1" = "--version" ] || exit 0
printf '%s\n' '$2'
EOF
  chmod +x "$1"
}
make_engine "$T/engines/godot-wrong" '4.7.0.stable.official.deadbeef'
make_engine "$T/engines/godot-right" "$(cat "$ROOT/.godot-version").a13da4feb"

# Callers that reach engine selection on any POSIX host. export_macos.sh is
# excluded by its own uname guard and is reported separately rather than
# silently counted as covered.
CALLERS='tools/probe_display.sh
tools/probe_visibility.sh
tools/probe_editor_level.sh
tools/probe_editor_prefabs.sh
tools/probe_editor_slabs.sh
tools/probe_editor_sources.sh
tools/determinism_probe.sh
tools/restore_probe.sh
ci/pipeline.sh'

args_for() {
  case "$1" in
    ci/vendor-gdunit4.sh) echo "update v6.2.0" ;;
    *) echo "" ;;
  esac
}

run_caller() {
  script="$1"
  engine="$2"
  : >"$LOG"
  status=0
  # shellcheck disable=SC2086
  env -u GODOT \
    HOME="$T/home" \
    ENGINE_CALL_LOG="$LOG" \
    UNSEEING_ENGINE_CANDIDATES="$T/engines/$engine" \
    SKIP_SMOKE=1 \
    "$REPO/$script" $(args_for "$script") >"$T/out" 2>&1 || status=$?
}

# --- an engine that fails the pin must be refused, before any real work ---
printf '%s\n' "$CALLERS" | while IFS= read -r script; do
  [ -n "$script" ] || continue
  echo "$script"
done >"$T/list"

while IFS= read -r script; do
  [ -n "$script" ] || continue
  run_caller "$script" godot-wrong
  if [ "$status" -eq 2 ]; then
    ok "$script refuses an engine that fails the pin (exit 2)"
  else
    bad "$script refuses an engine that fails the pin (expected exit 2, got $status)"
    sed 's/^/engine-callers:      /' "$T/out" | head -5
  fi
  if grep -q -v -- '--version' "$LOG" 2>/dev/null; then
    bad "$script never runs a refused engine for real work"
    sed 's/^/engine-callers:      /' "$LOG" | head -5
  else
    ok "$script never runs a refused engine for real work"
  fi
done <"$T/list"

# ci/vendor-gdunit4.sh takes a subcommand, so it gets its own invocation.
run_caller ci/vendor-gdunit4.sh godot-wrong
if [ "$status" -eq 2 ]; then
  ok "ci/vendor-gdunit4.sh update refuses an engine that fails the pin (exit 2)"
else
  bad "ci/vendor-gdunit4.sh update refuses an engine that fails the pin (got $status)"
fi
if grep -q '^vendor: FAILED no Godot matching' "$T/out"; then
  ok "ci/vendor-gdunit4.sh keeps its own vendor: message prefix"
else
  bad "ci/vendor-gdunit4.sh keeps its own vendor: message prefix"
  sed 's/^/engine-callers:      /' "$T/out" | head -3
fi
# The refusal must name WHICH engine was rejected, not merely say one was
# missing: "godot not found" sent readers hunting for an install they had.
if grep -q '4\.7\.0\.stable\.official\.deadbeef' "$T/out"; then
  ok "ci/vendor-gdunit4.sh names the rejected engine's actual version"
else
  bad "ci/vendor-gdunit4.sh names the rejected engine's actual version"
  sed 's/^/engine-callers:      /' "$T/out" | head -4
fi

# --- and an engine that satisfies the pin must actually be REACHED ---
# Without this the suite above would pass just as well against a caller that
# refuses everything, which is the classic way a gate test proves nothing.
while IFS= read -r script; do
  [ -n "$script" ] || continue
  run_caller "$script" godot-right
  if grep -q 'no Godot matching' "$T/out"; then
    bad "$script accepts an engine that satisfies the pin (it refused instead)"
    sed 's/^/engine-callers:      /' "$T/out" | head -4
  else
    ok "$script accepts an engine that satisfies the pin"
  fi
  # ci/pipeline.sh runs a dozen cheap gates before it ever launches Godot, so
  # only the probes can be asked to have invoked the engine by now.
  case "$script" in
    ci/*) continue ;;
  esac
  if grep -q -v -- '--version' "$LOG" 2>/dev/null; then
    ok "$script goes on to run the accepted engine"
  else
    bad "$script goes on to run the accepted engine (never invoked it)"
    sed 's/^/engine-callers:      /' "$T/out" | head -4
  fi
done <"$T/list"

# --- the macOS export path, honestly reported rather than vacuously passed ---
if [ "$(uname)" = "Darwin" ]; then
  run_caller tools/export_macos.sh godot-wrong
  if [ "$status" -eq 2 ]; then
    ok "tools/export_macos.sh refuses an engine that fails the pin (exit 2)"
  else
    bad "tools/export_macos.sh refuses an engine that fails the pin (got $status)"
  fi
else
  note "SKIP tools/export_macos.sh — its uname guard excludes $(uname); macOS CI covers it"
fi

exit "$FAIL"
