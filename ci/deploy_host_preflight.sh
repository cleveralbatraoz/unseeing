#!/bin/sh
# Cheap, complete host-capability gate for deploy.sh. This owns only external
# tool and endpoint checks; deploy.sh owns clean-main provenance.
set -eu

ROOT="${1:-}"
[ -n "$ROOT" ] && [ -d "$ROOT" ] || {
  echo "deploy: FAILED deploy preflight needs the repository root"
  exit 2
}

missing=''
if ! command -v cargo >/dev/null 2>&1 || ! cargo --version >/dev/null 2>&1; then
  missing="$missing\n  cargo (install rustup from https://rustup.rs)"
fi
# cargo-zigbuild's version flag belongs to its top-level executable. Invoking
# `cargo zigbuild --version` passes --version to the build subcommand instead
# and every supported release refuses it.
if ! command -v cargo-zigbuild >/dev/null 2>&1 \
  || ! cargo-zigbuild --version >/dev/null 2>&1; then
  missing="$missing\n  cargo-zigbuild (cargo install --locked cargo-zigbuild)"
fi
if ! command -v zig >/dev/null 2>&1 || ! zig version >/dev/null 2>&1; then
  missing="$missing\n  Zig (install a Zig toolchain and put 'zig' on PATH)"
fi
[ -x "$ROOT/rust/build-wasm.sh" ] \
  || missing="$missing\n  rust/build-wasm.sh (missing or not executable)"
command -v ssh >/dev/null 2>&1 || missing="$missing\n  ssh"
command -v scp >/dev/null 2>&1 || missing="$missing\n  scp"
command -v curl >/dev/null 2>&1 \
  || missing="$missing\n  curl (the deploy verifies what the site serves)"
if ! command -v git >/dev/null 2>&1 \
  || ! git -C "$ROOT" remote get-url production >/dev/null 2>&1; then
  missing="$missing\n  a 'production' git remote (git remote add production <droplet>)"
fi
# BatchMode makes an unreachable or unknown host a bounded refusal instead of
# a password prompt after builds and uploads have begun.
if ! command -v ssh >/dev/null 2>&1 \
  || ! ssh -o BatchMode=yes -o ConnectTimeout=10 vpn true >/dev/null 2>&1; then
  missing="$missing\n  a working 'vpn' ssh alias (it receives the prebuilt cores)"
fi

if [ -n "$missing" ]; then
  echo "deploy: FAILED this machine cannot complete a deploy:"
  printf '%b\n' "$missing"
  exit 2
fi
echo "deploy: preflight OK (cargo-zigbuild, Zig, wasm recipe, ssh to vpn, production remote, curl)"
