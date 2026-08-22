# gdlint:ignore = max-public-methods
extends GdUnitTestSuite
## The physical hero boundary: support motion, layer separation, relocation,
## and designer-injected fall configuration.

const WORLD_FIXTURE := preload("res://tests/world_fixture.gd")
const ELEVATION_FIXTURE := preload("res://tests/character_elevation_fixture.gd")

const PLAYER_CONFIG_FIELDS: Array[String] = [
	"player_fall_acceleration",
	"player_terminal_fall_speed",
	"player_landing_silent_speed",
	"player_landing_full_speed",
	"player_landing_max_gain",
	"player_landing_max_range",
]
const PLAYER_CONFIG_DEFAULTS := [9.8, 20.0, 1.5, 4.0, 0.85, 5.0]
const INVALID_THRESHOLD_ERROR := (
	"UnseeingGame: invalid player motion configuration — landing full speed 7 m/s "
	+ "must be greater than silent speed 8 m/s"
)

var _server_rids: Array[RID] = []


func after_test() -> void:
	for action: String in ["move_forward", "move_back", "move_left", "move_right"]:
		Input.action_release(action)
	for rid: RID in _server_rids:
		if rid.is_valid():
			PhysicsServer3D.free_rid(rid)
	_server_rids.clear()


func test_actor_layers_are_named_and_phase_derived() -> void:
	assert_str(ProjectSettings.get_setting("layer_names/3d_physics/layer_2", "")).is_equal(
		"Controlled Actor"
	)
	assert_str(ProjectSettings.get_setting("layer_names/3d_physics/layer_3", "")).is_equal(
		"Airborne Actor"
	)
	var player: UnseeingPlayer = auto_free(UnseeingPlayer.new())
	player.pulses = Pulses.new()
	add_child(player)
	assert_int(player.collision_layer).is_equal(2)
	assert_int(player.collision_mask).is_equal(4_294_967_291)


func test_player_capsule_bottom_meets_the_authored_flat_datum() -> void:
	var player: UnseeingPlayer = auto_free(UnseeingPlayer.new())
	player.pulses = Pulses.new()
	player.position = Vector3(0.0, 0.9, 0.0)
	add_child(player)
	var collisions := player.find_children("*", "CollisionShape3D", false, false)
	assert_int(collisions.size()).is_equal(1)
	var collision := collisions[0] as CollisionShape3D
	assert_object(collision).is_not_null()
	var capsule := collision.shape as CapsuleShape3D
	assert_object(capsule).is_not_null()
	assert_float(capsule.radius).is_equal_approx(0.35, 1.0e-7)
	assert_float(capsule.height).is_equal_approx(1.7, 1.0e-7)
	assert_float(collision.position.y).is_equal_approx(-0.05, 1.0e-7)
	assert_float(player.position.y + collision.position.y - capsule.height * 0.5).is_equal_approx(
		0.0, 1.0e-7
	)
	var cameras := player.find_children("*", "Camera3D", false, false)
	assert_int(cameras.size()).is_equal(1)
	var camera := cameras[0] as Camera3D
	assert_object(camera).is_not_null()
	assert_float(player.position.y).is_equal_approx(0.9, 5.960_464_477_539_063e-8)
	assert_float(camera.position.y).is_equal_approx(0.7, 5.960_464_477_539_063e-8)
	assert_float(camera.global_position.y).is_equal_approx(1.6, 1.192_092_895_507_812_5e-7)


func test_player_solver_contract_is_explicit_on_every_property() -> void:
	var player: UnseeingPlayer = auto_free(UnseeingPlayer.new())
	player.pulses = Pulses.new()
	add_child(player)
	assert_int(player.motion_mode).is_equal(CharacterBody3D.MOTION_MODE_GROUNDED)
	assert_vector(player.up_direction).is_equal(Vector3.UP)
	assert_float(player.floor_snap_length).is_equal_approx(0.10, 1.0e-7)
	assert_float(player.floor_max_angle).is_equal_approx(PI / 4.0, 1.0e-7)
	assert_float(player.safe_margin).is_equal_approx(0.001, 1.0e-7)
	assert_int(player.max_slides).is_equal(6)
	assert_bool(player.floor_stop_on_slope).is_true()
	assert_bool(player.floor_constant_speed).is_false()


