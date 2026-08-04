extends GdUnitTestSuite
## The delegation witness. Pulses is a thin shim over the Rust WaveCore;
## this suite drives the shim and a bare core through the SAME scripted
## sequences side by side and holds them BIT-identical — elementwise
## pos/dat/dir over all 64 slots, live counts, echo appointment books.
## Any drift the shim adds or loses — a swapped argument, a missed tick, a
## mangled wrapper — splits the two and fails here. Both paths run the one
## Rust heart, so exact equality is the bar everywhere: same code, same
## inputs, same physics queries, no tolerance earned or granted.
##
## (Before the handover this suite compared the retired GDScript pool
## against the core, with one measured tolerance on the physics path for
## the fans' f32-vs-f64 dust; the scene margins below — birth face at
## x = 0.45, half a cluster cell — date from that proof and keep every
## decision boundary comfortably away from float edges.)

const NOW := 10.0
const MAX_R := 6.0
const SPEED := 5.5
const GAIN := 1.0
const MAX_ECHOES := 6
## Born on the birth wall's front face, mid-cluster-cell (0.45 = 0.9 / 2).
const SOUND_AT := Vector3(0.45, 1.5, 0)
const NORMAL := Vector3(1, 0, 0)

var _space: PhysicsDirectSpaceState3D


func before_test() -> void:
	# the wall the parity sound is born on: front face at x = 0.45
	_add_box(Vector3(0.35, 1.5, 0), Vector3(0.2, 3, 6))
	# the answering wall: face at x = 1.95, well inside the fan's 4.8 m reach
	_add_box(Vector3(2.05, 1.5, 0), Vector3(0.2, 3, 6))
	# in the acoustic shadow behind the birth wall: must never answer
	_add_box(Vector3(-1.55, 1, 0), Vector3(1, 2, 2))
	# in range but occluded by the answering wall: must never answer either
	_add_box(Vector3(3.45, 1.5, 0), Vector3(1, 2, 1))
	await get_tree().physics_frame
	await get_tree().physics_frame
	_space = get_viewport().world_3d.direct_space_state


## One box: a static collider the ray fans can strike, freed after the test.
func _add_box(center: Vector3, size: Vector3) -> void:
	var body: StaticBody3D = auto_free(StaticBody3D.new())
	body.position = center
	var col := CollisionShape3D.new()
	var shape := BoxShape3D.new()
	shape.size = size
	col.shape = shape
	body.add_child(col)
	add_child(body)


## The same sound into both hearts.
func _emit_both(
	p: Pulses,
	core: WaveCore,
	type: int,
	at: Vector3,
	max_r: float,
	speed: float,
	gain: float,
	now: float,
	beam_dir := Vector3.ZERO,
	cos_half := -2.0
) -> void:
	p.emit(type, at, max_r, speed, gain, now, beam_dir, cos_half)
	core.emit(type, at, max_r, speed, gain, now, beam_dir, cos_half)


## Every slot of every shader lane, bit for bit.
func _assert_pools_identical(p: Pulses, core: WaveCore) -> void:
	var cpos := core.positions()
	var cdat := core.pulse_data()
	var cdir := core.pulse_dirs()
	for i: int in Pulses.MAXP:
		assert_vector(cpos[i]).override_failure_message("pos[%d] drifted" % i).is_equal(p.pos[i])
		assert_vector(cdat[i]).override_failure_message("dat[%d] drifted" % i).is_equal(p.dat[i])
		assert_vector(cdir[i]).override_failure_message("dir[%d] drifted" % i).is_equal(p.dir[i])


## Every voice of the game — tap, footstep, echo, beamed hum — plus both
## gain clamps, packed identically lane for lane, and expiring in step.
func test_pure_voices_pack_identically() -> void:
	var p := Pulses.new()
	var core := WaveCore.new()
	_emit_both(p, core, 0, Vector3(1, 2, 3), 6.0, 5.5, 1.0, NOW)
	_emit_both(p, core, 2, Vector3(4, 5, 6), 1.6, 4.0, 0.8, NOW + 0.1)
	_emit_both(p, core, 1, Vector3(-2, 0.5, 7), 2.2, 5.5, 0.31, NOW + 0.2)
	_emit_both(p, core, 2, Vector3.ZERO, 6.0, 5.5, 1.5, NOW + 0.3)  # clamped from above
	_emit_both(p, core, 2, Vector3.ONE, 6.0, 5.5, -1.0, NOW + 0.4)  # clamped from below
	var beam := Vector3(0, 0, -1)
	_emit_both(p, core, 3, Vector3(8.6, 1.15, 4.4), 9.0, 4.5, 0.75, NOW + 0.5, beam, 0.85)
	_assert_pools_identical(p, core)
	for t: float in [NOW + 0.6, NOW + 3.0, NOW + 13.0, NOW + 14.0, NOW + 17.0]:
		assert_int(core.live_count(t)).is_equal(p.live_count(t))


