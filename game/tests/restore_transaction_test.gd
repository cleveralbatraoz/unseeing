extends GdUnitTestSuite
# gdlint:ignore = max-line-length
## The restore TRANSACTION against a code-built live world: a captured blob applied
## back to a running game, and the proof that the fit is exact.
##
## The read side's own suite is `restore_test.gd`; this one is its write-side
## twin, split off because a suite has a public-method ceiling and the read
## side had reached it.

const WORLD_FIXTURE := preload("res://tests/world_fixture.gd")
const FIXTURE_SCENE_PATH := "res://tests/fixtures/restore_transaction_level.tscn"


## A world that has actually run: sources hold appointments, the hero body
## has built its viewmodel, and every clock has a reading. Capture refuses an
## unticked world by design, so every capture test starts here.
##
## Duplicated from `restore_test.gd::_boot_ticked`, which is where this
## idiom lives — the two suites are one story told from two sides.
func _boot_ticked() -> UnseeingGame:
	# Every collaborator exercised below is explicit: one wall for reflections,
	# one source for appointment restore, and one cat for living-state restore.
	var main: UnseeingGame = auto_free(
		WORLD_FIXTURE.game(WORLD_FIXTURE.DEFAULT_EXTENTS, true, true, true)
	)
	add_child(main)
	# PackedScenes built in memory have no resource path. Give this fixture a
	# stable identity so the wrong-map refusal proves both non-empty names.
	main.level.scene_file_path = FIXTURE_SCENE_PATH
	# one real process frame so sources book appointments and the viewmodel
	# exists — capture refuses an unticked world by design
	await _one_frame()
	return main


## A world with a life behind it, and appointments that are NOT their own
## interval.
##
## A fresh gate books its first beat one interval out, so at boot every
## source's appointment EQUALS its cadence knob and a restore that mixed the
## two up would be invisible. Jumping the clock past the fixture source's first
## beat and letting one frame run spends it: the jumped-clock law buys exactly
## one wave per source and rebooks from NOW, so every
## appointment in the blob is a date nothing else in the scene can be mistaken
## for. The tap adds real reflections — echo appointments the book must carry.
##
## The hero looks DOWN before tapping, and that is not decoration: a cane
## swung at eye level in open air strikes nothing and makes no wave at all
## (the air-swish voice), so a tap taken standing level would leave the echo
## book empty and every field of it untested.
##
## The tap comes AFTER the jumped clock has landed, for the same kind of
## reason: the cane is voiced on the physics tick off the clock the hero was
## last handed, so a tap queued before the jump is dated before it, and its
## reflections are all overdue and drained by the time the blob is taken.
##
## The queued wave is LAST, with no frame after it, and that is the only
## moment it can be caught: the out-tray is emptied by every physics tick, so
## a wave asked for one frame earlier is a wave already in the pool. Nothing
## the hero does by itself in a headless run leaves one there — footsteps
## need a walker and the demo tap needs its env switch — so a blob taken
## without this carries an empty queue, and the restore's replay of it would
## be covered by nothing at all. The three float arguments are deliberately
## unequal, so a transposed pair cannot restore to the same numbers.
func _lively(main: UnseeingGame) -> void:
	main.now += 1.0
	await _one_frame()
	main.player.look(Vector2(0.0, 100.0))
	main.player.tap()
	for _i in 2:
		await _one_frame()
	_queue_one(main, Vector3(2.5, 0.5, 3.25))


## One wave asked for and not yet emitted — a sound that WILL happen, which
## is why a blob carries the out-tray at all. Distinct values in every lane.
func _queue_one(main: UnseeingGame, at: Vector3) -> void:
	main.player.queue_wave(2, at, 6.25, 5.5, 0.75, 3, Vector3.UP)


## One process frame and one physics frame — the pair every clock in the
## game needs to see a change. The composition root advances `now` in
## `process()` and hands it to the hero and the cats there; the cane, the
## footsteps and every reflection cast run on the PHYSICS tick, off the copy
## that frame left them.
##
## WARNING: `await physics_frame` after `await process_frame` spans a SECOND
## `process()` call — any guard testing behavior that must land in exactly
## one frame should await only `process_frame` directly. The snapshot pass
## and shader reads in `test_process_pushes_this_frames_new_source_waves_into_the_materials`
## both depend on this distinction.
func _one_frame() -> void:
	await get_tree().process_frame
	await get_tree().physics_frame


