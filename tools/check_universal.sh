#!/bin/sh
# Universal-binary check — does this Mach-O file carry BOTH Apple
# architectures? Usage: tools/check_universal.sh <file>
#
# game/export_presets.cfg declares binary_format/architecture="universal" for
# the macOS preset, and game/unseeing.gdextension names ONE path for both
# macOS keys, with no architecture suffix. So that one file is the whole
# promise: single-arch, the bundle claims to be universal and the extension
# refuses to load on half the Macs it was exported for — silently, because
# nothing about a thin dylib looks wrong until the loader reaches it.
#
# Deliberately its own script rather than a function inside the export path:
# a check that can only run as part of the thing it checks cannot be shown to
# reject anything. test/macos_universal_test.sh points it at fixtures that
# must fail, which is the only evidence that it still binds.
#
# It reads the file's own fat header every time it is asked. No stamp, no
# mtime, no memory of an earlier run — `cargo build --release` (no --target)
# writes the very path the universal artifact occupies, so the only trustworthy
# answer is the one taken from the bytes on disk at the moment of asking.
#
# Exit: 0 universal, 1 the file is not, 2 the invocation or the host is wrong.
set -eu

[ "$#" -eq 1 ] || {
  echo "check_universal: usage: check_universal.sh <mach-o file>"
  exit 2
}
TARGET="$1"

command -v lipo >/dev/null 2>&1 || {
  echo "check_universal: lipo not found — this check needs macOS and the Xcode command line tools"
  exit 2
}

[ -f "$TARGET" ] || {
  echo "check_universal: FAILED $TARGET does not exist"
  echo "check_universal:        build it with tools/build_macos_core.sh"
  exit 1
}

if ! ARCHS="$(lipo -archs "$TARGET" 2>&1)"; then
  echo "check_universal: FAILED $TARGET is not a Mach-O binary lipo can read"
  printf 'check_universal:        %s\n' "$ARCHS"
  exit 1
fi

# Matched as whole words, never as substrings: arm64e is a different
# architecture that needs entitlements no shipped game has, and a `case`
# glob for *arm64* would wave a file carrying only that one straight
# through. Extra slices are fine — the law is that both are PRESENT.
has_arch() { # has_arch <arch> <list>
  for a in $2; do
    if [ "$a" = "$1" ]; then return 0; fi
  done
  return 1
}

MISSING=""
for want in arm64 x86_64; do
  if ! has_arch "$want" "$ARCHS"; then MISSING="${MISSING:+$MISSING }$want"; fi
done

if [ -n "$MISSING" ]; then
  echo "check_universal: FAILED $TARGET is missing $MISSING"
  echo "check_universal:        it carries [$ARCHS], and a universal macOS export needs arm64 and x86_64"
  echo "check_universal:        rebuild it with tools/build_macos_core.sh"
  exit 1
fi

echo "check_universal: OK   $TARGET is universal [$ARCHS]"
