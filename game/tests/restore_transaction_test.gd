# gdlint:ignore = max-public-methods
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


func _f32_bits(value: float) -> String:
	var bytes := PackedByteArray()
	bytes.resize(4)
	bytes.encode_float(0, value)
	return bytes.hex_encode()


func _uncaptured_rotation_bits(main: UnseeingGame, cat: WaveCat) -> Array[String]:
	return [
		_f32_bits(main.player.rotation.x),
		_f32_bits(main.player.rotation.z),
		_f32_bits(main.player.camera.rotation.y),
		_f32_bits(main.player.camera.rotation.z),
		_f32_bits(cat.global_rotation.x),
		_f32_bits(cat.global_rotation.z),
	]


## Replace the artifact label after a test deliberately changes one value.
## This is syntax hashing only: semantic restore validation would make every
## transaction case circular by refusing the fixture while it is built.
func _install_canonical_hash(main: UnseeingGame, blob: Dictionary) -> void:
	var diagnostic: Dictionary = main.observer.canonical_hash_of(blob)
	assert_str(str(diagnostic.get("unavailable", ""))).is_empty()
	if diagnostic.has("hash"):
		blob["hash"] = diagnostic["hash"]


func _set_airborne_motion(
	hero: Dictionary, planar_x: String, vertical: String, planar_z: String
) -> void:
	hero["velocity"] = [planar_x, vertical, planar_z]
	var motion: Dictionary = hero["motion"]
	motion["phase"] = {
		"kind": "airborne",
		"planar_velocity": [planar_x, planar_z],
		"vertical_velocity": vertical,
	}
	motion["support"] = null


func _set_controlled_support(hero: Dictionary) -> void:
	var motion: Dictionary = hero["motion"]
	motion["phase"] = {"kind": "controlled"}
	motion["support"] = {
		"point": ["2.0", "0.0", "-3.0"],
		"normal": ["0.0", "1.0", "0.0"],
	}
	motion["last_landing"] = null


func _assert_atomic_motion_refusal(main: UnseeingGame, blob: Dictionary, expected: String) -> void:
	_queue_one(main, Vector3(-8.25, 0.5, 7.125))
	var before_env: Dictionary = main.capture_env()
	var before: Dictionary = main.observer.capture(main.now, before_env)
	assert_bool(before.has("unavailable")).is_false()
	var before_transform := main.player.global_transform
	var before_velocity := main.player.velocity
	main.player.collision_layer = 8
	main.player.collision_mask = 16
	var holder: Array[Dictionary] = [{}]
	var invoke := func() -> void: holder[0] = main.restore_blob(blob)
	await assert_error(invoke).is_success()
	assert_bool(holder[0].has("unavailable")).is_true()
	assert_str(str(holder[0].get("unavailable", ""))).contains(expected)
	assert_bool(main.player.global_transform == before_transform).is_true()
	assert_vector(main.player.velocity).is_equal(before_velocity)
	assert_int(main.player.collision_layer).is_equal(8)
	assert_int(main.player.collision_mask).is_equal(16)
	assert_dict(main.capture_env()).is_equal(before_env)
	var after: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(after.has("unavailable")).is_false()
	assert_str(after["hash"]).is_equal(before["hash"])


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
	assert_str(str(verdict.get("unavailable", ""))).contains(expected)
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


