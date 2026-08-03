extends GdUnitTestSuite
## The companion cat against real physics: builds only when injected,
## wanders its patch of floor on its own, and voices its fore paws into
## the wave pool as soft kind-2 steps — the walking-lantern contract.

const DT := 1.0 / 60.0

var _pulses: Pulses
var _cat: WaveCat


func _add_floor() -> void:
	var body: StaticBody3D = auto_free(StaticBody3D.new())
	body.position = Vector3(0, -0.05, 0)
	var col := CollisionShape3D.new()
	var shape := BoxShape3D.new()
	shape.size = Vector3(20, 0.1, 20)
	col.shape = shape
	body.add_child(col)
	add_child(body)


func _add_cat() -> void:
	_pulses = Pulses.new()
	_cat = auto_free(WaveCat.new())
	_cat.pulses = _pulses
	_cat.data_mat = ShaderMaterial.new()
	_cat.position = Vector3(0, 0, 0)
	add_child(_cat)


## No silent nulls: a cat without its injected pool and material reports
## the miss, builds nothing, and disables its own processing.
func test_uninjected_cat_reports_and_disables() -> void:
	var bare: WaveCat = auto_free(WaveCat.new())
	var enter := func() -> void: add_child(bare)
	await assert_error(enter).is_push_error("WaveCat: pulses/data_mat not injected — cat disabled")
	assert_bool(bare.is_physics_processing()).is_false()
	assert_int(bare.get_child_count()).is_equal(0)


## The cat lives its own life: within a few simulated seconds it leaves
## its spawn (the brain's first pause is 0.8 s, then it wanders), and its
## fore paws sound as kind-2 pulses born at floor height with the paw
## voice — reach, speed and gain exactly as the engine constants say.
func test_cat_wanders_and_paw_waves_sound() -> void:
	_add_floor()
	_add_cat()
	var start := _cat.position
	var now := 0.0
	for i: int in 240:
		now = float(i) * DT
		_cat.tick(now)
		await get_tree().physics_frame
	var travelled := (_cat.position - start).length()
	assert_float(travelled).is_greater(0.3)
	assert_int(_pulses.live_count(now)).is_greater(0)
	var kind := int(floorf(_pulses.dat[0].w / 10.0))
	assert_int(kind).is_equal(2)
	assert_float(fmod(_pulses.dat[0].w, 10.0) / 9.0).is_equal_approx(WaveCat.paw_gain(), 0.001)
	# the pool's lanes are f32: 0.8 lands one ULP off the f64 constant
	assert_float(_pulses.dat[0].y).is_equal_approx(WaveCat.paw_range(), 0.000001)
	assert_float(_pulses.dat[0].z).is_equal(WaveCat.paw_speed())
	assert_float(_pulses.pos[0].y).is_equal_approx(0.02, 0.001)


## The silhouette exists: after a rendered frame the immediate mesh
## carries one surface — the whole cat, tubes and spheres and whiskers.
func test_cat_mesh_builds() -> void:
	_add_floor()
	_add_cat()
	await get_tree().physics_frame
	await get_tree().process_frame
	assert_int(_cat.cat_mesh().get_surface_count()).is_equal(1)


## Four paws on the floor: the observable paw positions stay at ground
## level (planted or barely lifted) while the cat lives its life.
func test_paws_ride_the_floor() -> void:
	_add_floor()
	_add_cat()
	for i: int in 180:
		_cat.tick(float(i) * DT)
		await get_tree().physics_frame
		var paws := _cat.paw_positions()
		assert_int(paws.size()).is_equal(4)
		for paw: Vector3 in paws:
			assert_float(paw.y).is_between(-0.001, 0.05)


## The voice constants cross the boundary intact — the suite's handle on
## the Rust-side design envelope. Softer and shorter than the hero's own
## footsteps (1.6 m, gain 0.8), but a real, visible ripple.
func test_paw_voice_constants() -> void:
	assert_float(WaveCat.paw_range()).is_equal(1.3)
	assert_float(WaveCat.paw_speed()).is_equal(4.0)
	assert_float(WaveCat.paw_gain()).is_equal_approx(0.6, 0.0001)
