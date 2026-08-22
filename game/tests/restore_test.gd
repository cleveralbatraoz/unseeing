extends GdUnitTestSuite
## The restore doors against a code-built live world. Each test freezes
## nothing — it awaits real process/physics frames and lets
## `UnseeingGame::process` drive the clock, never a hand-stepped one.

const WORLD_FIXTURE := preload("res://tests/world_fixture.gd")


func test_a_restored_cat_resumes_the_same_life() -> void:
	var main := await _boot_ticked()
	var cats := main.cats()
	if cats.is_empty():
		fail("the explicit restore fixture carries no cat")
		return
	var target_env: Dictionary = main.capture_env()
	target_env["now"] = 8.0
	main.apply_env(target_env)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var captured_cat: Dictionary = (blob["cats"] as Array)[0]
	captured_cat["presence_next"] = "8.0"
	var hash_result: Dictionary = main.observer.canonical_hash_of(blob)
	assert_str(str(hash_result.get("unavailable", ""))).is_empty()
	blob["hash"] = hash_result["hash"]

	# Let the composition root inject a later clock before rewinding. The
	# restored physics tick below then runs with root processing disabled, so
	# only the cat's prepared private clock can date the due pulse.
	for _i in 8:
		await get_tree().process_frame
	assert_float(main.now).is_greater(8.0)
	main.set_process(false)
	var was_paused := get_tree().paused
	get_tree().paused = true
	var verdict: Dictionary = main.restore_blob(blob)
	assert_str(str(verdict.get("unavailable", ""))).is_empty()
	main.player.tap()
	get_tree().paused = false
	await get_tree().physics_frame
	await get_tree().process_frame
	get_tree().paused = true
	var fresh: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_str(str(fresh.get("unavailable", ""))).is_empty()
	var restored_last_tap := str((fresh["hero"] as Dictionary)["last_tap"]).to_float()
	assert_float(restored_last_tap).is_equal(8.0)

	var snap: Dictionary = main.observer.snapshot(8.0)
	assert_str(str(snap.get("unavailable", ""))).is_empty()
	var matching_presence := 0
	for slot_value: Variant in snap["slots"] as Array:
		var slot: Dictionary = slot_value
		var state: String = slot["state"]
		var kind: int = slot["kind"]
		var birth: float = slot["birth"]
		var max_r: float = slot["max_r"]
		var gain: float = slot["gain"]
		if (
			state == "Live"
			and kind == 2
			and is_equal_approx(birth, 8.0)
			and is_equal_approx(max_r, WaveCat.presence_range())
			and is_equal_approx(gain, WaveCat.presence_gain())
		):
			matching_presence += 1
	assert_int(matching_presence).is_equal(1)
	main.set_process(true)
	get_tree().paused = was_paused


## A world that has actually run: sources hold appointments, the hero body
## has built its viewmodel, and every clock has a reading. Capture refuses
## an unticked world by design, so every capture test starts here.
func _boot_ticked() -> UnseeingGame:
	# The parser cases deliberately index one source and one cat. Supplying
	# those collaborators here keeps that non-vacuity while leaving shipped
	# level content entirely under the designer's control.
	var main: UnseeingGame = auto_free(
		WORLD_FIXTURE.game(WORLD_FIXTURE.DEFAULT_EXTENTS, true, true, true)
	)
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
	assert_int(blob["format_version"]).is_equal(2)
	assert_int((blob["slots"] as Array).size()).is_equal(64)
	assert_int((blob["cats"] as Array).size()).is_equal(main.cats().size())
	assert_str(blob["hash"]).has_length(16)
	var hero: Dictionary = blob["hero"]
	assert_bool(hero.has("support_collider_id")).is_false()
	for cat_value: Variant in blob["cats"] as Array:
		var cat: Dictionary = cat_value
		assert_bool(cat.has("support_collider_id")).is_false()
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
		# the key must BE there: two absent appointments compare equal as
		# null, and this loop would pass over a source that had none
		assert_bool(before_source.has("next_emit")).is_true()
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