func _assert_atomic_graph_refusal(
	main: UnseeingGame,
	blob: Dictionary,
	expected: String,
	damage: Callable,
	repair: Callable,
	restore_target: UnseeingGame,
) -> void:
	_queue_one(main, Vector3(-6.75, 0.425, 7.875))
	await get_tree().physics_frame
	var was_paused := get_tree().paused
	get_tree().paused = true
	var before_env: Dictionary = main.capture_env()
	var before: Dictionary = main.observer.capture(main.now, before_env)
	assert_bool(before.has("unavailable")).is_false()
	var before_position := main.player.global_position
	var before_rotation := main.player.rotation
	damage.call()
	var holder: Array[Dictionary] = [{}]
	var invoke := func() -> void: holder[0] = restore_target.restore_blob(blob)
	await assert_error(invoke).is_success()
	repair.call()
	assert_bool(holder[0].has("unavailable")).is_true()
	assert_str(str(holder[0].get("unavailable", ""))).contains(expected)
	assert_dict(main.capture_env()).is_equal(before_env)
	assert_vector(main.player.global_position).is_equal(before_position)
	assert_vector(main.player.rotation).is_equal(before_rotation)
	var after: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(after.has("unavailable")).is_false()
	assert_str(after["hash"]).is_equal(before["hash"])
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
	var before_wrapping_kind: Dictionary = main.observer.capture(main.now, main.capture_env())
	var wrapping_kind := func() -> void:
		main.wave_core.emit(2147483648, Vector3.ZERO, 6.0, 5.5, 1.0, main.now, Vector3.ZERO, -2.0)
	await assert_error(wrapping_kind).is_push_error(
		"Pulses.emit: field type: must fit the pulse kind lane — wave refused"
	)
	var after_wrapping_kind: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_str(after_wrapping_kind["hash"]).is_equal(before_wrapping_kind["hash"])
	await _lively(main)
	var cats := main.cats()
	assert_int(cats.size()).is_greater(0)
	var cat: WaveCat = cats[0]
	# Exact known YXZ image of (0.25, -0.5, 0.125). Player body and eye
	# exercise all four omitted local lanes; the cat keeps its brain-owned Y
	# while adding the two omitted global lanes.
	var complete_rotation := Vector3(0.25, -0.5000000596046448, 0.125)
	main.player.rotation = complete_rotation
	main.player.camera.rotation = complete_rotation
	var cat_rotation := cat.global_rotation
	cat_rotation.x = 0.25
	cat_rotation.z = 0.125
	cat.global_rotation = cat_rotation
	var expected_rotation_config := _uncaptured_rotation_bits(main, cat)
	var target_env: Dictionary = main.capture_env()
	target_env["demo_checked"] = true
	target_env["demo_armed"] = false
	target_env["demo_next"] = 13.25
	target_env["flicker_t"] = 12.5
	target_env["flicker_level"] = 0.9
	target_env["flicker_drop_until"] = -0.75
	target_env["flicker_next_drop"] = 8.75
	target_env["flicker_rng_state"] = 1234567
	main.apply_env(target_env)
	target_env = main.capture_env()
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(blob.has("unavailable")).is_false()
	# a blob of an empty world would round-trip perfectly and prove nothing:
	# these pin that there is something in it to get wrong — waves in the
	# air, reflections still owed, and a cat with a life behind it
	assert_int(main.observer.snapshot(main.now)["live_slots"]).is_greater(0)
	assert_int((blob["echoes"] as Array).size()).is_greater(0)
	assert_int((blob["hero"]["queued_waves"] as Array).size()).is_greater(0)
	await _one_frame()
	var expected_rotation_future := _uncaptured_rotation_bits(main, cat)
	main.player.look(Vector2(175.0, -75.0))
	for _i in 30:
		await _one_frame()
	# The omitted lanes are live configuration at restore time. Re-establish
	# compatible canonical decoys after ordinary look/yaw evolution, while
	# changing only the lanes the artifact owns.
	main.player.rotation = Vector3(0.25, 0.5, 0.125)
	main.player.camera.rotation = Vector3(-0.25, -0.5000000596046448, 0.125)
	cat_rotation = cat.global_rotation
	cat_rotation.x = 0.25
	cat_rotation.z = 0.125
	cat.global_rotation = cat_rotation
	assert_array(_uncaptured_rotation_bits(main, cat)).is_equal(expected_rotation_config)
	var changed: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(changed.has("unavailable")).is_false()
	assert_str((changed["hero"] as Dictionary)["yaw"]).is_not_equal(
		(blob["hero"] as Dictionary)["yaw"]
	)
	assert_str((changed["hero"] as Dictionary)["pitch"]).is_not_equal(
		(blob["hero"] as Dictionary)["pitch"]
	)
	var decoy_env := target_env.duplicate(true)
	var target_now: float = target_env["now"]
	decoy_env["now"] = target_now + 0.125
	decoy_env["demo_checked"] = false
	decoy_env["demo_armed"] = true
	decoy_env["demo_next"] = 14.25
	decoy_env["flicker_t"] = 13.5
	decoy_env["flicker_level"] = 0.8
	decoy_env["flicker_drop_until"] = 0.25
	decoy_env["flicker_next_drop"] = 7.75
	decoy_env["flicker_rng_state"] = 7654321
	for key: String in target_env:
		assert_bool(target_env[key] != decoy_env[key]).is_true()
	main.apply_env(decoy_env)
	assert_dict(main.capture_env()).is_equal(decoy_env)
	var verdict: Dictionary = main.restore_blob(blob)
	assert_str(str(verdict.get("unavailable", ""))).is_empty()
	assert_str(verdict["hash"]).is_equal(blob["hash"])
	assert_array(_uncaptured_rotation_bits(main, cat)).is_equal(expected_rotation_config)
	var fresh: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_str(str(fresh.get("unavailable", ""))).is_empty()
	assert_str(fresh["hash"]).is_equal(blob["hash"])
	assert_str(main.observer.blob_round_trip_ok(blob, fresh)).is_empty()
	await _one_frame()
	assert_array(_uncaptured_rotation_bits(main, cat)).is_equal(expected_rotation_future)


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


func test_cat_body_y_and_gait_support_y_mismatch_leaves_world_untouched() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob := _copy(main.observer.capture(main.now, main.capture_env()))
	var cat: Dictionary = (blob["cats"] as Array)[0]
	var gait: Dictionary = cat["gait"]
	var changed_support := str(str(gait["support_y"]).to_float() + 0.5)
	gait["support_y"] = changed_support
	for group_name: String in ["planted", "aim"]:
		for point: Array in gait[group_name] as Array:
			point[1] = changed_support
	_install_canonical_hash(main, blob)
	await _assert_atomic_refusal(main, blob, "cats[0].gait.support_y")


