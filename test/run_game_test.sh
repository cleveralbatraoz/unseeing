#!/bin/sh
# Behavioral contract for tools/run_game.sh — build the engine, then play the
# GAME, never the editor.
#
# The distinction is the whole point of the tool: `godot --path game` runs the
# world, `godot -e --path game` opens the authoring environment, and one letter
# separates them. Everything here runs against a copy of the checkout with a
# recording fake engine and a recording fake cargo, so no case can build the
# real core, launch a real window, or write into the developer's tree.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FAIL=0

ok() { echo "run-game: OK   $1"; }
bad() { echo "run-game: FAIL $1"; FAIL=1; }
require() {
  label="$1"
  shift
  if "$@"; then ok "$label"; else bad "$label"; fi
}
refute() {
  label="$1"
  shift
  if "$@"; then bad "$label"; else ok "$label"; fi
}
# `--` separates the helper's own arguments from a pattern that starts with a
# dash, which most of the interesting ones do.
logged() {
  case "${1:-}" in --) shift ;; esac
  grep -q -e "$1" "$LOG"
}

T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM HUP
REPO="$T/repo with spaces"
LOG="$T/calls.log"
mkdir -p "$REPO/tools/lib" "$REPO/game" "$REPO/rust/target/release" "$T/bin"
cp "$ROOT/tools/run_game.sh" "$REPO/tools/run_game.sh" 2>/dev/null || {
  echo "run-game: FAIL tools/run_game.sh does not exist"
  exit 1
}
cp "$ROOT/tools/lib/engine.sh" "$REPO/tools/lib/engine.sh"
chmod +x "$REPO/tools/run_game.sh"
printf '%s\n' '4.7.1.stable.official' >"$REPO/.godot-version"
printf '%s\n' 'fixture core' >"$REPO/rust/target/release/libunseeing_core.so"
printf '%s\n' 'fixture core' >"$REPO/rust/target/release/libunseeing_core.dylib"

make_engine() {
  cat >"$1" <<EOF
#!/bin/sh
printf 'engine %s\n' "\$*" >>"\$RUN_GAME_TEST_LOG"
# The engine reads override.cfg at startup, so the only question worth asking
# is whether the file existed at the moment the engine ran.
[ ! -f "\$RUN_GAME_TEST_OVERRIDE" ] || \
  printf 'override-present %s\n' "\$*" >>"\$RUN_GAME_TEST_LOG"
[ "\$1" = "--version" ] || exit "\${RUN_GAME_TEST_EXIT:-0}"
printf '%s\n' '$2'
EOF
  chmod +x "$1"
}
make_engine "$T/bin/godot-right" '4.7.1.stable.official.a13da4feb'
make_engine "$T/bin/godot-wrong" '4.7.0.stable.official.deadbeef'

cat >"$T/bin/cargo" <<'EOF'
#!/bin/sh
printf 'cargo %s\n' "$*" >>"$RUN_GAME_TEST_LOG"
exit "${RUN_GAME_TEST_CARGO_EXIT:-0}"
EOF
chmod +x "$T/bin/cargo"

run() { # run <engine> [args...]
  engine="$1"
  shift
  : >"$LOG"
  status=0
  env -u GODOT \
    RUN_GAME_TEST_LOG="$LOG" \
    RUN_GAME_TEST_OVERRIDE="$REPO/game/override.cfg" \
    UNSEEING_ENGINE_CANDIDATES="$T/bin/$engine" \
    UNSEEING_RUN_CARGO="$T/bin/cargo" \
    "$REPO/tools/run_game.sh" "$@" >"$T/out" 2>&1 || status=$?
}

OVERRIDE="$REPO/game/override.cfg"

# --- it plays the game, and it is not the editor ----------------------------
run godot-right
require "a default run completes" test "$status" -eq 0
require "the engine is launched against game/" logged -- "--path"
refute "the run never opens the editor" logged -- ' -e '
refute "the run never opens the editor by long flag" logged -- '--editor'
# The extension is recorded as failed-to-load in .godot/extension_list.cfg at
# import time and never retried, so a play that precedes the import gets a
# world with no engine classes in it at all.
import_at="$(grep -n -- '--import' "$LOG" | head -1 | cut -d: -f1)"
play_at="$(grep -n -- '--path' "$LOG" | grep -v -- '--import' | tail -1 | cut -d: -f1)"
if [ -z "$import_at" ] || [ -z "$play_at" ]; then
  bad "the project is imported before it is played (the run never reached both)"
else
  require "the project is imported before it is played" test "$import_at" -lt "$play_at"
fi

# --- the engine is built, through the pinned toolchain, from rust/ -----------
require "the core is built by default" logged -- 'cargo build --release'
require "the build carries the Inspector docs the editor build also carries" \
  logged -- '--features editor-docs'

run godot-right --skip-build
require "--skip-build completes" test "$status" -eq 0
refute "--skip-build does not build" logged -- 'cargo build'
require "--skip-build still plays the game" logged -- "--path"

# --- a wrong engine is refused before anything is built or launched ---------
run godot-wrong
require "an engine that fails the pin is refused" test "$status" -eq 2
refute "a refused engine is never built for" logged -- 'cargo build'
refute "a refused engine is never launched" logged -- "--path"

# --- a failing build never reaches the game ---------------------------------
RUN_GAME_TEST_CARGO_EXIT=7 run godot-right
require "a failed build propagates" test "$status" -eq 1
refute "a failed build never launches the game" logged -- "--path"
unset RUN_GAME_TEST_CARGO_EXIT

# --- the game's own exit status is the tool's -------------------------------
RUN_GAME_TEST_EXIT=3 run godot-right --skip-build
require "the game's exit status is passed through" test "$status" -eq 3
unset RUN_GAME_TEST_EXIT

# --- windowed mode, and the file it must never leave behind -----------------
run godot-right --windowed
require "--windowed completes" test "$status" -eq 0
refute "--windowed leaves no override.cfg behind" test -f "$OVERRIDE"

# The engine reads override.cfg at startup, so writing it after the launch
# would configure nothing. Recorded by the fake engine: the file must exist at
# the moment it is invoked.
run godot-right --windowed
require "override.cfg exists while the engine runs" logged -- 'override-present'

run godot-right --windowed 640x480
require "--windowed takes an explicit size" test "$status" -eq 0

# A pre-existing override.cfg belongs to whoever wrote it — another probe, or a
# designer mid-experiment. Overwriting and then deleting it destroys their work
# silently, which is exactly what tools/probe_visibility.sh refuses to do.
printf 'someone else\n' >"$OVERRIDE"
run godot-right --windowed
require "a pre-existing override.cfg is refused, not clobbered" test "$status" -eq 2
require "the refused run leaves the other file untouched" grep -q 'someone else' "$OVERRIDE"
refute "the refused run never launches" logged -- "--path"
rm -f "$OVERRIDE"

# --- passthrough and refusal of nonsense ------------------------------------
run godot-right --skip-build --scene 'res://scenes/level_02.tscn'
require "--scene reaches the engine" logged -- 'res://scenes/level_02.tscn'

run godot-right --skip-build -- --verbose
require "arguments after -- reach the engine" logged -- '--verbose'

run godot-right --no-such-option
require "an unknown option is refused" test "$status" -eq 2
require "the refusal prints usage" grep -q 'usage:' "$T/out"
refute "an unknown option never launches" logged -- "--path"

exit "$FAIL"
