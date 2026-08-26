extends GdUnitTestSuite
## Cross-actor collision and support: two controlled actors physically block
## each other on real world ground, elevation keeps unrelated actors from
## cross-registering as each other's support, an airborne actor passes
## clean through a controlled one to land on the world beneath it, and no
## actor's collider is ever accepted as another actor's floor.

const ELEVATION_FIXTURE := preload("res://tests/character_elevation_fixture.gd")


## Two controlled actors (layer 2, mask excludes only the airborne layer)
## collide with each other exactly like any other solid: standing nearly on
## top of each other on a shared world floor, they settle apart rather than
## interpenetrating.
func test_two_controlled_actors_block_each_other_on_world_floor() -> void:
	auto_free(ELEVATION_FIXTURE.add_floor(self))
	var player: UnseeingPlayer = auto_free(
		ELEVATION_FIXTURE.add_player(self, Vector3(0.0, 0.9, 0.0))
	)
	var cat: WaveCat = auto_free(ELEVATION_FIXTURE.add_cat(self, Vector3(0.15, 0.0, 0.0)))
	for _tick: int in 60:
		await get_tree().physics_frame
	assert_int(player.collision_layer).is_equal(2)
	assert_int(cat.collision_layer).is_equal(2)
	assert_bool(player.is_on_floor()).is_true()
	assert_bool(cat.is_on_floor()).is_true()
	# the player's capsule (radius 0.35) and the cat's capsule (radius
	# 0.11) cannot both occupy the same horizontal column: real collision
	# resolution must have separated their centres by at least the sum of
	# their radii, well past the 0.15 m they were dropped apart at.
	var delta := Vector2(
		player.global_position.x - cat.global_position.x,
		player.global_position.z - cat.global_position.z
	)
	assert_float(delta.length()).is_greater_equal(0.35 + 0.11 - 0.02)


## An actor standing on its own real support at one elevation never reports
## the other actor's identity as its own support, even when both are
## controlled and share the same patch of floor.
func test_controlled_actors_at_different_elevations_do_not_create_contact() -> void:
	auto_free(ELEVATION_FIXTURE.add_floor(self))
	auto_free(ELEVATION_FIXTURE.add_table(self, Vector3(3.0, 0.0, 0.0)))
	var player: UnseeingPlayer = auto_free(
		ELEVATION_FIXTURE.add_player(self, Vector3(0.0, 0.9, 0.0))
	)
	var cat: WaveCat = auto_free(ELEVATION_FIXTURE.add_cat(self, Vector3(3.0, 3.0, 0.0)))
	var player_settled: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return player.collision_layer == 2 and player.is_on_floor(), 60
	)
	var cat_settled: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 2 and cat.is_on_floor(), 240
	)
	assert_bool(player_settled).is_true()
	assert_bool(cat_settled).is_true()
	assert_float(player.global_position.y).is_equal_approx(0.9, 0.001)
	assert_float(cat.global_position.y).is_equal_approx(
		ELEVATION_FIXTURE.TABLE_TOP_Y, 0.0010001192092895508
	)
	var player_support: Variant = player.call("support_collider_id")
	var cat_support: Variant = cat.call("support_collider_id")
	if player_support != null:
		var player_support_id: int = player_support
		assert_int(player_support_id).is_not_equal(cat.get_instance_id())
	if cat_support != null:
		var cat_support_id: int = cat_support
		assert_int(cat_support_id).is_not_equal(player.get_instance_id())


## An airborne cat (layer 4, mask excludes both actor layers) falls clean
## through a controlled player standing in its path and lands on the real
## world floor beneath, never blocked by the player's body.
func test_centred_airborne_cat_passes_through_player_and_lands_on_world() -> void:
	auto_free(ELEVATION_FIXTURE.add_floor(self))
	var player: UnseeingPlayer = auto_free(
		ELEVATION_FIXTURE.add_player(self, Vector3(0.0, 0.9, 0.0))
	)
	var cat: WaveCat = auto_free(ELEVATION_FIXTURE.add_cat(self, Vector3(0.0, 5.0, 0.0)))
	var departed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 4, 12
	)
	assert_bool(departed).is_true()
	var landed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 2 and cat.is_on_floor(), 300
	)
	assert_bool(landed).is_true()
	assert_float(cat.global_position.y).is_equal_approx(0.0, 0.01)
	# the player was never disturbed: still standing, still controlled.
	assert_int(player.collision_layer).is_equal(2)
	assert_bool(player.is_on_floor()).is_true()
	assert_float(player.global_position.y).is_equal_approx(0.9, 0.001)


## A controlled player dropped onto a live cat's body never accepts the
## cat's collider as floor support — the actor-layer exclusion the support
## scan already proves against a synthetic actor-layer floor holds against
## a real one: the player is forced airborne, its mask then excludes the
## cat's layer entirely, and it falls clean through with no floor beneath.
func test_controlled_player_walking_off_world_onto_cat_rejects_actor_support() -> void:
	var cat: WaveCat = auto_free(ELEVATION_FIXTURE.add_cat(self, Vector3(0.0, 0.0, 0.0)))
	var player: UnseeingPlayer = auto_free(
		ELEVATION_FIXTURE.add_player(self, Vector3(0.0, 1.2, 0.0))
	)
	var rejected: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return player.collision_layer == 4, 90
	)
	assert_bool(rejected).is_true()
	assert_bool(player.call("support_collider_id") == null).is_true()
	var fell_through: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(),
		func() -> bool: return not player.is_on_floor() and player.global_position.y < -2.0,
		300
	)
	assert_bool(fell_through).is_true()