func test_cat_rect_rehash_stays_syntax_only_before_owner_preflight_refuses_it() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob := _copy(main.observer.capture(main.now, main.capture_env()))
	var brain: Dictionary = ((blob["cats"] as Array)[0] as Dictionary)["brain"]
	var rect: Dictionary = brain["rect"]
	rect["max_x"] = "1000000.25"
	_install_canonical_hash(main, blob)
	await _assert_atomic_refusal(main, blob, "cats[0].brain.rect.max_x")


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
	var cat_capture_holder: Array[Dictionary] = [{}]
	var capture_cat := func() -> void:
		cat_capture_holder[0] = cat_game.observer.capture(cat_game.now, cat_game.capture_env())
	await assert_error(capture_cat).is_success()
	assert_str(str(cat_capture_holder[0].get("unavailable", ""))).contains("cat")
	assert_str(str(cat_capture_holder[0].get("unavailable", ""))).contains("freed")
	var cat_holder: Array[Dictionary] = [{}]
	var invoke_cat := func() -> void: cat_holder[0] = cat_game.restore_blob(cat_blob)
	await assert_error(invoke_cat).is_success()
	assert_str(str(cat_holder[0].get("unavailable", ""))).contains("cat")
	assert_dict(cat_game.capture_env()).is_equal(cat_env)

	(sources[0] as Node3D).free()
	var source_capture_holder: Array[Dictionary] = [{}]
	var capture_source := func() -> void:
		source_capture_holder[0] = source_game.observer.capture(
			source_game.now, source_game.capture_env()
		)
	await assert_error(capture_source).is_success()
	assert_str(str(source_capture_holder[0].get("unavailable", ""))).contains("source")
	assert_str(str(source_capture_holder[0].get("unavailable", ""))).contains("freed")
	var source_holder: Array[Dictionary] = [{}]
	var invoke_source := func() -> void: source_holder[0] = source_game.restore_blob(source_blob)
	await assert_error(invoke_source).is_success()
	assert_str(str(source_holder[0].get("unavailable", ""))).contains("source")
	assert_dict(source_game.capture_env()).is_equal(source_env)
	cat_game.free()
	source_game.free()
	get_tree().paused = was_paused


func test_a_freed_observer_player_refuses_capture_before_any_handle_call() -> void:
	var main := await _boot_ticked()
	var before_env: Dictionary = main.capture_env()
	var was_paused := get_tree().paused
	get_tree().paused = true
	main.player.free()

	var holder: Array[Dictionary] = [{}]
	var capture := func() -> void: holder[0] = main.observer.capture(main.now, main.capture_env())
	await assert_error(capture).is_success()
	assert_str(str(holder[0].get("unavailable", ""))).contains("hero")
	assert_str(str(holder[0].get("unavailable", ""))).contains("freed")
	assert_dict(main.capture_env()).is_equal(before_env)

	main.free()
	get_tree().paused = was_paused


func test_capture_refuses_noncanonical_omitted_rotation_lanes_in_every_actor_route() -> void:
	var main := await _boot_ticked()
	# Each `.rotation =` assignment below is Godot's LOCAL euler setter, which
	# stores its argument VERBATIM with no trig and no Basis roundtrip
	# (docs/superpowers/specs/2026-08-28-deterministic-rotation-wire-design.md,
	# Decision 1's evidence). yaw_witness's Y lane (-2.6510882) sits inside the
	# wire law's closed domain [-PI_F32, PI_F32] (PI_F32 ~= 3.14159265), so it
	# is the OWNED yaw the read path narrows without complaint; its Z lane
	# (-3.7417357) has magnitude > PI_F32, so it is the OMITTED lane the read
	# path must still find non-canonical. Replacing omitted lanes with
	# synthetic zeroes would silently accept an artifact that exact restore
	# cannot reproduce.
	var yaw_witness := Vector3(-2.0493662, -2.6510882, -3.7417357)
	main.player.rotation = yaw_witness
	var refused: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_str(str(refused.get("unavailable", ""))).contains(
		"hero body does not preserve its configured X/Z rotation"
	)

	main.player.rotation = Vector3.ZERO
	var pitch_witness := Vector3(-4.5095935, -4.016193, 5.4389277)
	main.player.camera.rotation = pitch_witness
	refused = main.observer.capture(main.now, main.capture_env())
	assert_str(str(refused.get("unavailable", ""))).contains(
		"hero eye does not preserve its configured Y/Z rotation"
	)

	main.player.camera.rotation = Vector3.ZERO
	var cats := main.cats()
	assert_int(cats.size()).is_equal(1)
	var cat: WaveCat = cats[0]
	# The cat's rotation seam (Decision 2, same spec) stores and reads LOCAL
	# euler verbatim too, so `cat.rotation =` reproduces the hero body's
	# omitted-Z-lane fixture exactly. `cat.global_rotation =` would NOT: that
	# setter builds a real Basis from the given euler and marks the local
	# euler cache dirty, so the next `cat.rotation` read re-derives euler from
	# the basis via the ENGINE's own atan2/asin — and that decomposition
	# always lands X in [-pi/2, pi/2] and Y/Z in (-pi, pi], fully inside the
	# wire domain, so it would never exercise this refusal at all.
	cat.rotation = yaw_witness
	refused = main.observer.capture(main.now, main.capture_env())
	assert_str(str(refused.get("unavailable", ""))).contains(
		"cat body does not preserve its configured X/Z rotation"
	)


