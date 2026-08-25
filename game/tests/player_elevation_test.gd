# gdlint:ignore = max-public-methods
extends GdUnitTestSuite
## The physical hero boundary: support motion, layer separation, relocation,
## and designer-injected fall configuration.

const WORLD_FIXTURE := preload("res://tests/world_fixture.gd")
const ELEVATION_FIXTURE := preload("res://tests/character_elevation_fixture.gd")

const DT := 1.0 / 60.0
## One hand-derived f32 ULP at magnitude 2 — the transported body spans
## y < 2, and a single post-build f32 add stays within half of this.
const F32_ULP_AT_2 := 2.384185791015625e-7

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


## A hero body dressed onto an already-entered player, sharing its pulses
## and eye — the wiring main.gd performs, minus the materials' shaders.
func _add_hero(player: UnseeingPlayer) -> HeroBody:
	var hero: HeroBody = auto_free(HeroBody.new())
	hero.player = player
	hero.camera = player.camera
	hero.pulses = player.pulses
	hero.cane_mat = ShaderMaterial.new()
	hero.body_mat = ShaderMaterial.new()
	add_child(hero)
	return hero


## A scripted, physics-off player + hero pair: the test alone owns the
## clock and the reported velocity, exactly like the viewmodel suites.
func _scripted_pair(at: Vector3) -> Array:
	var player: UnseeingPlayer = auto_free(ELEVATION_FIXTURE.add_player(self, at))
	player.set_physics_process(false)
	var hero := _add_hero(player)
	return [player, hero]


func _cane_vertices(hero: HeroBody) -> PackedVector3Array:
	return hero.cane_mesh().surface_get_arrays(0)[Mesh.ARRAY_VERTEX]


## The one transport law, end to end: a raised player's baked body is the
## flat player's body translated up by exactly the support elevation —
## x/z bits untouched, every y within one f32 ULP of flat + support.
func test_raised_support_translates_every_player_body_vertex_once() -> void:
	var flat: Array = _scripted_pair(Vector3(2.0, 0.9, -3.0))
	var raised: Array = _scripted_pair(Vector3(2.0, 1.35, -3.0))
	var flat_player: UnseeingPlayer = flat[0]
	var raised_player: UnseeingPlayer = raised[0]
	var flat_hero: HeroBody = flat[1]
	var raised_hero: HeroBody = raised[1]
	var vel := Vector3(0, 0, -UnseeingPlayer.speed())
	var now := 0.0
	for frame: int in 5:
		now += DT
		flat_player.velocity = vel
		raised_player.velocity = vel
		flat_hero.update(now, DT)
		raised_hero.update(now, DT)
	var flat_arrays: Array = flat_hero.body_mesh().surface_get_arrays(0)
	var raised_arrays: Array = raised_hero.body_mesh().surface_get_arrays(0)
	var flat_verts: PackedVector3Array = flat_arrays[Mesh.ARRAY_VERTEX]
	var raised_verts: PackedVector3Array = raised_arrays[Mesh.ARRAY_VERTEX]
	assert_int(flat_verts.size()).is_greater(0)
	assert_int(raised_verts.size()).is_equal(flat_verts.size())
	# the transport moves ONLY vertex Y: normals and labels are untouched
	assert_array(raised_arrays[Mesh.ARRAY_NORMAL]).is_equal(flat_arrays[Mesh.ARRAY_NORMAL])
	assert_array(raised_arrays[Mesh.ARRAY_CUSTOM0]).is_equal(flat_arrays[Mesh.ARRAY_CUSTOM0])
	var support := raised_player.position.y - flat_player.position.y
	var broken := 0
	for i: int in flat_verts.size():
		var f := flat_verts[i]
		var r := raised_verts[i]
		if f.x != r.x or f.z != r.z:
			broken += 1
			continue
		if absf(r.y - (f.y + support)) > F32_ULP_AT_2:
			broken += 1
	assert_int(broken).is_equal(0)


