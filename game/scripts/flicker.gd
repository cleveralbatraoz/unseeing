class_name Flicker
extends RefCounted
## Nervous light: the reveal intensity wavers around 1.0, with rare brief
## dropouts — part of the mood, not noise; envelope carried over from the
## validated design. A pure state machine over an injected
## RandomNumberGenerator: no global randomness anywhere, so a seeded stream
## replays bit-identically (movie-maker runs, frame-comparison CI).

const LEVEL_MIN := 0.72
const LEVEL_MAX := 1.2
const DROP_DEPTH := 0.55  # a dropout dims the clamped level to just over half
const DROP_LEN_MIN := 0.08
const DROP_LEN_JITTER := 0.1
const DROP_SPACING_MIN := 8.0
const DROP_SPACING_JITTER := 10.0

var _rng: RandomNumberGenerator
var _t := 0.0
var _level := 1.0
var _drop_until := -1.0
var _next_drop := 9.0


func _init(rng: RandomNumberGenerator) -> void:
	# total at the door: a null stream falls back to a self-seeded one
	_rng = rng if rng != null else RandomNumberGenerator.new()


## Advance the envelope by one frame and return this frame's intensity.
## The level relaxes toward 1.0 under jitter, clamped to [LEVEL_MIN,
## LEVEL_MAX]; during a dropout the STORED level is dimmed each frame (the
## validated design compounds it), so the floor is LEVEL_MIN * DROP_DEPTH.
func next(dt: float) -> float:
	_t += dt
	_level += (1.0 - _level) * 0.12 + (_rng.randf() - 0.5) * 0.09
	_level = clampf(_level, LEVEL_MIN, LEVEL_MAX)
	_next_drop -= dt
	if _next_drop <= 0.0:
		_drop_until = _t + DROP_LEN_MIN + _rng.randf() * DROP_LEN_JITTER
		_next_drop = DROP_SPACING_MIN + _rng.randf() * DROP_SPACING_JITTER
	if _t < _drop_until:
		_level *= DROP_DEPTH
	return _level