func test_a_freed_root_restorer_refuses_before_clone_or_world_write() -> void:
	var main := await _boot_ticked()
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var before_env: Dictionary = main.capture_env()
	var was_paused := get_tree().paused
	get_tree().paused = true
	main.restorer.free()

	var holder: Array[Dictionary] = [{}]
	var restore := func() -> void: holder[0] = main.restore_blob(blob)
	await assert_error(restore).is_success()
	assert_str(str(holder[0].get("unavailable", ""))).contains("restorer")
	assert_str(str(holder[0].get("unavailable", ""))).contains("freed")
	assert_dict(main.capture_env()).is_equal(before_env)

	main.free()
	get_tree().paused = was_paused


func test_runtime_queue_uses_checked_wave_admission_before_append() -> void:
	var main := await _boot_ticked()
	var before: Dictionary = main.observer.capture(main.now, main.capture_env())
	var before_count: int = main.player.queued_waves().size()

	var queue := func() -> void:
		main.player.queue_wave(0, Vector3.ZERO, 1.7976931348623157e308, 5.5, 1.0, 0, Vector3.UP)
	await (assert_error(queue).is_push_error(
		(
			"UnseeingPlayer.queue_wave: field max_r: must narrow to a finite positive "
			+ "shader lane — wave refused"
		)
	))
	assert_int(main.player.queued_waves().size()).is_equal(before_count)
	var after: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_str(after["hash"]).is_equal(before["hash"])


func test_runtime_queue_refuses_invalid_reflection_geometry_before_append() -> void:
	for witness: Array in [
		[
			Vector3(NAN, 0.0, 0.0),
			(
				"UnseeingPlayer.queue_wave: field normal.x: reflection request refused: "
				+ "origin, normal, and derived fan geometry must remain finite in f32"
			)
		],
		[
			Vector3(3.4028234663852886e38, -3.4028234663852886e38, 3.4028234663852886e38),
			(
				"UnseeingPlayer.queue_wave: field normal: reflection request refused: "
				+ "origin, normal, and derived fan geometry must remain finite in f32"
			)
		]
	]:
		var bad_normal: Vector3 = witness[0]
		var main := await _boot_ticked()
		var before: Dictionary = main.observer.capture(main.now, main.capture_env())
		var before_count: int = main.player.queued_waves().size()
		var queue := func() -> void:
			main.player.queue_wave(0, Vector3.ZERO, 6.0, 5.5, 1.0, 6, bad_normal)
		await assert_error(queue).is_push_error(witness[1])
		assert_int(main.player.queued_waves().size()).is_equal(before_count)
		var after: Dictionary = main.observer.capture(main.now, main.capture_env())
		assert_str(after["hash"]).is_equal(before["hash"])
		main.free()


func test_restore_refuses_overflowing_reflection_normal_before_any_write() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob := _copy(main.observer.capture(main.now, main.capture_env()))
	var wave: Dictionary = ((blob["hero"] as Dictionary)["queued_waves"] as Array)[0]
	var normal: Array = wave["normal"]
	normal[0] = "3.4028234663852886e38"
	normal[1] = "-3.4028234663852886e38"
	normal[2] = "3.4028234663852886e38"
	_install_canonical_hash(main, blob)

	await _assert_atomic_refusal(main, blob, "hero.queued_waves[0].normal")


func test_prepared_restore_rejects_an_out_of_domain_slot_origin_before_writes() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob := _copy(main.observer.capture(main.now, main.capture_env()))
	var slot: Dictionary = (blob["slots"] as Array)[0]
	(slot["pos"] as Array)[0] = "3.4028234663852886e38"
	_install_canonical_hash(main, blob)

	await _assert_atomic_refusal(main, blob, "slots[0].pos.x")


