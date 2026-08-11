#!/usr/bin/env bash
# Advance-and-compare: a captured world, restored into a fresh boot, must
# live the next N frames IDENTICALLY to the original that kept running.
# Catches omission — the one failure class round-trip hashing cannot see.
# A missing hash is a failure, never a pass.
set -euo pipefail
DIR="$(cd "$(dirname "$0")/.." && pwd)"
GODOT="${GODOT:-godot}"
# a full path template, not `mktemp -t`: GNU mktemp demands the X's at the
# end of the template, and the droplet running this pipeline is Linux
BLOB="$(mktemp "${TMPDIR:-/tmp}/unseeing-blob.XXXXXX")"
# KEEP_BLOB=1 skips the cleanup trap and prints the path instead, so a
# diverging CI run can be pulled back and post-mortemed instead of re-run
# blind. Default behavior (delete on exit) is unchanged.
if [ "${KEEP_BLOB:-0}" = 1 ]; then
  trap 'echo "restore: KEEP_BLOB=1 — blob kept at $BLOB"' EXIT
else
  trap 'rm -f "$BLOB"' EXIT
fi

leg() {
  UNSEEING_SEED=1 UNSEEING_RESTORE_MODE="$1" UNSEEING_RESTORE_BLOB="$BLOB" \
    "$GODOT" --headless --fixed-fps 60 --path "$DIR/game" \
    -s res://tests/probe/restore_probe.gd 2>&1 \
    | grep '^RESTORE_HASH=' | head -1
}

A="$(leg capture || true)"
B="$(leg restore || true)"
[ -n "$A" ] || { echo "restore: FAILED — no hash from the capture leg"; exit 1; }
[ -n "$B" ] || { echo "restore: FAILED — no hash from the restore leg"; exit 1; }
if [ "$A" != "$B" ]; then
  echo "restore: FAILED — the restored run diverged from the original:"
  echo "  original: $A"
  echo "  restored: $B"
  exit 1
fi
echo "restore: OK $A"
