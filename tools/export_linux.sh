#!/bin/sh
# The Linux export — build the wave core (a plain, single-arch release
# build; Linux has no universal-binary concept the way macOS does), verify
# it exists, export the requested preset, then verify what came out.
# Ships on demand rather than on every push, matching tools/export_macos.sh's
# reasoning exactly: ci/pipeline.sh's default path builds and exports web
# only, and its rust stage stays a plain `cargo build --release`.
#
# Godot's own GDExtension-aware export copies the matching engine library
# (rust/target/release/libunseeing_core.so, the same path
# game/unseeing.gdextension already names for both Linux keys) into the
# export output automatically -- there is no custom packaging step here,
# unlike the designer editor bundle, because export produces a compiled
# game, not an editable project.
#
# Code signing is out of scope for Linux entirely (no OS-level signing
# concept the way macOS/Windows have); ships unsigned, same v1 decision
# already made for the other desktop platforms.
#
# Usage: export_linux.sh <preset-name> <export-relative-path>
#   e.g. export_linux.sh "Linux x86_64" build/linux/unseeing
#        export_linux.sh "Linux arm64" build/linux-arm64/unseeing
#
# Env knobs: GODOT (binary).
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

[ "$(uname)" = "Linux" ] || {
  echo "export-linux: a Linux export needs Linux (uname says $(uname))"
  exit 2
}
[ $# -eq 2 ] || {
  echo "export-linux: usage: export_linux.sh <preset-name> <export-relative-path>"
  exit 2
}
PRESET="$1"
EXPORT_REL="$2"

# shellcheck source=tools/lib/engine.sh
. "$DIR/tools/lib/engine.sh"
GODOT="$(unseeing_engine_select "$DIR" "${GODOT:-}")" || {
  echo "export-linux: FAILED no Godot matching .godot-version; set GODOT=/path/to/godot"
  exit 2
}

CORE="$DIR/rust/target/release/libunseeing_core.so"

echo "export-linux: building the release core"
( cd "$DIR/rust" && cargo build --release ) || exit 1

[ -s "$CORE" ] || {
  echo "export-linux: FAILED cargo exited 0 but $CORE is missing or empty"
  exit 1
}

echo "export-linux: exporting the '$PRESET' preset (clean)"
OUT_DIR="$DIR/game/$(dirname "$EXPORT_REL")"
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"
# game/build/ is gitignored and must stay out of Godot's resource scan.
touch "$DIR/game/build/.gdignore"
# A sibling of OUT_DIR, not inside it: OUT_DIR is exactly what a packaging
# step ships to players (export_windows.sh matches; export_macos.sh's own
# export target is already a self-contained zip Godot builds directly, so
# it never had this problem).
LOG="$OUT_DIR.log"
if ! "$GODOT" --headless --path "$DIR/game" \
  --export-release "$PRESET" "$EXPORT_REL" > "$LOG" 2>&1; then
  tail -20 "$LOG"
  echo "export-linux: FAILED export exited non-zero (full log: $LOG)"
  exit 1
fi

# Judged by the artifact, never by the log -- same reasoning as
# export_macos.sh: Godot prints the word "error" in contexts that are not
# errors, so the question that matters is what it produced, not what it said.
OUT="$DIR/game/$EXPORT_REL"
[ -s "$OUT" ] || {
  echo "export-linux: FAILED no executable at $OUT after a green export"
  exit 1
}
LIB="$OUT_DIR/libunseeing_core.so"
[ -s "$LIB" ] || {
  echo "export-linux: FAILED the exported bundle has no libunseeing_core.so beside it"
  exit 1
}
# The preset asks for binary_format/embed_pck=true; a loose .pck here means
# that stopped taking effect and players would get a stray file back.
PCK="$OUT_DIR/$(basename "$EXPORT_REL").pck"
[ ! -e "$PCK" ] || {
  echo "export-linux: FAILED found $PCK -- embed_pck should have folded it into the executable"
  exit 1
}

echo "export-linux: OK   $OUT ($(wc -c < "$OUT" | tr -d ' ') bytes)"
