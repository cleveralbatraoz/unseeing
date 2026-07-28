extends RefCounted
## The wave system: a fixed pool of 64 pulse slots shared with BOTH shaders
## as uniform arrays. A pulse is an expanding spherical wavefront; shaders
## derive everything (ring position, outline reveal, fade) from its birth
## time — the CPU never animates waves, it only records that a sound happened.
##
## Slot data layout (mirrors the web reference exactly):
##   pos[i]         — world origin of the sound
##   dat[i]         — (birth time, max radius, speed m/s, type*10 + gain*9)
##   dir[i]         — beam direction xyz + cos(half-angle); w = -2 means
##                    omnidirectional (cane taps, footsteps)
## Types: 0 = cane tap, 1 = beam (unused, kept for shader parity),
##        2 = footstep, 3 = phantom (audio-only, never emitted as light).

const MAXP := 64

var pos := PackedVector3Array()
var dat := PackedVector4Array()
var dir := PackedVector4Array()
var _t0 := PackedFloat64Array()
var _end := PackedFloat64Array()
var _type := PackedInt32Array()

func _init() -> void:
	pos.resize(MAXP)
	dat.resize(MAXP)
	dir.resize(MAXP)
	_t0.resize(MAXP)
	_end.resize(MAXP)
	_type.resize(MAXP)
	for i: int in MAXP:
		dat[i] = Vector4(-1, 0, 0, 0)
		_end[i] = -1.0

## Record a sound. Slot eviction prefers expired slots, then the oldest
## footstep (least precious memory), then the oldest of anything.
func emit(type: int, at: Vector3, max_r: float, speed: float, gain: float,
		now: float, beam_dir := Vector3.ZERO, cos_half := -2.0) -> void:
	var slot := -1
	var old_step := -1
	var oldest := -1
	var t_old_step := INF
	var t_old := INF
	for i: int in MAXP:
		if _end[i] < now:
			slot = i
			break
		if _type[i] == 2 and _t0[i] < t_old_step:
			t_old_step = _t0[i]
			old_step = i
		if _t0[i] < t_old:
			t_old = _t0[i]
			oldest = i
	if slot < 0:
		slot = old_step if old_step >= 0 else oldest
	pos[slot] = at
	dat[slot] = Vector4(now, max_r, speed, type * 10.0 + minf(gain, 1.0) * 9.0)
	var omni := beam_dir == Vector3.ZERO
	dir[slot] = Vector4(beam_dir.x, beam_dir.y, beam_dir.z, -2.0 if omni else cos_half)
	_t0[slot] = now
	_end[slot] = now + max_r / speed + 6.0   # ring time + outline fade tail
	_type[slot] = type

## Highest live slot + 1 — lets the shaders break out of dead loop iterations.
func live_count(now: float) -> int:
	var n := 0
	for i: int in MAXP:
		if _end[i] >= now:
			n = i + 1
	return n

## Push the pool into every material that renders waves.
func apply(now: float, mats: Array) -> void:
	var count := live_count(now)
	for m: ShaderMaterial in mats:
		m.set_shader_parameter("u_count", count)
		m.set_shader_parameter("u_ppos", pos)
		m.set_shader_parameter("u_pdat", dat)
		m.set_shader_parameter("u_pdir", dir)