func _copy(blob: Dictionary) -> Dictionary:
	return JSON.parse_string(JSON.stringify(blob, "", true, true)) as Dictionary


## Replace the artifact label after a test deliberately changes one value.
## This is syntax hashing only: semantic restore validation would make every
## transaction case circular by refusing the fixture while it is built.
func _install_canonical_hash(main: UnseeingGame, blob: Dictionary) -> void:
	var diagnostic: Dictionary = main.observer.canonical_hash_of(blob)
	assert_str(str(diagnostic.get("unavailable", ""))).is_empty()
	if diagnostic.has("hash"):
		blob["hash"] = diagnostic["hash"]


## One invalid artifact against a deliberately changed live world. A full
## capture hash covers env, pool, actors, sources and every private pure owner;
## process flags are adjacent runtime state and are pinned separately. The
## error monitor also proves that refusal emits no repair warning.
func _assert_atomic_refusal(main: UnseeingGame, blob: Dictionary, expected: String) -> void:
	_queue_one(main, Vector3(8.125, 0.375, -7.25))
	await get_tree().physics_frame
	var was_paused := get_tree().paused
	get_tree().paused = true
	var before_env: Dictionary = main.capture_env()
	var before: Dictionary = main.observer.capture(main.now, before_env)
	assert_bool(before.has("unavailable")).is_false()
	var player_process := main.player.is_processing()
	var player_physics := main.player.is_physics_processing()
	var cat_process: Array[bool] = []
	var cat_physics: Array[bool] = []
	for cat: WaveCat in main.cats():
		cat_process.append(cat.is_processing())
		cat_physics.append(cat.is_physics_processing())

	var holder: Array[Dictionary] = [{}]
	var invoke := func() -> void: holder[0] = main.restore_blob(blob)
	await assert_error(invoke).is_success()
	var verdict: Dictionary = holder[0]
	assert_bool(verdict.has("unavailable")).is_true()
	assert_str(verdict["unavailable"]).contains(expected)
	assert_dict(main.capture_env()).is_equal(before_env)
	var after: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(after.has("unavailable")).is_false()
	assert_str(after["hash"]).is_equal(before["hash"])
	assert_bool(main.player.is_processing()).is_equal(player_process)
	assert_bool(main.player.is_physics_processing()).is_equal(player_physics)
	var cats := main.cats()
	for i: int in cats.size():
		assert_bool(cats[i].is_processing()).is_equal(cat_process[i])
		assert_bool(cats[i].is_physics_processing()).is_equal(cat_physics[i])
	get_tree().paused = was_paused


## The headline property: capture, restore, capture again, and the two blobs
## are the same instant — bit for bit, over every field the hash covers.
##
## The world is deliberately allowed to LIVE ON between the capture and the
## restore, because a restore into the world it was taken from proves almost
## nothing: every door could be a no-op and the hashes would still agree. Here
## the cat has wandered, the sources have beaten, echoes have fired and the
## pool has turned over — so a door that does not write lands on a field that
## has moved, and the proof names it.
func test_round_trip_capture_restore_capture_is_exact() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(blob.has("unavailable")).is_false()
	# a blob of an empty world would round-trip perfectly and prove nothing:
	# these pin that there is something in it to get wrong — waves in the
	# air, reflections still owed, and a cat with a life behind it
	assert_int(main.observer.snapshot(main.now)["live_slots"]).is_greater(0)
	assert_int((blob["echoes"] as Array).size()).is_greater(0)
	assert_int((blob["hero"]["queued_waves"] as Array).size()).is_greater(0)
	for _i in 30:
		await _one_frame()
	var verdict: Dictionary = main.restore_blob(blob)
	assert_str(str(verdict.get("unavailable", ""))).is_empty()
	assert_str(verdict["hash"]).is_equal(blob["hash"])


