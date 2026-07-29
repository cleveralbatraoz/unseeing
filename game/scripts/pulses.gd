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
## Types: 0 = cane tap, 1 = ECHO (secondary reflection), 2 = footstep,
##        3 = phantom (audio-only, never emitted as light).
##
## REFLECTIONS — the heart of echo-location. A primary sound samples the
## world with real physics rays from its origin; every surface point struck
## becomes a secondary emitter that fires exactly when the primary wavefront
## arrives there (t = distance / speed). Parts in the wave's shadow receive
## no rays and stay silent — only the swept, line-of-sight parts of an
## object ever answer. Echoes never spawn further echoes.

const MAXP := 64

var pos := PackedVector3Array()
var dat := PackedVector4Array()
var dir := PackedVector4Array()
var _t0 := PackedFloat64Array()
var _end := PackedFloat64Array()
var _type := PackedInt32Array()
var _echoes: Array = []   # scheduled reflections: {at_t, pos, gain}

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
	# ring time + outline-fade tail; echoes and footsteps expire sooner so the
	# live-slot count (which both shaders loop over per pixel) stays small
	var tail := 6.0
	if type == 1:
		tail = 3.5
	elif type == 2:
		tail = 2.5
	_end[slot] = now + max_r / speed + tail
	_type[slot] = type

## Emit a primary sound AND schedule its reflections off the environment.
## `space` is the physics space to sample; `max_echoes` caps slot pressure.
## `origin_normal` is the normal of the surface the sound was born on: rays
## sample only the hemisphere in FRONT of it, cast from just off the surface —
## otherwise rays start inside the struck collider and leak through into the
## acoustic shadow, answering from places the wave never reached.
func emit_reflecting(type: int, at: Vector3, max_r: float, speed: float,
		gain: float, now: float, space: PhysicsDirectSpaceState3D, max_echoes: int,
		origin_normal := Vector3.ZERO) -> void:
	emit(type, at, max_r, speed, gain, now)
	if space == null:
		return
	var origin := at + origin_normal * 0.08
	# Fibonacci-sphere ray fan: uniform directions, each ray asks the real
	# colliders what the wave will touch first in that direction
	const RAYS := 26
	var cells := {}
	for i: int in RAYS:
		var y := 1.0 - 2.0 * (float(i) + 0.5) / RAYS
		var r := sqrt(maxf(0.0, 1.0 - y * y))
		var phi := float(i) * 2.399963
		var d3 := Vector3(r * cos(phi), y, r * sin(phi))
		if origin_normal != Vector3.ZERO and d3.dot(origin_normal) < 0.05:
			continue   # into the surface: that direction is the wave's shadow
		var query := PhysicsRayQueryParameters3D.create(origin, origin + d3 * minf(max_r * 0.8, 6.0))
		var hit := space.intersect_ray(query)
		if hit.is_empty():
			continue
		var dist: float = (hit.position - origin).length()
		if dist < 0.3:
			continue   # the surface the sound itself was born on
		# cluster nearby hits so a flat wall answers as a few points, not 26
		var key := Vector3i((hit.position / 0.9).floor())
		if not cells.has(key) or cells[key].d > dist:
			cells[key] = { d = dist, p = hit.position + hit.normal * 0.02 }
	var found: Array = cells.values()
	found.sort_custom(func(a, b): return a.d < b.d)
	for j: int in mini(found.size(), max_echoes):
		var e: Dictionary = found[j]
		_echoes.append({
			at_t = now + e.d / speed,
			pos = e.p,
			gain = gain * 0.55 / (1.0 + e.d * 0.4),
		})

## Fire reflections whose moment has come (the wavefront reached them).
func _drain_echoes(now: float) -> void:
	for i: int in range(_echoes.size() - 1, -1, -1):
		if _echoes[i].at_t <= now:
			var e: Dictionary = _echoes[i]
			_echoes.remove_at(i)
			emit(1, e.pos, 2.2, 5.5, e.gain, now)

## Highest live slot + 1 — lets the shaders break out of dead loop iterations.
func live_count(now: float) -> int:
	var n := 0
	for i: int in MAXP:
		if _end[i] >= now:
			n = i + 1
	return n

## Push the pool into every material that renders waves.
func apply(now: float, mats: Array) -> void:
	_drain_echoes(now)
	var count := live_count(now)
	for m: ShaderMaterial in mats:
		m.set_shader_parameter("u_count", count)
		m.set_shader_parameter("u_ppos", pos)
		m.set_shader_parameter("u_pdat", dat)
		m.set_shader_parameter("u_pdir", dir)