func test_player_solver_disables_ambient_platform_motion() -> void:
	var player: UnseeingPlayer = auto_free(UnseeingPlayer.new())
	player.pulses = Pulses.new()
	add_child(player)
	assert_int(player.platform_floor_layers).is_equal(0)
	assert_int(player.platform_wall_layers).is_equal(0)
	assert_int(player.platform_on_leave).is_equal(CharacterBody3D.PLATFORM_ON_LEAVE_DO_NOTHING)


func test_unsupported_player_falls_and_stops_at_terminal_speed() -> void:
	var player: UnseeingPlayer = auto_free(
		ELEVATION_FIXTURE.add_player(self, Vector3(0.0, 8.0, 0.0))
	)
	var pulses := player.pulses as Pulses
	assert_object(pulses).is_not_null()
	var reached_terminal: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return player.velocity.y == -20.0, 180
	)
	assert_bool(reached_terminal).is_true()
	assert_float(player.velocity.y).is_equal(-20.0)
	assert_float(player.global_position.y).is_less(8.0)
	assert_bool(player.global_transform.is_finite()).is_true()
	assert_bool(player.velocity.is_finite()).is_true()
	assert_bool(player.is_on_floor()).is_false()
	assert_int(player.collision_layer).is_equal(4)
	assert_int(player.collision_mask).is_equal(4_294_967_289)
	assert_array(player.queued_waves()).is_empty()
	assert_int(pulses.live_count(0.0)).is_equal(0)


func test_airborne_input_cannot_reverse_the_departure_trajectory() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(
			self, Vector3(-0.5, 1.95, 0.0), Vector3(2.0, 0.1, 2.0), "Departure"
		)
	)
	var player: UnseeingPlayer = auto_free(
		ELEVATION_FIXTURE.add_player(self, Vector3(-0.8, 2.9, 0.0))
	)
	assert_bool(await _poll_initial_control(player)).is_true()
	Input.action_press("move_right")
	var departed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return player.collision_layer == 4, 90
	)
	assert_bool(departed).is_true()
	var departure_x := player.global_position.x
	var held_x := player.velocity.x
	Input.action_release("move_right")
	Input.action_press("move_left")
	for _tick: int in 20:
		await get_tree().physics_frame
	assert_float(held_x).is_greater(0.0)
	assert_float(player.velocity.x).is_equal(held_x)
	assert_float(player.global_position.x).is_greater(departure_x)
	assert_int(player.collision_layer).is_equal(4)


func test_airborne_wall_contact_removes_only_the_blocked_planar_component_without_a_wave() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(
			self, Vector3(-1.0, 1.95, 0.0), Vector3(2.0, 0.1, 2.0), "Departure"
		)
	)
	auto_free(
		ELEVATION_FIXTURE.add_box(self, Vector3(1.2, 1.5, -1.5), Vector3(0.1, 5.0, 4.0), "AirWall")
	)
	var player: UnseeingPlayer = auto_free(
		ELEVATION_FIXTURE.add_player(self, Vector3(-1.0, 2.9, 0.0))
	)
	var pulses := player.pulses as Pulses
	assert_object(pulses).is_not_null()
	assert_bool(await _poll_initial_control(player)).is_true()
	Input.action_press("move_right")
	Input.action_press("move_forward")
	var departed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return player.collision_layer == 4, 120
	)
	assert_bool(departed).is_true()
	Input.action_release("move_right")
	Input.action_release("move_forward")
	var struck_wall: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(),
		func() -> bool: return absf(player.velocity.x) < 0.05 and player.velocity.z < -0.5,
		90
	)
	assert_bool(struck_wall).is_true()
	var z_at_contact := player.global_position.z
	for _tick: int in 8:
		await get_tree().physics_frame
	assert_float(absf(player.velocity.x)).is_less(0.05)
	assert_float(player.velocity.z).is_less(-0.5)
	assert_float(player.global_position.z).is_less(z_at_contact)
	assert_int(player.collision_layer).is_equal(4)
	assert_int(pulses.live_count(0.0)).is_equal(0)


