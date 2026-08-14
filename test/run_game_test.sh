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
# One line per argument as well, so assertions can match a WHOLE argument.
# Grepping the joined string for ' -e ' only finds -e in a non-final position:
# a launch ending in -e opens the editor and slips past unnoticed.
for a in "\$@"; do printf 'arg %s\n' "\$a" >>"\$RUN_GAME_TEST_LOG"; done
# The engine reads override.cfg at startup, so the questions worth asking are
# whether it existed at that moment and what it actually said.
if [ -f "\$RUN_GAME_TEST_OVERRIDE" ]; then
  printf 'override-present %s\n' "\$*" >>"\$RUN_GAME_TEST_LOG"
  sed 's/^/override-line /' "\$RUN_GAME_TEST_OVERRIDE" >>"\$RUN_GAME_TEST_LOG"
fi
[ "\$1" = "--version" ] || exit_now=1
if [ "\$1" = "--version" ]; then
  printf '%s\n' '$2'
  exit 0
fi
# A run in progress, so the signal arms of the caller's trap can be reached.
# Only the real play invocation parks; the import must still return.
case " \$* " in
  *" --import "*) exit "\${RUN_GAME_TEST_EXIT:-0}" ;;
esac
if [ -n "\${RUN_GAME_TEST_HANG:-}" ]; then
  printf '%s\n' "\$\$" >"\$RUN_GAME_TEST_HANG"
  while :; do sleep 1; done
fi
exit "\${RUN_GAME_TEST_EXIT:-0}"
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
# Matched as a whole argument, on its own line. A launch whose LAST argument is
# -e opens the authoring environment, and a joined-string search for ' -e '
# cannot see it — measured: appending -e to the launch left all assertions green.
refute "the run never opens the editor" grep -qx -- 'arg -e' "$LOG"
refute "the run never opens the editor by long flag" grep -qx -- 'arg --editor' "$LOG"
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

# What the file SAYS, not merely that it existed. Nothing read it before, so a
# tool writing mode=2 (full screen) and ignoring the requested size passed every
# windowed assertion here.
run godot-right --windowed 640x480
require "--windowed takes an explicit size" test "$status" -eq 0
require "the override asks for windowed mode, not full screen" \
  grep -qx 'override-line window/size/mode=0' "$LOG"
require "the override carries the requested width" \
  grep -qx 'override-line window/size/viewport_width=640' "$LOG"
require "the override carries the requested height" \
  grep -qx 'override-line window/size/viewport_height=480' "$LOG"

run godot-right --windowed
require "the default size is 1280x720" \
  grep -qx 'override-line window/size/viewport_width=1280' "$LOG"

# A size the tool cannot split must not reach override.cfg as junk — the engine
# would read a non-numeric viewport and nothing downstream would catch it. It is
# refused rather than quietly replaced by the default, because someone who typed
# a size meant that size and a silent fallback hides the typo.
run godot-right --windowed 1280x720p
require "a malformed size is refused" test "$status" -eq 2
refute "a malformed size never reaches override.cfg" \
  grep -q '1280x720p' "$LOG"
refute "a malformed size never launches" logged -- "--path"

# ...while a flag following --windowed is still a flag, not a size.
run godot-right --skip-build --windowed --demo
require "--windowed does not swallow the option after it" test "$status" -eq 0
require "the option after --windowed still took effect" \
  grep -qx 'override-line window/size/viewport_width=1280' "$LOG"

# The signal arms of the trap were never exercised — every case above exits
# normally, so deleting `INT TERM HUP` left the suite green while the failure
# they exist for (closing a terminal on a windowed run) leaked the file.
: >"$LOG"
ENGINE_PID_FILE="$T/engine.pid"
rm -f "$ENGINE_PID_FILE"
env -u GODOT \
  RUN_GAME_TEST_LOG="$LOG" \
  RUN_GAME_TEST_OVERRIDE="$OVERRIDE" \
  RUN_GAME_TEST_HANG="$ENGINE_PID_FILE" \
  UNSEEING_ENGINE_CANDIDATES="$T/bin/godot-right" \
  UNSEEING_RUN_CARGO="$T/bin/cargo" \
  "$REPO/tools/run_game.sh" --skip-build --windowed >"$T/out" 2>&1 &
hang_pid=$!
# Poll for the engine reporting itself in play, not a guessed interval. The
# bound exists only so a broken tool cannot hang the suite.
tries=0
while [ ! -s "$ENGINE_PID_FILE" ] && [ "$tries" -lt 400 ]; do
  tries=$((tries + 1))
  sleep 0.01 2>/dev/null || true
done
if [ -s "$ENGINE_PID_FILE" ] && [ -f "$OVERRIDE" ]; then
  # What closing a terminal actually does: HUP reaches the shell and the child
  # it is waiting on. Without HUP in the trap the shell dies from the signal
  # outright, the EXIT arm never runs, and the file is left behind.
  kill -HUP "$hang_pid" 2>/dev/null || true
  kill -HUP "$(cat "$ENGINE_PID_FILE")" 2>/dev/null || true
  wait "$hang_pid" 2>/dev/null || true
  refute "a hung-up windowed run still removes override.cfg" test -f "$OVERRIDE"
else
  kill "$hang_pid" 2>/dev/null || true
  wait "$hang_pid" 2>/dev/null || true
  bad "a hung-up windowed run still removes override.cfg (the run never reached play)"
fi
rm -f "$OVERRIDE" "$ENGINE_PID_FILE"

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

run godot-right --skip-build --scene --demo
require "--scene refuses an option as its value" test "$status" -eq 2
refute "--scene never launches with an option as its scene" logged -- "--path"

run godot-right --skip-build -- --verbose
require "arguments after -- reach the engine" logged -- '--verbose'

# One argument, not three. Flattening the passthrough into a string and letting
# the shell re-split it tore apart any Godot argument containing a space — a
# --write-movie path, for one, and this fixture checkout is itself under a path
# with a space in it.
run godot-right --skip-build -- --write-movie '/tmp/My Frames/out.avi'
require "a passthrough argument containing a space arrives whole" \
  grep -qx -- 'arg /tmp/My Frames/out.avi' "$LOG"
refute "a passthrough argument containing a space is not split" \
  grep -qx -- 'arg /tmp/My' "$LOG"

run godot-right --no-such-option
require "an unknown option is refused" test "$status" -eq 2
require "the refusal prints usage" grep -q 'usage:' "$T/out"
refute "an unknown option never launches" logged -- "--path"

exit "$FAIL"