func test_format_2_restore_installs_controlled_support_and_airborne_collision_pairs() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var controlled := _copy(main.observer.capture(main.now, main.capture_env()))
	assert_float(controlled["format_version"]).is_equal(2.0)
	_set_controlled_support(controlled["hero"] as Dictionary)
	_install_canonical_hash(main, controlled)
	main.player.collision_layer = 64
	main.player.collision_mask = 128
	var controlled_verdict: Dictionary = main.restore_blob(controlled)
	assert_str(str(controlled_verdict.get("unavailable", ""))).is_empty()
	assert_int(main.player.collision_layer).is_equal(2)
	assert_int(main.player.collision_mask).is_equal(4_294_967_291)
	assert_bool(main.player.call("support_collider_id") == null).is_true()

	var airborne := _copy(main.observer.capture(main.now, main.capture_env()))
	assert_float(airborne["format_version"]).is_equal(2.0)
	_set_airborne_motion(airborne["hero"] as Dictionary, "1.25", "-3.5", "-0.75")
	_install_canonical_hash(main, airborne)
	main.player.collision_layer = 64
	main.player.collision_mask = 128
	var airborne_verdict: Dictionary = main.restore_blob(airborne)
	assert_str(str(airborne_verdict.get("unavailable", ""))).is_empty()
	assert_int(main.player.collision_layer).is_equal(4)
	assert_int(main.player.collision_mask).is_equal(4_294_967_289)
	assert_bool(main.player.call("support_collider_id") == null).is_true()
	var restored := main.observer.capture(main.now, main.capture_env())
	assert_str(restored["hash"]).is_equal(airborne["hash"])


func test_airborne_restore_planar_mismatch_is_atomic_for_both_axes() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	for mismatch: Array in [["1.5", "-0.75", "x"], ["1.25", "-0.5", "z"]]:
		var blob := _copy(main.observer.capture(main.now, main.capture_env()))
		var hero: Dictionary = blob["hero"]
		_set_airborne_motion(hero, "1.25", "-3.5", "-0.75")
		var phase: Dictionary = (hero["motion"] as Dictionary)["phase"]
		phase["planar_velocity"] = [mismatch[0], mismatch[1]]
		_install_canonical_hash(main, blob)
		await _assert_atomic_motion_refusal(
			main, blob, "hero.motion.phase.planar_velocity.%s" % mismatch[2]
		)


func test_airborne_restore_uses_injected_terminal_config_and_refuses_excess_atomically() -> void:
	var main: UnseeingGame = auto_free(
		WORLD_FIXTURE.game(WORLD_FIXTURE.DEFAULT_EXTENTS, true, true, true)
	)
	main.player_terminal_fall_speed = 6.0
	add_child(main)
	main.level.scene_file_path = FIXTURE_SCENE_PATH
	await _one_frame()
	await _lively(main)
	var edge := _copy(main.observer.capture(main.now, main.capture_env()))
	_set_airborne_motion(edge["hero"] as Dictionary, "1.25", "-6.0", "-0.75")
	_install_canonical_hash(main, edge)
	var edge_verdict: Dictionary = main.restore_blob(edge)
	assert_str(str(edge_verdict.get("unavailable", ""))).is_empty()
	assert_int(main.player.collision_layer).is_equal(4)
	assert_int(main.player.collision_mask).is_equal(4_294_967_289)

	# Root properties are construction-time staging. Changing one afterward
	# cannot rewrite the checked config already owned by the live Player.
	main.player_terminal_fall_speed = 50.0
	var excess := _copy(main.observer.capture(main.now, main.capture_env()))
	_set_airborne_motion(excess["hero"] as Dictionary, "1.25", "-6.000000476837158", "-0.75")
	_install_canonical_hash(main, excess)
	await _assert_atomic_motion_refusal(main, excess, "hero.motion.phase.vertical_velocity")


## A quiet booted world — a wall for reflections but no source and no
## cat, so the pool only ever holds what the hero itself makes and a
## landing or footstep re-emission cannot hide in ambient waves.
func _boot_quiet() -> UnseeingGame:
	var main: UnseeingGame = auto_free(
		WORLD_FIXTURE.game(WORLD_FIXTURE.DEFAULT_EXTENTS, true, false, false)
	)
	add_child(main)
	main.level.scene_file_path = FIXTURE_SCENE_PATH
	await _one_frame()
	return main


## Live pulses whose max radius matches exactly — the discriminator
## between footstep (1.6), a queued fixture wave (6.25), and any landing.
func _pulse_range_count(main: UnseeingGame, radius: float) -> int:
	var count := 0
	var core: WaveCore = main.wave_core
	for i: int in core.live_count(main.now):
		if absf(core.pulse_data()[i].y - radius) < 1e-6:
			count += 1
	return count


## The hero's captured suppression latch, read through the wire format.
func _hero_pending(main: UnseeingGame) -> bool:
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(blob.has("unavailable")).is_false()
	return (blob["hero"] as Dictionary)["footstep_suppression_pending"]


## Format 2 restores a PENDING suppression latch verbatim, and only a real
## hero cadence evaluation may spend it: physics ticks alone never do, and
## the swallowed first step emits no footstep wave.
func test_format_2_restore_accepts_pending_suppression_until_hero_ack() -> void:
	var main := await _boot_quiet()
	await _lively(main)
	var pending := _copy(main.observer.capture(main.now, main.capture_env()))
	(pending["hero"] as Dictionary)["footstep_suppression_pending"] = true
	_install_canonical_hash(main, pending)
	var verdict: Dictionary = main.restore_blob(pending)
	assert_str(str(verdict.get("unavailable", ""))).is_empty()
	assert_bool(_hero_pending(main)).is_true()
	for _i: int in 5:
		await _one_frame()
	assert_bool(_hero_pending(main)).is_true()  # ticks alone never acknowledge
	Input.action_press("move_forward")
	var acknowledged := false
	for i: int in 240:
		await _one_frame()
		if i % 5 == 4 and not _hero_pending(main):
			acknowledged = true
			break
	Input.action_release("move_forward")
	assert_bool(acknowledged).is_true()
	assert_int(_pulse_range_count(main, 1.6)).is_equal(0)  # the ack was silent