func test_player_returns_to_control_on_lower_world_geometry_once() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(
			self, Vector3(-1.0, 1.95, 0.0), Vector3(2.0, 0.1, 2.0), "UpperFloor"
		)
	)
	auto_free(
		ELEVATION_FIXTURE.add_box(
			self, Vector3(3.0, -0.05, 0.0), Vector3(6.0, 0.1, 4.0), "LowerFloor"
		)
	)
	var player: UnseeingPlayer = auto_free(
		ELEVATION_FIXTURE.add_player(self, Vector3(-1.0, 2.9, 0.0))
	)
	assert_bool(await _poll_initial_control(player)).is_true()
	Input.action_press("move_right")
	var departed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return player.collision_layer == 4, 120
	)
	assert_bool(departed).is_true()
	Input.action_release("move_right")
	var transitions := 0
	var prior_layer := player.collision_layer
	var settled := false
	for _tick: int in 180:
		await get_tree().physics_frame
		if prior_layer == 4 and player.collision_layer == 2:
			transitions += 1
		prior_layer = player.collision_layer
		if player.collision_layer == 2 and player.is_on_floor():
			settled = true
			break
	assert_bool(settled).is_true()
	for _tick: int in 30:
		await get_tree().physics_frame
		if prior_layer == 4 and player.collision_layer == 2:
			transitions += 1
		prior_layer = player.collision_layer
		assert_int(player.collision_layer).is_equal(2)
		assert_bool(player.is_on_floor()).is_true()
	assert_int(transitions).is_equal(1)
	assert_int(player.collision_mask).is_equal(4_294_967_291)
	assert_bool(player.call("support_collider_id") != null).is_true()
	assert_float(player.global_position.y).is_equal_approx(0.9, 0.0010001192092895508)


func test_player_rejects_actor_layer_floor_before_cat_adapter_exists() -> void:
	var actor_floor: StaticBody3D = auto_free(ELEVATION_FIXTURE.add_floor(self))
	actor_floor.collision_layer = 2
	actor_floor.collision_mask = 4_294_967_295
	var player: UnseeingPlayer = auto_free(ELEVATION_FIXTURE.add_player(self))
	var rejected_while_engine_floor: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return player.collision_layer == 4 and player.is_on_floor(), 12
	)
	assert_bool(rejected_while_engine_floor).is_true()
	assert_bool(player.call("support_collider_id") == null).is_true()
	assert_int(player.collision_mask).is_equal(4_294_967_289)
	var fell_through: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(),
		func() -> bool: return not player.is_on_floor() and player.global_position.y < 0.8,
		30
	)
	assert_bool(fell_through).is_true()


func test_server_backed_world_body_without_node_is_accepted_support() -> void:
	var observation_id := get_instance_id()
	_add_server_floor(0.0, observation_id)
	var player: UnseeingPlayer = auto_free(ELEVATION_FIXTURE.add_player(self))
	var accepted: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(),
		func() -> bool:
			return (
				player.collision_layer == 2
				and player.is_on_floor()
				and player.call("support_collider_id") == observation_id
			),
		12
	)
	assert_bool(accepted).is_true()


func test_server_backed_zero_object_id_is_accepted_with_null_identity() -> void:
	var body := _add_server_floor(0.0, 0)
	assert_int(PhysicsServer3D.body_get_object_instance_id(body)).is_equal(0)
	var player: UnseeingPlayer = auto_free(ELEVATION_FIXTURE.add_player(self))
	var accepted: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return player.collision_layer == 2 and player.is_on_floor(), 12
	)
	assert_bool(accepted).is_true()
	assert_bool(player.call("support_collider_id") == null).is_true()


