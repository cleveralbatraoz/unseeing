#!/bin/sh
# The macOS export — the whole act, in one place: build the wave core as a
# universal dylib, verify it, export the "macOS" preset, then verify what came
# out. macOS ships on demand rather than on every push, so this is a tool you
# run, not a stage of ci/pipeline.sh (whose default path builds and exports web
# only, and whose rust stage is a plain single-arch `cargo build --release`
# that must stay exactly that for everyday development).
#
# The universal checks live HERE, bracketing the export, because the act of
# producing a macOS build is the only moment at which a single-arch core is
# wrong. There is no knob to skip either of them: an env switch on a gate is
# a way to ship the bug the gate exists for.
#
# The clobber trap, and why the check is run twice. `cargo build --release`
# (no --target) writes to target/release/, which is exactly where the universal
# core sits, so any ordinary build replaces it with a thin one. Nothing here
# can stop that, so nothing here trusts the file it finds: the core is rebuilt
# and re-fused every run (cheap — the per-slice artifacts live in
# target/<triple>/release/, which a host build never touches), read back off
# disk immediately before Godot is invoked, and then read again out of the
# bundle Godot produced. A build landing mid-export — another session, the same
# worktree — is caught by the last of those three.
#
# Code signing and notarization are out of scope: game/export_presets.cfg
# asks for built-in ad-hoc signing, and nothing here adds an identity.
#
# Env knobs: GODOT (binary).
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

[ "$(uname)" = "Darwin" ] || {
  echo "export-macos: a macOS export needs macOS (uname says $(uname))"
  exit 2
}

GODOT="${GODOT:-}"
if [ -z "$GODOT" ]; then
  for g in godot "$HOME/bin/godot" /opt/homebrew/bin/godot; do
    if command -v "$g" >/dev/null 2>&1 || [ -x "$g" ]; then GODOT="$g"; break; fi
  done
fi
[ -n "$GODOT" ] || { echo "export-macos: godot not found; set GODOT=/path/to/godot"; exit 2; }

# The export templates are versioned with the engine, so a mismatched binary
# would wrap this build in someone else's runtime.
if [ -f "$DIR/.godot-version" ]; then
  WANT="$(cat "$DIR/.godot-version")"
  HAVE="$("$GODOT" --version 2>/dev/null | head -1)"
  case "$HAVE" in
    "$WANT"*) : ;;
    *)
      echo "export-macos: FAILED godot version '$HAVE' != pinned '$WANT' (set GODOT= to a matching binary)"
      exit 2
      ;;
  esac
fi

# The path game/unseeing.gdextension names for both macOS keys.
CORE="$DIR/rust/target/release/libunseeing_core.dylib"

echo "export-macos: building the universal wave core"
"$DIR/tools/build_macos_core.sh" || exit 1

echo "export-macos: verifying the core Godot is about to be handed"
"$DIR/tools/check_universal.sh" "$CORE" || exit 1

echo "export-macos: exporting the macOS preset (clean)"
rm -rf "$DIR/game/build/macos"
mkdir -p "$DIR/game/build/macos"
# game/build/ is gitignored and must stay out of Godot's resource scan.
touch "$DIR/game/build/.gdignore"
LOG="$DIR/game/build/macos/export.log"
if ! "$GODOT" --headless --path "$DIR/game" \
  --export-release "macOS" build/macos/unseeing.zip > "$LOG" 2>&1; then
  tail -20 "$LOG"
  echo "export-macos: FAILED export exited non-zero (full log: $LOG)"
  exit 1
fi

# Judged by the artifact, never by the log. ci/pipeline.sh's export stage
# works the same way, and for the same reason: Godot prints the word "error"
# in contexts that are not errors — `update_scripts_classes` names every
# class it registers, and gdUnit4 alone contributes GdUnitError,
# ErrorLogEntry and GdUnitScriptErrorCollector to a perfectly clean run. The
# question that matters is not what the exporter said but what it produced,
# and the answer to that is read out of the bundle below.
ZIP="$DIR/game/build/macos/unseeing.zip"
echo "export-macos: verifying the bundle that came out"
"$DIR/tools/check_export_universal.sh" "$ZIP" || exit 1

echo "export-macos: OK   $ZIP ($(wc -c < "$ZIP" | tr -d ' ') bytes)"
