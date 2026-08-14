#!/bin/sh
# Build the engine and play the GAME — not the editor.
#
# tools/bootstrap.sh gets a fresh machine to a working editor. This is the loop
# you live in afterwards: change some Rust, see the world. It selects the pinned
# Godot, rebuilds the core, lets the engine re-import it, and launches the game
# itself. Windows uses tools\run_game.cmd, which provides the same contract.
#
# Usage: tools/run_game.sh [options] [-- <godot arguments>]
#   --windowed [WxH]   run in a window (default 1280x720) instead of the
#                      project's shipped full screen
#   --scene <res://…>  play one scene instead of the project's main scene
#   --seed <n>         set UNSEEING_SEED, for a reproducible world
#   --demo             set UNSEEING_DEMO=1
#   --skip-build       play what is already built
#
# Env knobs: GODOT (binary), UNSEEING_RUN_CARGO (cargo to build with).
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

usage() {
  cat >&2 <<'USAGE'
usage: tools/run_game.sh [--windowed [WxH]] [--scene res://path.tscn]
                        [--seed <n>] [--demo] [--skip-build] [-- <godot args>]
USAGE
}

WINDOWED=0
GEOMETRY=1280x720
SCENE=''
SEED=''
DEMO=0
BUILD=1

while [ "$#" -gt 0 ]; do
  case "$1" in
    --windowed)
      WINDOWED=1
      # The size is optional, so only swallow the next argument when it IS one.
      # `[0-9]*x[0-9]*` alone would accept 1280x720p and 1920x1080x2 and then
      # split them blindly into override.cfg, so the shape is checked in full.
      case "${2:-}" in
        *[!0-9x]*|x*|*x) : ;;
        *x*)
          case "${2%x*}${2#*x}" in
            ''|*[!0-9]*) : ;;
            *) GEOMETRY="$2"; shift ;;
          esac
          ;;
      esac
      ;;
    --scene)
      SCENE="${2:-}"
      # Refuses an option as its value, the way --seed does. `--scene --demo`
      # otherwise set SCENE=--demo, dropped --demo, and handed the engine a
      # positional argument that is not a scene.
      case "$SCENE" in
        ''|-*) echo "run-game: --scene needs a res:// path" >&2; usage; exit 2 ;;
      esac
      shift
      ;;
    --seed)
      SEED="${2:-}"
      case "$SEED" in
        ''|*[!0-9]*) echo "run-game: --seed needs a whole number" >&2; usage; exit 2 ;;
      esac
      shift
      ;;
    --demo) DEMO=1 ;;
    --skip-build) BUILD=0 ;;
    -h|--help) usage; exit 0 ;;
    # break BEFORE the trailing shift, so "$@" is exactly the arguments after
    # the separator. They were flattened into one string and re-split, which
    # tore apart any Godot argument containing a space — a --write-movie path,
    # for one, and this repository tests itself under "repo with spaces".
    --) shift; break ;;
    *)
      echo "run-game: unknown option '$1'" >&2
      usage
      exit 2
      ;;
  esac
  shift
done

# The engine gate first, for the same reason tools/bootstrap.sh does it first:
# it costs milliseconds, and rebuilding the core for an editor that will be
# refused afterwards is time spent for nothing.
# shellcheck source=tools/lib/engine.sh
. "$DIR/tools/lib/engine.sh"
GODOT="$(unseeing_engine_select "$DIR" "${GODOT:-}")" || {
  echo "run-game: no Godot matching .godot-version; set GODOT=/path/to/godot" >&2
  exit 2
}

case "$(uname)" in
  Darwin) ARTIFACT="$DIR/rust/target/release/libunseeing_core.dylib" ;;
  Linux) ARTIFACT="$DIR/rust/target/release/libunseeing_core.so" ;;
  *)
    echo "run-game: this entry point covers macOS and Linux (uname says $(uname))" >&2
    printf '%s\n' 'run-game: on Windows run tools\run_game.cmd' >&2
    exit 2
    ;;