## Format 2 restores a controlled-contact queue entry with its future
## intact: a controlled tick emits it, and a departing tick consumes it
## silently — exactly the life the original request would have had.
func test_format_2_restore_preserves_controlled_contact_gate_and_future() -> void:
	var main := await _boot_quiet()
	await _lively(main)
	var gated := _copy(main.observer.capture(main.now, main.capture_env()))
	var wave: Dictionary = ((gated["hero"] as Dictionary)["queued_waves"] as Array)[0]
	wave["gate"] = "controlled_contact"
	_install_canonical_hash(main, gated)
	var verdict: Dictionary = main.restore_blob(gated)
	assert_str(str(verdict.get("unavailable", ""))).is_empty()
	var queued := main.player.queued_waves()
	assert_int(queued.size()).is_equal(1)
	var gate: String = queued[0].gate
	assert_str(gate).is_equal("controlled_contact")
	# future A: controlled -> controlled, the restored request still fires
	assert_int(_pulse_range_count(main, 6.25)).is_equal(0)
	for _i: int in 2:
		await _one_frame()
	assert_array(main.player.queued_waves()).is_empty()
	assert_int(_pulse_range_count(main, 6.25)).is_equal(1)
	# future B: the same restored gate is consumed silently when the very
	# next tick leaves support
	verdict = main.restore_blob(gated)
	assert_str(str(verdict.get("unavailable", ""))).is_empty()
	var relocated: Dictionary = main.player.call("relocate", Vector3(2.0, 5.0, 6.0))
	assert_dict(relocated).is_equal({"relocated": true})
	for _i: int in 2:
		await _one_frame()
	assert_array(main.player.queued_waves()).is_empty()
	assert_int(_pulse_range_count(main, 6.25)).is_equal(0)


## A restored `last_landing` is memory, not an event: nothing re-emits its
## voice and the suppression latch stays clear.
func test_restored_old_landing_is_inert() -> void:
	var main := await _boot_quiet()
	await _lively(main)
	var blob := _copy(main.observer.capture(main.now, main.capture_env()))
	var hero: Dictionary = blob["hero"]
	_set_controlled_support(hero)
	var motion: Dictionary = hero["motion"]
	motion["last_landing"] = {
		"impact_speed": "3.5",
		"support": {"point": ["2.0", "0.0", "-3.0"], "normal": ["0.0", "1.0", "0.0"]},
	}
	_install_canonical_hash(main, blob)
	var verdict: Dictionary = main.restore_blob(blob)
	assert_str(str(verdict.get("unavailable", ""))).is_empty()
	for _i: int in 5:
		await _one_frame()
	# a 3.5 m/s landing would speak at exactly severity 0.8: range 4.0,
	# kind 2 — no such pulse may exist, and the latch stays clear
	var relanded := 0
	var core: WaveCore = main.wave_core
	for i: int in core.live_count(main.now):
		var dat: Vector4 = core.pulse_data()[i]
		if int(floorf(dat.w / 10.0)) == 2 and dat.y == 4.0:
			relanded += 1
	assert_int(relanded).is_equal(0)
	assert_bool(_hero_pending(main)).is_false()


## A blob whose PLAYER group is thoroughly non-dormant — pending latch,
## controlled-contact queue entry — still restores atomically: when a
## LATER group refuses, not one prepared player value lands. The poison
## sits in the cat group, which the restorer prepares AFTER the hero
## (waves, then hero, then cats, then sources), so the non-dormant player
## preparation genuinely runs and must still leave the world untouched.
func test_non_dormant_player_preparation_is_still_atomic_when_a_later_group_refuses() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob := _copy(main.observer.capture(main.now, main.capture_env()))
	var hero: Dictionary = blob["hero"]
	hero["footstep_suppression_pending"] = true
	var wave: Dictionary = (hero["queued_waves"] as Array)[0]
	wave["gate"] = "controlled_contact"
	var brain: Dictionary = ((blob["cats"] as Array)[0] as Dictionary)["brain"]
	brain["speed"] = "NaN"
	_install_canonical_hash(main, blob)
	await _assert_atomic_refusal(main, blob, "cats[0].brain.speed")


