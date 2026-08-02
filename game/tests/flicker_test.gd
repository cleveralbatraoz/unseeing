extends GdUnitTestSuite
## The mood's envelope, pinned: bounded forever, deterministic per seed —
## the flicker may never black the screen out nor spike it, and a seeded
## stream must replay bit-identically for frame-comparison runs.

const DT := 1.0 / 60.0


func _seeded(seed_value: int) -> Flicker:
	var rng := RandomNumberGenerator.new()
	rng.seed = seed_value
	return Flicker.new(rng)


func _sequence(seed_value: int, frames: int) -> PackedFloat64Array:
	var f := _seeded(seed_value)
	var out := PackedFloat64Array()
	for i: int in frames:
		out.append(f.next(DT))
	return out


## 100k fixed-dt frames (~28 minutes) never leave the envelope: the ceiling
## is the clamp, the floor is a dropout dimming the clamped minimum. Dropouts
## must actually occur — the floor of the observed range dips below the clamp.
func test_envelope_bounds_hold_forever() -> void:
	var f := _seeded(0x5EED)
	var lo := INF
	var hi := -INF
	for i: int in 100_000:
		var v := f.next(DT)
		lo = minf(lo, v)
		hi = maxf(hi, v)
	assert_float(lo).is_greater_equal(Flicker.LEVEL_MIN * Flicker.DROP_DEPTH)
	assert_float(hi).is_less_equal(Flicker.LEVEL_MAX)
	assert_float(lo).is_less(Flicker.LEVEL_MIN)  # the dropout path really ran


func test_same_seed_runs_bit_identical() -> void:
	var first_run := _sequence(1234, 10_000)
	var second_run := _sequence(1234, 10_000)
	assert_bool(first_run == second_run).is_true()


func test_different_seeds_differ() -> void:
	var seed_one := _sequence(1, 1_000)
	var seed_two := _sequence(2, 1_000)
	assert_bool(seed_one == seed_two).is_false()