esac

if [ "$BUILD" = 1 ]; then
  CARGO="${UNSEEING_RUN_CARGO:-}"
  if [ -z "$CARGO" ]; then
    CARGO_DIR="${CARGO_HOME:-${HOME:-}/.cargo}"
    [ -f "$CARGO_DIR/env" ] && . "$CARGO_DIR/env"
    CARGO=cargo
  fi
  command -v "$CARGO" >/dev/null 2>&1 || [ -x "$CARGO" ] || {
    echo "run-game: cargo not found — run tools/bootstrap.sh first, it installs the toolchain" >&2
    exit 2
  }
  echo "run-game: building the engine"
  # From rust/, so rustup's directory override applies rust-toolchain.toml and
  # the pinned compiler is used without naming it here. editor-docs matches what
  # tools/bootstrap.sh builds into this same path, so alternating between the
  # editor and the game does not rebuild the world each time.
  (cd "$DIR/rust" && "$CARGO" build --release --features editor-docs) || {
    echo "run-game: FAILED rust build (see errors above)" >&2
    exit 1
  }
  [ -f "$ARTIFACT" ] || {
    echo "run-game: FAILED cargo exited 0 but there is no core at $ARTIFACT" >&2
    exit 1
  }
fi

[ -f "$ARTIFACT" ] || {
  echo "run-game: FAILED no engine core at $ARTIFACT" >&2
  echo "run-game:        drop --skip-build, or run tools/bootstrap.sh" >&2
  exit 1
}

# The game boots full screen at its native resolution because the PROJECT says
# so, and Godot's own window flags lose to the project setting. override.cfg is
# the documented escape hatch: merged over project.godot before the window
# exists. Written before the launch, removed however this script ends —
# including HUP, which is how closing a terminal on a windowed run arrives, and
# the case that used to leave a file the repository forbids shipping.
OVERRIDE="$DIR/game/override.cfg"
if [ "$WINDOWED" = 1 ]; then
  if [ -e "$OVERRIDE" ]; then
    echo "run-game: FAILED game/override.cfg already exists — a windowed run would overwrite and then delete it." >&2
    echo "run-game:        remove it if it is a leftover, or wait for the run that owns it to finish." >&2
    exit 2
  fi
  trap 'rm -f "$OVERRIDE"' EXIT INT TERM HUP
  WIDTH="${GEOMETRY%x*}"
  HEIGHT="${GEOMETRY#*x}"
  cat >"$OVERRIDE" <<CFG
[display]

window/size/mode=0
window/size/viewport_width=$WIDTH
window/size/viewport_height=$HEIGHT
CFG
fi

# After any build, never before: a failed extension load is recorded in
# .godot/extension_list.cfg at import time and never retried, so a play that
# runs first gets a world with no engine classes in it at all.
"$GODOT" --headless --path "$DIR/game" --import >/dev/null 2>&1 || true

[ "$SEED" = '' ] || export UNSEEING_SEED="$SEED"
[ "$DEMO" = 0 ] || export UNSEEING_DEMO=1

# Built up rather than interpolated: ${WINDOWED:+ } tests for a NON-EMPTY
# value, and WINDOWED is "0" when off, so the conditional space was never
# conditional and every ordinary run announced itself with a trailing blank.
ANNOUNCE="run-game: playing"
[ -z "$SCENE" ] || ANNOUNCE="$ANNOUNCE $SCENE"
[ "$WINDOWED" = 0 ] || ANNOUNCE="$ANNOUNCE (windowed $GEOMETRY)"
echo "$ANNOUNCE"
# No -e and no --editor: this is the world, not the authoring environment.
if [ -n "$SCENE" ]; then
  "$GODOT" --path "$DIR/game" "$@" "$SCENE"
else
  "$GODOT" --path "$DIR/game" "$@"
fi