## The spurious-beat trap. A source's gate is re-pinned AFTER the clock lands,
## so the restored appointment stands; a gate left holding a stale date beats
## the moment the level is ticked and the world sounds a wave the original
## never made.
##
## The tick is driven by hand at the SAME instant the blob is dated at: the
## clock does not move, so nothing can expire and no appointment can
## legitimately fall due — a slot that appears was a spurious beat and nothing
## else.
func test_restore_repins_appointments_no_spurious_beat() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(blob.has("unavailable")).is_false()
	var live_before: int = main.observer.snapshot(main.now)["live_slots"]
	var verdict: Dictionary = main.restore_blob(blob)
	assert_str(str(verdict.get("unavailable", ""))).is_empty()
	main.level.tick_sources(main.now, main.player.camera.global_position)
	var live_after: int = main.observer.snapshot(main.now)["live_slots"]
	assert_int(live_after).is_equal(live_before)


## A blob from another format is refused whole, before a single field is
## written — the version is the first thing checked and the world is left
## exactly as it stood. The env group is moved too, because the env half is
## applied by the composition root BEFORE the engine half is asked: a
## transaction that refused after that half had landed would leave the clock
## a thousand seconds from the world it dates.
func test_a_wrong_version_refuses_before_touching_anything() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	blob["format_version"] = 999
	(blob["env"] as Dictionary)["now"] = "999.0"
	_install_canonical_hash(main, blob)
	var before: Dictionary = main.observer.snapshot(main.now)
	var clock: float = main.now
	var verdict: Dictionary = main.restore_blob(blob)
	assert_bool(verdict.has("unavailable")).is_true()
	# the version, named as a version — "999" alone would also match the env
	# field this test moves, and would pass for a refusal about the clock
	assert_str(verdict["unavailable"]).contains("capture format 999")
	assert_float(main.now).is_equal(clock)
	var after: Dictionary = main.observer.snapshot(main.now)
	assert_int(after["live_slots"]).is_equal(before["live_slots"])


## The other header fact: a blob restores into the map it was taken from and
## no other. Both sides are named — "wrong level" without the two paths
## leaves the reader to work out which of them is the one they have.
func test_a_blob_from_another_map_names_both_scenes() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var here: String = blob["level_scene"]
	assert_str(here).is_equal(FIXTURE_SCENE_PATH)
	blob["level_scene"] = "res://scenes/somewhere_else.tscn"
	_install_canonical_hash(main, blob)
	var verdict: Dictionary = main.restore_blob(blob)
	assert_bool(verdict.has("unavailable")).is_true()
	assert_str(verdict["unavailable"]).contains("res://scenes/somewhere_else.tscn")
	assert_str(verdict["unavailable"]).contains(here)


## The stored hash is artifact validation, so preflight checks it before any
## write and names both the label and the independently computed canonical
## value. A file edited in transit never becomes the running world first.
func test_a_blob_that_lies_about_its_own_hash_is_refused() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var honest: String = blob["hash"]
	blob["hash"] = "0000000000000000"
	var verdict: Dictionary = main.restore_blob(blob)
	assert_bool(verdict.has("unavailable")).is_true()
	assert_str(verdict["unavailable"]).contains("stored 0000000000000000")
	assert_str(verdict["unavailable"]).contains("canonical %s" % honest)


## A semantic artifact failure is named by complete preflight before the
## engine's narrowing or clamping doors ever see it. The diagnostic hash is
## deliberately recomputed so this case reaches pitch capability validation
## independently of the stale-hash guard above.
func test_a_tampered_blob_is_named_at_its_divergent_field() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	(blob["hero"] as Dictionary)["pitch"] = "99.0"
	_install_canonical_hash(main, blob)
	var verdict: Dictionary = main.restore_blob(blob)
	assert_bool(verdict.has("unavailable")).is_true()
	assert_str(verdict["unavailable"]).contains("hero.pitch")


## The hero's two out-trays are state like any other, and both travel BOTH
## ways — a restore has to be able to REMOVE intent, not only add it.
##
## A tap and a wave are asked for after the capture is taken, so the live
## world holds more than the blob does. `tap()` can only ever set the flag,
## so a restore using it as its one door could never clear one and would
## refuse itself at `hero.tap_queued`; and a wave queue rebuilt by appending
## would end up holding the stale request AND the captured one, which the
## proof reads as a queue of the wrong length.
func test_the_heros_out_trays_lose_what_the_blob_never_held() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(blob["hero"]["tap_queued"]).is_false()
	var carried: int = (blob["hero"]["queued_waves"] as Array).size()
	assert_int(carried).is_greater(0)
	main.player.tap()
	_queue_one(main, Vector3(9.5, 0.25, 8.75))
	assert_int(main.player.queued_waves().size()).is_equal(carried + 1)
	var verdict: Dictionary = main.restore_blob(blob)
	assert_str(str(verdict.get("unavailable", ""))).is_empty()
	assert_str(verdict["hash"]).is_equal(blob["hash"])
	assert_int(main.player.queued_waves().size()).is_equal(carried)


