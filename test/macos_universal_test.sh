#!/bin/sh
# The two universal-binary checks, exercised against fixtures they must accept
# and fixtures they must reject.
#
# tools/check_universal.sh (one Mach-O file) and
# tools/check_export_universal.sh (a whole exported bundle) are the only things
# standing between a macOS build and the bug they exist for: an arm64-only
# extension wrapped in a bundle whose preset says "universal", which an Intel
# Mac cannot load at all. Both are deliberately absent from ci/pipeline.sh's
# always-on path — a plain `cargo build --release` is single-arch by design and
# must stay green — so nothing else in this repository would notice if they
# stopped binding. This is what notices.
#
# Fixtures are eight-byte dylibs built here, never the real core: the subject
# is the fat header, and a header is the same shape whatever it wraps.
#
# Pure POSIX sh. macOS-only by nature (lipo, Apple slices) — announces itself
# as skipped anywhere else rather than passing on nothing.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"
CHECK="$DIR/tools/check_universal.sh"
CHECK_EXPORT="$DIR/tools/check_export_universal.sh"
FAIL=0

ok() { echo "universal: OK   $1"; }
bad() { echo "universal: FAIL $1"; FAIL=1; }
skip() { echo "universal: SKIP $1"; }

if [ "$(uname)" != "Darwin" ]; then
  skip "not macOS — lipo and Apple slices do not exist here"
  exit 0
fi
for t in lipo cc zip unzip; do
  command -v "$t" >/dev/null 2>&1 || {
    skip "$t not found (install the Xcode command line tools)"
    exit 0
  }
done
for c in "$CHECK" "$CHECK_EXPORT"; do
  [ -x "$c" ] || {
    bad "$c missing or not executable"
    exit 1
  }
done

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

