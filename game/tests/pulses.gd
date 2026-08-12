class_name Pulses
extends RefCounted
## Test-facing shim: the engine talks to WaveCore directly.
##
## The wave system's GDScript face — a thin shim over the Rust WaveCore.
## A pulse is an expanding spherical wavefront in a fixed pool of 64 slots
## shared with BOTH shaders as uniform arrays; shaders derive everything
## (ring position, outline reveal, fade) from its birth time — the CPU never
## animates waves, it only records that a sound happened. Every law of the
## pool — packing, lifetimes, eviction, the golden-angle reflection fan,
## clustering, echo appointments — lives in the Rust core (rust/src/), where
## it is pinned by cargo tests; this class only carries values across the
## boundary and preserves the public surface the game and suites grew on.
##
## Slot data layout (the shader-side contract lives in pulse_pool.gdshaderinc):
##   pos[i]         — world origin of the sound
##   dat[i]         — (birth time, max radius, speed m/s, type*10 + gain*9)
##   dir[i]         — beam direction xyz + cos(half-angle); w = -2 means
##                    omnidirectional (cane taps, footsteps)
## Types: 0 = cane tap, 1 = ECHO (secondary reflection), 2 = footstep,
##        3 = source hum (constant world sources like the fan — the one
##        sound the hero did not make; drawn through walls, muffled).
##
## REFLECTIONS — the heart of echo-location. A primary sound samples the
## world with real physics rays from its origin; every surface point struck
## becomes a secondary emitter that fires exactly when the primary wavefront
## arrives there (t = distance / speed). Parts in the wave's shadow receive
## no rays and stay silent — only the swept, line-of-sight parts of an
## object ever answer. Echoes never spawn further echoes.

## Pool capacity, mirroring the Rust core's source of truth
## (rust/src/pulse_pool.rs, MAXP): the size of the uniform arrays both
## shaders loop over per pixel. The shader include pins the same number —
## shader_contract_test holds this mirror against the include, and
## pulses_test's eviction suite catches a drift in the core itself.
const MAXP := 64

## The shader-bound lanes, read straight from the core — copies of the
## uniform arrays, exactly what apply() pushes to the materials.
var pos: PackedVector3Array:
	get:
		return _core.positions()
var dat: PackedVector4Array:
	get:
		return _core.pulse_data()
var dir: PackedVector4Array:
	get:
		return _core.pulse_dirs()

## The one heart: pool, fan, clustering and echo book all live inside.
var _core := WaveCore.new()


## A scheduled reflection: a surface point that answers at the exact moment
## the primary wavefront reaches it.
class Echo:
	var at_t: float
	var pos: Vector3
	var gain: float

	func _init(t: float, p: Vector3, g: float) -> void:
		at_t = t
		pos = p
		gain = g


## Record a sound. Total at the door: gain is clamped to [0, 1] (a raw value
## would bleed into the packed type digits) and non-positive speed or radius
## is refused loudly — a zero-speed wave would occupy its slot forever.
## Slot eviction prefers expired slots, then the oldest footstep or hum
## (least precious — both recur), then the oldest of anything.
func emit(
	type: int,
	at: Vector3,
	max_r: float,
	speed: float,
	gain: float,
	now: float,
	beam_dir := Vector3.ZERO,
	cos_half := -2.0
) -> void:
	_core.emit(type, at, max_r, speed, gain, now, beam_dir, cos_half)


## Emit a primary sound AND schedule its reflections off the environment.
## `space` is the physics space to sample; `max_echoes` caps slot pressure.
## `origin_normal` is the normal of the surface the sound was born on: rays
## sample only the hemisphere in FRONT of it, cast from just off the surface —
## otherwise rays start inside the struck collider and leak through into the
## acoustic shadow, answering from places the wave never reached.
## PHYSICS CONTEXT: with a space this must run inside the physics tick.
func emit_reflecting(
	type: int,
	at: Vector3,
	max_r: float,
	speed: float,
	gain: float,
	now: float,
	space: PhysicsDirectSpaceState3D,
	max_echoes: int,
	origin_normal := Vector3.ZERO
) -> void:
	_core.emit_reflecting(type, at, max_r, speed, gain, now, space, max_echoes, origin_normal)


## Reflections scheduled but not yet fired — observable for tests and debug.
func pending_echo_count() -> int:
	return _core.pending_echo_count()


## The scheduled reflections themselves, copied out — observable for tests
## and debug.
func pending_echoes() -> Array[Echo]:
	var out: Array[Echo] = []
	for entry: Dictionary in _core.pending_echoes():
		var at_t: float = entry.at_t
		var point: Vector3 = entry.pos
		var loudness: float = entry.gain
		out.append(Echo.new(at_t, point, loudness))
	return out


## The Rust core this shim carries values to — handed out so the debug
## observer can READ the pool it already owns (slot by slot, and the
## eviction it would next make) without a second reference to the same pool
## being injected anywhere. Nothing drives the wave system through here:
## every mutating door is a method on this class.
func core() -> WaveCore:
	return _core


## Highest live slot + 1 — lets the shaders break out of dead loop iterations.
func live_count(now: float) -> int:
	return _core.live_count(now)


## Push the pool into every material that renders waves, firing every echo
## whose appointment has come first.
func apply(now: float, mats: Array[ShaderMaterial]) -> void:
	_core.tick(now)
	var count := _core.live_count(now)
	var ppos := _core.positions()
	var pdat := _core.pulse_data()
	var pdir := _core.pulse_dirs()
	for m: ShaderMaterial in mats:
		m.set_shader_parameter("u_count", count)
		m.set_shader_parameter("u_ppos", ppos)
		m.set_shader_parameter("u_pdat", pdat)
		m.set_shader_parameter("u_pdir", pdir)