## Shoes and the footstep's birth point both ride the platform: on a root
## at 1.35 the fresh walker's first step is born at the raised support
## plus the contact height, under a shoe at or above the raised floor.
func test_shoes_and_footstep_origin_follow_platform_height() -> void:
	var pair: Array = _scripted_pair(Vector3(0.0, 1.35, 0.0))
	var player: UnseeingPlayer = pair[0]
	var hero: HeroBody = pair[1]
	player.velocity = Vector3(0, 0, -UnseeingPlayer.speed())
	hero.update(DT, DT)
	var shoes := hero.shoes()
	assert_float(shoes[0].y).is_greater_equal(_raised_support() + 0.0649)
	assert_float(shoes[1].y).is_greater_equal(_raised_support() + 0.0649)
	var queue := player.queued_waves()
	assert_int(queue.size()).is_equal(1)
	var wave: Dictionary = queue[0]
	var at: Vector3 = wave.at
	assert_float(at.y).is_equal(_f32(_raised_support() + _f32(0.04)))


## The camera's local height carries only the bob around its base — root
## height reaches the eye exactly once, through the parent transform.
func test_camera_inherits_root_height_exactly_once() -> void:
	var pair: Array = _scripted_pair(Vector3(0.0, 1.35, 0.0))
	var player: UnseeingPlayer = pair[0]
	var hero: HeroBody = pair[1]
	var now := 0.0
	for frame: int in 8:
		now += DT
		player.velocity = Vector3(0, 0, -UnseeingPlayer.speed())
		hero.update(now, DT)
	assert_bool(hero.bob_offset != 0.0).is_true()
	var base := UnseeingPlayer.cam_base_y()
	assert_float(player.camera.position.y).is_equal_approx(
		base + hero.bob_offset, 5.960464477539063e-8
	)
	assert_float(player.camera.global_position.y).is_equal_approx(
		player.global_position.y + player.camera.position.y, F32_ULP_AT_2
	)


## The arm anchors to THIS frame's camera — the one already carrying this
## frame's bob — never to a stale pre-bob transform. The hand is read back
## from the cane mesh (tube of 60 vertices, then the hand sphere's pole)
## and compared against the live camera plus the hand-derived viewmodel
## offsets for the exact frame count driven.
func test_nonzero_bob_anchors_hand_and_elbow_to_the_same_frame_camera() -> void:
	var pair: Array = _scripted_pair(Vector3(0.0, 1.35, 0.0))
	var player: UnseeingPlayer = pair[0]
	var hero: HeroBody = pair[1]
	var frames := 10
	var now := 0.0
	for frame: int in frames:
		now += DT
		player.velocity = Vector3(0, 0, -UnseeingPlayer.speed())
		hero.update(now, DT)
	# hand-derived from the published viewmodel constants: walk_amp eases
	# by (1 - amp) * dt * 6 per moving frame, leg_phase walks at 7.4 rad/s
	var amp := 0.0
	var phase := 0.0
	for frame: int in frames:
		amp += (1.0 - amp) * minf(DT * 6.0, 1.0)
		phase += DT * 7.4
	var bob := 0.028 * sin(phase * 2.0) * amp
	assert_bool(hero.bob_offset != 0.0).is_true()
	assert_float(hero.bob_offset).is_equal_approx(bob, 1e-9)
	assert_float(player.camera.position.y).is_equal_approx(UnseeingPlayer.cam_base_y() + bob, 1e-7)
	var bx := 0.016 * sin(phase) * amp
	var by := 0.012 * sin(phase * 2.0) * amp
	var cam := player.camera.global_position
	var expected_hand := (
		cam + Vector3.RIGHT * (0.30 + bx) + Vector3.UP * (-0.40 + by) + Vector3.FORWARD * 0.55
	)
	var cane := _cane_vertices(hero)
	var hand := cane[60] - Vector3.UP * 0.055
	assert_vector(hand).is_equal_approx(expected_hand, Vector3(2e-6, 2e-6, 2e-6))


