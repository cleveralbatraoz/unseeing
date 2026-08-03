extends GdUnitTestSuite
## The demo taps on schedule, provably: first at 0.6 s, then every 4 s from
## each fire, always at the level scene's derived wall point — and an
## unarmed schedule never fires at all.

const LEVEL_SCENE := preload("res://scenes/level_01.tscn")
const DT := 1.0 / 60.0


func test_fires_at_expected_times_with_pinned_point() -> void:
	var lvl: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	lvl.inject(ShaderMaterial.new(), Pulses.new())
	add_child(lvl)
	var tap := DemoTap.new(lvl.demo_tap(), lvl.demo_tap_normal())
	tap.armed = true
	var fires := PackedFloat64Array()
	var now := 0.0
	while now < 10.0:
		now += DT
		if tap.fire_due(now):
			fires.append(now)
	assert_int(fires.size()).is_equal(3)
	# one frame of quantization each, plus accumulated float dust
	assert_float(fires[0]).is_equal_approx(0.6, DT * 1.5)
	assert_float(fires[1]).is_equal_approx(4.6, DT * 1.5)
	assert_float(fires[2]).is_equal_approx(8.6, DT * 1.5)
	assert_vector(tap.point).is_equal(lvl.demo_tap())
	assert_vector(tap.normal).is_equal(Vector3(-1, 0, 0))


func test_unarmed_never_fires() -> void:
	var tap := DemoTap.new(Vector3.ZERO, Vector3.UP)
	var fired := 0
	var now := 0.0
	while now < 10.0:
		now += DT
		if tap.fire_due(now):
			fired += 1
	assert_int(fired).is_equal(0)