## The env group crosses the Godot Variant boundary as a Dictionary, so it
## is the one group Rust cannot type-check at compile time. A missing key
## must name itself: "the env group is malformed" would send a reader
## auditing nine fields.
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
	# the env grammar plus the offending key, not merely the word "now" —
	# which occurs inside half the messages this boundary can produce
	assert_str(blob["unavailable"]).contains("malformed: now")
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


## The env is the Rust composition root's own state. Godot's dynamic values
## cannot safely parse the blob's spelling of every float (measured:
## `String.to_float` is not correctly rounded, drops the sign of "-0", and
## reads "NaN" as zero), so the observer parses it before UnseeingGame applies
## it in exactly the shape `capture_env()` produced. The proof is the
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


## A blob is a FILE by the time anything reads it back, so the parser is
## the only thing standing between a damaged one and a world restored to
## whatever the scene happened to be holding. It defaults nothing, and
## these cases pin that every guard names its own dotted path — a parser
## that only said "malformed" would hand its reader five kilobytes and a
## shrug.
##
## Each takes a fresh, independent copy through JSON, breaks exactly one
## field in it, and asks the boundary what it thinks. `_copy` is what a
## damaged file or a hand-edit actually looks like, and it leaves the
## original the comparison is made against untouched.
func _copy(blob: Dictionary) -> Dictionary:
	return JSON.parse_string(JSON.stringify(blob, "", true, true)) as Dictionary


## The diagnostic hashes syntax only: it is safe to use while constructing a
## deliberately invalid semantic fixture, and it never changes the world it
## inspects. That independence is what lets the transaction suite prove each
## preflight refusal without inheriting the original blob's stale hash.
func test_canonical_hash_of_is_read_only_and_names_syntax_faults() -> void:
	var main := await _boot_ticked()
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var before_env: Dictionary = main.capture_env()
	var before_hash: String = blob["hash"]
	var diagnostic: Dictionary = main.observer.canonical_hash_of(_copy(blob))
	assert_str(str(diagnostic.get("unavailable", ""))).is_empty()
	assert_str(diagnostic["hash"]).is_equal(before_hash)
	assert_dict(main.capture_env()).is_equal(before_env)
	var after: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_str(after["hash"]).is_equal(before_hash)

	var broken := _copy(blob)
	((broken["hero"] as Dictionary)["motion"] as Dictionary).erase("phase")
	var refused: Dictionary = main.observer.canonical_hash_of(broken)
	assert_str(refused["unavailable"]).contains("hero.motion.phase: missing")