## `restore_blob` forces the tree paused FOR the transaction and restores
## whatever the tree's own pause state was before it — never unconditionally
## unpausing on the way out. A restore run from the settings overlay (which
## pauses the world to open) must not quietly resume gameplay underneath it.
##
## The save/restore pair has two sides; this pins the TRUE one — the tree
## was already paused going in, and must still be paused coming out.
func test_restore_preserves_the_trees_paused_state() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(blob.has("unavailable")).is_false()
	get_tree().paused = true
	var verdict: Dictionary = main.restore_blob(blob)
	assert_str(str(verdict.get("unavailable", ""))).is_empty()
	assert_bool(get_tree().paused).is_true()
	get_tree().paused = false


func test_invalid_environment_only_leaves_env_pool_actors_sources_and_warning_state_untouched(
) -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob := _copy(main.observer.capture(main.now, main.capture_env()))
	(blob["env"] as Dictionary)["now"] = "-1.0"
	_install_canonical_hash(main, blob)
	await _assert_atomic_refusal(main, blob, "env.now")


func test_wrong_or_malformed_stored_hash_only_leaves_world_untouched() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var honest := _copy(main.observer.capture(main.now, main.capture_env()))
	var wrong := _copy(honest)
	wrong["hash"] = "0000000000000000"
	await _assert_atomic_refusal(main, wrong, "stored hash")
	var malformed := _copy(honest)
	malformed["hash"] = "ABC"
	await _assert_atomic_refusal(main, malformed, "hash")


func test_clamped_hero_pitch_and_lossy_hero_yaw_refuse_before_writes() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob := _copy(main.observer.capture(main.now, main.capture_env()))
	(blob["hero"] as Dictionary)["pitch"] = "99.0"
	_install_canonical_hash(main, blob)
	await _assert_atomic_refusal(main, blob, "hero.pitch")
	var yaw_blob := _copy(main.observer.capture(main.now, main.capture_env()))
	(yaw_blob["hero"] as Dictionary)["yaw"] = "0.1"
	_install_canonical_hash(main, yaw_blob)
	await _assert_atomic_refusal(main, yaw_blob, "hero.yaw")


func test_lossy_cat_yaw_and_poisoned_brain_refuse_before_writes() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var yaw_blob := _copy(main.observer.capture(main.now, main.capture_env()))
	((yaw_blob["cats"] as Array)[0] as Dictionary)["yaw"] = "0.1"
	_install_canonical_hash(main, yaw_blob)
	await _assert_atomic_refusal(main, yaw_blob, "cats[0].yaw")
	var brain_blob := _copy(main.observer.capture(main.now, main.capture_env()))
	var brain: Dictionary = ((brain_blob["cats"] as Array)[0] as Dictionary)["brain"]
	brain["speed"] = "NaN"
	_install_canonical_hash(main, brain_blob)
	await _assert_atomic_refusal(main, brain_blob, "cats[0].brain.speed")


func test_cat_pose_position_mismatch_on_x_or_y_or_z_refuses_before_writes() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	for axis: int in 3:
		var blob := _copy(main.observer.capture(main.now, main.capture_env()))
		var cat: Dictionary = (blob["cats"] as Array)[0]
		var pose: Dictionary = cat["pose"]
		var pos: Array = pose["pos"]
		pos[axis] = str(str(pos[axis]).to_float() + 0.25)
		_install_canonical_hash(main, blob)
		await _assert_atomic_refusal(main, blob, "cats[0].pose.pos")


func test_cat_gait_internal_support_mismatch_refuses_before_writes() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob := _copy(main.observer.capture(main.now, main.capture_env()))
	var gait: Dictionary = ((blob["cats"] as Array)[0] as Dictionary)["gait"]
	var planted: Array = gait["planted"]
	var first: Array = planted[0]
	first[1] = str(str(first[1]).to_float() + 0.5)
	_install_canonical_hash(main, blob)
	await _assert_atomic_refusal(main, blob, "cats[0].gait.planted[0].y")


