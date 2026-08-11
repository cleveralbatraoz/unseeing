# The headless boot-check gate's predicate (#21) — sourced by
# ci/pipeline.sh at both the pass/fail test and the line that prints the
# offenders, and by test/ci_boot_error_gate.sh, so the three call sites
# can never drift apart.
#
# Godot's own parse/shader failures, plus every engine node class that
# refuses to run half-wired rather than limp: WaveLevel (level integrity —
# starved object ids, no SpawnPoint marker, more walls than the sight
# shaders have slots for), SoundFan/SoundRadio/WaveCat/UnseeingPlayer/
# hero_body (composition-root injection). This is the literal set of
# `godot_error!` prefixes under rust/src/ as of this writing — grep
# `godot_error!` there before adding to it, rather than guessing a class
# name from its Rust struct: hero_body carries its refusal in snake_case,
# not "HeroBody:", because that is what the Rust source actually prints.
#
# Deliberately NOT a catch-all `^ERROR:`: Godot's own engine prints ERROR:
# for conditions this gate has no business failing on — WaveCore's
# REFUSAL_MESSAGE (a single refused wave request) deliberately keeps the
# legacy "Pulses.emit:" text rather than a class prefix, and does not
# belong here. Widening this to everything would make the cheapest gate
# in the pipeline flaky, which is worse than the hole it used to have.
BOOT_ERROR_PATTERN="SCRIPT ERROR|SHADER ERROR|Parse Error|ERROR: Failed to|ERROR: WaveLevel|ERROR: SoundFan|ERROR: SoundRadio|ERROR: WaveCat|ERROR: UnseeingPlayer|ERROR: hero_body"
