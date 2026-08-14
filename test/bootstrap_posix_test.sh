#!/bin/sh
# Behavioral contract for the macOS/Linux designer bootstrap. It executes a
# copied production script inside a checkout fixture whose path contains spaces;
# Rustup and Godot are recording boundary fakes, never mirrors of bootstrap
# decisions.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SUBJECT="${BOOTSTRAP_SUBJECT:-$ROOT/tools/bootstrap.sh}"
FAIL=0

ok() { echo "bootstrap-posix: OK   $1"; }
bad() { echo "bootstrap-posix: FAIL $1"; FAIL=1; }
require() {
  label="$1"
  shift
  if "$@"; then ok "$label"; else bad "$label"; fi
}
require_absent() {
  label="$1"
  needle="$2"
  file="$3"
  if grep -q -- "$needle" "$file"; then bad "$label"; else ok "$label"; fi
}

T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM
REPO="$T/repo with spaces"
FAKE="$T/fake bin"
INSTALL="$T/install bin"
LOG="$T/calls.log"
OUT="$T/output.log"
mkdir -p "$REPO/tools/lib" "$REPO/rust" "$REPO/game" "$FAKE" "$INSTALL" "$T/home"
cp "$SUBJECT" "$REPO/tools/bootstrap.sh"
# The engine gate is part of the subject: bootstrap.sh sources it, and it is
# what decides whether the fixture editor is the pinned one.
cp "$ROOT/tools/lib/engine.sh" "$REPO/tools/lib/engine.sh"
cp "$ROOT/rust/rust-toolchain.toml" "$REPO/rust/rust-toolchain.toml"
chmod +x "$REPO/tools/bootstrap.sh"
printf '%s\n' '4.7.1.stable.official' >"$REPO/.godot-version"

cat >"$FAKE/rustup" <<'EOF'
#!/bin/sh
printf 'rustup %s\n' "$*" >>"$BOOTSTRAP_TEST_LOG"
if [ "$*" = "--version" ]; then
  echo 'rustup 1.28.2 (fixture)'
  exit 0
fi
if [ "$*" = "run 1.97.1 rustc --version" ]; then
  if [ ! -f "$BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL" ]; then
    echo 'error: pinned fixture toolchain is not installed' >&2
    exit 1
  fi
  echo "${BOOTSTRAP_TEST_RUSTC_VERSION:-rustc 1.97.1 (fixture)}"
  exit 0
fi
if [ "$*" = "toolchain install 1.97.1 --profile minimal" ]; then
  : >"$BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL"
  exit 0
fi
if [ "$*" = "run 1.97.1 cargo --version" ]; then
  echo 'cargo 1.97.1 (fixture)'
  exit 0
fi
case "$*" in
  'run 1.97.1 cargo build --release --features editor-docs --target-dir '*)
    [ "${BOOTSTRAP_TEST_CARGO_FAIL:-0}" != 1 ] || exit 19
    if [ "${BOOTSTRAP_TEST_SKIP_ARTIFACT:-0}" != 1 ]; then
      mkdir -p "$(dirname "$BOOTSTRAP_TEST_ARTIFACT")"
      printf '%s\n' 'fixture library' >"$BOOTSTRAP_TEST_ARTIFACT"
    fi
    exit 0
    ;;
esac
exit 0
EOF

cat >"$FAKE/godot" <<'EOF'
#!/bin/sh
printf 'godot %s\n' "$*" >>"$BOOTSTRAP_TEST_LOG"
if [ "$*" = "--version" ]; then
  echo "${BOOTSTRAP_TEST_GODOT_VERSION:-4.7.1.stable.official.fixture}"
  exit 0
fi
case " $* " in
  *' --import '*) [ "${BOOTSTRAP_TEST_IMPORT_FAIL:-0}" != 1 ] || exit 17 ;;
  *engine_census_probe.gd*)
    [ "${BOOTSTRAP_TEST_CENSUS_FAIL:-0}" != 1 ] || exit 23
    if [ "${BOOTSTRAP_TEST_WRONG_CENSUS:-0}" = 1 ]; then
      echo 'probe: PASS (18 checks)'
    else
      echo 'probe: PASS (19 checks)'
    fi
    ;;
esac
exit 0
EOF

cat >"$FAKE/cc" <<'EOF'
#!/bin/sh
exit 0
EOF
chmod +x "$FAKE/rustup" "$FAKE/godot" "$FAKE/cc"