func test_disabled_runtime_actor_refuses_capture() -> void:
	var main := await _boot_ticked()
	main.player.set_physics_process(false)
	var player_blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_str(player_blob["unavailable"]).contains("hero")
	assert_str(player_blob["unavailable"]).contains("disabled")

	var other := await _boot_ticked()
	var cats := other.cats()
	assert_bool(cats.is_empty()).is_false()
	if cats.is_empty():
		return
	(cats[0] as WaveCat).set_physics_process(false)
	var cat_blob: Dictionary = other.observer.capture(other.now, other.capture_env())
	assert_str(cat_blob["unavailable"]).contains("cat")
	assert_str(cat_blob["unavailable"]).contains("disabled")


func test_freed_cat_or_source_handle_refuses_before_environment_or_warning_write() -> void:
	var cat_game := await _boot_ticked()
	await _lively(cat_game)
	var cats := cat_game.cats()
	assert_bool(cats.is_empty()).is_false()
	if cats.is_empty():
		return

	var source_game := await _boot_ticked()
	await _lively(source_game)
	var source_blob: Dictionary = source_game.observer.capture(
		source_game.now, source_game.capture_env()
	)
	var source_env: Dictionary = source_game.capture_env()
	var sources: Array = source_game.level.sources()
	assert_bool(sources.is_empty()).is_false()
	if sources.is_empty():
		return
	# Building the second fixture advances the first one too. Take both of the
	# first fixture's observations only after every awaited setup step.
	var cat_blob: Dictionary = cat_game.observer.capture(cat_game.now, cat_game.capture_env())
	var cat_env: Dictionary = cat_game.capture_env()

	# A deliberately freed cached child also makes the world's ordinary process
	# census unusable. Hold the same pause bracket restore itself promises while
	# exercising the read-only refusal, then destroy both malformed fixtures
	# before thawing the shared test tree.
	var was_paused := get_tree().paused
	get_tree().paused = true
	(cats[0] as WaveCat).free()
	var cat_holder: Array[Dictionary] = [{}]
	var invoke_cat := func() -> void: cat_holder[0] = cat_game.restore_blob(cat_blob)
	await assert_error(invoke_cat).is_success()
	assert_str(str(cat_holder[0].get("unavailable", ""))).contains("cat")
	assert_dict(cat_game.capture_env()).is_equal(cat_env)

	(sources[0] as Node3D).free()
	var source_holder: Array[Dictionary] = [{}]
	var invoke_source := func() -> void: source_holder[0] = source_game.restore_blob(source_blob)
	await assert_error(invoke_source).is_success()
	assert_str(str(source_holder[0].get("unavailable", ""))).contains("source")
	assert_dict(source_game.capture_env()).is_equal(source_env)
	cat_game.free()
	source_game.free()
	get_tree().paused = was_paused


func test_dormant_schema_refuses_airborne_pending_or_controlled_contact_state() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var airborne := _copy(main.observer.capture(main.now, main.capture_env()))
	var motion: Dictionary = (airborne["hero"] as Dictionary)["motion"]
	motion["phase"] = {
		"kind": "airborne", "planar_velocity": ["0.0", "0.0"], "vertical_velocity": "-1.0"
	}
	motion["support"] = null
	_install_canonical_hash(main, airborne)
	await _assert_atomic_refusal(main, airborne, "hero.motion")

	var pending := _copy(main.observer.capture(main.now, main.capture_env()))
	(pending["hero"] as Dictionary)["footstep_suppression_pending"] = true
	_install_canonical_hash(main, pending)
	await _assert_atomic_refusal(main, pending, "hero.footstep_suppression_pending")

	_queue_one(main, Vector3(4.25, 0.625, -2.75))
	var gated := _copy(main.observer.capture(main.now, main.capture_env()))
	var wave: Dictionary = ((gated["hero"] as Dictionary)["queued_waves"] as Array)[0]
	wave["gate"] = "controlled_contact"
	_install_canonical_hash(main, gated)
	await _assert_atomic_refusal(main, gated, "hero.queued_waves[0].gate")