func test_snap_only_step_down_stays_controlled() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(
			self, Vector3(-1.5, -0.05, 0.0), Vector3(3.0, 0.1, 3.0), "UpperStep"
		)
	)
	auto_free(
		ELEVATION_FIXTURE.add_box(
			self, Vector3(2.0, -0.13, 0.0), Vector3(4.0, 0.1, 3.0), "LowerStep"
		)
	)
	var player: UnseeingPlayer = auto_free(
		ELEVATION_FIXTURE.add_player(self, Vector3(-0.7, 0.9, 0.0))
	)
	assert_bool(await _poll_initial_control(player)).is_true()
	Input.action_press("move_right")
	var reached_lower := false
	for _tick: int in 90:
		await get_tree().physics_frame
		assert_int(player.collision_layer).is_equal(2)
		assert_bool(player.is_on_floor()).is_true()
		if player.global_position.x > 0.75:
			reached_lower = true
			break
	Input.action_release("move_right")
	assert_bool(reached_lower).is_true()
	assert_int(player.collision_mask).is_equal(4_294_967_291)
	assert_float(player.global_position.y).is_equal_approx(0.82, 0.0010001192092895508)
	assert_bool(player.call("support_collider_id") != null).is_true()


func test_player_ramp_up_and_down_never_becomes_airborne() -> void:
	const PLATFORM_ROOT_Y := 1.35
	const ROOT_TOLERANCE_M := 0.0010001192092895508
	var datum := Vector3.ZERO
	auto_free(
		ELEVATION_FIXTURE.add_box(
			self, Vector3(-1.7, -0.05, 0.0), Vector3(2.0, 0.1, 2.0), "RampApproach"
		)
	)
	auto_free(ELEVATION_FIXTURE.add_ramp(self, datum))
	auto_free(ELEVATION_FIXTURE.add_ramp_platform(self, datum))
	var player: UnseeingPlayer = auto_free(
		ELEVATION_FIXTURE.add_player(self, Vector3(-1.45, 0.9, 0.0))
	)
	var pulses := player.pulses as Pulses
	assert_object(pulses).is_not_null()
	assert_bool(await _poll_initial_control(player)).is_true()
	Input.action_press("move_right")
	var reached_platform := false
	for _tick: int in 120:
		await get_tree().physics_frame
		assert_int(player.collision_layer).is_equal(2)
		assert_int(player.collision_mask).is_equal(4_294_967_291)
		assert_bool(player.is_on_floor()).is_true()
		assert_bool(player.call("support_collider_id") != null).is_true()
		if player.global_position.x > 1.15:
			reached_platform = true
			break
	Input.action_release("move_right")
	assert_bool(reached_platform).is_true()
	assert_float(absf(player.global_position.y - PLATFORM_ROOT_Y)).is_less_equal(ROOT_TOLERANCE_M)
	Input.action_press("move_left")
	var returned_to_datum := false
	for _tick: int in 120:
		await get_tree().physics_frame
		assert_int(player.collision_layer).is_equal(2)
		assert_int(player.collision_mask).is_equal(4_294_967_291)
		assert_bool(player.is_on_floor()).is_true()
		assert_bool(player.call("support_collider_id") != null).is_true()
		if player.global_position.x < -1.15:
			returned_to_datum = true
			break
	Input.action_release("move_left")
	assert_bool(returned_to_datum).is_true()
	assert_float(player.global_position.y).is_equal_approx(0.9, ROOT_TOLERANCE_M)
	assert_int(pulses.live_count(0.0)).is_equal(0)


func test_poisoned_player_pre_move_transform_or_rotation_refuses_without_move_or_wave() -> void:
	var player: UnseeingPlayer = auto_free(ELEVATION_FIXTURE.add_player(self))
	var pulses := player.pulses as Pulses
	assert_object(pulses).is_not_null()
	player.velocity = Vector3(1.0, -2.0, 3.0)
	var poisoned := player.global_transform
	poisoned.origin.x = NAN
	player.global_transform = poisoned
	var poisoned_bits := var_to_bytes(player.global_transform)
	var run_boundary := func() -> void:
		await get_tree().physics_frame
		await get_tree().physics_frame
	await assert_error(run_boundary).is_push_error(
		"UnseeingPlayer: physics transform refused: actor_position.x must be finite"
	)
	assert_bool(player.is_physics_processing()).is_false()
	assert_array(var_to_bytes(player.global_transform)).is_equal(poisoned_bits)
	assert_vector(player.velocity).is_equal(Vector3.ZERO)
	assert_int(player.collision_layer).is_equal(2)
	assert_int(player.collision_mask).is_equal(4_294_967_291)
	assert_array(player.queued_waves()).is_empty()
	assert_int(pulses.live_count(0.0)).is_equal(0)


