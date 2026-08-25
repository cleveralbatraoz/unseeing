extends GdUnitTestSuite
## Footsteps keep honest time. The viewmodel's footstep clock is driven
## with fixed 1/60 s steps over a scripted walk and held to its clock: each
## footfall queues exactly one type-2 wave on the player (where the physics
## tick would emit it), the cadence never drifts, the wave carries the
## footstep voice verbatim, and the shoes take turns. The player's physics
## is switched off so the suite alone owns time — nothing drains the queue
## or overwrites the scripted velocity between steps. The queue is read
## through the player's queued_waves() observable — the engine node's
## public face of the once-private script array.

const DT := 1.0 / 60.0

var _walk_vel := Vector3(0, 0, -UnseeingPlayer.speed())
var _pulses: Pulses
var _player: UnseeingPlayer
var _hero: HeroBody
var _now := 0.0


func before_test() -> void:
	_pulses = Pulses.new()
	_player = auto_free(UnseeingPlayer.new())
	_player.pulses = _pulses
	_player.position = Vector3(0, 0.9, 0)
	_player.rotation.y = 0.0  # face -Z: the shoes sit at world x = ±0.07
	add_child(_player)
	_player.set_physics_process(false)  # the test owns the clock and queue
	_hero = auto_free(HeroBody.new())
	_hero.player = _player
	_hero.camera = _player.camera
	_hero.pulses = _pulses
	_hero.cane_mat = ShaderMaterial.new()
	_hero.body_mat = ShaderMaterial.new()
	add_child(_hero)
	_now = 0.0


func after_test() -> void:
	for action: String in ["move_forward", "move_back", "move_left", "move_right"]:
		Input.action_release(action)


## One scripted frame: the player reports the given velocity and the
## viewmodel updates once at the fixed step.
func _step(vel: Vector3) -> void:
	_player.velocity = vel
	_now += DT
	_hero.update(_now, DT)


## 1.5 s of walking: the first-ever step falls on the first moving frame
## (the step clock starts spent), then every following step lands exactly
## when the 0.42 s cadence has fully elapsed — 26 fixed frames, never
## drifting.
func test_walk_cadence_never_drifts() -> void:
	var fires: Array[int] = []
	for frame: int in 90:
		_step(_walk_vel)
		if _player.queued_waves().size() > fires.size():
			fires.append(frame)
	assert_int(fires.size()).is_equal(4)
	assert_int(fires[0]).is_equal(0)  # a fresh walker steps at once
	for k: int in fires.size() - 1:
		assert_int(fires[k + 1] - fires[k]).is_equal(26)  # first frame past 0.42 s


## Every footfall speaks with the footstep voice, verbatim from the source:
## type 2, radius 1.6, speed 4.0, gain 0.8, two echoes, born flat on the
## ground (y 0.04) under the striking shoe — and the shoes alternate,
## starting with the right one (the step side begins at +1).
func test_step_waves_carry_the_footstep_voice_and_alternate() -> void:
	for frame: int in 90:
		_step(_walk_vel)
	var queue := _player.queued_waves()
	assert_int(queue.size()).is_equal(4)
	var side := 1.0  # right shoe first: hip + right-vector * 0.07
	for w: Dictionary in queue:
		var w_type: int = w.type
		var w_max_r: float = w.max_r
		var w_speed: float = w.speed
		var w_gain: float = w.gain
		var w_echoes: int = w.echoes
		var w_normal: Vector3 = w.normal
		var w_at: Vector3 = w.at
		var w_gate: String = w.gate
		assert_int(w_type).is_equal(2)
		assert_float(w_max_r).is_equal(1.6)
		assert_float(w_speed).is_equal(4.0)
		assert_float(w_gain).is_equal(0.8)
		assert_int(w_echoes).is_equal(2)
		assert_vector(w_normal).is_equal(Vector3.UP)
		assert_str(w_gate).is_equal("controlled_contact")
		# flat support: birth height is exactly the f32 contact height,
		# narrowed through a Vector3 lane rather than a decimal literal
		assert_float(w_at.y).is_equal(Vector3(0.04, 0.0, 0.0).x)
		# facing -Z the shoe rides the hip offset exactly: one hand-derived
		# f32 ULP at the 0.07 magnitude
		assert_float(absf(w_at.x - side * 0.07)).is_less_equal(7.450_580_596_923_828e-9)
		side = -side


