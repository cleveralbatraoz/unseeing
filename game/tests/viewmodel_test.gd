extends GdUnitTestSuite
## The viewmodel's math stays bounded. HeroBody is wired to a real player in
## the tree and driven through one scripted life — a nervous walk, a cane
## tap, a stop — with the envelope held on EVERY frame: the head-bob inside
## its amplitude, the look-sway inside its clamps, both shoes at or above
## their floor, and both immediate meshes rebuilt with at least one surface
## of all-finite vertices. The player's physics is switched off so the suite
## alone owns time and the scripted velocity.

const DT := 1.0 / 60.0

var _walk_vel := Vector3(0, 0, -UnseeingPlayer.speed())
var _player: UnseeingPlayer
var _hero: HeroBody
var _now := 0.0


func before_test() -> void:
	var pulses := Pulses.new()
	_player = auto_free(UnseeingPlayer.new())
	_player.pulses = pulses
	_player.position = Vector3(0, 0.9, 0)
	_player.rotation.y = 0.0
	add_child(_player)
	_player.set_physics_process(false)  # the test owns the clock and velocity
	_hero = auto_free(HeroBody.new())
	_hero.player = _player
	_hero.camera = _player.camera
	_hero.pulses = pulses
	_hero.cane_mat = ShaderMaterial.new()
	_hero.body_mat = ShaderMaterial.new()
	add_child(_hero)
	_now = 0.0


## One scripted frame, then the whole envelope is re-checked.
func _step(vel: Vector3) -> void:
	_player.velocity = vel
	_now += DT
	_hero.update(_now, DT)
	_assert_frame_envelope()


func _assert_frame_envelope() -> void:
	assert_float(absf(_hero.bob_offset)).is_less_equal(0.028)  # bob amplitude
	assert_float(absf(_hero.sway_x())).is_less_equal(0.07)  # sway clamps
	assert_float(absf(_hero.sway_y())).is_less_equal(0.06)
	var shoes := _hero.shoes()
	assert_float(shoes[0].y).is_greater_equal(0.0649)  # shoes on/above floor
	assert_float(shoes[1].y).is_greater_equal(0.0649)
	_assert_mesh_built_finite(_hero.cane_mesh())
	_assert_mesh_built_finite(_hero.body_mesh())


## A rebuilt immediate mesh must be sane: at least one surface, vertices in
## it, and not a single NaN or infinity among them.
func _assert_mesh_built_finite(mesh: ImmediateMesh) -> void:
	assert_int(mesh.get_surface_count()).is_greater(0)
	for s: int in mesh.get_surface_count():
		var arrays: Array = mesh.surface_get_arrays(s)
		var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		assert_int(verts.size()).is_greater(0)
		var broken := 0
		for v: Vector3 in verts:
			if not v.is_finite():
				broken += 1
		assert_int(broken).is_equal(0)


## 2 s of walking under an aggressive look wander (the sway targets saturate
## their clamps), then a tap (the strike envelope kicks the cane tip out to
## the tap target), then a stop (everything eases home) — bounded throughout.
func test_walk_tap_stop_stays_bounded() -> void:
	for frame: int in 120:
		_player.rotation.y += 0.2 * sin(frame * 0.3)
		_player.camera.rotation.x = 0.9 * sin(frame * 0.17)
		_step(_walk_vel)
	_player.last_tap = _now
	_player.tap_target = Vector3(0.4, 1.1, -1.4)
	for frame: int in 30:
		_step(_walk_vel)
	for frame: int in 60:
		_step(Vector3.ZERO)
