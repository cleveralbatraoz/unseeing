# The headless boot-check gate's predicate (#21) — sourced by
# ci/pipeline.sh at both the pass/fail test and the line that prints the
# offenders, and by test/ci_boot_error_gate.sh, so the three call sites
# can never drift apart.
#
# Godot's own parse/shader failures, plus every engine node class that
# refuses to run half-wired rather than limp: WaveLevel (level integrity —
# starved superface classes, no typed WaveSpawn, more walls than the sight
# shaders have slots for), SoundFan/SoundRadio/WaveCat/UnseeingPlayer/
# hero_body (composition-root injection), UnseeingGame (the composition
# root itself: a shader or the level scene that fails to load). This is
# the set of message openings under rust/src/ as of this writing.
#
# Do NOT derive an addition by grepping `godot_error!`: the message text
# usually is not there. The two-layer rule builds it in the pure,
# cargo-testable module (level_plan.rs writes the "WaveLevel: …" strings)
# and the node relays it with godot_error!("{}", complaint). Grep for the
# LITERAL instead — test/ci_boot_error_gate.sh censuses every one of them
# and fails if this list has fallen behind. And read the opening off the
# Rust source rather than guessing a class name from its struct:
# hero_body carries its refusal in snake_case, not "HeroBody:", because
# that is what the source actually prints.
#
# Deliberately NOT a catch-all `^ERROR:`: Godot's own engine prints ERROR:
# for conditions this gate has no business failing on — WaveCore's
# REFUSAL_MESSAGE (a single refused wave request) deliberately keeps the
# legacy "Pulses.emit:" text rather than a class prefix, and does not
# belong here. Widening this to everything would make the cheapest gate
# in the pipeline flaky, which is worse than the hole it used to have.
#
# WaveWall and WaveRun are here for classes that only WARN today (wall.rs,
# run.rs, including the quarter-turn snap). That is on purpose and it is not speculative
# widening: the volume a message is said at is a run-time choice —
# level.rs:765-766 sends one level_plan Budget text to godot_error! or
# godot_warn! depending on severity — so "which classes can error" is not a
# fact the source can be asked. Godot prints warnings as "WARNING: ", which
# no entry here matches, so this costs the boot check nothing and covers
# WaveWall the day it raises its voice.
BOOT_ERROR_PATTERN="SCRIPT ERROR|SHADER ERROR|Parse Error|ERROR: Failed to|ERROR: WaveLevel|ERROR: SoundFan|ERROR: SoundRadio|ERROR: WaveCat|ERROR: UnseeingPlayer|ERROR: WaveWall|ERROR: WaveRun|ERROR: hero_body|ERROR: UnseeingGame"