## Motion's wire is deliberately narrower than a Godot Vector: planar
## velocity has exactly two f32 text lanes and every optional group is either
## null or a dictionary. Each damaged value is independent and must name its
## own dotted location instead of being defaulted to dormant state.
func test_motion_wire_refuses_missing_wrong_and_inconsistent_values_by_path() -> void:
	var main := await _boot_ticked()
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var cases: Array[Array] = []

	var missing_motion := _copy(blob)
	(missing_motion["hero"] as Dictionary).erase("motion")
	cases.append([missing_motion, "hero.motion: missing"])
	var wrong_optional := _copy(blob)
	((wrong_optional["hero"] as Dictionary)["motion"] as Dictionary)["support"] = []
	cases.append([wrong_optional, "hero.motion.support: expected null or a dictionary"])
	var unknown_phase := _copy(blob)
	var unknown_phase_group: Dictionary = (
		(unknown_phase["hero"] as Dictionary)["motion"] as Dictionary
	)["phase"]
	unknown_phase_group["kind"] = "floating"
	cases.append([unknown_phase, 'hero.motion.phase.kind: unknown motion phase "floating"'])
	var short_planar := _copy(blob)
	var short_motion: Dictionary = (short_planar["hero"] as Dictionary)["motion"]
	short_motion["phase"] = {
		"kind": "airborne", "planar_velocity": ["1.0"], "vertical_velocity": "-1.0"
	}
	short_motion["support"] = null
	cases.append([short_planar, "hero.motion.phase.planar_velocity: expected 2 entries, found 1"])
	var wrong_planar_lane := _copy(blob)
	var wrong_lane_motion: Dictionary = (wrong_planar_lane["hero"] as Dictionary)["motion"]
	wrong_lane_motion["phase"] = {
		"kind": "airborne", "planar_velocity": ["1.0", true], "vertical_velocity": "-1.0"
	}
	wrong_lane_motion["support"] = null
	cases.append([wrong_planar_lane, "hero.motion.phase.planar_velocity[1]: expected a string"])
	var nonfinite := _copy(blob)
	var nonfinite_motion: Dictionary = (nonfinite["hero"] as Dictionary)["motion"]
	nonfinite_motion["phase"] = {
		"kind": "airborne", "planar_velocity": ["NaN", "0.0"], "vertical_velocity": "-1.0"
	}
	nonfinite_motion["support"] = null
	cases.append([nonfinite, "hero.motion.phase.planar_velocity[0]: must be finite"])
	var positive_y := _copy(blob)
	var positive_motion: Dictionary = (positive_y["hero"] as Dictionary)["motion"]
	positive_motion["phase"] = {
		"kind": "airborne", "planar_velocity": ["0.0", "0.0"], "vertical_velocity": "0.25"
	}
	positive_motion["support"] = null
	cases.append([positive_y, "hero.motion.phase.vertical_velocity: is inconsistent"])
	var airborne_support := _copy(blob)
	var supported_motion: Dictionary = (airborne_support["hero"] as Dictionary)["motion"]
	supported_motion["phase"] = {
		"kind": "airborne", "planar_velocity": ["0.0", "0.0"], "vertical_velocity": "-0.25"
	}
	supported_motion["support"] = {"point": ["0.0", "0.0", "0.0"], "normal": ["0.0", "1.0", "0.0"]}
	cases.append([airborne_support, "hero.motion.support: is inconsistent"])
	var zero_normal := _copy(blob)
	var zero_motion: Dictionary = (zero_normal["hero"] as Dictionary)["motion"]
	zero_motion["support"] = {"point": ["0.0", "0.0", "0.0"], "normal": ["0.0", "0.0", "0.0"]}
	cases.append([zero_normal, "hero.motion.support.normal: must be a nonzero vector"])
	var negative_landing := _copy(blob)
	var landing_motion: Dictionary = (negative_landing["hero"] as Dictionary)["motion"]
	landing_motion["last_landing"] = {
		"impact_speed": "-0.25",
		"support": {"point": ["0.0", "0.0", "0.0"], "normal": ["0.0", "1.0", "0.0"]}
	}
	cases.append([negative_landing, "hero.motion.last_landing.impact_speed: must be non-negative"])
	# Gates, the player latch, and gait support are required schema fields too.
	# Their spellings and common support datum are parser questions, not live
	# adapter capability questions.
	main.player.queue_wave(2, Vector3(1.0, 0.5, 2.0), 3.0, 4.0, 0.5, 0, Vector3.UP)
	var queued_blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var missing_gate := _copy(queued_blob)
	var wave: Dictionary = ((missing_gate["hero"] as Dictionary)["queued_waves"] as Array)[0]
	wave.erase("gate")
	cases.append([missing_gate, "hero.queued_waves[0].gate: missing"])
	var unknown_gate := _copy(queued_blob)
	var other_wave: Dictionary = ((unknown_gate["hero"] as Dictionary)["queued_waves"] as Array)[0]
	other_wave["gate"] = "perhaps"
	cases.append([unknown_gate, 'hero.queued_waves[0].gate: unknown queued-wave gate "perhaps"'])
	var missing_suppression := _copy(queued_blob)
	(missing_suppression["hero"] as Dictionary).erase("footstep_suppression_pending")
	cases.append([missing_suppression, "hero.footstep_suppression_pending: missing"])
	var missing_support_y := _copy(queued_blob)
	var gait: Dictionary = ((missing_support_y["cats"] as Array)[0] as Dictionary)["gait"]
	gait.erase("support_y")
	cases.append([missing_support_y, "cats[0].gait.support_y: missing"])
	var poisoned_support_y := _copy(queued_blob)
	var poisoned_gait: Dictionary = ((poisoned_support_y["cats"] as Array)[0] as Dictionary)["gait"]
	poisoned_gait["support_y"] = "NaN"
	cases.append([poisoned_support_y, "cats[0].gait.support_y: must be finite"])

	for one: Array in cases:
		var refusal: Dictionary = main.observer.canonical_hash_of(one[0] as Dictionary)
		assert_str(refusal["unavailable"]).contains(str(one[1]))


