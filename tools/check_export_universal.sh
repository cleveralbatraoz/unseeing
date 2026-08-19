#!/bin/sh
# Universal-binary check over a whole macOS export — the .zip Godot produced,
# or the unpacked .app directory.
# Usage: tools/check_export_universal.sh <exported .zip | .app directory>
#
# tools/check_universal.sh asks whether the extension the export was HANDED is
# universal. This asks whether the thing that came out is, which is a different
# question and the only one that describes what a collaborator downloads.
#
# It is also where the clobber trap finally closes. `cargo build --release`
# (no --target) writes the same path the universal core occupies, so a build
# started in another session — another agent, the same worktree — can replace
# it in the seconds between the pre-export check and Godot copying the file
# into the bundle. Every earlier verdict would still read green. Reading the
# shipped bundle is the only statement immune to that.
#
# Three ways to pass that must never be mistaken for passing:
#   - no Mach-O in the export at all (a wrong path argument)
#   - a bundle with no libunseeing_core.dylib in it (Godot silently not
#     copying the GDExtension, which boots into a world with no engine nodes)
#   - a bundle that still carries a loose .pck (embed_pck silently not
#     taking effect, which ships a stray file the preset promised not to)
# All three are failures here, because "I checked every binary and they were
# fine" is a lie when there were none, or when something else slipped past.
#
# Exit: 0 the export is universal throughout, 1 it is not, 2 the invocation or
# the host is wrong.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

[ "$#" -eq 1 ] || {
  echo "check-export: usage: check_export_universal.sh <exported .zip | .app directory>"
  exit 2
}
TARGET="$1"

command -v lipo >/dev/null 2>&1 || {
  echo "check-export: lipo not found — this check needs macOS and the Xcode command line tools"
  exit 2
}

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT INT TERM

if [ -d "$TARGET" ]; then
  ROOT="$TARGET"
elif [ -f "$TARGET" ]; then
  command -v unzip >/dev/null 2>&1 || {
    echo "check-export: unzip not found — needed to look inside $TARGET"
    exit 2
  }
  if ! unzip -q "$TARGET" -d "$WORK/x" 2>"$WORK/unzip.err"; then
    echo "check-export: FAILED $TARGET could not be unpacked"
    sed 's/^/check-export:        /' "$WORK/unzip.err"
    exit 1
  fi
  ROOT="$WORK/x"
else
  echo "check-export: FAILED $TARGET does not exist"
  echo "check-export:        run tools/export_macos.sh to produce it"
  exit 1
fi

# The file list is materialised rather than piped into the loop: a pipeline
# runs its right-hand side in a subshell, and every count below would be
# discarded the moment the loop ended.
find "$ROOT" -type f -print > "$WORK/files"

MACHO=0
THIN=0
CORE=""
PCK=""
while IFS= read -r f; do
  case "${f##*/}" in
    *.pck) PCK="$f" ;;
  esac
  # lipo IS the Mach-O test: it reads fat and thin headers and refuses
  # everything else, so a .pck, a plist or a code signature falls out here
  # without needing a list of extensions to keep up to date.
  if ! lipo -archs "$f" >/dev/null 2>&1; then continue; fi
  MACHO=$((MACHO + 1))
  case "${f##*/}" in
    libunseeing_core.dylib) CORE="$f" ;;
  esac
  if ! "$DIR/tools/check_universal.sh" "$f"; then THIN=$((THIN + 1)); fi
done < "$WORK/files"

if [ "$MACHO" -eq 0 ]; then
  echo "check-export: FAILED no Mach-O binary anywhere in $TARGET"
  echo "check-export:        nothing was verified — is that really an export?"
  exit 1
fi

if [ -z "$CORE" ]; then
  echo "check-export: FAILED $TARGET carries no libunseeing_core.dylib"
  echo "check-export:        the wave core IS the engine; without it the export boots an empty world"
  exit 1
fi

if [ "$THIN" -gt 0 ]; then
  echo "check-export: FAILED $THIN of $MACHO binaries in $TARGET are not universal"
  echo "check-export:        the macOS preset declares binary_format/architecture=\"universal\""
  exit 1
fi

if [ -n "$PCK" ]; then
  echo "check-export: FAILED $TARGET still carries $(basename "$PCK")"
  echo "check-export:        the macOS preset declares binary_format/embed_pck=true"
  exit 1
fi

echo "check-export: OK   $TARGET ships $MACHO universal binaries, libunseeing_core.dylib among them"
