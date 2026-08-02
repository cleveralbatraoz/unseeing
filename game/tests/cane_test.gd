extends GdUnitTestSuite
## The cane's three voices, driven end to end against real physics: a left
## click queues the tap, the next physics tick executes it in physics
## context. Pins each mode's wave — the aimed strike where the player looks,
## the rest tap on whatever the cane tip touches, the silent air swish —
## and the 0.15 s cooldown that swallows a too-eager second tap.

const NOW := 1.0
const EYE_POS := Vector3(0, 1.6, 0)  # camera: player (0, 0.9, 0) + local 0.7

var _pulses: Pulses
var _player: UnseeingPlayer


func before_test() -> void:
	_pulses = Pulses.new()
	_player = auto_free(UnseeingPlayer.new())
	_player.pulses = _pulses
	_player.position = Vector3(0, 0.9, 0)
	_player.rotation.y = 0.0  # face -Z: deterministic aim for every fixture
	add_child(_player)
	_player.now = NOW


## One box: a static collider for the cane's rays, freed after the test.
func _add_box(center: Vector3, size: Vector3) -> void:
	var body: StaticBody3D = auto_free(StaticBody3D.new())
	body.position = center
	var col := CollisionShape3D.new()
	var shape := BoxShape3D.new()
	shape.size = size
	col.shape = shape
	body.add_child(col)
	add_child(body)


## Tap the way the mouse does — a left click through the viewport's input
## chain — then run the physics tick that executes the queued tap.
func _tap() -> void:
	var press := InputEventMouseButton.new()
	press.button_index = MOUSE_BUTTON_LEFT
	press.pressed = true
	get_viewport().push_input(press)
	await get_tree().physics_frame
	await get_tree().physics_frame


## Aimed strike: the gaze ray reaches a wall within cane reach, so the wave
## is born exactly where the player looked — type 0, radius 6, full gain.
func test_wall_strike_wave() -> void:
	_add_box(Vector3(0, 1.5, -1.1), Vector3(3, 3, 0.3))  # face at z = -0.95
	await get_tree().physics_frame
	await get_tree().physics_frame
	await _tap()
	assert_int(_pulses.live_count(NOW + 0.1)).is_equal(1)
	assert_int(int(floor(_pulses.dat[0].w / 10.0))).is_equal(0)
	assert_float(_pulses.dat[0].y).is_equal(6.0)
	assert_float(_pulses.dat[0].z).is_equal(5.5)
	assert_float(fmod(_pulses.dat[0].w, 10.0) / 9.0).is_equal_approx(1.0, 0.001)
	var expected := Vector3(0, 1.6, -0.95)
	assert_vector(_pulses.pos[0]).is_equal_approx(expected, Vector3(0.03, 0.03, 0.03))
	assert_vector(_player.tap_target).is_equal_approx(expected, Vector3(0.03, 0.03, 0.03))


## Rest tap, raised: the gaze hits nothing, but the cane tip rests on a
## tabletop — the wave is born on the table, not where the player looked.
func test_table_rest_tap_wave() -> void:
	_add_box(Vector3(0, 0.35, -1.6), Vector3(1.0, 0.7, 1.0))  # top at y = 0.70
	await get_tree().physics_frame
	await get_tree().physics_frame
	await _tap()
	assert_int(_pulses.live_count(NOW + 0.1)).is_equal(1)
	assert_int(int(floor(_pulses.dat[0].w / 10.0))).is_equal(0)
	assert_float(_pulses.dat[0].y).is_equal(6.0)
	assert_float(fmod(_pulses.dat[0].w, 10.0) / 9.0).is_equal_approx(1.0, 0.001)
	var expected := Vector3(0, 0.72, -1.7)  # cane reach ahead, on the top
	assert_vector(_pulses.pos[0]).is_equal_approx(expected, Vector3(0.03, 0.03, 0.03))


## Rest tap, floor: looking down over open floor — the tap lands at the cane
## tip on the ground with the softer floor voice: radius 5, gain 0.85.
func test_floor_tap_wave() -> void:
	_add_box(Vector3(0, -0.05, 0), Vector3(20, 0.1, 20))  # floor top at y = 0
	_player.camera.rotation.x = -0.5  # looking down, past the -0.12 threshold
	await get_tree().physics_frame
	await get_tree().physics_frame
	await _tap()
	assert_int(_pulses.live_count(NOW + 0.1)).is_equal(1)
	assert_int(int(floor(_pulses.dat[0].w / 10.0))).is_equal(0)
	assert_float(_pulses.dat[0].y).is_equal(5.0)
	assert_float(_pulses.dat[0].z).is_equal(5.5)
	assert_float(fmod(_pulses.dat[0].w, 10.0) / 9.0).is_equal_approx(0.85, 0.001)
	var expected := Vector3(0, 0.02, -1.7)
	assert_vector(_pulses.pos[0]).is_equal_approx(expected, Vector3(0.03, 0.03, 0.03))


## Air swish: nothing within reach, the cane rests on nothing raised, the
## gaze is level — the tap consumes the click but emits NO wave at all.
func test_air_swish_emits_nothing() -> void:
	await get_tree().physics_frame
	await get_tree().physics_frame
	await _tap()
	assert_float(_player.last_tap).is_equal(NOW)  # the tap did happen
	assert_int(_pulses.live_count(NOW + 0.1)).is_equal(0)
	assert_int(_pulses.pending_echo_count()).is_equal(0)
	var swish := Vector3(0, 1.6, -1.5)  # remembered for the strike animation
	assert_vector(_player.tap_target).is_equal_approx(swish, Vector3(0.03, 0.03, 0.03))


## The cooldown: a second tap 0.05 s after the first is swallowed whole;
## once 0.15 s have passed the cane speaks again.
func test_second_tap_within_cooldown_is_swallowed() -> void:
	_add_box(Vector3(0, 1.5, -1.1), Vector3(3, 3, 0.3))
	await get_tree().physics_frame
	await get_tree().physics_frame
	await _tap()
	assert_int(_pulses.live_count(NOW + 0.1)).is_equal(1)
	_player.now = NOW + 0.05
	await _tap()
	assert_float(_player.last_tap).is_equal(NOW)  # swallowed: no new tap
	assert_int(_pulses.live_count(NOW + 0.1)).is_equal(1)
	_player.now = NOW + 0.3
	await _tap()
	assert_float(_player.last_tap).is_equal(NOW + 0.3)
	assert_int(_pulses.live_count(NOW + 0.35)).is_equal(2)