## The marker a hand-built level needs to have somewhere to wake the hero
## (the same fixture observer_test.gd builds its bare levels from).
func _spawn_marker() -> WaveSpawn:
	var marker := WaveSpawn.new()
	return marker


## An integer field accepts a whole float — the JSON road writes `2` for
## `2.0` and a hand-edit may well write `2.0` back — but a FRACTIONAL one
## is a corrupt blob, not a roundable one. A pulse kind of 2.5 silently
## truncated to 2 is a wave of the wrong class in the pool.
func test_a_fractional_integer_is_named_rather_than_rounded() -> void:
	var main := await _boot_ticked()
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var broken := _copy(blob)
	var slot: Dictionary = (broken["slots"] as Array)[0]
	slot["kind"] = 2.5
	assert_str(main.observer.blob_round_trip_ok(blob, broken)).contains(
		"slots[0].kind: expected a whole number"
	)


## Every fixed-arity run in the blob has a length the FORMAT fixes: 64 pool
## slots, five tail nodes, four paws. A short one is a truncated file, never
## a smaller world — and a parser that just walked what it found would build
## a pool with 63 slots and hash it as though that were the truth.
func test_a_truncated_run_is_named_rather_than_walked() -> void:
	var main := await _boot_ticked()
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var short_pool := _copy(blob)
	(short_pool["slots"] as Array).resize(63)
	assert_str(main.observer.blob_round_trip_ok(blob, short_pool)).contains(
		"slots: expected 64 entries, found 63"
	)
	var short_tail := _copy(blob)
	var cat: Dictionary = (short_tail["cats"] as Array)[0]
	(cat["tail"] as Array).resize(4)
	assert_str(main.observer.blob_round_trip_ok(blob, short_tail)).contains(
		"cats[0].tail: expected 5 entries, found 4"
	)


## Types are checked, not coerced. Godot would happily give a number for a
## string and a truthy value for a bool; a boundary that let it would
## restore a source matched against the name "5" and a paw that is swinging
## because its entry was not empty.
func test_a_wrong_typed_field_is_named_rather_than_coerced() -> void:
	var main := await _boot_ticked()
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var numbered_source := _copy(blob)
	var source: Dictionary = (numbered_source["sources"] as Array)[0]
	source["name"] = 5
	assert_str(main.observer.blob_round_trip_ok(blob, numbered_source)).contains(
		"sources[0].name: expected a string"
	)
	var wordy_paw := _copy(blob)
	var gait: Dictionary = ((wordy_paw["cats"] as Array)[0] as Dictionary)["gait"]
	(gait["in_swing"] as Array)[0] = "yes"
	assert_str(main.observer.blob_round_trip_ok(blob, wordy_paw)).contains(
		"cats[0].gait.in_swing[0]: expected a bool"
	)