func test_nonfinite_player_relocation_is_atomic() -> void:
	var player: UnseeingPlayer = auto_free(UnseeingPlayer.new())
	player.pulses = Pulses.new()
	player.position = Vector3(3.0, 4.0, -5.0)
	add_child(player)
	player.velocity = Vector3(1.25, -2.5, 3.75)
	player.collision_layer = 8
	player.collision_mask = 16
	player.queue_wave(2, Vector3(1.0, 0.5, -1.0), 6.0, 5.5, 0.75, 2, Vector3.UP)
	var before_transform := player.global_transform
	var before_velocity := player.velocity
	var before_queue := player.queued_waves()
	var before_id: Variant = player.call("support_collider_id")
	for lane: int in 3:
		for poison: float in [NAN, INF, -INF, 1_000_001.0, -1_000_001.0]:
			var target := Vector3(7.0, 8.0, 9.0)
			target[lane] = poison
			var verdict: Dictionary = player.call("relocate", target)
			assert_bool(verdict.has("unavailable")).is_true()
			assert_bool(player.global_transform == before_transform).is_true()
			assert_vector(player.velocity).is_equal(before_velocity)
			assert_int(player.collision_layer).is_equal(8)
			assert_int(player.collision_mask).is_equal(16)
			assert_array(player.queued_waves()).is_equal(before_queue)
			assert_bool(player.call("support_collider_id") == before_id).is_true()


func test_valid_player_relocation_clears_motion_and_restores_control_pair() -> void:
	auto_free(ELEVATION_FIXTURE.add_floor(self))
	var player: UnseeingPlayer = auto_free(ELEVATION_FIXTURE.add_player(self))
	var acquired_world_support: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(),
		func() -> bool:
			return (
				player.collision_layer == 2
				and player.is_on_floor()
				and player.call("support_collider_id") != null
			),
		12
	)
	assert_bool(acquired_world_support).is_true()
	player.velocity = Vector3(1.25, -2.5, 3.75)
	player.collision_layer = 8
	player.collision_mask = 16
	player.queue_wave(2, Vector3(1.0, 0.5, -1.0), 6.0, 5.5, 0.75, 2, Vector3.UP)
	var before_queue := player.queued_waves()
	var verdict: Dictionary = player.call("relocate", Vector3(7.0, 8.0, 9.0))
	assert_dict(verdict).is_equal({"relocated": true})
	assert_vector(player.global_position).is_equal(Vector3(7.0, 8.0, 9.0))
	assert_vector(player.velocity).is_equal(Vector3.ZERO)
	assert_int(player.collision_layer).is_equal(2)
	assert_int(player.collision_mask).is_equal(4_294_967_291)
	assert_array(player.queued_waves()).is_equal(before_queue)
	assert_bool(player.call("support_collider_id") == null).is_true()

	var became_airborne: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return player.collision_layer == 4, 12
	)
	assert_bool(became_airborne).is_true()
	player.queue_wave(3, Vector3(-2.0, 1.0, 4.0), 3.0, 4.0, 0.5, 1, Vector3.RIGHT)
	var airborne_queue := player.queued_waves()
	player.velocity = Vector3(-1.0, -2.0, -3.0)
	player.collision_layer = 8
	player.collision_mask = 16
	verdict = player.call("relocate", Vector3(6.0, 7.0, 8.0))
	assert_dict(verdict).is_equal({"relocated": true})
	assert_vector(player.global_position).is_equal(Vector3(6.0, 7.0, 8.0))
	assert_vector(player.velocity).is_equal(Vector3.ZERO)
	assert_int(player.collision_layer).is_equal(2)
	assert_int(player.collision_mask).is_equal(4_294_967_291)
	assert_array(player.queued_waves()).is_equal(airborne_queue)
	assert_bool(player.call("support_collider_id") == null).is_true()


func test_player_knobs_reach_the_runtime_player_before_tree_entry() -> void:
	var game: UnseeingGame = auto_free(WORLD_FIXTURE.game())
	var authored := PackedFloat64Array([12.3, 27.5, 2.0, 6.0, 0.7, 7.5])
	for index: int in PLAYER_CONFIG_FIELDS.size():
		game.set(PLAYER_CONFIG_FIELDS[index], authored[index])
	add_child(game)
	var active: PackedFloat64Array = game.player.call("motion_config_snapshot")
	assert_array(active).is_equal(authored)


