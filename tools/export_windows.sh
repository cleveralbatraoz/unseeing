#!/bin/sh
# The Windows export — build the wave core for the requested triple, verify
# it exists, export the requested preset, then verify what came out. Same
# shape as tools/export_linux.sh and tools/export_macos.sh: ships on demand,
# not on every push.
#
# Windows x86_64 and arm64 are genuinely separate builds (no universal-binary
# concept), matching game/unseeing.gdextension's own per-triple DLL paths and
# game/export_presets.cfg's two separate "Windows x86_64"/"Windows arm64"
# presets.
#
# Runs under `sh` (Git Bash on a real Windows host, or the bash GitHub
# Actions provides via `shell: bash` on a windows-latest/windows-11-arm
# runner) -- cargo and the MSVC toolchain are set up system/user-wide by
# rustup and Visual Studio respectively, not per-shell, so building from a
# POSIX shell here works identically to PowerShell.
#
# Code signing is out of scope (v1 ships unsigned on every desktop
# platform); a SmartScreen warning is expected and accepted.
#
# Usage: export_windows.sh <preset-name> <cargo-target-triple> <export-relative-path>
#   e.g. export_windows.sh "Windows x86_64" x86_64-pc-windows-msvc build/windows/unseeing.exe
#        export_windows.sh "Windows arm64" aarch64-pc-windows-msvc build/windows-arm64/unseeing.exe
#
# Env knobs: GODOT (binary).
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

[ $# -eq 3 ] || {
  echo "export-windows: usage: export_windows.sh <preset-name> <cargo-target-triple> <export-relative-path>"
  exit 2
}
PRESET="$1"
TRIPLE="$2"
EXPORT_REL="$3"

# shellcheck source=tools/lib/engine.sh
. "$DIR/tools/lib/engine.sh"
GODOT="$(unseeing_engine_select "$DIR" "${GODOT:-}")" || {
  echo "export-windows: FAILED no Godot matching .godot-version; set GODOT=/path/to/godot"
  exit 2
}

CORE="$DIR/rust/target/$TRIPLE/release/unseeing_core.dll"

echo "export-windows: building the release core for $TRIPLE"
( cd "$DIR/rust" && cargo build --release --target "$TRIPLE" ) || exit 1

[ -s "$CORE" ] || {
  echo "export-windows: FAILED cargo exited 0 but $CORE is missing or empty"
  exit 1
}

echo "export-windows: exporting the '$PRESET' preset (clean)"
OUT_DIR="$DIR/game/$(dirname "$EXPORT_REL")"
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"
# game/build/ is gitignored and must stay out of Godot's resource scan.
touch "$DIR/game/build/.gdignore"
# A sibling of OUT_DIR, not inside it -- see export_linux.sh's comment.
LOG="$OUT_DIR.log"
if ! "$GODOT" --headless --path "$DIR/game" \
  --export-release "$PRESET" "$EXPORT_REL" > "$LOG" 2>&1; then
  tail -20 "$LOG"
  echo "export-windows: FAILED export exited non-zero (full log: $LOG)"
  exit 1
fi

# Judged by the artifact, never by the log -- same reasoning as
# export_macos.sh and export_linux.sh.
OUT="$DIR/game/$EXPORT_REL"
[ -s "$OUT" ] || {
  echo "export-windows: FAILED no executable at $OUT after a green export"
  exit 1
}
DLL="$OUT_DIR/unseeing_core.dll"
[ -s "$DLL" ] || {
  echo "export-windows: FAILED the exported bundle has no unseeing_core.dll beside it"
  exit 1
}

echo "export-windows: OK   $OUT ($(wc -c < "$OUT" | tr -d ' ') bytes)"