# Mirrors the real rustup-init on the one point that matters here: it installs
# into CARGO_HOME when that is set, not into $HOME/.cargo.
cat >"$INSTALL/install-rustup" <<'EOF'
#!/bin/sh
printf '%s\n' 'rustup installer invoked' >>"$BOOTSTRAP_TEST_LOG"
target="${CARGO_HOME:-$HOME/.cargo}/bin"
mkdir -p "$target"
cp "$BOOTSTRAP_TEST_INSTALL_RUSTUP_SOURCE" "$target/rustup"
chmod +x "$target/rustup"
EOF
chmod +x "$INSTALL/install-rustup"
cp "$FAKE/cc" "$INSTALL/cc"
chmod +x "$INSTALL/cc"

clear_flags() {
  unset BOOTSTRAP_TEST_CARGO_FAIL BOOTSTRAP_TEST_GODOT_VERSION
  unset BOOTSTRAP_TEST_IMPORT_FAIL BOOTSTRAP_TEST_CENSUS_FAIL
  unset BOOTSTRAP_TEST_WRONG_CENSUS BOOTSTRAP_TEST_SKIP_ARTIFACT
  unset BOOTSTRAP_TEST_RUSTC_VERSION
}

run_fixture() {
  : >"$LOG"
  mkdir -p "$REPO/rust/target/release"
  printf '%s\n' 'stale library' >"$REPO/rust/target/release/libunseeing_core.so"
  printf '%s\n' 'stale library' >"$REPO/rust/target/release/libunseeing_core.dylib"
  BOOTSTRAP_TEST_ARTIFACT="$REPO/rust/target/release/libunseeing_core.so"
  if [ "$(uname)" = Darwin ]; then
    BOOTSTRAP_TEST_ARTIFACT="$REPO/rust/target/release/libunseeing_core.dylib"
  fi
  export BOOTSTRAP_TEST_ARTIFACT
  BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL="$T/toolchain-installed"
  export BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL
  status=0
  HOME="$T/home" PATH="$FAKE:/usr/bin:/bin" GODOT="$FAKE/godot" \
    UNSEEING_BOOTSTRAP_RUSTUP="$FAKE/rustup" \
    BOOTSTRAP_TEST_LOG="$LOG" "$REPO/tools/bootstrap.sh" >"$OUT" 2>&1 || status=$?
}

run_install_fixture() {
  : >"$LOG"
  mkdir -p "$REPO/rust/target/release"
  BOOTSTRAP_TEST_ARTIFACT="$REPO/rust/target/release/libunseeing_core.so"
  if [ "$(uname)" = Darwin ]; then
    BOOTSTRAP_TEST_ARTIFACT="$REPO/rust/target/release/libunseeing_core.dylib"
  fi
  export BOOTSTRAP_TEST_ARTIFACT
  BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL="$T/install-toolchain-installed"
  export BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL
  BOOTSTRAP_TEST_INSTALL_RUSTUP_SOURCE="$FAKE/rustup"
  export BOOTSTRAP_TEST_INSTALL_RUSTUP_SOURCE
  status=0
  HOME="$T/home" PATH="$INSTALL:/usr/bin:/bin" GODOT="$FAKE/godot" \
    UNSEEING_BOOTSTRAP_INSTALL_RUSTUP="$INSTALL/install-rustup" \
    BOOTSTRAP_TEST_LOG="$LOG" "$REPO/tools/bootstrap.sh" >"$OUT" 2>&1 || status=$?
}

clear_flags
run_fixture
require "the checkout path with spaces completes" test "$status" -eq 0
require "the release editor-docs artifact is built" \
  grep -q "rustup run 1.97.1 cargo build --release --features editor-docs --target-dir $REPO/rust/target" "$LOG"
require "the exact compiler pin is selected through rustup" \
  grep -q "rustup run 1.97.1 rustc --version" "$LOG"
require "a fresh rustup receives the pinned toolchain without a second command" \
  grep -q "rustup toolchain install 1.97.1 --profile minimal" "$LOG"
require "the exact census permits success" grep -q "bootstrap: OK" "$OUT"
import_line="$(grep -n -- '--import' "$LOG" | cut -d: -f1 | head -1)"
census_line="$(grep -n -- 'engine_census_probe.gd' "$LOG" | cut -d: -f1 | head -1)"
# Both must actually be there: an empty log used to reach `test '' -lt ''`,
# which reports a shell error alongside the failure and says nothing about why.
if [ -z "$import_line" ] || [ -z "$census_line" ]; then
  bad "import happens before census (the run never reached both stages)"
else
  require "import happens before census" test "$import_line" -lt "$census_line"
fi

find "$T/home/.cargo" -type f -delete 2>/dev/null || true
find "$T/home/.cargo" -type d -delete 2>/dev/null || true
run_install_fixture
require "rustup absence invokes the installer and completes in one command" test "$status" -eq 0
require "the rustup installer boundary was actually crossed" \
  grep -q "rustup installer invoked" "$LOG"
require "the newly installed rustup is discovered in the current process" \
  grep -q "rustup run 1.97.1 cargo build" "$LOG"