## A missing, freed, or foreign camera refuses the whole frame before any
## installed state moves: meshes, bob, sway, shoes, the player's eye and
## the wave queue all hold their prior values.
func test_missing_freed_or_mismatched_visual_camera_refuses_before_mutation() -> void:
	var pair: Array = _scripted_pair(Vector3(0.0, 0.9, 0.0))
	var player: UnseeingPlayer = pair[0]
	var hero: HeroBody = pair[1]
	var now := 0.0
	for frame: int in 3:
		now += DT
		player.velocity = Vector3(0, 0, -UnseeingPlayer.speed())
		hero.update(now, DT)
	var before := _visual_state(player, hero)

	var foreign: Camera3D = auto_free(Camera3D.new())
	add_child(foreign)
	hero.camera = foreign
	var mismatched := func() -> void: hero.update(now + DT, DT)
	await assert_error(mismatched).is_push_error(
		"hero_body: visual camera refused — not the player's live eye"
	)
	assert_array(_visual_state(player, hero)).is_equal(before)

	var doomed := Camera3D.new()
	add_child(doomed)
	hero.camera = doomed
	doomed.free()
	var freed := func() -> void: hero.update(now + 2.0 * DT, DT)
	await assert_error(freed).is_push_error(
		"hero_body: visual camera refused — not the player's live eye"
	)
	assert_array(_visual_state(player, hero)).is_equal(before)

	hero.camera = null
	hero.update(now + 3.0 * DT, DT)
	assert_array(_visual_state(player, hero)).is_equal(before)

	# a player whose own eye reference is gone cannot be half-committed:
	# the proof token cannot be produced, so the whole frame refuses
	hero.camera = player.camera
	var stolen: Camera3D = player.camera
	player.camera = null
	var orphaned := func() -> void: hero.update(now + 4.0 * DT, DT)
	await assert_error(orphaned).is_push_error(
		"hero_body: visual camera refused — not the player's live eye"
	)
	player.camera = stolen
	assert_array(_visual_state(player, hero)).is_equal(before)


## Airborne planar velocity is trajectory, not gait: while the player is
## off support the walk cycle must not advance and no footstep may fire,
## however fast the body is still translating.
func test_airborne_planar_trajectory_does_not_drive_walk_or_steps() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(
			self, Vector3(0.0, 1.95, 0.0), Vector3(2.0, 0.1, 2.0), "Departure"
		)
	)
	var player: UnseeingPlayer = auto_free(
		ELEVATION_FIXTURE.add_player(self, Vector3(0.5, 2.9, 0.0))
	)
	var hero := _add_hero(player)
	assert_bool(await _poll_initial_control(player)).is_true()
	Input.action_press("move_right")
	var departed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return player.collision_layer == 4, 90
	)
	Input.action_release("move_right")
	assert_bool(departed).is_true()
	var now := 0.0
	for frame: int in 20:
		now += DT
		player.tick(now)
		hero.update(now, DT)
		assert_float(hero.bob_offset).is_equal(0.0)
		assert_array(player.queued_waves()).is_empty()


## A poisoned engine sample refuses the whole visual frame atomically:
## the viewmodel, both meshes (vertices, normals, labels), the shoes, the
## bob, the eye, the queue and the suppression all retain their installed
## values, and the untouched viewmodel proves it by keeping exact cadence.
# gdlint:ignore = max-line-length
func test_poisoned_player_visual_sample_retains_vm_both_meshes_normals_labels_shoes_bob_cane_queue_and_suppression(
) -> void:
	var pair: Array = _scripted_pair(Vector3(0.0, 0.9, 0.0))
	var player: UnseeingPlayer = pair[0]
	var hero: HeroBody = pair[1]
	var vel := Vector3(0, 0, -UnseeingPlayer.speed())
	var now := 0.0
	for frame: int in 5:
		now += DT
		player.velocity = vel
		hero.update(now, DT)
	assert_int(player.queued_waves().size()).is_equal(1)  # the frame-0 step
	var before := _visual_state(player, hero)

	player.velocity = Vector3(NAN, 0.0, 0.0)
	now += DT
	var poisoned := func() -> void: hero.update(now, DT)
	await assert_error(poisoned).is_push_error(
		"hero_body: visual sample refused: actor_velocity.x must be finite"
	)
	assert_array(_visual_state(player, hero)).is_equal(before)

	# the retained viewmodel keeps honest time: walking on, the next step
	# still lands exactly on moving frame 26 of the walk — a reset or
	# half-written viewmodel (or a falsely armed suppression) cannot.
	var fires: Array[int] = []
	for frame: int in range(5, 30):
		now += DT
		player.velocity = vel
		hero.update(now, DT)
		if player.queued_waves().size() > 1 + fires.size():
			fires.append(frame)
	assert_array(fires).is_equal([26])


## Every installed visual fact as one comparable value — the mesh hashes
## cover vertices, normals and CUSTOM0 labels for both layers, digested so
## a mismatch names itself without dumping raw bytes.
func _visual_state(player: UnseeingPlayer, hero: HeroBody) -> Array:
	return [
		hash(var_to_bytes(hero.cane_mesh().surface_get_arrays(0))),
		hash(var_to_bytes(hero.body_mesh().surface_get_arrays(0))),
		hero.bob_offset,
		hero.sway_x(),
		hero.sway_y(),
		hero.shoes(),
		player.camera.position.y,
		player.queued_waves().size(),
	]


