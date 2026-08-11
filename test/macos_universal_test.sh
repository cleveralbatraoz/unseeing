#!/bin/sh
# The universal-binary check, exercised against fixtures it must accept and
# fixtures it must reject.
#
# tools/check_universal.sh is the only thing standing between a macOS export
# and the bug it exists for: an arm64-only extension wrapped in a bundle whose
# preset says "universal", which an Intel Mac cannot load at all. That check is
# deliberately absent from ci/pipeline.sh's always-on path — a plain
# `cargo build --release` is single-arch by design and must stay green — so
# nothing else in this repository would notice if it stopped binding. This is
# what notices.
#
# Fixtures are eight-byte dylibs built here, never the real core: the check's
# subject is the fat header, and a header is the same shape whatever it wraps.
#
# Pure POSIX sh. macOS-only by nature (lipo, Apple slices) — announces itself
# as skipped anywhere else rather than passing on nothing.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"
CHECK="$DIR/tools/check_universal.sh"
FAIL=0

ok() { echo "universal: OK   $1"; }
bad() { echo "universal: FAIL $1"; FAIL=1; }
skip() { echo "universal: SKIP $1"; }

if [ "$(uname)" != "Darwin" ]; then
  skip "not macOS — lipo and Apple slices do not exist here"
  exit 0
fi
for t in lipo cc; do
  command -v "$t" >/dev/null 2>&1 || {
    skip "$t not found (install the Xcode command line tools)"
    exit 0
  }
done
[ -x "$CHECK" ] || {
  bad "tools/check_universal.sh missing or not executable"
  exit 1
}

T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM
printf 'int unseeing_fixture(void){return 0;}\n' > "$T/u.c"

slice() { # slice <arch> <out>
  cc -arch "$1" -dynamiclib -o "$2" "$T/u.c" 2>/dev/null
}
slice arm64 "$T/arm64.dylib"
slice x86_64 "$T/x86_64.dylib"
lipo -create -output "$T/universal.dylib" "$T/arm64.dylib" "$T/x86_64.dylib"
printf 'this is not a Mach-O file\n' > "$T/text.dylib"

# `probe <expected-exit> <label> [argument...]` — a non-zero exit is the
# expected result of most cases here, so `|| got=$?` keeps errexit from
# mistaking the point of the test for a failure of the harness.
probe() {
  want="$1"
  label="$2"
  shift 2
  got=0
  "$CHECK" "$@" >"$T/out" 2>&1 || got=$?
  if [ "$got" -eq "$want" ]; then
    ok "$label"
  else
    bad "$label (expected exit $want, got $got)"
    sed 's/^/universal:      /' "$T/out"
  fi
}

names() { # names <needle> <label> — the rejection has to be diagnosable
  if grep -q -- "$1" "$T/out"; then
    ok "$2"
  else
    bad "$2 (no '$1' in the output)"
    sed 's/^/universal:      /' "$T/out"
  fi
}

probe 0 "accepts a genuine universal dylib" "$T/universal.dylib"

# The shipping bug itself: this is exactly what `cargo build --release`
# produces on an Apple Silicon laptop, and exactly what must never reach a
# bundle declaring binary_format/architecture="universal".
probe 1 "rejects an arm64-only dylib" "$T/arm64.dylib"
names "missing x86_64" "arm64-only rejection names the architecture that is missing"
names "arm64.dylib" "arm64-only rejection names the file"

# The mirror case. A check written against the laptop's failure alone would
# pass this one, and an arm64-only Mac is not the only way to ship half a
# binary — a cross-build with one --target dropped lands here.
probe 1 "rejects an x86_64-only dylib" "$T/x86_64.dylib"
names "missing arm64" "x86_64-only rejection names the architecture that is missing"

# Substring blindness, which is the natural way to write this check wrong:
# `case "$archs" in *arm64*)` is satisfied by arm64e. An arm64e slice needs
# entitlements no shipped game has and loads on no customer's Mac, so a fat
# file carrying x86_64 + arm64e is still broken for every Apple Silicon user
# while looking universal to a glob.
if cc -arch arm64e -dynamiclib -o "$T/arm64e.dylib" "$T/u.c" 2>/dev/null; then
  lipo -create -output "$T/arm64e-fat.dylib" "$T/arm64e.dylib" "$T/x86_64.dylib"
  probe 1 "rejects a fat dylib whose Apple slice is arm64e, not arm64" "$T/arm64e-fat.dylib"
  names "missing arm64" "arm64e rejection names the architecture that is missing"

  # ...and the other half of that law: extra slices are not a reason to
  # reject. The requirement is that both architectures are PRESENT, never
  # that they are the only ones there.
  lipo -create -output "$T/three.dylib" \
    "$T/arm64.dylib" "$T/x86_64.dylib" "$T/arm64e.dylib"
  probe 0 "accepts a fat dylib carrying arm64 and x86_64 among extra slices" "$T/three.dylib"
else
  skip "arm64e fixtures (this cc cannot target arm64e)"
fi

# A missing artifact must be a failure, not a vacuous pass: the export path
# runs this check immediately before handing the file to Godot, and "nothing
# is there" is the one answer that must never read as "verified".
probe 1 "rejects a missing file" "$T/absent.dylib"
names absent.dylib "missing-file rejection names the file"

# Something at the path that is not a Mach-O at all — a stale placeholder, a
# half-written file from a killed build. lipo fails on it; the check must
# turn that into a verdict rather than leaking a raw fatal error and exit 1
# that happens to look right by accident.
probe 1 "rejects a file that is not Mach-O" "$T/text.dylib"

# Called with no artifact it has nothing to verify. Exit 2, the project's
# code for "the environment or the invocation is wrong", so a caller can tell
# a broken call apart from a broken binary.
probe 2 "refuses an invocation with no file to check"

exit "$FAIL"