func _assert_semantically_poisoned_wave_state(main: UnseeingGame) -> void:
	var nonfinite_appointment := _copy(main.observer.capture(main.now, main.capture_env()))
	var appointment: Dictionary = (nonfinite_appointment["echoes"] as Array)[0]
	appointment["at_t"] = "Infinity"
	_install_canonical_hash(main, nonfinite_appointment)

	var zero_speed := _copy(main.observer.capture(main.now, main.capture_env()))
	var zero_slot: Dictionary = (zero_speed["slots"] as Array)[0]
	(zero_slot["dat"] as Array)[2] = "0.0"
	_install_canonical_hash(main, zero_speed)
	await _assert_atomic_refusal(main, zero_speed, "slots[0].dat.z")

	var kind_mismatch := _copy(main.observer.capture(main.now, main.capture_env()))
	var kind_slot: Dictionary = (kind_mismatch["slots"] as Array)[0]
	var old_kind: int = kind_slot["kind"] as int
	kind_slot["kind"] = old_kind + 1
	_install_canonical_hash(main, kind_mismatch)
	await _assert_atomic_refusal(main, kind_mismatch, "slots[0].kind")

	var forged_end := _copy(main.observer.capture(main.now, main.capture_env()))
	var end_slot: Dictionary = (forged_end["slots"] as Array)[0]
	end_slot["end"] = str(str(end_slot["end"]).to_float() + 1000.0)
	_install_canonical_hash(main, forged_end)
	await _assert_atomic_refusal(main, forged_end, "slots[0].end")

	_queue_one(main, Vector3(2.125, 0.375, -1.625))
	var huge_range := _copy(main.observer.capture(main.now, main.capture_env()))
	var huge_wave: Dictionary = ((huge_range["hero"] as Dictionary)["queued_waves"] as Array)[0]
	huge_wave["max_r"] = "1.7976931348623157e308"
	_install_canonical_hash(main, huge_range)
	await _assert_atomic_refusal(main, huge_range, "hero.queued_waves[0].max_r")

	_queue_one(main, Vector3(-2.375, 0.625, 1.875))
	var tiny_speed := _copy(main.observer.capture(main.now, main.capture_env()))
	var tiny_wave: Dictionary = ((tiny_speed["hero"] as Dictionary)["queued_waves"] as Array)[0]
	tiny_wave["speed"] = "2.2250738585072014e-308"
	_install_canonical_hash(main, tiny_speed)
	await _assert_atomic_refusal(main, tiny_speed, "hero.queued_waves[0].speed")

	await _assert_atomic_refusal(main, nonfinite_appointment, "echoes[0].at_t")


func test_restore_refuses_noncanonical_actor_angles_and_unsafe_private_numbers_before_writes(
) -> void:
	var main := await _boot_ticked()
	await _lively(main)
	await _assert_semantically_poisoned_wave_state(main)
	var cases: Array[Array] = []

	var leg_phase := _copy(main.observer.capture(main.now, main.capture_env()))
	var poisoned_vm: Dictionary = (leg_phase["hero"] as Dictionary)["viewmodel"]
	poisoned_vm["leg_phase"] = "1.7976931348623157e308"
	cases.append([leg_phase, "hero.viewmodel.leg_phase"])

	var brain_poison := _copy(main.observer.capture(main.now, main.capture_env()))
	var poisoned_brain: Dictionary = ((brain_poison["cats"] as Array)[0] as Dictionary)["brain"]
	poisoned_brain["yaw"] = "1.7976931348623157e308"
	cases.append([brain_poison, "cats[0].brain.yaw"])

	var pose_poison := _copy(main.observer.capture(main.now, main.capture_env()))
	var poisoned_pose: Dictionary = ((pose_poison["cats"] as Array)[0] as Dictionary)["pose"]
	poisoned_pose["yaw"] = "1.7976931348623157e308"
	cases.append([pose_poison, "cats[0].pose.yaw"])

	var player_yaw := _copy(main.observer.capture(main.now, main.capture_env()))
	(player_yaw["hero"] as Dictionary)["yaw"] = "4.0"
	cases.append([player_yaw, "hero.yaw"])

	var cat_yaw := _copy(main.observer.capture(main.now, main.capture_env()))
	((cat_yaw["cats"] as Array)[0] as Dictionary)["yaw"] = "4.0"
	cases.append([cat_yaw, "cats[0].yaw"])

	var coordinated_cat_yaw := _copy(main.observer.capture(main.now, main.capture_env()))
	var coordinated_cat: Dictionary = (coordinated_cat_yaw["cats"] as Array)[0]
	coordinated_cat["yaw"] = "4.0"
	(coordinated_cat["brain"] as Dictionary)["yaw"] = "4.0"
	(coordinated_cat["pose"] as Dictionary)["yaw"] = "4.0"
	cases.append([coordinated_cat_yaw, "cats[0].yaw"])

	var unlinked_cat_body := _copy(main.observer.capture(main.now, main.capture_env()))
	var unlinked_cat: Dictionary = (unlinked_cat_body["cats"] as Array)[0]
	# This is the exact canonical f32 YXZ image of raw yaw 4.0, so the body
	# angle is valid by itself while remaining unrelated to this cat's brain.
	var other_canonical_yaw := "-2.2831852436065674"
	assert_str(str(unlinked_cat["yaw"])).is_not_equal(other_canonical_yaw)
	unlinked_cat["yaw"] = other_canonical_yaw
	cases.append([unlinked_cat_body, "cats[0].yaw"])

	var brain_yaw := _copy(main.observer.capture(main.now, main.capture_env()))
	(((brain_yaw["cats"] as Array)[0] as Dictionary)["brain"] as Dictionary)["yaw"] = "4.0"
	cases.append([brain_yaw, "cats[0].pose.yaw"])

	var pose_yaw := _copy(main.observer.capture(main.now, main.capture_env()))
	(((pose_yaw["cats"] as Array)[0] as Dictionary)["pose"] as Dictionary)["yaw"] = "4.0"
	cases.append([pose_yaw, "cats[0].pose.yaw"])

	for case: Array in cases:
		var blob: Dictionary = case[0]
		_install_canonical_hash(main, blob)
		await _assert_atomic_refusal(main, blob, str(case[1]))


