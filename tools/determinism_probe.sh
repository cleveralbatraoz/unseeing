#!/usr/bin/env bash
# Two seeded headless boots must produce byte-identical state hashes.
# Catches: unseeded randomness, wall-clock leaks into the sim, and
# run-to-run divergence in anything the snapshot can see — the substrate
# every reproduction artifact (capture blob, action tape) rides on.
# A MISSING hash is a failure, never a pass: a probe that crashed or
# refused must not read as "the runs agreed".
set -euo pipefail
DIR="$(cd "$(dirname "$0")/.." && pwd)"
GODOT="${GODOT:-godot}"

run_once() {
  UNSEEING_SEED=1 "$GODOT" --headless --fixed-fps 60 --path "$DIR/game" \
    -s res://tests/probe/determinism_probe.gd 2>&1 \
    | grep '^DETERMINISM_HASH=' | head -1
}

A="$(run_once || true)"
B="$(run_once || true)"
[ -n "$A" ] || { echo "determinism: FAILED — no hash from run A (probe crashed or refused)"; exit 1; }
[ -n "$B" ] || { echo "determinism: FAILED — no hash from run B (probe crashed or refused)"; exit 1; }
if [ "$A" != "$B" ]; then
  echo "determinism: FAILED — two seeded boots disagree:"
  echo "  run A: $A"
  echo "  run B: $B"
  exit 1
fi
echo "determinism: OK $A"
