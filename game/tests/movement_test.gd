extends GdUnitTestSuite
## The hero's walk against real physics: a careful pace on open ground,
## a flat map with no vertical drift, and walls that truly stop the body.

const TICKS := 30

var _player: UnseeingPlayer


func before_test() -> void:
	_player = auto_free(UnseeingPlayer.new())
	_player.pulses = Pulses.new()
	_player.position = Vector3(0, 0.9, 0)
	_player.rotation.y = 0.0  # face -Z
	add_child(_player)


func after_test() -> void:
	Input.action_release("move_forward")


func _add_box(center: Vector3, size: Vector3) -> void:
	var body: StaticBody3D = auto_free(StaticBody3D.new())
	body.position = center
	var col := CollisionShape3D.new()
	var shape := BoxShape3D.new()
	shape.size = size
	col.shape = shape
	body.add_child(col)
	add_child(body)


## The player owns its camera: the viewmodel reports a bob offset, and the
## player alone moves the eye around the fixed base height.
func test_head_bob_moves_camera_around_base() -> void:
	_player.set_head_bob(0.02)
	var base := UnseeingPlayer.cam_base_y()
	assert_float(_player.camera.position.y).is_equal_approx(base + 0.02, 0.0001)
	_player.set_head_bob(0.0)
	assert_float(_player.camera.position.y).is_equal_approx(base, 0.0001)


## No silent nulls: a player without its injected pulse pool reports the miss
## and disables its own physics — it never runs half-wired.
func test_uninjected_player_reports_and_disables() -> void:
	var bare: UnseeingPlayer = auto_free(UnseeingPlayer.new())
	var enter := func() -> void: add_child(bare)
	await assert_error(enter).is_push_error(
		"UnseeingPlayer: pulses not injected — physics disabled"
	)
	assert_bool(bare.is_physics_processing()).is_false()


## The player registers its own senses: a bare instance defined the actions
## in before_test, and re-registering leaves exactly one key event per action
## — main's boot call plus any number of players never stack duplicates.
func test_move_actions_register_once() -> void:
	UnseeingPlayer.ensure_actions()
	UnseeingPlayer.ensure_actions()
	for action: String in UnseeingPlayer.move_keys():
		assert_bool(InputMap.has_action(action)).is_true()
		assert_int(InputMap.action_get_events(action).size()).is_equal(1)


## Open ground: TICKS physics frames of forward input advance the hero along
## its facing at ~SPEED, and the flat-map law holds — velocity.y is zero on
## every single tick.
func test_open_floor_walk_at_speed() -> void:
	await get_tree().physics_frame
	var start := _player.global_position
	Input.action_press("move_forward")
	for i: int in TICKS:
		await get_tree().physics_frame
		assert_float(_player.velocity.y).is_equal(0.0)
	Input.action_release("move_forward")
	var walked := (_player.global_position - start).length()
	var expected := UnseeingPlayer.speed() * TICKS / float(Engine.physics_ticks_per_second)
	assert_float(walked).is_equal_approx(expected, expected * 0.08)
	# the walk went where the body faces: straight down -Z, no sideways drift
	assert_float(absf(_player.global_position.x - start.x)).is_less(0.01)
	assert_bool(_player.global_position.z < start.z).is_true()


## A wall dead ahead: the hero walks into it and stays put — the capsule
## never crosses the wall's near face, however long the input is held.
func test_wall_stops_the_hero() -> void:
	_add_box(Vector3(0, 1.5, -2.0), Vector3(6, 3, 0.3))  # near face at z = -1.85
	await get_tree().physics_frame
	Input.action_press("move_forward")
	const CAPSULE_RADIUS := 0.35
	for i: int in TICKS * 2:
		await get_tree().physics_frame
		assert_bool(_player.global_position.z >= -1.85 - CAPSULE_RADIUS).is_true()
	Input.action_release("move_forward")
	# pressed against the wall, not bounced away from it: within a step of it
	assert_float(_player.global_position.z).is_between(-1.85 + CAPSULE_RADIUS - 0.15, 0.0)


## The eye stays level-limited — as far as this run can prove it. Reality
## first: a headless display server has no mouse, so requesting
## MOUSE_MODE_CAPTURED is silently refused and the captured-look branch is
## unreachable; what CI pins is the capture gate itself — an uncaptured
## motion event, however violent, leaves yaw and pitch exactly untouched.
## A windowed run (editor) does capture, and then the same test pins the
## real law: pitch clamps at ±PITCH_LIMIT and yaw follows relative.x.
func test_mouse_look_capture_gate_and_pitch_clamp() -> void:
	Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
	if Input.mouse_mode != Input.MOUSE_MODE_CAPTURED:
		var yaw_before := _player.rotation.y
		var pitch_before := _player.camera.rotation.x
		get_viewport().push_input(_motion(Vector2(4000.0, 100000.0)))
		assert_float(_player.rotation.y).is_equal(yaw_before)
		assert_float(_player.camera.rotation.x).is_equal(pitch_before)
		return
	var limit := UnseeingPlayer.pitch_limit()
	get_viewport().push_input(_motion(Vector2(0.0, 1.0e6)))  # yank far down
	assert_float(_player.camera.rotation.x).is_equal_approx(-limit, 0.0001)
	get_viewport().push_input(_motion(Vector2(0.0, -1.0e6)))  # yank far up
	assert_float(_player.camera.rotation.x).is_equal_approx(limit, 0.0001)
	var yaw_start := _player.rotation.y
	get_viewport().push_input(_motion(Vector2(100.0, 0.0)))
	var turned := yaw_start - 100.0 * UnseeingPlayer.mouse_sens()
	assert_float(_player.rotation.y).is_equal_approx(turned, 0.0001)
	Input.mouse_mode = Input.MOUSE_MODE_VISIBLE


func _motion(relative: Vector2) -> InputEventMouseMotion:
	var motion := InputEventMouseMotion.new()
	motion.relative = relative
	return motion
