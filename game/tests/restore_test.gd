extends GdUnitTestSuite
## The restore doors against the live scene. Each test freezes nothing —
## it drives the clock by hand, exactly as observer_test does.

const MAIN_SCENE := preload("res://scenes/main.tscn")


func test_a_restored_cat_resumes_the_same_life() -> void:
	var main: UnseeingMain = auto_free(MAIN_SCENE.instantiate() as UnseeingMain)
	add_child(main)
	var cat: WaveCat = main.cats[0]
	# let the cat live a little, on real physics — main's own _process
	# advances the clock and ticks every cat in the tree each frame, so
	# no manual clock-driving is needed here (see observer_test's helpers
	# for the same idiom)
	for _i in 30:
		await get_tree().process_frame
	var mood_at_capture: int = cat.mood()
	var paws_at_capture: PackedVector3Array = cat.paw_positions()
	# the capture_state/restore_state doors built this task are pub(crate)
	# Rust, with no #[func] surface yet — the real equivalence gates are
	# the cargo lockstep tests (cat_brain, cat_gait, cat_body) plus Task
	# 9/10's blob round trip; this test only pins that a live scene cat is
	# capturable at all, i.e. never in the "never built" refusal state.
	assert_int(mood_at_capture).is_not_equal(-1)
	assert_int(paws_at_capture.size()).is_equal(4)


## A world that has actually run: sources hold appointments, the hero body
## has built its viewmodel, and every clock has a reading. Capture refuses
## an unticked world by design, so every capture test starts here.
func _boot_ticked() -> UnseeingMain:
	var main: UnseeingMain = auto_free(MAIN_SCENE.instantiate() as UnseeingMain)
	add_child(main)
	# one real process frame so sources book appointments and the
	# viewmodel exists — capture refuses an unticked world by design
	await get_tree().process_frame
	await get_tree().physics_frame
	return main


## The blob is ALL-OR-NOTHING and self-describing: every group present, the
## pool at its full 64 slots, and the hash of the whole thing carried inside
## it. The viewmodel clocks are the proof that capture is WIDER than
## snapshot — `snapshot()` has never been able to see them, because the
## viewmodel lives on HeroBody and the observer was never handed one.
func test_capture_is_total_and_carries_its_own_hash() -> void:
	var main := await _boot_ticked()
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(blob.has("unavailable")).is_false()
	assert_int(blob["format_version"]).is_equal(1)
	assert_int((blob["slots"] as Array).size()).is_equal(64)
	assert_int((blob["cats"] as Array).size()).is_equal(main.cats.size())
	assert_str(blob["hash"]).has_length(16)
	# hero group carries the viewmodel clocks snapshot() never had
	var vm: Dictionary = blob["hero"]["viewmodel"]
	assert_bool(vm.has("step_t")).is_true()
	assert_bool(vm.has("step_side")).is_true()


## Reading the world must not move it. A capture that advanced a cadence,
## drained an echo or drew from an RNG would leave the second snapshot
## disagreeing with the first — which is exactly the bug that makes a
## debugging tool untrustworthy.
func test_capture_never_mutates() -> void:
	var main := await _boot_ticked()
	var before: Dictionary = main.observer.snapshot(main.now)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	# a REFUSED capture reads nothing and would pass this trivially
	assert_bool(blob.has("unavailable")).is_false()
	var after: Dictionary = main.observer.snapshot(main.now)
	assert_int((after["echoes"] as Array).size()).is_equal((before["echoes"] as Array).size())
	assert_int(after["live_slots"]).is_equal(before["live_slots"])
	var was: Array = before["sources"]
	var is_now: Array = after["sources"]
	assert_int(is_now.size()).is_equal(was.size())
	for i in was.size():
		# a capture that advanced a cadence or an RNG would differ here
		var before_source: Dictionary = was[i]
		var after_source: Dictionary = is_now[i]
		assert_that(after_source.get("next_emit")).is_equal(before_source.get("next_emit"))


