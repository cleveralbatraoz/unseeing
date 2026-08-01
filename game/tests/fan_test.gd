extends GdUnitTestSuite
## The fan's motion and acoustics envelope. Ported 1:1 from the retired
## custom runner.


## The head's oscillation must actually sweep, and never exceed its range —
## the collider rides the same curve, so this is also a physics bound.
func test_fan_motion_envelope() -> void:
	var lo := 0.0
	var hi := 0.0
	for i: int in 200:
		var a := SoundFan.pivot_angle(float(i) * 0.1)
		lo = minf(lo, a)
		hi = maxf(hi, a)
	assert_bool(hi <= SoundFan.PIVOT_RANGE + 0.001).is_true()
	assert_bool(lo >= -SoundFan.PIVOT_RANGE - 0.001).is_true()
	assert_bool(hi > SoundFan.PIVOT_RANGE * 0.9).is_true()
	assert_bool(lo < -SoundFan.PIVOT_RANGE * 0.9).is_true()


func test_fan_blades_spin() -> void:
	assert_bool(SoundFan.spin_angle(1.0) != SoundFan.spin_angle(1.1)).is_true()


## A hum slot lives ring + 2s; the constant wash must not flood the pool.
func test_fan_wash_stays_within_slot_headroom() -> void:
	var concurrent := (SoundFan.HUM_RANGE / SoundFan.HUM_SPEED + 2.0) / SoundFan.WHOOSH_EVERY
	assert_bool(concurrent <= 12.0).is_true()


func test_fan_wash_is_directed_cone() -> void:
	assert_bool(SoundFan.BEAM_COS > 0.7).is_true()
	assert_bool(SoundFan.BEAM_COS < 0.95).is_true()