func test_restore_refuses_future_taps_and_contradictory_cat_copies_before_writes() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var cases: Array[Array] = []

	var negative_tap := _copy(main.observer.capture(main.now, main.capture_env()))
	(negative_tap["hero"] as Dictionary)["last_tap"] = "-9.0"
	cases.append([negative_tap, "hero.last_tap"])

	var future_tap := _copy(main.observer.capture(main.now, main.capture_env()))
	var capture_now := str((future_tap["env"] as Dictionary)["now"]).to_float()
	(future_tap["hero"] as Dictionary)["last_tap"] = str(capture_now + 1.0)
	cases.append([future_tap, "hero.last_tap"])

	var pose_yaw := _copy(main.observer.capture(main.now, main.capture_env()))
	var yaw_pose: Dictionary = ((pose_yaw["cats"] as Array)[0] as Dictionary)["pose"]
	yaw_pose["yaw"] = str(str(yaw_pose["yaw"]).to_float() + 0.25)
	cases.append([pose_yaw, "cats[0].pose.yaw"])

	var pose_amp := _copy(main.observer.capture(main.now, main.capture_env()))
	var amp_pose: Dictionary = ((pose_amp["cats"] as Array)[0] as Dictionary)["pose"]
	var old_amp := str(amp_pose["amp"]).to_float()
	amp_pose["amp"] = str(0.75 if old_amp < 0.5 else 0.25)
	cases.append([pose_amp, "cats[0].pose.amp"])

	var pose_sit := _copy(main.observer.capture(main.now, main.capture_env()))
	var sit_pose: Dictionary = ((pose_sit["cats"] as Array)[0] as Dictionary)["pose"]
	var old_sit := str(sit_pose["sit"]).to_float()
	sit_pose["sit"] = str(0.75 if old_sit < 0.5 else 0.25)
	cases.append([pose_sit, "cats[0].pose.sit"])

	for case: Array in cases:
		var blob: Dictionary = case[0]
		_install_canonical_hash(main, blob)
		await _assert_atomic_refusal(main, blob, str(case[1]))


func test_restore_preflights_the_observers_exact_graph_live_eye_and_root_rng() -> void:
	var main := await _boot_ticked()
	var other := await _boot_ticked()
	await _lively(main)
	var blob := _copy(main.observer.capture(main.now, main.capture_env()))

	await _assert_atomic_graph_refusal(
		main,
		blob,
		"observer body",
		func() -> void: main.observer.inject_body(null),
		func() -> void: main.observer.inject_body(main.hero),
		main,
	)
	await _assert_atomic_graph_refusal(
		main,
		blob,
		"observer body",
		func() -> void: main.observer.inject_body(other.hero),
		func() -> void: main.observer.inject_body(main.hero),
		main,
	)
	await _assert_atomic_graph_refusal(
		main,
		blob,
		"observer hero",
		func() -> void: main.observer.inject_hero(other.player),
		func() -> void: main.observer.inject_hero(main.player),
		main,
	)
	await _assert_atomic_graph_refusal(
		main,
		blob,
		"observer level",
		func() -> void: main.observer.inject(other.level, other.player.camera),
		func() -> void: main.observer.inject(main.level, main.player.camera),
		main,
	)

	var shell: UnseeingGame = auto_free(UnseeingGame.new())
	shell.restorer = main.restorer
	await _assert_atomic_graph_refusal(
		main,
		blob,
		"RNG",
		func() -> void: pass,
		func() -> void: pass,
		shell,
	)

	var old_camera: Camera3D = main.player.camera
	var old_camera_position := old_camera.position
	var old_camera_rotation := old_camera.rotation
	var replacement := Camera3D.new()
	replacement.position = old_camera_position
	replacement.rotation = old_camera_rotation
	main.player.add_child(replacement)
	main.player.camera = replacement
	await _assert_atomic_graph_refusal(
		main,
		blob,
		"eye",
		func() -> void: replacement.free(),
		func() -> void: main.player.camera = old_camera,
		main,
	)
# gdlint:ignore = max-file-lines
