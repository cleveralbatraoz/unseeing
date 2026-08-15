#!/bin/sh
# Behavioral contract for the cheap host gate that runs before any deploy
# build, upload, or push. The regression this catches is probing
# `cargo zigbuild --version`: Cargo passes `zigbuild` to the external command,
# but cargo-zigbuild owns --version only at its top level.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SUBJECT="${DEPLOY_HOST_PREFLIGHT_SUBJECT:-$ROOT/ci/deploy_host_preflight.sh}"
FAIL=0

ok() { echo "deploy-preflight: OK   $1"; }
bad() { echo "deploy-preflight: FAIL $1"; FAIL=1; }
require() {
  label="$1"
  shift
  if "$@"; then ok "$label"; else bad "$label"; fi
}
contains() { case "$2" in *"$1"*) return 0 ;; *) return 1 ;; esac; }

[ -f "$SUBJECT" ] || {
  echo "deploy-preflight: FAIL $SUBJECT does not exist"
  exit 1
}

T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM HUP
BIN="$T/bin"
REPO="$T/repo with spaces"
mkdir -p "$BIN" "$REPO/rust"
printf '#!/bin/sh\nexit 0\n' >"$REPO/rust/build-wasm.sh"
chmod +x "$REPO/rust/build-wasm.sh"

write_ok() {
  name="$1"
  printf '#!/bin/sh\nexit 0\n' >"$BIN/$name"
  chmod +x "$BIN/$name"
}

write_ok cargo
write_ok scp
write_ok curl

cat >"$BIN/cargo-zigbuild" <<'EOF'
#!/bin/sh
[ "$#" -eq 1 ] && [ "$1" = "--version" ] || exit 9
printf '%s\n' 'cargo-zigbuild fixture'
EOF
chmod +x "$BIN/cargo-zigbuild"

cat >"$BIN/zig" <<'EOF'
#!/bin/sh
[ "$#" -eq 1 ] && [ "$1" = "version" ] || exit 9
printf '%s\n' '0.fixture'
EOF
chmod +x "$BIN/zig"

cat >"$BIN/git" <<'EOF'
#!/bin/sh
[ "$#" -eq 5 ] || exit 9
[ "$1" = "-C" ] || exit 9
[ "$3" = "remote" ] || exit 9
[ "$4" = "get-url" ] || exit 9
[ "$5" = "production" ] || exit 9
printf '%s\n' 'vpn:git/unseeing.git'
EOF
chmod +x "$BIN/git"

cat >"$BIN/ssh" <<'EOF'
#!/bin/sh
[ "$#" -eq 6 ] || exit 9
[ "$1" = "-o" ] && [ "$2" = "BatchMode=yes" ] || exit 9
[ "$3" = "-o" ] && [ "$4" = "ConnectTimeout=10" ] || exit 9
[ "$5" = "vpn" ] && [ "$6" = "true" ] || exit 9
EOF
chmod +x "$BIN/ssh"

run_subject() {
  status=0
  output="$(PATH="$BIN" "$SUBJECT" "$REPO" 2>&1)" || status=$?
}

run_subject
require "top-level cargo-zigbuild and Zig version probes admit a usable host" \
  test "$status" -eq 0
require "a usable host receives the one success record" \
  contains 'deploy: preflight OK' "$output"

# Mutation: the old probe. This fake accepts only the malformed Cargo-shaped
# argument list. A gate that regresses to `cargo zigbuild --version` goes green;
# the required top-level call must refuse it.
cat >"$BIN/cargo-zigbuild" <<'EOF'
#!/bin/sh
[ "$#" -eq 2 ] && [ "$1" = "zigbuild" ] && [ "$2" = "--version" ]
EOF
chmod +x "$BIN/cargo-zigbuild"
run_subject
require "a tool that rejects top-level --version is refused" test "$status" -eq 2
require "the cargo-zigbuild refusal names the missing capability" \
  contains 'cargo-zigbuild' "$output"

cat >"$BIN/cargo-zigbuild" <<'EOF'
#!/bin/sh
[ "$#" -eq 1 ] && [ "$1" = "--version" ]
EOF
chmod +x "$BIN/cargo-zigbuild"
rm "$BIN/zig"
run_subject
require "a missing Zig executable is refused" test "$status" -eq 2
require "the missing-Zig refusal names Zig" contains 'Zig' "$output"

cat >"$BIN/zig" <<'EOF'
#!/bin/sh
exit 7
EOF
chmod +x "$BIN/zig"
run_subject
require "a Zig executable whose version call fails is refused" test "$status" -eq 2

cat >"$BIN/zig" <<'EOF'
#!/bin/sh
[ "$#" -eq 1 ] && [ "$1" = "version" ]
EOF
chmod +x "$BIN/zig"
cat >"$BIN/git" <<'EOF'
#!/bin/sh
exit 8
EOF
chmod +x "$BIN/git"
run_subject
require "a missing production remote is refused" test "$status" -eq 2
require "the remote refusal names production" contains 'production' "$output"

cat >"$BIN/git" <<'EOF'
#!/bin/sh
printf '%s\n' 'vpn:git/unseeing.git'
EOF
chmod +x "$BIN/git"
cat >"$BIN/ssh" <<'EOF'
#!/bin/sh
exit 8
EOF
chmod +x "$BIN/ssh"
run_subject
require "an unreachable vpn endpoint is refused" test "$status" -eq 2
require "the SSH refusal names vpn" contains "'vpn' ssh alias" "$output"

exit "$FAIL"