## One value narrowed through a Vector3 lane — the engine's exact f32
## arithmetic, bit for bit, without trusting the GDScript float parser's
## last-ULP conversion of a decimal literal.
func _f32(value: float) -> float:
	return Vector3(value, 0.0, 0.0).x


## f32(f32(1.35) - f32(0.9)): the exact support lane a root at 1.35 yields.
func _raised_support() -> float:
	return _f32(_f32(1.35) - _f32(0.9))


## Airborne wall contact is never support: a capsule sliding down a wall
## books no landing, speaks no landing voice, and fires no footstep, no
## matter how long the contact lasts.
func test_airborne_wall_contact_never_becomes_a_landing_or_step_wave() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(
			self, Vector3(0.0, 1.95, 0.0), Vector3(2.0, 0.1, 2.0), "Departure"
		)
	)
	auto_free(
		ELEVATION_FIXTURE.add_box(self, Vector3(2.05, 1.5, 0.0), Vector3(0.1, 9.0, 4.0), "AirWall")
	)
	var player: UnseeingPlayer = auto_free(
		ELEVATION_FIXTURE.add_player(self, Vector3(0.5, 2.9, 0.0))
	)
	var pulses := player.pulses as Pulses
	var hero := _add_hero(player)
	assert_bool(await _poll_initial_control(player)).is_true()
	Input.action_press("move_right")
	var departed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return player.collision_layer == 4, 90
	)
	Input.action_release("move_right")
	assert_bool(departed).is_true()
	var now := 0.0
	for _tick: int in 30:
		await get_tree().physics_frame
		now += DT
		player.tick(now)
		hero.update(now, DT)
		assert_int(player.collision_layer).is_equal(4)
		assert_int(pulses.live_count(now + 0.05)).is_equal(0)
		assert_int(pulses.pending_echo_count()).is_equal(0)
		assert_array(player.queued_waves()).is_empty()


## A drop under the silent threshold lands without a sound — no pulse, no
## echo capacity — yet the landing itself is retained: the armed latch
## swallows the walker's next cadence-ready footstep.
func test_small_player_drop_retains_landing_but_emits_nothing() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(
			self, Vector3(0.0, -0.155, 0.0), Vector3(20.0, 0.1, 20.0), "Floor"
		)
	)
	var player: UnseeingPlayer = auto_free(
		ELEVATION_FIXTURE.add_player(self, Vector3(0.0, 0.9, 0.0))
	)
	var pulses := player.pulses as Pulses
	var hero := _add_hero(player)
	var went_airborne := false
	var landed := false
	for _tick: int in 60:
		await get_tree().physics_frame
		if player.collision_layer == 4:
			went_airborne = true
		if went_airborne and player.collision_layer == 2 and player.is_on_floor():
			landed = true
			break
	assert_bool(went_airborne).is_true()
	assert_bool(landed).is_true()
	assert_int(pulses.live_count(0.1)).is_equal(0)
	assert_int(pulses.pending_echo_count()).is_equal(0)
	Input.action_press("move_forward")
	var now := 0.0
	for _frame: int in 10:
		await get_tree().physics_frame
		now += DT
		player.tick(now)
		hero.update(now, DT)
	# the landing was retained: the instant first step was consumed whole
	assert_int(pulses.live_count(now + 0.05)).is_equal(0)
	assert_array(player.queued_waves()).is_empty()


