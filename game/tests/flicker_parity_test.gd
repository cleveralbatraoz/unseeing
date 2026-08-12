extends GdUnitTestSuite
## The bit-exactness proof for the flicker law's move to Rust. Both sides
## draw from the SAME seeded RandomNumberGenerator stream — the GDScript
## Flicker directly, WaveCore's flicker_probe() test door through its own
## seeded RNG — over hundreds of frames of VARYING dt, and must return
## identical arrays, frame for frame. Draw order and draw count matter as
## much as the arithmetic: a reordered or miscounted randf() call desyncs
## the shared stream even when every individual formula stays correct.
## This is the equivalence harness the migration rule requires, committed
## green while flicker.gd still exists — the proof that deleting it later
## loses nothing.

const SEED := 0x5EED
const FRAME_COUNT := 600


## 1/60 and 1/45 alternating — the brief's exact varying-dt scenario.
func _alternating_dts(count: int) -> PackedFloat64Array:
	var out := PackedFloat64Array()
	for i: int in count:
		out.append(1.0 / 60.0 if i % 2 == 0 else 1.0 / 45.0)
	return out


func test_rust_flicker_matches_gdscript_bit_for_bit_over_varying_dt() -> void:
	var dts := _alternating_dts(FRAME_COUNT)
	var rng := RandomNumberGenerator.new()
	rng.seed = SEED
	var gd_flicker := Flicker.new(rng)
	var gd_values := PackedFloat64Array()
	for dt: float in dts:
		gd_values.append(gd_flicker.next(dt))
	var rust_values := WaveCore.new().flicker_probe(SEED, dts)
	assert_int(rust_values.size()).is_equal(gd_values.size())
	for i: int in gd_values.size():
		(
			assert_float(rust_values[i])
			. override_failure_message(
				"frame %d diverged: gdscript=%s rust=%s" % [i, gd_values[i], rust_values[i]]
			)
			. is_equal(gd_values[i])
		)