## Standing still re-arms the 0.1 s stop grace every idle frame: the pause
## itself is silent, and the resumed walk fires exactly ONE step only after
## the grace has elapsed — never an instant double-fire on the first frame.
func test_stop_grace_prevents_a_double_fire_on_resume() -> void:
	for frame: int in 30:
		_step(_walk_vel)
	var walked := _player.queued_waves().size()
	assert_int(walked).is_equal(2)  # frames 0 and 26
	for frame: int in 30:
		_step(Vector3.ZERO)
	assert_int(_player.queued_waves().size()).is_equal(walked)  # silence while still
	var resume_fires: Array[int] = []
	for frame: int in 12:
		_step(_walk_vel)
		if _player.queued_waves().size() > walked + resume_fires.size():
			resume_fires.append(frame)
	assert_int(resume_fires.size()).is_equal(1)
	assert_int(resume_fires[0]).is_between(5, 7)  # ~0.1 s of walking first


## One box: a static collider under or below the walker, freed after the
## test — the same helper shape the movement suite builds with.
func _add_box(center: Vector3, size: Vector3) -> void:
	var body: StaticBody3D = auto_free(StaticBody3D.new())
	body.position = center
	var col := CollisionShape3D.new()
	var shape := BoxShape3D.new()
	shape.size = size
	col.shape = shape
	body.add_child(col)
	add_child(body)


## Pulses in the pool carrying the footstep voice's 1.6 m radius — the
## discriminator between a footstep and any landing voice. The lane is
## f32, so the comparison tolerates only its narrowing, never a voice.
func _footstep_pulse_count() -> int:
	var count := 0
	for i: int in _pulses.live_count(_now + 0.05):
		if absf(_pulses.dat[i].y - 1.6) < 1e-6:
			count += 1
	return count


## One physics tick, then one scripted hero frame off the advancing clock.
func _physics_then_hero_frame() -> void:
	await get_tree().physics_frame
	_now += DT
	_player.tick(_now)
	_hero.update(_now, DT)


## A controlled-contact footstep queued on the frame BEFORE the walker
## leaves support is consumed silently: the physics tick that departs
## Controlled sees the gate closed and emits nothing.
func test_footstep_queued_before_edge_is_consumed_without_emission() -> void:
	_step(_walk_vel)  # fresh walker: the pre-edge footstep queues at once
	var queue := _player.queued_waves()
	assert_int(queue.size()).is_equal(1)
	var gate: String = queue[0].gate
	assert_str(gate).is_equal("controlled_contact")
	# no floor exists: the next physics tick departs Controlled -> Airborne
	_player.set_physics_process(true)
	await get_tree().physics_frame
	await get_tree().physics_frame
	assert_array(_player.queued_waves()).is_empty()
	assert_int(_pulses.live_count(0.1)).is_equal(0)
	assert_int(_pulses.pending_echo_count()).is_equal(0)


## The same departing tick still honors an always-open wave: general and
## demo requests survive the edge untouched.
func test_always_wave_queued_before_edge_still_emits() -> void:
	_player.queue_wave(2, Vector3(1.0, 0.5, -1.0), 6.0, 5.5, 0.75, 2, Vector3.UP)
	var queue := _player.queued_waves()
	assert_int(queue.size()).is_equal(1)
	var gate: String = queue[0].gate
	assert_str(gate).is_equal("always")
	_player.set_physics_process(true)
	await get_tree().physics_frame
	await get_tree().physics_frame
	assert_array(_player.queued_waves()).is_empty()
	assert_int(_pulses.live_count(0.1)).is_equal(1)


