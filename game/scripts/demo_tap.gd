class_name DemoTap
extends RefCounted
## Dev-only cadence for the input-less demo tap: first fire at ~0.6 s, then
## every 4 s measured from each fire, always at the same pinned wall point —
## so movie-maker runs and the deployed ?demo build always catch a wave on
## screen. Pure schedule: main owns the arming (env var / URL) and the wave
## queueing; this class only answers "is a tap due now?".

const FIRST_AT := 0.6
const REPEAT_EVERY := 4.0

var armed := false
var point: Vector3
var normal: Vector3

var _next := FIRST_AT


func _init(tap_point: Vector3, tap_normal: Vector3) -> void:
	point = tap_point
	normal = tap_normal


## True when an armed schedule has a tap due, advancing the schedule to
## now + REPEAT_EVERY — the next due moment rides on the actual fire time,
## so any frame cadence lands within one frame of the ideal beat.
func fire_due(now: float) -> bool:
	if not armed or now < _next:
		return false
	_next = now + REPEAT_EVERY
	return true
