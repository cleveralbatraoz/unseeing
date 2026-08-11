extends GdUnitTestSuite
## The scripted cane: tap() must ride the SAME queued-intent path as the
## left click — executed next physics tick, through the full aimed/rest/
## swish decision tree, swallowed by the cooldown. queue_wave() fakes a
## wave; tap() taps the cane. These tests break if tap() ever bypasses
## the tree (e.g. emits directly) or executes outside the physics tick.

var _player: UnseeingPlayer
var _pulses: Pulses


func before_test() -> void:
	_pulses = Pulses.new()
	_player = auto_free(UnseeingPlayer.new())
	_player.pulses = _pulses
	_player.position = Vector3(0, 0.9, 0)
	add_child(_player)
	_add_floor()


func _add_floor() -> void:
	var body: StaticBody3D = auto_free(StaticBody3D.new())
	body.position = Vector3(0, -0.5, 0)
	var col := CollisionShape3D.new()
	var shape := BoxShape3D.new()
	shape.size = Vector3(20, 1, 20)
	col.shape = shape
	body.add_child(col)
	add_child(body)


func test_tap_waits_for_the_physics_tick_then_runs_the_tree() -> void:
	await get_tree().physics_frame
	_player.tick(5.0)
	_player.tap()
	# queued intent only: the clock of the last ACCEPTED tap is untouched
	# until the physics tick runs the decision tree
	assert_float(_player.last_tap).is_equal(-10.0)
	await get_tree().physics_frame
	await get_tree().physics_frame
	assert_float(_player.last_tap).is_equal_approx(5.0, 1e-9)


func test_an_aimed_down_tap_births_a_real_wave() -> void:
	await get_tree().physics_frame
	# pitch below -0.12 with the cane resting on the floor: the rest-tap
	# voice — a kind-0 wave born at the tip. Slot 0's birth lane leaves
	# the virgin sentinel (-1) only when a wave truly entered the pool.
	_player.camera.rotation.x = -0.5
	_player.tick(5.0)
	_player.tap()
	await get_tree().physics_frame
	await get_tree().physics_frame
	assert_float(_player.tap_target.y).is_less(0.2)
	assert_float(_pulses.dat[0].x).is_greater_equal(0.0)


func test_a_second_tap_inside_the_cooldown_is_swallowed() -> void:
	await get_tree().physics_frame
	_player.tick(5.0)
	_player.tap()
	await get_tree().physics_frame
	await get_tree().physics_frame
	_player.tick(5.05)
	_player.tap()
	await get_tree().physics_frame
	await get_tree().physics_frame
	# 0.05 s < TAP_COOLDOWN 0.15 s: the second tap must not restamp
	assert_float(_player.last_tap).is_equal_approx(5.0, 1e-9)