## An audible landing is born once, at the support point lifted by the
## contact height, and reflects off the SUPPORT normal: every scheduled
## echo answers from the normal's hemisphere — geometry below the platform
## plane stays silent.
func test_audible_player_landing_uses_support_normal_and_relative_origin_once() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(self, Vector3(0.0, 0.95, 0.0), Vector3(2.0, 0.1, 2.0), "Platform")
	)
	auto_free(
		ELEVATION_FIXTURE.add_box(self, Vector3(1.55, 2.5, 0.0), Vector3(0.1, 3.0, 4.0), "EchoWall")
	)
	auto_free(
		ELEVATION_FIXTURE.add_box(self, Vector3(0.0, -0.05, 0.0), Vector3(20.0, 0.1, 20.0), "Below")
	)
	var player: UnseeingPlayer = auto_free(
		ELEVATION_FIXTURE.add_player(self, Vector3(0.0, 2.4, 0.0))
	)
	var pulses := player.pulses as Pulses
	var landed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return player.collision_layer == 2 and player.is_on_floor(), 120
	)
	assert_bool(landed).is_true()
	assert_int(pulses.live_count(0.1)).is_equal(1)
	# once: further ticks re-emit nothing
	for _tick: int in 5:
		await get_tree().physics_frame
	assert_int(pulses.live_count(0.1)).is_equal(1)
	# support-relative origin: platform top + 0.04, under the capsule
	var origin: Vector3 = pulses.pos[0]
	assert_float(origin.y).is_equal_approx(1.0 + 0.04, 0.0010001192092895508)
	assert_float(absf(origin.x)).is_less(0.4)
	assert_float(absf(origin.z)).is_less(0.4)
	# the reflecting emitter ran with the UP support normal: echoes exist
	# and every appointment sits in the up hemisphere — never on the
	# tempting floor 1 m below the platform plane
	assert_int(pulses.pending_echo_count()).is_greater(0)
	for echo: Pulses.Echo in pulses.pending_echoes():
		assert_float(echo.pos.y).is_greater_equal(1.0 - 0.05)


## A hard drop caps the landing voice at the authored maxima: gain 0.85,
## range 5.0 — read back from the pool's packed lanes.
func test_high_player_drop_caps_gain_and_range() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(self, Vector3(0.0, -0.05, 0.0), Vector3(20.0, 0.1, 20.0), "Floor")
	)
	var player: UnseeingPlayer = auto_free(
		ELEVATION_FIXTURE.add_player(self, Vector3(0.0, 3.9, 0.0))
	)
	var pulses := player.pulses as Pulses
	var landed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return player.collision_layer == 2 and player.is_on_floor(), 180
	)
	assert_bool(landed).is_true()
	assert_int(pulses.live_count(0.1)).is_equal(1)
	var dat: Vector4 = pulses.dat[0]
	assert_int(int(floorf(dat.w / 10.0))).is_equal(2)  # landing voice: kind 2
	assert_float(dat.y).is_equal(5.0)  # capped range
	assert_float(dat.z).is_equal(4.0)  # the wave law speed
	assert_float(fmod(dat.w, 10.0) / 9.0).is_equal_approx(0.85, 1e-6)  # capped gain
	var origin: Vector3 = pulses.pos[0]
	assert_float(origin.y).is_equal_approx(0.04, 0.0010001192092895508)


## Zero authored landing gain silences every landing completely: neither
## emitter runs, no pulse slot and no echo appointment is consumed.
func test_zero_player_landing_gain_consumes_no_pulse_or_echo() -> void:
	var game: UnseeingGame = auto_free(WORLD_FIXTURE.game())
	game.set("player_landing_max_gain", 0.0)
	add_child(game)
	await get_tree().process_frame
	await get_tree().physics_frame
	var verdict: Dictionary = game.player.call("relocate", Vector3(2.0, 5.0, 6.0))
	assert_dict(verdict).is_equal({"relocated": true})
	var landed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(),
		func() -> bool: return game.player.collision_layer == 2 and game.player.is_on_floor(),
		240
	)
	assert_bool(landed).is_true()
	assert_int(game.wave_core.live_count(game.now)).is_equal(0)
	assert_int(game.wave_core.pending_echo_count()).is_equal(0)


## Zero authored landing range is the same silence through the other knob.
func test_zero_player_landing_range_consumes_no_pulse_or_echo() -> void:
	var game: UnseeingGame = auto_free(WORLD_FIXTURE.game())
	game.set("player_landing_max_range", 0.0)
	add_child(game)
	await get_tree().process_frame
	await get_tree().physics_frame
	var verdict: Dictionary = game.player.call("relocate", Vector3(2.0, 5.0, 6.0))
	assert_dict(verdict).is_equal({"relocated": true})
	var landed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(),
		func() -> bool: return game.player.collision_layer == 2 and game.player.is_on_floor(),
		240
	)
	assert_bool(landed).is_true()
	assert_int(game.wave_core.live_count(game.now)).is_equal(0)
	assert_int(game.wave_core.pending_echo_count()).is_equal(0)