# Docker's rust images, and plenty of CI images, export CARGO_HOME away from
# $HOME/.cargo. rustup-init honours it; the bootstrap looked only at
# $HOME/.cargo, so a perfectly successful install was reported as a failed one
# — with advice to reopen the terminal, which could never help.
: >"$LOG"
mkdir -p "$T/empty home" "$T/cargo home"
BOOTSTRAP_TEST_INSTALL_RUSTUP_SOURCE="$FAKE/rustup"
export BOOTSTRAP_TEST_INSTALL_RUSTUP_SOURCE
BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL="$T/cargo-home-toolchain"
export BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL
BOOTSTRAP_TEST_ARTIFACT="$REPO/rust/target/release/libunseeing_core.so"
if [ "$(uname)" = Darwin ]; then
  BOOTSTRAP_TEST_ARTIFACT="$REPO/rust/target/release/libunseeing_core.dylib"
fi
export BOOTSTRAP_TEST_ARTIFACT
status=0
HOME="$T/empty home" CARGO_HOME="$T/cargo home" \
  PATH="$INSTALL:/usr/bin:/bin" GODOT="$FAKE/godot" \
  UNSEEING_BOOTSTRAP_INSTALL_RUSTUP="$INSTALL/install-rustup" \
  BOOTSTRAP_TEST_LOG="$LOG" "$REPO/tools/bootstrap.sh" >"$OUT" 2>&1 || status=$?
require "an install into CARGO_HOME is found instead of reported as failed" \
  test "$status" -eq 0
require_absent "a CARGO_HOME install is never called unusable" \
  "did not leave a usable rustup" "$OUT"

clear_flags
export BOOTSTRAP_TEST_IMPORT_FAIL=1
run_fixture
require "a noisy cache import yields to the authoritative census" test "$status" -eq 0

clear_flags
export BOOTSTRAP_TEST_GODOT_VERSION='4.7.0.stable.official.fixture'
run_fixture
require "a nearby Godot version is refused" test "$status" -eq 2
require_absent "a version refusal never imports" "--import" "$LOG"
# The engine gate costs milliseconds and needs nothing the build produces, so
# paying for a full release build before discovering the editor is wrong is
# pure waste — 45 s of it, measured on the audit machine. The Windows entry
# point has always checked Godot first; this is the POSIX side catching up.
require_absent "a version refusal never builds" "cargo build" "$LOG"

# A Mono/.NET editor is still the pinned editor: .godot-version pins a version,
# not a build flavour. This rejection is what made the most convenient Linux
# install unusable.
clear_flags
export BOOTSTRAP_TEST_GODOT_VERSION='4.7.1.stable.mono.official.fixture'
run_fixture
require "a Mono build of the pinned version is accepted" test "$status" -eq 0
require "the accepted Mono build reaches the census" \
  grep -q "engine_census_probe.gd" "$LOG"

# Without the pin file the gate used to vanish entirely — any engine at all was
# accepted — and then $WANT went unbound at the very last line, so a run that
# had built, imported and passed the census died reporting a Godot problem.
clear_flags
mv "$REPO/.godot-version" "$T/godot-version.away"
run_fixture
require "a checkout with no .godot-version is refused" test "$status" -eq 2
require_absent "a missing pin never builds" "cargo build" "$LOG"
require_absent "a missing pin never announces success" "bootstrap: OK" "$OUT"
mv "$T/godot-version.away" "$REPO/.godot-version"

clear_flags
export BOOTSTRAP_TEST_CARGO_FAIL=1
run_fixture
require "a failed Rust build propagates" test "$status" -eq 1
require_absent "a failed build never imports a stale extension" "--import" "$LOG"

clear_flags
export BOOTSTRAP_TEST_SKIP_ARTIFACT=1
run_fixture
require "a no-op build cannot reuse the library left by an earlier checkout" test "$status" -eq 1
require_absent "a missing fresh artifact never reaches import" "--import" "$LOG"

clear_flags
export BOOTSTRAP_TEST_RUSTC_VERSION='rustc 1.97.0 (fixture)'
run_fixture
require "a rustup toolchain with the wrong compiler is refused" test "$status" -eq 2
require_absent "a compiler-pin refusal never builds" "cargo build" "$LOG"

clear_flags
export BOOTSTRAP_TEST_CENSUS_FAIL=1
run_fixture
require "a failed class census propagates" test "$status" -eq 1
require_absent "a failed census never announces success" "bootstrap: OK" "$OUT"

clear_flags
export BOOTSTRAP_TEST_WRONG_CENSUS=1
run_fixture
require "a successful process with the wrong class count is refused" test "$status" -eq 1
require_absent "the wrong census never announces success" "bootstrap: OK" "$OUT"

clear_flags
unset BOOTSTRAP_TEST_ARTIFACT
exit "$FAIL"
