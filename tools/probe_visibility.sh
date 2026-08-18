#!/bin/sh
# Rendered visibility probe — the acoustic-image pixel pins. Boots the real
# game WINDOWED (real GPU frames; headless renders nothing), taps the
# divider the fan is behind, and asserts no reveal — not a shell wash, not
# a borrowed outline, not a tap's echo — leaks past the wall onto the
# always-on-top fan. Run on demand — deliberately NOT part of
# ci/pipeline.sh (headless CI cannot see shader-reveal leaks).
#
# Warm-boot law (memory, 2026-08-04): the first boot after a shader edit
# compiles subtly different GL programs than every boot after it — the
# probe therefore runs TWICE and both verdicts must agree; only a
# reproduced PASS counts. set -eu fails the script on the first FAIL.
#
# Every check here is a before/after DELTA, so a wave the run did not ask
# for mostly subtracts out — but only mostly, because an emitter that MOVES
# between the two readings does not cancel. So the boot still keeps the
# room quiet: it seeds with UNSEEING_SEED (determinism only) and must NEVER
# be switched to UNSEEING_DEMO, whose automatic tap (rust/src/demo_tap.rs)
# fires near the spawn-room sample points on a 0.6 s/4 s schedule; the probe
# itself silences the level's creatures for the same reason.
#
# DO NOT ADD --fixed-fps. It was tried, to pin the fan's 11.42 s sweep to
# the frame count, and it breaks the tap cases: at a fixed 60 fps a
# 12-frame baseline is 0.2 s against the fan's own 0.4 s cadence, so the
# baseline misses a throb of the fan's own body that the 26-frame window
# catches, and the difference is charged to the tap — measured as a 0.329
# reveal on the fan where the correct answer is 0.000. Left free-running,
# each readback frame costs enough wall time that both windows span
# several cadences and the fan's own rhythm cancels. The sweep phase does
# not need pinning any more: every case is a delta across the fan's own
# voice, and the positive control refuses a phase in which it is dark.
#
# TWO scenes run here, and the first one is a measurement rather than an
# assertion about the game. channel_probe measures how many levels the
# screen texture actually preserves per channel — the platform fact
# rust/src/render/channel.rs pins as CHANNEL_LEVELS, and which the whole
# B-channel reconstruction guard turns on. The project had two stories about
# it (the brief said 8-bit LDR; an earlier probe claimed RGB10_A2) and at 8
# bits the guard is already broken by a factor of four. It is 1024.
#
# Env knobs: GODOT (binary).
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

# One owner decides which engine is the pinned one, and refuses anything
# else — including an explicitly supplied mismatch. tools/lib/engine.sh.
# shellcheck source=tools/lib/engine.sh
. "$DIR/tools/lib/engine.sh"
GODOT="$(unseeing_engine_select "$DIR" "${GODOT:-}")" || {
  echo "probe: no Godot matching .godot-version; set GODOT=/path/to/godot"
  exit 2
}

KEEP_AWAKE=""
command -v caffeinate >/dev/null 2>&1 && KEEP_AWAKE="caffeinate -dis"

# The project boots FULL SCREEN at the monitor's own resolution, and this
# probe must inherit neither: it samples pixels at coordinates derived from
# the viewport size, so the frame has to be the same box on every machine —
# and a probe that seizes the whole screen is a probe nobody runs twice.
#
# The engine's --windowed / --resolution flags CANNOT do this, which is
# measured, not assumed: main/main.cpp parses them into window_mode and then
# overwrites it wholesale from display/window/size/mode a thousand lines
# later, so the project setting always wins for the initial window. The
# flags are consumed on the way through, too — OS.get_cmdline_args() never
# sees them — so no script can notice and compensate either.
#
# override.cfg is the engine's documented escape hatch: merged over
# project.godot at startup, before the window is created. Written here and
# removed on the way out, however this script ends; git ignores it, and
# test/repo_hygiene.sh pins that so a crashed run cannot leave a committable
# stray behind.
OVERRIDE="$DIR/game/override.cfg"
# A pre-existing override.cfg belongs to whoever wrote it — another probe run,
# or a designer debugging a window setting by hand. Clobbering it and then
# deleting it on the way out destroys their file and leaves no trace of having
# done so, so refuse instead. probe_display.sh already refuses on the same
# grounds; this is the writer end of that agreement.
if [ -e "$OVERRIDE" ]; then
  echo "probe: FAILED game/override.cfg already exists — this probe would overwrite and then delete it."
  echo "probe: remove it yourself if it is a leftover, or wait for the run that owns it to finish."
  exit 2
fi
# HUP as well as INT and TERM: closing the terminal on a windowed probe is the
# ordinary way these runs end, and it is exactly the case that used to leave a
# stray override.cfg behind — a file the repository forbids shipping.
trap 'rm -f "$OVERRIDE"' EXIT INT TERM HUP
cat > "$OVERRIDE" <<'CFG'
[display]

window/size/mode=0
window/size/viewport_width=1280
window/size/viewport_height=720
CFG

# shellcheck disable=SC2086
run_scene() {
  UNSEEING_SEED=1 $KEEP_AWAKE "$GODOT" --path "$DIR/game" "$@"
}

for scene in res://tests/probe/channel_probe.tscn \
  res://tests/probe/depth_texture_probe.tscn \
  res://tests/probe/occlusion_probe.tscn; do
  echo "probe: $scene — run 1 (cold cache legal; only agreement counts)"
  run_scene "$scene"
  echo "probe: $scene — run 2 (warm boot, the trusted one)"
  run_scene "$scene"
  echo "probe: $scene — PASS reproduced across two boots"
done
