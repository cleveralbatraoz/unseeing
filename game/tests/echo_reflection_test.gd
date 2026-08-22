extends GdUnitTestSuite
## The reflection pipeline against REAL physics. A sound born on a wall
## samples the world with a golden-angle ray fan; every struck surface point
## becomes a scheduled echo that fires when the wavefront arrives. These
## tests pin the fan's laws with live StaticBody3D geometry: echoes exist,
## the max_echoes cap holds, acoustic shadows stay silent, and each echo's
## timing and gain follow its distance.

const SOUND_AT := Vector3(0, 1.5, 0)  # born on the origin wall's front face
const NORMAL := Vector3(1, 0, 0)  # that wall faces +X; rays sample that side
const ORIGIN := SOUND_AT + NORMAL * 0.08  # where emit_reflecting starts rays
const MAX_R := 6.0
const SPEED := 5.5
const GAIN := 1.0
const NOW := 10.0
const RAY_LEN := 4.8  # min(MAX_R * 0.8, 6.0): the fan's reach

var _space: PhysicsDirectSpaceState3D


func before_test() -> void:
	# the wall the sound is born on: its front face is the plane x = 0
	_add_box(Vector3(-0.1, 1.5, 0), Vector3(0.2, 3, 6))
	# the answering wall in front: face at x = 1.5, well inside the fan's reach
	_add_box(Vector3(1.6, 1.5, 0), Vector3(0.2, 3, 6))
	# in the acoustic shadow BEHIND the origin wall: must never answer
	_add_box(Vector3(-2, 1, 0), Vector3(1, 2, 2))
	# in range but occluded by the answering wall: must never answer either
	_add_box(Vector3(3, 1.5, 0), Vector3(1, 2, 1))
	await get_tree().physics_frame
	await get_tree().physics_frame
	_space = get_viewport().world_3d.direct_space_state


## One box: a static collider the ray fan can strike, freed after the test.
func _add_box(center: Vector3, size: Vector3) -> void:
	var body: StaticBody3D = auto_free(StaticBody3D.new())
	body.position = center
	var col := CollisionShape3D.new()
	var shape := BoxShape3D.new()
	shape.size = size
	col.shape = shape
	body.add_child(col)
	add_child(body)


## The primary pulse lands in the pool, reflections ARE scheduled off the
## line-of-sight geometry, and max_echoes caps how many.
func test_echoes_scheduled_and_capped() -> void:
	var p := Pulses.new()
	p.emit_reflecting(0, SOUND_AT, MAX_R, SPEED, GAIN, NOW, _space, 6, NORMAL)
	assert_int(p.live_count(NOW + 0.1)).is_equal(1)
	assert_int(p.pending_echo_count()).is_between(2, 6)
	var capped := Pulses.new()
	capped.emit_reflecting(0, SOUND_AT, MAX_R, SPEED, GAIN, NOW, _space, 1, NORMAL)
	assert_int(capped.pending_echo_count()).is_equal(1)


## The acoustic shadow: no echo lands behind the plane the sound was born
## on, and the box hidden behind the answering wall stays silent too — only
## swept, line-of-sight surfaces answer. The answering wall itself does.
func test_no_echoes_in_acoustic_shadow() -> void:
	var p := Pulses.new()
	p.emit_reflecting(0, SOUND_AT, MAX_R, SPEED, GAIN, NOW, _space, 6, NORMAL)
	var echoes := p.pending_echoes()
	assert_int(echoes.size()).is_greater(0)
	var struck_answering_wall := false
	for e: Pulses.Echo in echoes:
		var at := e.pos
		assert_bool(at.x >= -0.05).is_true()  # never behind the birth plane
		assert_bool(at.x <= 1.8).is_true()  # never past the answering wall
		if at.x > 1.3:
			struck_answering_wall = true
	assert_bool(struck_answering_wall).is_true()


## An echo is an appointment: apply() must not fire it a moment early, and
## must fire it once its at_t has passed — the reflection enters the pool
## as type 1 with max radius 2.2 and speed 5.5, born at the drain time.
func test_echo_drain_keeps_its_appointment() -> void:
	var p := Pulses.new()
	p.emit_reflecting(0, SOUND_AT, MAX_R, SPEED, GAIN, NOW, _space, 1, NORMAL)
	assert_int(p.pending_echo_count()).is_equal(1)
	var e := p.pending_echoes()[0]
	var at_t := e.at_t
	p.apply(at_t - 0.01, [])
	assert_int(p.pending_echo_count()).is_equal(1)  # too early: still pending
	assert_int(p.live_count(at_t - 0.01)).is_equal(1)  # only the primary
	p.apply(at_t + 0.01, [])
	assert_int(p.pending_echo_count()).is_equal(0)
	assert_int(p.live_count(at_t + 0.01)).is_equal(2)
	assert_int(int(floorf(p.dat[1].w / 10.0))).is_equal(1)  # type: ECHO
	assert_float(p.dat[1].x).is_equal_approx(at_t + 0.01, 0.0001)
	assert_float(p.dat[1].y).is_equal_approx(2.2, 0.000001)
	assert_float(p.dat[1].z).is_equal(5.5)
	assert_float(fmod(p.dat[1].w, 10.0) / 9.0).is_equal_approx(e.gain, 0.001)
	assert_vector(p.pos[1]).is_equal(e.pos)


## Each echo keeps the wave equation: it fires exactly when the wavefront
## arrives (at_t = now + d / speed) and its gain follows the distance law
## gain * 0.55 / (1 + 0.4 * d). The scheduled position sits d away from the
## ray origin (plus the tiny off-surface offset).
func test_echo_timing_and_gain_follow_distance() -> void:
	var p := Pulses.new()
	p.emit_reflecting(0, SOUND_AT, MAX_R, SPEED, GAIN, NOW, _space, 6, NORMAL)
	var echoes := p.pending_echoes()
	assert_int(echoes.size()).is_greater(0)
	for e: Pulses.Echo in echoes:
		var at_t := e.at_t
		var d := (at_t - NOW) * SPEED
		assert_bool(d > 0.3).is_true()  # closer hits are the birth surface
		assert_bool(d <= RAY_LEN + 0.01).is_true()
		assert_float(e.gain).is_equal_approx(GAIN * 0.55 / (1.0 + 0.4 * d), 0.0001)
		var pos := e.pos
		assert_float((pos - ORIGIN).length()).is_equal_approx(d, 0.06)


func test_echo_gain_uses_the_exact_packed_shader_image() -> void:
	var p := Pulses.new()
	p.emit_reflecting(1_000_000, SOUND_AT, MAX_R, SPEED, 0.5, NOW, _space, 6, NORMAL)
	assert_float(p.dat[0].w).is_equal(10_000_004.0)
	var effective_gain := 0.4444444477558136  # f32 0x3ee38e39 widened to f64
	var echoes := p.pending_echoes()
	assert_int(echoes.size()).is_greater(0)
	for e: Pulses.Echo in echoes:
		var d := (e.at_t - NOW) * SPEED
		var expected := effective_gain * 0.55 / (1.0 + 0.4 * d)
		assert_float(e.gain).is_equal_approx(expected, 0.000000000001)