## Past 64 slots the eviction order is the pool's most delicate law: the
## old hum falls first, then the old footstep, then the oldest tap — in the
## SAME slots on both sides, with every lane still bit-identical.
func test_eviction_pressure_matches() -> void:
	var p := Pulses.new()
	var core := WaveCore.new()
	for i: int in Pulses.MAXP:
		var type := 0
		if i == 7:
			type = 3
		elif i == 10:
			type = 2
		_emit_both(p, core, type, Vector3(i, 0, 0), 6.0, 5.5, 1.0, 100.0 + i * 0.001)
	_assert_pools_identical(p, core)
	_emit_both(p, core, 0, Vector3(101, 0, 0), 6.0, 5.5, 1.0, 100.1)
	_emit_both(p, core, 0, Vector3(102, 0, 0), 6.0, 5.5, 1.0, 100.2)
	_emit_both(p, core, 0, Vector3(103, 0, 0), 6.0, 5.5, 1.0, 100.3)
	_assert_pools_identical(p, core)
	assert_vector(p.pos[7]).is_equal(Vector3(101, 0, 0))
	assert_vector(p.pos[10]).is_equal(Vector3(102, 0, 0))
	assert_vector(p.pos[0]).is_equal(Vector3(103, 0, 0))


## Real physics: shim and core sample the same scene and must book the
## same appointments — same count, same order, same times, positions and
## gains, exactly — and the echo cap must bite identically. This also pins
## the shim's Echo wrapper against the core's raw dictionaries.
func test_reflection_schedules_identical_echoes() -> void:
	var p := Pulses.new()
	var core := WaveCore.new()
	p.emit_reflecting(0, SOUND_AT, MAX_R, SPEED, GAIN, NOW, _space, MAX_ECHOES, NORMAL)
	core.emit_reflecting(0, SOUND_AT, MAX_R, SPEED, GAIN, NOW, _space, MAX_ECHOES, NORMAL)
	assert_int(core.live_count(NOW + 0.1)).is_equal(p.live_count(NOW + 0.1))
	_assert_pools_identical(p, core)
	assert_int(core.pending_echo_count()).is_equal(p.pending_echo_count())
	assert_int(p.pending_echo_count()).is_between(2, MAX_ECHOES)
	var gd_echoes := p.pending_echoes()
	var core_echoes := core.pending_echoes()
	for i: int in gd_echoes.size():
		var mirror: Dictionary = core_echoes[i]
		var at_t: float = mirror.at_t
		var gain: float = mirror.gain
		var pos: Vector3 = mirror.pos
		assert_float(at_t).is_equal(gd_echoes[i].at_t)
		assert_float(gain).is_equal(gd_echoes[i].gain)
		assert_vector(pos).is_equal(gd_echoes[i].pos)
	var p_capped := Pulses.new()
	var core_capped := WaveCore.new()
	p_capped.emit_reflecting(0, SOUND_AT, MAX_R, SPEED, GAIN, NOW, _space, 1, NORMAL)
	core_capped.emit_reflecting(0, SOUND_AT, MAX_R, SPEED, GAIN, NOW, _space, 1, NORMAL)
	assert_int(core_capped.pending_echo_count()).is_equal(p_capped.pending_echo_count())
	assert_int(p_capped.pending_echo_count()).is_equal(1)


## Time marches through every appointment: after each firing — too-early
## drains included — both pools hold the same slots, bit for bit. The
## 1e-3 s nudge past each at_t stays far under the >= 6e-3 s gap between
## this scene's appointments, so each checkpoint fires exactly one echo.
func test_echo_firings_drive_identical_pools() -> void:
	var p := Pulses.new()
	var core := WaveCore.new()
	p.emit_reflecting(0, SOUND_AT, MAX_R, SPEED, GAIN, NOW, _space, MAX_ECHOES, NORMAL)
	core.emit_reflecting(0, SOUND_AT, MAX_R, SPEED, GAIN, NOW, _space, MAX_ECHOES, NORMAL)
	var moments: Array[float] = []
	for e: Pulses.Echo in p.pending_echoes():
		moments.append(e.at_t)
	moments.sort()
	assert_int(moments.size()).is_greater(1)
	var early := moments[0] - 0.001
	p.apply(early, [])
	core.tick(early)
	assert_int(core.pending_echo_count()).is_equal(p.pending_echo_count())
	assert_int(p.pending_echo_count()).is_equal(moments.size())  # nothing fired early
	for k: int in moments.size():
		var t := moments[k] + 0.001
		p.apply(t, [])
		core.tick(t)
		assert_int(core.pending_echo_count()).is_equal(p.pending_echo_count())
		assert_int(core.live_count(t)).is_equal(p.live_count(t))
		_assert_pools_identical(p, core)
	assert_int(p.pending_echo_count()).is_equal(0)  # every appointment kept