## A landing too soft to speak (a 0.105 m drop lands at ~1.47 m/s, under
## the 1.5 m/s silent threshold) still arms the suppression latch: the
## next cadence-ready footstep is swallowed whole.
func test_silent_or_zero_voice_landing_still_suppresses_a_cadence_ready_step() -> void:
	_add_box(Vector3(0.0, -0.155, 0.0), Vector3(20.0, 0.1, 20.0))  # top at -0.105
	_player.set_physics_process(true)
	var went_airborne := false
	var landed := false
	for _tick: int in 60:
		await get_tree().physics_frame
		if _player.collision_layer == 4:
			went_airborne = true
		if went_airborne and _player.collision_layer == 2 and _player.is_on_floor():
			landed = true
			break
	assert_bool(went_airborne).is_true()
	assert_bool(landed).is_true()
	assert_int(_pulses.live_count(0.1)).is_equal(0)  # silent landing
	Input.action_press("move_forward")
	for _frame: int in 10:
		await _physics_then_hero_frame()
	# the fresh walker's instant first step was cadence-ready — consumed
	assert_int(_footstep_pulse_count()).is_equal(0)
	assert_array(_player.queued_waves()).is_empty()


## The landing tick speaks exactly once, with the landing voice — never a
## regular step: one pulse lives and its radius is the landing's, not the
## footstep's 1.6 m.
func test_landing_tick_has_one_landing_voice_and_no_regular_step() -> void:
	_add_box(Vector3(0.0, -1.55, 0.0), Vector3(20.0, 0.1, 20.0))  # top at -1.5
	_player.set_physics_process(true)
	var landed := false
	for _tick: int in 180:
		await _physics_then_hero_frame()
		if _player.collision_layer == 2 and _player.is_on_floor():
			landed = true
			break
	assert_bool(landed).is_true()
	assert_int(_pulses.live_count(_now + 0.05)).is_equal(1)
	# a 1.5 m drop caps severity: the landing voice's full 5.0 m radius
	assert_float(_pulses.dat[0].y).is_equal(5.0)
	assert_int(_footstep_pulse_count()).is_equal(0)
	Input.action_press("move_forward")
	for _frame: int in 10:
		await _physics_then_hero_frame()
	# the landing armed suppression: the instant first step is swallowed
	assert_int(_footstep_pulse_count()).is_equal(0)


## The latch survives every physics tick between the landing and the next
## hero frame: only a real cadence evaluation may acknowledge it.
func test_suppression_survives_multiple_physics_ticks_before_hero_update() -> void:
	_add_box(Vector3(0.0, -0.155, 0.0), Vector3(20.0, 0.1, 20.0))
	_player.set_physics_process(true)
	var landed := false
	for _tick: int in 60:
		await get_tree().physics_frame
		if _player.collision_layer == 2 and _player.is_on_floor():
			landed = true
			break
	assert_bool(landed).is_true()
	for _tick: int in 20:
		await get_tree().physics_frame  # no hero frames: nothing may acknowledge
	Input.action_press("move_forward")
	for _frame: int in 10:
		await _physics_then_hero_frame()
	assert_int(_footstep_pulse_count()).is_equal(0)  # still consumed here


## One acknowledgement spends the latch: the NEXT cadence-ready footstep
## emits normally — a walk after a landing goes quiet for exactly one step.
func test_landing_acknowledgement_allows_next_controlled_footstep() -> void:
	_add_box(Vector3(0.0, -0.155, 0.0), Vector3(20.0, 0.1, 20.0))
	_player.set_physics_process(true)
	var landed := false
	for _tick: int in 60:
		await get_tree().physics_frame
		if _player.collision_layer == 2 and _player.is_on_floor():
			landed = true
			break
	assert_bool(landed).is_true()
	Input.action_press("move_forward")
	for _frame: int in 45:
		await _physics_then_hero_frame()
	# first fire (instant) consumed; second fire (+26 moving frames) emits
	assert_int(_footstep_pulse_count()).is_equal(1)