# `probe <script> <expected-exit> <label> [argument...]` — a non-zero exit is
# the expected result of most cases here, so `|| got=$?` keeps errexit from
# mistaking the point of the test for a failure of the harness.
probe() {
  script="$1"
  want="$2"
  label="$3"
  shift 3
  got=0
  "$script" "$@" >"$T/out" 2>&1 || got=$?
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

# --- one file: tools/check_universal.sh -------------------------------------

probe "$CHECK" 0 "accepts a genuine universal dylib" "$T/universal.dylib"

# The shipping bug itself: this is exactly what `cargo build --release`
# produces on an Apple Silicon laptop, and exactly what must never reach a
# bundle declaring binary_format/architecture="universal".
probe "$CHECK" 1 "rejects an arm64-only dylib" "$T/arm64.dylib"
names "missing x86_64" "arm64-only rejection names the architecture that is missing"
names "arm64.dylib" "arm64-only rejection names the file"

# The mirror case. A check written against the laptop's failure alone would
# pass this one, and an arm64-only Mac is not the only way to ship half a
# binary — a cross-build with one --target dropped lands here.
probe "$CHECK" 1 "rejects an x86_64-only dylib" "$T/x86_64.dylib"
names "missing arm64" "x86_64-only rejection names the architecture that is missing"

# Substring blindness, which is the natural way to write this check wrong:
# `case "$archs" in *arm64*)` is satisfied by arm64e. An arm64e slice needs
# entitlements no shipped game has and loads on no customer's Mac, so a fat
# file carrying x86_64 + arm64e is still broken for every Apple Silicon user
# while looking universal to a glob.
if cc -arch arm64e -dynamiclib -o "$T/arm64e.dylib" "$T/u.c" 2>/dev/null; then
  lipo -create -output "$T/arm64e-fat.dylib" "$T/arm64e.dylib" "$T/x86_64.dylib"
  probe "$CHECK" 1 "rejects a fat dylib whose Apple slice is arm64e, not arm64" "$T/arm64e-fat.dylib"
  names "missing arm64" "arm64e rejection names the architecture that is missing"

  # ...and the other half of that law: extra slices are not a reason to
  # reject. The requirement is that both architectures are PRESENT, never
  # that they are the only ones there.
  lipo -create -output "$T/three.dylib" \
    "$T/arm64.dylib" "$T/x86_64.dylib" "$T/arm64e.dylib"
  probe "$CHECK" 0 "accepts a fat dylib carrying arm64 and x86_64 among extra slices" "$T/three.dylib"
else
  skip "arm64e fixtures (this cc cannot target arm64e)"
fi

# A missing artifact must be a failure, not a vacuous pass: the export path
# runs this check immediately before handing the file to Godot, and "nothing
# is there" is the one answer that must never read as "verified".
probe "$CHECK" 1 "rejects a missing file" "$T/absent.dylib"
names absent.dylib "missing-file rejection names the file"
# ...and says it is ABSENT. `names absent.dylib` alone cannot pin that: delete
# the [ -f ] guard and lipo rejects the missing file too, exit 1 and all, with
# a message that interpolates the same path. The two verdicts want different
# things from the reader — "build it" versus "something wrong is sitting at
# this path" — so the distinguishing words are the assertion.
names "does not exist" "missing-file rejection says it is absent, not that it is a broken Mach-O"

# Something at the path that is not a Mach-O at all — a stale placeholder, a
# half-written file from a killed build. lipo fails on it; the check must
# turn that into a verdict rather than leaking a raw fatal error and exit 1
# that happens to look right by accident.
probe "$CHECK" 1 "rejects a file that is not Mach-O" "$T/text.dylib"

# Called with no artifact it has nothing to verify. Exit 2, the project's
# code for "the environment or the invocation is wrong", so a caller can tell
# a broken call apart from a broken binary.
probe "$CHECK" 2 "refuses an invocation with no file to check"

# --- a whole export: tools/check_export_universal.sh ------------------------
#
# Checking the dylib on the way IN proves what the export was handed. This
# checks what it produced, which is a different claim and the only one that is
# about what a collaborator downloads. It also closes the clobber trap at its
# last possible moment: a `cargo build --release` landing between the check and
# Godot's copy — another session, another agent, the same worktree — would put
# a thin core into the bundle with every earlier verdict still reading green.

bundle() { # bundle <name> <core dylib or "none"> [executable dylib]
  root="$T/$1"
  app="$root/unseeing.app/Contents"
  mkdir -p "$app/MacOS" "$app/Frameworks" "$app/Resources"
  cp "${3:-$T/universal.dylib}" "$app/MacOS/unseeing"
  [ "$2" = "none" ] || cp "$2" "$app/Frameworks/libunseeing_core.dylib"
  # A .pck and a plist: real bundle contents that are not Mach-O at all. The
  # walk must step over them rather than call them broken binaries.
  printf 'GDPC not a mach-o\n' > "$app/Resources/unseeing.pck"
  printf '<plist/>\n' > "$app/Info.plist"
  (cd "$root" && zip -qr "$T/$1.zip" unseeing.app)
}

bundle good "$T/universal.dylib"
probe "$CHECK_EXPORT" 0 "accepts an export whose binaries are all universal" "$T/good/unseeing.app"
probe "$CHECK_EXPORT" 0 "accepts that same export as the shipped .zip" "$T/good.zip"

# THE defect, seen from the artifact end: the bundle Godot actually produced,
# carrying the arm64-only core a plain cargo build leaves at the .gdextension
# path. Every other signal in the build says universal; this is the one that
# reads the bytes that were shipped.
bundle thin "$T/arm64.dylib"
probe "$CHECK_EXPORT" 1 "rejects an export whose wave core is arm64-only" "$T/thin.zip"
names "missing x86_64" "the thin-core rejection names the architecture that is missing"
names "libunseeing_core.dylib" "the thin-core rejection names the binary inside the bundle"

# The template binary is universal, the core is missing entirely — Godot
# quietly not copying the GDExtension. Every Mach-O present is universal, so a
# check that only walked what it found would call this export perfect while it
# boots into a world with no engine nodes at all.
bundle coreless none
probe "$CHECK_EXPORT" 1 "rejects an export with no wave core in it" "$T/coreless.zip"
names "libunseeing_core.dylib" "the coreless rejection names what is missing"

# ...and the sharper version of that same claim, because the case above cannot
# make it alone. Widen the match from `libunseeing_core.dylib)` to `*.dylib)`
# and every other case stays green: no fixture until this one put a SECOND
# dylib in a bundle, so nothing distinguished "the wave core is present" from
# "some dylib is present" — which is the whole of what that guard asserts. It
# fails closed today only because Godot's bundle happens to contain exactly one
# dylib, and this is the check that has the last word on what ships.
#
# Everything here is universal and there are two Mach-O files, so neither the
# vacuity guard nor the thin-binary count can answer. Only a core-SPECIFIC
# guard rejects this bundle.
mkdir -p "$T/decoy/unseeing.app/Contents/MacOS" "$T/decoy/unseeing.app/Contents/Frameworks"
cp "$T/universal.dylib" "$T/decoy/unseeing.app/Contents/MacOS/unseeing"
cp "$T/universal.dylib" "$T/decoy/unseeing.app/Contents/Frameworks/libsomething.dylib"
(cd "$T/decoy" && zip -qr "$T/decoy.zip" unseeing.app)
probe "$CHECK_EXPORT" 1 "rejects an export carrying some other dylib but no wave core" "$T/decoy.zip"
names "libunseeing_core.dylib" "the decoy rejection names the core itself, not merely 'a dylib'"

# Vacuity: nothing Mach-O anywhere. "I checked every binary and they were all
# fine" is a lie when there were none, and this is the shape a wrong path
# argument takes.
#
# The exit code alone cannot pin this, and a mutation proved it: an export
# with no Mach-O in it also has no wave core, so the core-presence rejection
# fires on the same fixture and answers 1 too. Weakening the vacuity guard to
# a condition that never holds changed nothing any assertion could see. So the
# MESSAGE is the assertion here — "you pointed me at something that is not an
# export" and "this export is missing its engine" are different failures with
# different fixes, and a check that cannot tell them apart is worth less than
# one that can.
mkdir -p "$T/empty/unseeing.app/Contents/Resources"
printf 'GDPC\n' > "$T/empty/unseeing.app/Contents/Resources/unseeing.pck"
probe "$CHECK_EXPORT" 1 "rejects an export containing no Mach-O binary at all" "$T/empty"
names "no Mach-O binary anywhere" "the empty-export rejection says nothing was verified, not that a core is missing"

# The template's own executable is half a binary while the core is fine — the
# other way this bundle fails its preset. binary_format/architecture is a
# promise about everything inside, not only about the extension.
bundle halftemplate "$T/universal.dylib" "$T/arm64.dylib"
probe "$CHECK_EXPORT" 1 "rejects an export whose main executable is arm64-only" "$T/halftemplate.zip"

probe "$CHECK_EXPORT" 1 "rejects an export path that does not exist" "$T/never-exported.zip"
probe "$CHECK_EXPORT" 2 "refuses an invocation with no export to check"

# --- the build path: a slice must be proof that THIS run produced it --------
#
# `cargo exited 0` and `a file sits at the conventional path` are two facts,
# and neither implies the other. A warm target/ — the normal state of a
# developer's machine — holds slices from earlier checkouts indefinitely, so
# any build that succeeds WITHOUT writing there leaves a stale artifact to be
# fused and shipped under this commit's name. Three ways in, none exotic:
# a dropped `--target`, a CARGO_TARGET_DIR or --target-dir redirect, and a
# `[build] target-dir` in a config file nobody in this repo can see.
#
# The stub cargo below is all three at once: it exits 0 and writes nothing.
BUILDER="$DIR/tools/build_macos_core.sh"
if [ ! -x "$BUILDER" ]; then
  bad "tools/build_macos_core.sh missing or not executable"
else
  FAKE="$T/fake"
  mkdir -p "$FAKE/tools" "$FAKE/rust/target/release" \
    "$FAKE/rust/target/aarch64-apple-darwin/release" \
    "$FAKE/rust/target/x86_64-apple-darwin/release" \
    "$T/stub" "$T/stubfail" "$T/nohome"
  cp "$BUILDER" "$CHECK" "$FAKE/tools/"

  printf '#!/bin/sh\nexit 0\n' > "$T/stub/cargo"
  printf '#!/bin/sh\nexit 1\n' > "$T/stubfail/cargo"
  for d in stub stubfail; do
    printf '#!/bin/sh\nprintf "aarch64-apple-darwin\\nx86_64-apple-darwin\\n"\n' \
      > "$T/$d/rustup"
    chmod +x "$T/$d/cargo" "$T/$d/rustup"
  done

  # HOME is redirected because build_macos_core.sh sources $HOME/.cargo/env
  # when it exists, and that puts the REAL cargo ahead of the stub on PATH.
  runner() { # runner <stub dir> <out script>
    cat > "$2" <<EOF
#!/bin/sh
PATH="$T/$1:\$PATH" HOME="$T/nohome" exec "$FAKE/tools/build_macos_core.sh"
EOF
    chmod +x "$2"
  }
  runner stub "$T/run_stub.sh"
  runner stubfail "$T/run_stubfail.sh"

  seed_stale() { # leftovers from an earlier checkout, both triples
    rm -f "$FAKE/rust/target/release/libunseeing_core.dylib"
    cp "$T/arm64.dylib" "$FAKE/rust/target/aarch64-apple-darwin/release/libunseeing_core.dylib"
    cp "$T/x86_64.dylib" "$FAKE/rust/target/x86_64-apple-darwin/release/libunseeing_core.dylib"
  }

  # The one that matters. Both stale slices are in place and together they
  # WOULD fuse into a perfectly valid universal binary — which is exactly why
  # no check downstream can catch this: the artifact is not malformed, it is
  # merely not this commit's.
  seed_stale
  probe "$T/run_stub.sh" 1 "refuses a slice this run did not build, however valid it looks"
  names "is not there" "the stale-slice refusal names the guard that caught it"
  if [ -e "$FAKE/rust/target/release/libunseeing_core.dylib" ]; then
    bad "a stale slice was fused into the core anyway"
  else
    ok "no core is produced when a slice could not be rebuilt"
  fi

  # The neighbouring branch, so the case above cannot pass by firing the wrong
  # guard: cargo failing outright must be reported as a failed build, not as a
  # missing file.
  seed_stale
  probe "$T/run_stubfail.sh" 1 "refuses when cargo itself fails, stale slice or not"
  names "FAILED cargo build" "the failed-build refusal names cargo, not the missing file"
fi

exit "$FAIL"
