extends GdUnitTestSuite
## Pins the engine claim the env restore rests on: RandomNumberGenerator's
## `state` property is the complete stream position — read it, draw, write
## it back, and the stream REPLAYS the same draw. If a Godot upgrade ever
## breaks this, the flicker restore silently diverges; this suite is the
## tripwire. (`seed` is NOT sufficient: its getter returns the last seed
## assigned, not the current position — also pinned here.)


func test_state_round_trip_replays_the_stream() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 0x5EED
	rng.randf()  # advance somewhere mid-stream
	var mark: int = rng.state
	var expected := rng.randf()
	rng.state = mark
	assert_float(rng.randf()).is_equal(expected)


func test_seed_alone_does_not_carry_the_position() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 0x5EED
	var first := rng.randf()
	rng.randf()
	# seed reads back as assigned even though the stream has moved on
	assert_int(rng.seed).is_equal(0x5EED)
	var again := RandomNumberGenerator.new()
	again.seed = 0x5EED
	assert_float(again.randf()).is_equal(first)