## No `unknown` array here, unlike a snapshot: a subsystem the observer
## cannot read refuses the WHOLE blob. A blob missing its hero and a blob
## of a heroless world would otherwise serialise the same, and only one of
## them is the truth.
func test_capture_without_the_body_refuses_whole() -> void:
	var main := await _boot_ticked()
	main.observer.inject_body(null)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(blob.has("unavailable")).is_true()
	assert_bool(blob.has("slots")).is_false()


## The env group comes from GDScript, so it is the one group the boundary
## cannot type-check at the language level. A missing key must name itself:
## "the env group is malformed" would send a reader auditing nine fields.
func test_capture_with_malformed_env_names_the_key() -> void:
	var main := await _boot_ticked()
	var env: Dictionary = main.capture_env()
	env.erase("flicker_rng_state")
	var blob: Dictionary = main.observer.capture(main.now, env)
	assert_bool(blob.has("unavailable")).is_true()
	assert_str(blob["unavailable"]).contains("flicker_rng_state")


## `now` and `env.now` are one instant said twice, and neither is trusted
## over the other. Every appointment in the blob — every pool birth, every
## echo, every source's next beat — is dated against this clock, so a blob
## dated at two instants restores into neither.
func test_capture_refuses_a_clock_that_disagrees_with_its_env() -> void:
	var main := await _boot_ticked()
	var blob: Dictionary = main.observer.capture(main.now + 1.0, main.capture_env())
	assert_bool(blob.has("unavailable")).is_true()
	assert_str(blob["unavailable"]).contains("now")
	assert_bool(blob.has("slots")).is_false()


## The blob's real destination is a FILE, and the trip through JSON is
## lossy three separate ways, all measured on this build: `JSON.stringify`
## renders a Vector3 through its pretty-printer (it comes back a String),
## every number comes back a float (an int past 2^53 comes back
## corrupted), and Godot's own float text is not round-trip exact even at
## full precision (1/60 comes back one ULP away). This pins that the
## shipped encoding survives all three — every field compared, so a lane
## that lost a bit is named here rather than surfacing as a mystery
## divergence inside the restore gate.
func test_a_json_round_tripped_blob_still_parses() -> void:
	var main := await _boot_ticked()
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(blob.has("unavailable")).is_false()
	var text := JSON.stringify(blob, "", true, true)
	var raw: Variant = JSON.parse_string(text)
	# a blob carrying a type JSON cannot spell fails HERE, before the parser
	assert_that(raw).is_not_null()
	var parsed := raw as Dictionary
	assert_str(main.observer.blob_round_trip_ok(blob, parsed)).is_equal("")


## The env is the composition root's own state, so putting it back is
## GDScript's job — but GDScript cannot read the blob's spelling of a
## float (measured: `String.to_float` is not correctly rounded, drops the
## sign of "-0", and reads "NaN" as zero). So it comes home through Rust,
## in exactly the shape `capture_env()` produced, and the proof is the
## hash: capture the same unmoved world through the returned env and every
## env bit must land where it started.
func test_the_env_group_comes_home_in_the_shape_capture_env_made_it() -> void:
	var main := await _boot_ticked()
	var env := main.capture_env()
	var blob: Dictionary = main.observer.capture(main.now, env)
	var parsed := JSON.parse_string(JSON.stringify(blob, "", true, true)) as Dictionary
	var back: Dictionary = main.observer.env_of(parsed)
	assert_bool(back.has("unavailable")).is_false()
	assert_int(back.size()).is_equal(env.size())
	var again: Dictionary = main.observer.capture(main.now, back)
	assert_str(again["hash"]).is_equal(blob["hash"])


## The SIGN OF ZERO is part of the state: `reproduce/blob.rs` compares
## float bit patterns, so a velocity of -0.0 and one of 0.0 are different
## worlds. Godot's JSON writer drops that sign — measured on this build,
## -0.0 is rendered "0.0" — which is one of the three reasons no float in
## the blob crosses as a JSON number at all. A hero standing still is the
## cheapest world that can carry one.
func test_a_negative_zero_survives_the_journey() -> void:
	var main := await _boot_ticked()
	main.player.velocity = Vector3(-0.0, -0.0, -0.0)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var parsed := JSON.parse_string(JSON.stringify(blob, "", true, true)) as Dictionary
	assert_str(main.observer.blob_round_trip_ok(blob, parsed)).is_equal("")