## The cat's mood is spelled out on the wire precisely so a rename cannot
## pass silently. An unknown name is refused rather than defaulted to Roam:
## a cat restored wandering when it was sitting is a different cat, and the
## hash would agree with itself about it forever.
func test_an_unknown_mood_is_named_rather_than_defaulted() -> void:
	var main := await _boot_ticked()
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var broken := _copy(blob)
	var brain: Dictionary = ((broken["cats"] as Array)[0] as Dictionary)["brain"]
	(brain["state"] as Dictionary)["kind"] = "Nap"
	assert_str(main.observer.blob_round_trip_ok(blob, broken)).contains(
		'cats[0].brain.state.kind: unknown brain state "Nap"'
	)


## The PCG words cross as 16 hex characters because 64 bits cannot survive
## JSON as a number. Text that is not those characters is refused: a word
## quietly read as zero is a cat whose whole future is a different stream.
func test_a_malformed_rng_word_is_named_rather_than_zeroed() -> void:
	var main := await _boot_ticked()
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var broken := _copy(blob)
	var brain: Dictionary = ((broken["cats"] as Array)[0] as Dictionary)["brain"]
	brain["rng_state"] = "not-sixteen-hex"
	assert_str(main.observer.blob_round_trip_ok(blob, broken)).contains(
		"cats[0].brain.rng_state: expected 16 hex characters"
	)


## The hero is not optional equipment for a blob, though it IS for a
## snapshot: a snapshot names the absence in `unknown` and reports the world
## around it, where a blob without its hero would restore as a different
## world entirely.
func test_capture_without_the_hero_refuses_whole() -> void:
	var main := await _boot_ticked()
	main.observer.inject_hero(null)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_int(blob.size()).is_equal(1)
	assert_str(blob["unavailable"]).contains("never injected the hero")


## A body that refused to build has no viewmodel state — and a DEFAULT pose
## is not a harmless stand-in: it restores a walker mid-stride as one
## standing still, with the footstep clock reset under them.
func test_capture_refuses_a_body_that_never_built_its_viewmodel() -> void:
	var main := await _boot_ticked()
	var bare: HeroBody = auto_free(HeroBody.new())
	add_child(bare)  # uninjected: _ready refuses, so there is no viewmodel
	main.observer.inject_body(bare)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_int(blob.size()).is_equal(1)
	assert_str(blob["unavailable"]).contains("never built its viewmodel")


## A source whose gate can never fire keeps no appointment, and that is a
## refusal rather than a zero: restoring it would leave the gate to book a
## fresh date off the restored clock, and the level would sound one wave the
## original never made.
##
## The level is hand-built and swapped in under the observer, whose hero and
## body injections still stand — a level derives its source list once, in
## _ready, so a live one cannot be given a silenced source after the fact.
func test_capture_refuses_a_source_holding_no_appointment() -> void:
	var main := await _boot_ticked()
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker())
	var fan := SoundFan.new()
	fan.name = "Fan"
	fan.cadence = 0.0  # a non-positive interval never fires (sound_source.rs)
	level.add_child(fan)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	main.observer.inject(level, main.player.camera)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_int(blob.size()).is_equal(1)
	assert_str(blob["unavailable"]).contains("no beat appointment")
	assert_str(blob["unavailable"]).contains("Fan")


## A cat that never built has no mind, stride, tail or pose to read, and a
## defaulted one is a cat with a different life. Same swap as above: the
## stray joins the level AFTER injection, so it is counted by the level's
## census but was never handed a pool, exactly as a designer's mistake would
## leave it.
func test_capture_refuses_a_cat_that_never_built() -> void:
	var main := await _boot_ticked()
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker())
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var stray := WaveCat.new()
	stray.name = "Stray"
	level.add_child(stray)
	add_child(level)
	main.observer.inject(level, main.player.camera)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_int(blob.size()).is_equal(1)
	assert_str(blob["unavailable"]).contains("never built")
	assert_str(blob["unavailable"]).contains("Stray")
