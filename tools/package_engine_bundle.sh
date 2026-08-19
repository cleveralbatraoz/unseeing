#!/bin/sh
# Assembles one designer-facing editor bundle: the tracked game/ tree plus a
# single already-built engine library, laid out exactly as
# game/unseeing.gdextension expects it (the library one level ABOVE game/,
# under rust/target/<dest-relative-path>), wrapped in one named top-level
# folder so extracting the zip never splatters files into wherever the
# designer unzipped it.
#
# Usage: package_engine_bundle.sh <platform-label> <source-library> <dest-relative-path> <output-zip>
#   platform-label      e.g. linux-x86_64 -- becomes the wrapping folder
#                        name "unseeing-editor-<platform-label>"
#   source-library       path to the already-built release library (this
#                        script never builds anything)
#   dest-relative-path   where that library lands under rust/target/, e.g.
#                        "release/libunseeing_core.so" or
#                        "aarch64-pc-windows-msvc/release/unseeing_core.dll"
#                        -- must match a key in game/unseeing.gdextension
#   output-zip            where to write the finished zip
#
# Env knobs: none. The commit stamped into ENGINE_COMMIT is always
# `git rev-parse HEAD` of the checkout this runs in -- there is exactly one
# honest answer to "which commit was this built from" and it is not a
# parameter.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

[ $# -eq 4 ] || {
  echo "package-engine-bundle: usage: package_engine_bundle.sh <platform-label> <source-library> <dest-relative-path> <output-zip>"
  exit 2
}
PLATFORM="$1"
SRC_LIB="$2"
DEST_REL="$3"
OUT_ZIP="$4"
case "$OUT_ZIP" in
  /*) : ;;
  *) OUT_ZIP="$(pwd)/$OUT_ZIP" ;;
esac

[ -s "$SRC_LIB" ] || {
  echo "package-engine-bundle: FAILED source library missing or empty: $SRC_LIB"
  exit 2
}
command -v git >/dev/null 2>&1 || {
  echo "package-engine-bundle: FAILED git not found"
  exit 2
}
# Not the `zip` CLI: it is not reliably present on GitHub's windows-latest
# hosted runner (only 7z is guaranteed there, and even that has shipped
# with gaps). python3's stdlib zipfile module is already relied on
# elsewhere in this same release pipeline (the Godot-fetch steps extract
# with it), so it is already proven present on every runner this script
# needs to run on -- Linux, Windows (via `shell: bash`, which still runs
# whatever python3 the runner has on PATH), and macOS.
command -v python3 >/dev/null 2>&1 || {
  echo "package-engine-bundle: FAILED python3 not found (used to create the zip portably)"
  exit 2
}
COMMIT="$(cd "$DIR" && git rev-parse HEAD 2>/dev/null)" || {
  echo "package-engine-bundle: FAILED cannot resolve HEAD -- is $DIR a git checkout?"
  exit 2
}

ROOT="unseeing-editor-$PLATFORM"
T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM
STAGE="$T/$ROOT"
mkdir -p "$STAGE"

echo "package-engine-bundle: archiving the tracked game/ tree"
( cd "$DIR" && git archive HEAD -- game ) | ( cd "$STAGE" && tar -x )
[ -f "$STAGE/game/project.godot" ] || {
  echo "package-engine-bundle: FAILED git archive did not produce game/project.godot"
  exit 1
}

echo "package-engine-bundle: placing the engine library at rust/target/$DEST_REL"
LIB_DEST="$STAGE/rust/target/$DEST_REL"
mkdir -p "$(dirname "$LIB_DEST")"
cp "$SRC_LIB" "$LIB_DEST"

printf '%s\n' "$COMMIT" > "$STAGE/ENGINE_COMMIT"

mkdir -p "$(dirname "$OUT_ZIP")"
rm -f "$OUT_ZIP"
python3 -c '
import os, sys, zipfile
stage, root, out_zip = sys.argv[1], sys.argv[2], sys.argv[3]
with zipfile.ZipFile(out_zip, "w", zipfile.ZIP_DEFLATED) as zf:
    for dirpath, _dirnames, filenames in os.walk(os.path.join(stage, root)):
        for name in filenames:
            full = os.path.join(dirpath, name)
            rel = os.path.relpath(full, stage)
            zf.write(full, rel)
' "$T" "$ROOT" "$OUT_ZIP" || {
  echo "package-engine-bundle: FAILED could not create $OUT_ZIP"
  exit 1
}
echo "package-engine-bundle: OK   $OUT_ZIP ($(du -k "$OUT_ZIP" | cut -f1) KB)"