func test_out_of_range_player_knob_retains_the_prior_scalar() -> void:
	var refused: Array[Array] = [
		[0, 0.09],
		[0, 30.01],
		[1, 0.49],
		[1, 50.01],
		[2, -0.01],
		[2, 10.01],
		[3, 0.09],
		[3, 20.01],
		[4, -0.01],
		[4, 1.01],
		[5, -0.01],
		[5, 10.01],
	]
	for case: Array in refused:
		var game: UnseeingGame = auto_free(UnseeingGame.new())
		var index: int = case[0]
		game.set(PLAYER_CONFIG_FIELDS[index], case[1])
		assert_float(game.get(PLAYER_CONFIG_FIELDS[index])).is_equal(PLAYER_CONFIG_DEFAULTS[index])
	for index: int in PLAYER_CONFIG_FIELDS.size():
		for poison: float in [NAN, INF, -INF]:
			var game: UnseeingGame = auto_free(UnseeingGame.new())
			game.set(PLAYER_CONFIG_FIELDS[index], poison)
			assert_float(game.get(PLAYER_CONFIG_FIELDS[index])).is_equal(
				PLAYER_CONFIG_DEFAULTS[index]
			)


func test_valid_player_threshold_pairs_round_trip_above_and_below_defaults() -> void:
	for pair: Vector2 in [Vector2(8.0, 9.0), Vector2(0.1, 0.2)]:
		for silent_first: bool in [true, false]:
			var authored := UnseeingGame.new()
			authored.level_scene = WORLD_FIXTURE.level_scene()
			if silent_first:
				authored.set("player_landing_silent_speed", pair.x)
				authored.set("player_landing_full_speed", pair.y)
			else:
				authored.set("player_landing_full_speed", pair.y)
				authored.set("player_landing_silent_speed", pair.x)
			var packed := PackedScene.new()
			assert_int(packed.pack(authored)).is_equal(OK)
			authored.free()
			var game: UnseeingGame = auto_free(packed.instantiate() as UnseeingGame)
			assert_object(game).is_not_null()
			add_child(game)
			var expected: Array = PLAYER_CONFIG_DEFAULTS.duplicate()
			expected[2] = pair.x
			expected[3] = pair.y
			var active: PackedFloat64Array = game.player.call("motion_config_snapshot")
			assert_array(active).is_equal(expected)


func test_invalid_final_player_threshold_pair_refuses_before_player_construction() -> void:
	var game: UnseeingGame = auto_free(WORLD_FIXTURE.game())
	game.set("player_landing_silent_speed", 8.0)
	game.set("player_landing_full_speed", 7.0)
	var enter := func() -> void: add_child(game)
	await assert_error(enter).is_push_error(INVALID_THRESHOLD_ERROR)
	assert_object(game.player).is_null()


func _add_server_floor(top_y: float, object_id: int) -> RID:
	var shape := PhysicsServer3D.box_shape_create()
	PhysicsServer3D.shape_set_data(shape, Vector3(10.0, 0.05, 10.0))
	var body := PhysicsServer3D.body_create()
	PhysicsServer3D.body_set_mode(body, PhysicsServer3D.BODY_MODE_STATIC)
	PhysicsServer3D.body_set_collision_layer(body, 1)
	PhysicsServer3D.body_set_collision_mask(body, 4_294_967_295)
	PhysicsServer3D.body_add_shape(body, shape)
	PhysicsServer3D.body_set_state(
		body,
		PhysicsServer3D.BODY_STATE_TRANSFORM,
		Transform3D(Basis.IDENTITY, Vector3(0.0, top_y - 0.05, 0.0))
	)
	if object_id != 0:
		PhysicsServer3D.body_attach_object_instance_id(body, object_id)
	var space: RID = get_tree().root.world_3d.space
	PhysicsServer3D.body_set_space(body, space)
	_server_rids.append(body)
	_server_rids.append(shape)
	return body


func _poll_initial_control(player: UnseeingPlayer) -> bool:
	return await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return player.collision_layer == 2 and player.is_on_floor(), 12
	)
