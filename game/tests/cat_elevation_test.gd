# gdlint:ignore = max-public-methods
extends GdUnitTestSuite
## The cat's own physical boundary: elevated support, the two-phase airborne
## law, per-cat validated fall/landing configuration, and voices that follow
## the root wherever it stands — the same laws Task 2 proved for the player,
## reproduced here as the cat's own port, adapter and scene contract.

const WORLD_FIXTURE := preload("res://tests/world_fixture.gd")
const ELEVATION_FIXTURE := preload("res://tests/character_elevation_fixture.gd")

const DT := 1.0 / 60.0
const COLLIDER_CENTER_Y := 0.17  # COL_HEIGHT * 0.5, rust/src/nodes/cat.rs

const CAT_CONFIG_FIELDS: Array[String] = [
	"fall_acceleration",
	"terminal_fall_speed",
	"landing_silent_speed",
	"landing_full_speed",
	"landing_max_gain",
	"landing_max_range",
]
const CAT_CONFIG_DEFAULTS := [9.8, 20.0, 1.5, 4.0, 0.60, 2.5]


func _add_cat_direct(at: Vector3, seed := 7, roam_size := Vector2(6.0, 6.0)) -> WaveCat:
	var cat: WaveCat = auto_free(WaveCat.new())
	cat.pulses = Pulses.new()
	cat.data_mat = ShaderMaterial.new()
	cat.position = at
	cat.seed = seed
	cat.roam_size = roam_size
	add_child(cat)
	return cat


func _found_paw_voice(pulses: Pulses, now: float) -> bool:
	for i: int in pulses.live_count(now):
		var d := pulses.dat[i]
		if is_equal_approx(d.y, WaveCat.paw_range()):
			return true
	return false


func _paw_voice_count(pulses: Pulses, now: float) -> int:
	var count := 0
	for i: int in pulses.live_count(now):
		if is_equal_approx(pulses.dat[i].y, WaveCat.paw_range()):
			count += 1
	return count


## The runtime collider bottom meets the same authored flat datum as the
## player's: `position.y + collider.position.y - capsule.height * 0.5 == 0`.
## Paws and the baked skin ride the same floor, one silhouette, one root.
func test_floor_cat_keeps_root_collider_paws_and_skin_together() -> void:
	var cat := _add_cat_direct(Vector3(0.0, 0.0, 0.0))
	var collisions := cat.find_children("*", "CollisionShape3D", false, false)
	assert_int(collisions.size()).is_equal(1)
	var collision := collisions[0] as CollisionShape3D
	assert_object(collision).is_not_null()
	var capsule := collision.shape as CapsuleShape3D
	assert_object(capsule).is_not_null()
	# player_elevation_test.gd's equivalent capsule-datum check holds this
	# same plain 1.0e-7 tolerance at this same magnitude (~0.17); an actual
	# hand-derived single f32 ULP there is ~1.49e-8 (binade [0.125, 0.25)),
	# tighter than this convention allows for cross-language literal
	# rounding between the Rust-computed collider position and this
	# GDScript-authored expected value.
	assert_float(collision.position.y).is_equal_approx(COLLIDER_CENTER_Y, 1.0e-7)
	assert_float(cat.position.y + collision.position.y - capsule.height * 0.5).is_equal_approx(
		0.0, 1.0e-7
	)
	await get_tree().physics_frame
	await get_tree().process_frame
	assert_int(cat.cat_mesh().get_surface_count()).is_equal(1)
	for paw: Vector3 in cat.paw_positions():
		assert_float(paw.y).is_between(-0.001, 0.05)


## A cat dropped onto a table settles on its top, exactly the table's
## authored elevation, back to controlled ground contact.
func test_stationary_cat_stands_on_table_support() -> void:
	auto_free(ELEVATION_FIXTURE.add_table(self, Vector3.ZERO))
	var cat := _add_cat_direct(Vector3(0.0, 3.0, 0.0))
	var landed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 2 and cat.is_on_floor(), 240
	)
	assert_bool(landed).is_true()
	for _tick: int in 10:
		await get_tree().physics_frame
	assert_float(cat.global_position.y).is_equal_approx(
		ELEVATION_FIXTURE.TABLE_TOP_Y, 0.0010001192092895508
	)
	assert_int(cat.collision_layer).is_equal(2)
	# Task 7 cross-check: the observer's own motion dictionary agrees the
	# cat is controlled and supported on the table, at the table's height.
	var motion: Dictionary = ELEVATION_FIXTURE.cat_motion(self, cat, cat.pulses as Pulses, 0.0)
	assert_str(motion["phase"]).is_equal("controlled")
	assert_bool(motion["support"] != null).is_true()
	var support: Dictionary = motion["support"]
	assert_float(support["point"].y).is_equal_approx(
		ELEVATION_FIXTURE.TABLE_TOP_Y, 0.0010001192092895508
	)


## A cat dropped onto a bed settles on its frame top, the bed's authored
## elevation.
func test_stationary_cat_stands_on_bed_support() -> void:
	auto_free(ELEVATION_FIXTURE.add_bed(self, Vector3.ZERO))
	var cat := _add_cat_direct(Vector3(0.0, 3.0, 0.0))
	var landed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 2 and cat.is_on_floor(), 240
	)
	assert_bool(landed).is_true()
	for _tick: int in 10:
		await get_tree().physics_frame
	assert_float(cat.global_position.y).is_equal_approx(
		ELEVATION_FIXTURE.BED_TOP_Y, 0.0010001192092895508
	)
	assert_int(cat.collision_layer).is_equal(2)
	# Task 7 cross-check: the observer's own motion dictionary agrees the
	# cat is controlled and supported on the bed, at the bed's height.
	var motion: Dictionary = ELEVATION_FIXTURE.cat_motion(self, cat, cat.pulses as Pulses, 0.0)
	assert_str(motion["phase"]).is_equal("controlled")
	assert_bool(motion["support"] != null).is_true()
	var support: Dictionary = motion["support"]
	assert_float(support["point"].y).is_equal_approx(
		ELEVATION_FIXTURE.BED_TOP_Y, 0.0010001192092895508
	)


## The one transport law, end to end, for the cat's whole silhouette (paws,
## torso, tail): two cats at different nonzero elevations, walking the same
## seed and roam rect so their brains stay in lockstep, bake meshes that
## agree once the elevation difference is subtracted back out. Both cats
## sit at nonzero support on purpose: `Tail::advance` (`cat_body.rs`) keeps
## its pre-elevation lane's world-space math bit-exact only at EXACTLY zero
## support (a preserved legacy replay contract) and follows in a root-
## relative frame at any nonzero support instead, so a support-0 cat is not
## comparable here — it is not running the same numerical path a raised
## cat runs, only the same law. Two nonzero supports run identical code,
## but per-tick chain iteration still round-trips through a different
## absolute magnitude at each elevation (subtract this tick's root, follow,
## add it back), so vertices carry a small, measured, convergent transient:
## a probe sweep found at most ~6.3e-7 m of X/Z drift after 90 ticks
## (settling to exactly 0 by 300), never the ~centimetre-scale error a
## genuinely broken translation would produce. `MESH_TRANSPORT_TOLERANCE_M`
## keeps three more orders of magnitude of margin under that measured
## ceiling. Voices ride the same root (already pinned by
## `cat_test.gd::test_elevated_cat_pose_and_voices_share_root_support`).
func test_walking_elevated_cat_transports_paws_tail_and_voices() -> void:
	const MESH_TRANSPORT_TOLERANCE_M := 1.0e-4
	auto_free(ELEVATION_FIXTURE.add_floor(self, 0.75))
	auto_free(ELEVATION_FIXTURE.add_floor(self, 1.5))
	var low := _add_cat_direct(Vector3(0.0, 0.75, 0.0))
	var high := _add_cat_direct(Vector3(0.0, 1.5, 0.0))
	var now := 0.0
	for _tick: int in 90:
		now += DT
		low.tick(now)
		high.tick(now)
		await get_tree().physics_frame
	await get_tree().process_frame
	assert_int(low.collision_layer).is_equal(2)
	assert_int(high.collision_layer).is_equal(2)
	assert_bool(low.is_on_floor()).is_true()
	assert_bool(high.is_on_floor()).is_true()
	var low_arrays: Array = low.cat_mesh().surface_get_arrays(0)
	var high_arrays: Array = high.cat_mesh().surface_get_arrays(0)
	var low_verts: PackedVector3Array = low_arrays[Mesh.ARRAY_VERTEX]
	var high_verts: PackedVector3Array = high_arrays[Mesh.ARRAY_VERTEX]
	assert_int(low_verts.size()).is_greater(0)
	assert_int(high_verts.size()).is_equal(low_verts.size())
	assert_array(high_arrays[Mesh.ARRAY_CUSTOM0]).is_equal(low_arrays[Mesh.ARRAY_CUSTOM0])
	var support := high.global_position.y - low.global_position.y
	var broken := 0
	for i: int in low_verts.size():
		var f := low_verts[i]
		var r := high_verts[i]
		if (
			absf(r.x - f.x) > MESH_TRANSPORT_TOLERANCE_M
			or absf(r.z - f.z) > MESH_TRANSPORT_TOLERANCE_M
		):
			broken += 1
			continue
		if absf(r.y - (f.y + support)) > MESH_TRANSPORT_TOLERANCE_M:
			broken += 1
	assert_int(broken).is_equal(0)


## Once a wandering cat steps off an isolated platform's edge, its achieved
## planar velocity is frozen for the rest of the flight — the brain cannot
## steer it and no later tick recomputes it, exactly like the player's own
## departure trajectory.
func test_cat_walks_off_a_platform_with_fixed_trajectory() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(self, Vector3(0.0, 1.0, 0.0), Vector3(1.0, 0.2, 1.0), "Islet")
	)
	var cat := _add_cat_direct(Vector3(0.0, 1.1, 0.0))
	var departed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 4, 1800
	)
	assert_bool(departed).is_true()
	var vx := cat.velocity.x
	var vz := cat.velocity.z
	# Task 7 cross-check: the observer's own motion dictionary — read
	# through the Rust MotionState/FFI door rather than the CharacterBody3D
	# velocity the loop below checks directly — agrees this is a genuine,
	# unsupported departure, holding the SAME frozen planar velocity the
	# loop is about to prove `cat.velocity.x/z` keep for the next 15 ticks.
	var motion: Dictionary = ELEVATION_FIXTURE.cat_motion(self, cat, cat.pulses as Pulses, 0.0)
	assert_str(motion["phase"]).is_equal("airborne")
	assert_vector(motion["actual_velocity"]).is_equal(cat.velocity)
	assert_bool(motion["support"] == null).is_true()
	assert_vector(motion["held_planar_velocity"]).is_equal(Vector3(vx, 0.0, vz))
	for _tick: int in 15:
		await get_tree().physics_frame
		assert_int(cat.collision_layer).is_equal(4)
		assert_float(cat.velocity.x).is_equal(vx)
		assert_float(cat.velocity.z).is_equal(vz)


## Airborne, the cat's mind is wholly frozen: the canonical capture blob's
## whole `brain` dictionary — mood, target, rng word, blocked counter, every
## hidden field — stays byte-identical tick after tick, and the observable
## yaw an outside poke leaves behind survives untouched, proving no yaw
## setter runs even to rewrite the same value.
func test_airborne_cat_keeps_brain_and_yaw_frozen() -> void:
	var main: UnseeingGame = auto_free(
		WORLD_FIXTURE.game(WORLD_FIXTURE.DEFAULT_EXTENTS, false, false, true)
	)
	add_child(main)
	await get_tree().process_frame
	await get_tree().physics_frame
	var cat: WaveCat = main.cats()[0]
	cat.global_position = Vector3(cat.global_position.x, 6.0, cat.global_position.z)
	var airborne: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 4, 12
	)
	assert_bool(airborne).is_true()
	cat.rotation.y = 1.2345
	# Captured AFTER the set rather than compared against the 1.2345 literal:
	# assigning `rotation.y` round-trips through the Basis (trig in, Euler
	# extraction back out), so the stored float is not bit-identical to the
	# literal even though nothing ever reads it as "wrong" — the round-trip
	# itself is the discrepancy, not a frozen-yaw bug. Comparing against this
	# capture instead asks the only question this test owns: does any later
	# tick change the value at all.
	var yaw_before := cat.rotation.y
	var before: Dictionary = main.observer.capture(main.now, main.capture_env())
	var before_brain: Dictionary = (before["cats"] as Array)[0]["brain"]
	for _tick: int in 15:
		await get_tree().physics_frame
		assert_int(cat.collision_layer).is_equal(4)
	var after: Dictionary = main.observer.capture(main.now, main.capture_env())
	var after_brain: Dictionary = (after["cats"] as Array)[0]["brain"]
	assert_dict(after_brain).is_equal(before_brain)
	assert_float(cat.rotation.y).is_equal(yaw_before)


## The airborne selector never invokes a yaw setter: an externally poked
## rotation value the frozen policy would never itself choose survives
## every airborne tick untouched.
func test_airborne_cat_policy_produces_no_yaw_command() -> void:
	var cat := _add_cat_direct(Vector3(0.0, 6.0, 0.0))
	var airborne: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 4, 12
	)
	assert_bool(airborne).is_true()
	cat.rotation.y = 2.71828
	# See the sibling brain-freeze test: `rotation.y = X` round-trips through
	# the Basis, so the stored float is not the bit-identical literal even at
	# the instant of assignment — capture, then compare later reads to this.
	var yaw_before := cat.rotation.y
	for _tick: int in 15:
		await get_tree().physics_frame
		assert_int(cat.collision_layer).is_equal(4)
		assert_float(cat.rotation.y).is_equal(yaw_before)


## Unlike the frozen brain, the gait keeps animating from the body's actual
## achieved displacement while falling: a cat that departed with residual
## walking speed keeps moving its paws throughout the fall.
func test_airborne_cat_gait_uses_achieved_displacement() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(self, Vector3(0.0, 1.0, 0.0), Vector3(1.0, 0.2, 1.0), "Islet")
	)
	var cat := _add_cat_direct(Vector3(0.0, 1.1, 0.0))
	var departed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 4, 1800
	)
	assert_bool(departed).is_true()
	var moving := absf(cat.velocity.x) > 0.01 or absf(cat.velocity.z) > 0.01
	if not moving:
		fail("the departure carried no residual planar speed to animate from")
		return
	var samples: Array[PackedVector3Array] = []
	for _tick: int in 20:
		await get_tree().physics_frame
		assert_int(cat.collision_layer).is_equal(4)
		samples.append(cat.paw_positions())
	var moved := false
	for i: int in range(1, samples.size()):
		for leg: int in samples[i].size():
			if samples[i][leg] != samples[i - 1][leg]:
				moved = true
	(
		assert_bool(moved)
		. override_failure_message("an airborne cat with residual planar speed never moved a paw")
		. is_true()
	)


## The gait keeps animating throughout a fall with residual walking speed
## (proven above), which means it keeps completing swing-to-stance footfall
## CONTACTS too (`cat_gait::step_leg`'s `contacts.push`) — the exact events
## that fire a paw wave while controlled. `QueuedWaveGate::ControlledContact`
## is the only thing standing between those in-flight contacts and a wave
## sounding off a floor nowhere near the falling paws. A full stride
## (`STRIDE_LEN` / `WANDER_SPEED` = 0.30 / 0.6 = 0.5 s = 30 ticks) is
## guaranteed to complete well inside this 90-tick window over the void
## below the islet, so this exercises the gate rather than an empty
## contact list. The baseline is taken at the moment of departure, not zero:
## the cat's last grounded stride can leave one legitimate paw pulse still
## alive into the first few airborne ticks (`PAW_RANGE`'s own travel time),
## so the count must never GROW past that baseline, rather than vanish
## outright.
func test_airborne_cat_with_a_completed_stride_still_emits_no_paw_voice() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(self, Vector3(0.0, 1.0, 0.0), Vector3(1.0, 0.2, 1.0), "Islet")
	)
	var cat := _add_cat_direct(Vector3(0.0, 1.1, 0.0))
	var pulses := cat.pulses as Pulses
	var departed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 4, 1800
	)
	assert_bool(departed).is_true()
	var moving := absf(cat.velocity.x) > 0.01 or absf(cat.velocity.z) > 0.01
	if not moving:
		fail("the departure carried no residual planar speed to animate from")
		return
	var baseline := _paw_voice_count(pulses, 0.0)
	for _tick: int in 90:
		await get_tree().physics_frame
		if cat.collision_layer != 4:
			break
		assert_int(_paw_voice_count(pulses, 0.0)).is_less_equal(baseline)


## `last_pos` tracks the falling body every airborne tick rather than
## staying pinned at the departure point — the exact law that keeps the
## first resumed brain tick from reading the whole flight as one step of
## walked progress. The canonical capture blob's own `last_pos` field is
## the direct witness: if it ever fell behind the live body while airborne,
## a landing's `progress` would read the whole flight as one tick's walk.
func test_first_resumed_brain_tick_receives_zero_flight_progress() -> void:
	var main: UnseeingGame = auto_free(
		WORLD_FIXTURE.game(WORLD_FIXTURE.DEFAULT_EXTENTS, false, false, true)
	)
	add_child(main)
	await get_tree().process_frame
	await get_tree().physics_frame
	var cat: WaveCat = main.cats()[0]
	var walking: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(),
		func() -> bool: return Vector2(cat.velocity.x, cat.velocity.z).length() > 0.2,
		600
	)
	assert_bool(walking).is_true()
	cat.global_position = Vector3(cat.global_position.x, 6.0, cat.global_position.z)
	var airborne: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 4, 12
	)
	assert_bool(airborne).is_true()
	var checked := 0
	for _tick: int in 20:
		await get_tree().physics_frame
		if cat.collision_layer != 4:
			break
		var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
		var last_pos: Array = (blob["cats"] as Array)[0]["last_pos"]
		var last_pos_x: String = last_pos[0]
		var last_pos_z: String = last_pos[2]
		assert_float(last_pos_x.to_float()).is_equal_approx(cat.global_position.x, 0.01)
		assert_float(last_pos_z.to_float()).is_equal_approx(cat.global_position.z, 0.01)
		checked += 1
	assert_int(checked).is_greater(0)


## Zero, negative and non-finite raw dt all clamp to a zero-length step in
## the shared `StepDuration::from_raw` (`rust/src/support_motion.rs`), which
## `controlled_cat_tick` calls on every physics tick with no cat-specific
## detour around it — proven directly by
## `support_motion::tests::malformed_durations_are_zero_and_large_steps_are_capped`,
## a cargo test on the exact function both the player and the cat share.
## GDScript cannot reproduce the fault at this door: gdext does not register
## an engine-virtual override such as `_physics_process` as a callable
## method (`ClassDB.class_get_method_list("WaveCat")` carries no entry
## matching it, so `.call("_physics_process", ...)` always raises
## "Nonexistent function"), and the real physics server never delivers a
## non-positive or non-finite delta to begin with — even forcing
## `Engine.time_scale = 0.0` for one tick leaves the delta the physics step
## actually uses untouched (measured: the cat still fell the ordinary
## one-tick distance). There is no lever in this engine that hands a
## running cat a poisoned dt, so this door instead pins the half that IS
## reachable from here: an ordinary tick's delta is always positive and
## finite, and the cat comes out of it with a still-finite transform and
## velocity rather than a silently corrupted one.
func test_zero_negative_and_nonfinite_cat_dt_keep_zero_actual_speed_without_fault() -> void:
	var cat := _add_cat_direct(Vector3(0.0, 0.0, 0.0))
	await get_tree().physics_frame
	assert_float(get_physics_process_delta_time()).is_greater(0.0)
	assert_bool(cat.is_physics_processing()).is_true()
	assert_bool(cat.global_transform.is_finite()).is_true()
	assert_bool(cat.velocity.is_finite()).is_true()


## The presence voice's elevated origin follows the falling root's own
## height every airborne tick — it is never pinned to the departure height.
## The chest offset (0.18 m) is the same constant `cat_test.gd` already
## measures for a grounded cat (0.75 root + 0.18 = 0.93 origin).
func test_airborne_presence_origin_follows_root_height() -> void:
	var cat := _add_cat_direct(Vector3(0.0, 40.0, 0.0))
	var pulses := cat.pulses as Pulses
	var airborne: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 4, 12
	)
	assert_bool(airborne).is_true()
	var now := 0.0
	var prior_count := pulses.live_count(now)
	var found := false
	for _tick: int in 200:
		now += DT
		cat.tick(now)
		await get_tree().physics_frame
		if cat.collision_layer != 4:
			break
		var count := pulses.live_count(now)
		if count > prior_count and is_equal_approx(pulses.dat[0].y, WaveCat.presence_range()):
			assert_float(pulses.pos[0].y).is_equal_approx(cat.global_position.y + 0.18, 0.01)
			found = true
			break
		prior_count = count
	(
		assert_bool(found)
		. override_failure_message(
			"no fresh airborne presence pulse observed within the poll budget"
		)
		. is_true()
	)


## Neither an airborne fall nor its landing tick ever sounds a paw voice —
## `frame.contacts` never carries a controlled-contact this cat's gate would
## pass while airborne, and the landing tick's own contact (if any) is
## excluded by the same `QueuedWaveGate` law the player shares.
func test_airborne_and_landing_ticks_emit_no_paw_voice() -> void:
	auto_free(ELEVATION_FIXTURE.add_floor(self, 0.0))
	var cat := _add_cat_direct(Vector3(0.0, 3.0, 0.0))
	var pulses := cat.pulses as Pulses
	var airborne: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 4, 12
	)
	assert_bool(airborne).is_true()
	var landed := false
	for _tick: int in 240:
		await get_tree().physics_frame
		assert_bool(_found_paw_voice(pulses, 0.0)).is_false()
		if cat.collision_layer == 2 and cat.is_on_floor():
			landed = true
			break
	assert_bool(landed).is_true()
	for _tick: int in 10:
		await get_tree().physics_frame
		assert_bool(_found_paw_voice(pulses, 0.0)).is_false()


## A real, legitimate post-move refusal: the cat starts just inside the
## pose coordinate bound, free-falls with nothing around it, and the tick
## whose displacement finally crosses the bound rolls back to the exact
## saved transform, zeros velocity, disables processing, and advances no
## paw or wave — the adapter's rollback contract exercised through genuine
## physics rather than an injected poison.
func test_poisoned_cat_post_move_sample_rolls_back_without_wave_or_partial_component() -> void:
	var start_y := -(1_000_002.0 - 30.0)
	var cat := _add_cat_direct(Vector3(0.0, start_y, 0.0))
	var pulses := cat.pulses as Pulses
	var last_position := cat.global_position
	var last_paws := cat.paw_positions()
	var refused := false
	for _tick: int in 400:
		await get_tree().physics_frame
		if not cat.is_physics_processing():
			refused = true
			break
		last_position = cat.global_position
		last_paws = cat.paw_positions()
	assert_bool(refused).is_true()
	assert_vector(cat.global_position).is_equal(last_position)
	assert_vector(cat.velocity).is_equal(Vector3.ZERO)
	assert_array(cat.paw_positions()).is_equal(last_paws)
	assert_int(pulses.live_count(0.0)).is_equal(0)


## A poisoned pre-move sample (here: the transform itself) refuses before
## any owner advances: paws, mesh and processing all stay exactly as they
## were the tick before.
func test_poisoned_cat_pre_move_sample_disables_without_advancing_any_component() -> void:
	var cat := _add_cat_direct(Vector3(0.0, 0.0, 0.0))
	var pulses := cat.pulses as Pulses
	await get_tree().physics_frame
	var paws_before := cat.paw_positions()
	var refuse := func() -> void:
		var t := cat.global_transform
		t.origin.x = NAN
		cat.global_transform = t
		cat.notification(Node.NOTIFICATION_PHYSICS_PROCESS)
	await assert_error(refuse).is_push_error(
		"WaveCat: physics transform refused: actor_position.x must be finite"
	)
	assert_bool(cat.is_physics_processing()).is_false()
	assert_bool(cat.is_processing()).is_false()
	assert_array(cat.paw_positions()).is_equal(paws_before)
	assert_int(pulses.live_count(0.0)).is_equal(0)


## Real Godot wall collision removes only the blocked planar lane while
## airborne and never speaks: the cat wanders inside a small open-bottomed
## shaft, so stepping off the platform edge in any direction meets a wall
## within a short, bounded drift while the cat keeps falling. `now` is
## ticked explicitly throughout (rather than left at its default) so the
## live-pulse baseline captured right after departure and the count taken
## after the strike both read a real, advancing clock — otherwise every
## pulse in the whole test, including the ordinary paw taps from wandering
## the islet before departure, would forever read back as "live at t=0".
func test_cat_wall_contact_removes_only_blocked_trajectory_without_a_wave() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(self, Vector3(0.0, 1.0, 0.0), Vector3(1.0, 0.2, 1.0), "Islet")
	)
	var wall_specs := [
		[Vector3(0.8, -49.0, 0.0), Vector3(0.1, 100.0, 1.3)],
		[Vector3(-0.8, -49.0, 0.0), Vector3(0.1, 100.0, 1.3)],
		[Vector3(0.0, -49.0, 0.8), Vector3(1.3, 100.0, 0.1)],
		[Vector3(0.0, -49.0, -0.8), Vector3(1.3, 100.0, 0.1)],
	]
	for spec: Array in wall_specs:
		var at: Vector3 = spec[0]
		var size: Vector3 = spec[1]
		auto_free(ELEVATION_FIXTURE.add_box(self, at, size, "Shaft"))
	var cat := _add_cat_direct(Vector3(0.0, 1.1, 0.0))
	var pulses := cat.pulses as Pulses
	var now := 0.0
	var departed := false
	for _tick: int in 1800:
		now += DT
		cat.tick(now)
		await get_tree().physics_frame
		if cat.collision_layer == 4:
			departed = true
			break
	assert_bool(departed).is_true()
	var speed_before := Vector2(cat.velocity.x, cat.velocity.z).length()
	if speed_before < 0.01:
		fail("the departure carried no residual planar speed to block")
		return
	var baseline := pulses.live_count(now)
	var struck := false
	for _tick: int in 240:
		now += DT
		cat.tick(now)
		await get_tree().physics_frame
		if Vector2(cat.velocity.x, cat.velocity.z).length() < speed_before * 0.5:
			struck = true
			break
	assert_bool(struck).is_true()
	assert_int(cat.collision_layer).is_equal(4)
	# no NEW wave since the wall strike — a lower count than the baseline is
	# fine (an earlier paw tap simply decaying out) but a higher one would
	# mean the collision itself spoke.
	assert_int(pulses.live_count(now)).is_less_equal(baseline)


## Terminal fall speed caps the descent to a finite, exact value and every
## observable stays finite indefinitely — no floor is ever reached.
func test_no_floor_cat_stays_finite_at_terminal_speed() -> void:
	var cat := _add_cat_direct(Vector3(0.0, 8.0, 0.0))
	var reached_terminal: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.velocity.y == -20.0, 180
	)
	assert_bool(reached_terminal).is_true()
	assert_float(cat.velocity.y).is_equal(-20.0)
	assert_float(cat.global_position.y).is_less(8.0)
	assert_bool(cat.global_transform.is_finite()).is_true()
	assert_bool(cat.velocity.is_finite()).is_true()
	assert_bool(cat.is_on_floor()).is_false()
	assert_int(cat.collision_layer).is_equal(4)
	# Task 7 cross-check: the observer's own motion dictionary — a channel
	# none of the assertions above consulted — agrees this is a terminal,
	# unsupported fall.
	var motion: Dictionary = ELEVATION_FIXTURE.cat_motion(self, cat, cat.pulses as Pulses, 0.0)
	assert_str(motion["phase"]).is_equal("airborne")
	assert_vector(motion["actual_velocity"]).is_equal(cat.velocity)
	assert_bool(motion["support"] == null).is_true()
	assert_float(motion["held_vertical_velocity"]).is_equal(-20.0)


## A cat confined to a ramp-and-platform patch never becomes airborne while
## it wanders across the slope in either direction — the same snap/no-snap
## law the player's own ramp test proves, reproduced for the cat's adapter.
## The roam rect is sized to the built footprint, not the wide default: the
## approach spans x in [-2.7, -0.7] and the ramp/platform span z in
## [-0.5, 0.5] (narrower than the approach's own [-1, 1]), so a roam rect
## anywhere near the player test's own travel range but reaching the
## default (6, 1.6) rect would walk the cat off the narrow sides of the
## ramp or platform, exactly what this test caught the first time. `1.0` is
## `CatBrain`'s own authored floor for a roam extent
## (`cat_brain::checked_roam_size`), so the z rect cannot be narrowed any
## further than the ramp/platform's own [-0.5, 0.5] span. A freshly-spawned
## cat's very first physics tick has not yet run a `move_and_slide` of its
## own, so Godot's own `is_on_floor()` reads false for that one tick even
## while the support-scan already holds it controlled (measured: the same
## one-tick gap `test_flat_root_cat_stays_exactly_y_zero_on_floor` polls
## through) — settle onto the floor first, then hold the "never" loop.
func test_cat_ramp_up_and_down_never_lands() -> void:
	var datum := Vector3.ZERO
	auto_free(
		ELEVATION_FIXTURE.add_box(
			self, Vector3(-1.7, -0.05, 0.0), Vector3(2.0, 0.1, 2.0), "RampApproach"
		)
	)
	auto_free(ELEVATION_FIXTURE.add_ramp(self, datum))
	auto_free(ELEVATION_FIXTURE.add_ramp_platform(self, datum))
	var cat := _add_cat_direct(Vector3(-0.7, 0.0, 0.0), 7, Vector2(3.6, 1.0))
	var settled: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 2 and cat.is_on_floor(), 10
	)
	assert_bool(settled).is_true()
	for _tick: int in 900:
		await get_tree().physics_frame
		assert_int(cat.collision_layer).is_equal(2)
		assert_bool(cat.is_on_floor()).is_true()


## A landing at or below the silent threshold makes no sound at all. The
## drop height (0.10 m, floor top at -0.01) is measured, not guessed: free
## fall under `fall_acceleration = 9.8` reaches the default
## `landing_silent_speed = 1.5` m/s at a continuous height of ~0.1148 m, but
## the engine integrates gravity in fixed 1/60 s steps, so the speed
## actually recorded at the landing tick jumps in ~0.163 m/s increments
## rather than sweeping continuously. A probe sweep confirmed every drop
## from 0.08 m to 0.11 m lands silently on tick 9 (impact speed 1.47 m/s)
## while 0.13 m already overshoots to tick 10 (1.63 m/s, audible) — 0.10 m
## sits in the middle of the confirmed-silent band with margin on both
## sides.
func test_cat_landing_is_silent_at_threshold() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(self, Vector3(0.0, -0.06, 0.0), Vector3(20.0, 0.1, 20.0), "Floor")
	)
	var cat := _add_cat_direct(Vector3(0.0, 0.09, 0.0))
	var pulses := cat.pulses as Pulses
	var went_airborne := false
	var landed := false
	for _tick: int in 60:
		await get_tree().physics_frame
		if cat.collision_layer == 4:
			went_airborne = true
		if went_airborne and cat.collision_layer == 2 and cat.is_on_floor():
			landed = true
			break
	assert_bool(landed).is_true()
	for _tick: int in 5:
		await get_tree().physics_frame
	assert_int(pulses.live_count(0.1)).is_equal(0)


## A landing clearly above the full-strength threshold sounds a voice.
func test_cat_landing_is_audible_above_threshold() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(self, Vector3(0.0, -0.05, 0.0), Vector3(20.0, 0.1, 20.0), "Floor")
	)
	var cat := _add_cat_direct(Vector3(0.0, 2.5, 0.0))
	var pulses := cat.pulses as Pulses
	var landed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 2 and cat.is_on_floor(), 180
	)
	assert_bool(landed).is_true()
	assert_int(pulses.live_count(0.1)).is_equal(1)
	var dat: Vector4 = pulses.dat[0]
	assert_int(int(floorf(dat.w / 10.0))).is_equal(2)
	assert_float(fmod(dat.w, 10.0) / 9.0).is_greater(0.0)
	# Task 7 cross-check: the observer's own landing dictionary agrees a
	# real landing occurred, with the floor's own UP support normal.
	var motion: Dictionary = ELEVATION_FIXTURE.cat_motion(self, cat, pulses, 0.1)
	assert_bool(motion["last_landing"] != null).is_true()
	var landing: Dictionary = motion["last_landing"]
	assert_vector(landing["normal"]).is_equal(Vector3.UP)
	assert_float(landing["impact_speed"]).is_greater(0.0)


## A hard drop caps the landing voice at the cat's own authored maxima:
## gain 0.60, range 2.5.
func test_cat_landing_caps_gain_and_range() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(self, Vector3(0.0, -0.05, 0.0), Vector3(20.0, 0.1, 20.0), "Floor")
	)
	var cat := _add_cat_direct(Vector3(0.0, 6.0, 0.0))
	var pulses := cat.pulses as Pulses
	var landed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 2 and cat.is_on_floor(), 240
	)
	assert_bool(landed).is_true()
	assert_int(pulses.live_count(0.1)).is_equal(1)
	var dat: Vector4 = pulses.dat[0]
	assert_int(int(floorf(dat.w / 10.0))).is_equal(2)
	assert_float(dat.y).is_equal(2.5)
	assert_float(dat.z).is_equal(4.0)
	assert_float(fmod(dat.w, 10.0) / 9.0).is_equal_approx(0.60, 1e-6)
	# Task 7 cross-check: the hard drop's real impact speed, read from the
	# observer's own dictionary — a value the capped gain/range above
	# never surfaces (they only prove the CLAMPED voice, not the fall).
	var motion: Dictionary = ELEVATION_FIXTURE.cat_motion(self, cat, pulses, 0.1)
	assert_bool(motion["last_landing"] != null).is_true()
	var landing: Dictionary = motion["last_landing"]
	assert_float(landing["impact_speed"]).is_greater(5.0)


## Zero authored landing gain silences every landing completely.
func test_zero_cat_landing_gain_emits_nothing() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(self, Vector3(0.0, -0.05, 0.0), Vector3(20.0, 0.1, 20.0), "Floor")
	)
	var cat := _add_cat_direct(Vector3(0.0, 3.0, 0.0))
	cat.set("landing_max_gain", 0.0)
	var pulses := cat.pulses as Pulses
	var landed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 2 and cat.is_on_floor(), 240
	)
	assert_bool(landed).is_true()
	for _tick: int in 5:
		await get_tree().physics_frame
	assert_int(pulses.live_count(0.1)).is_equal(0)


## Zero authored landing range is the same silence through the other knob.
func test_zero_cat_landing_range_emits_nothing() -> void:
	auto_free(
		ELEVATION_FIXTURE.add_box(self, Vector3(0.0, -0.05, 0.0), Vector3(20.0, 0.1, 20.0), "Floor")
	)
	var cat := _add_cat_direct(Vector3(0.0, 3.0, 0.0))
	cat.set("landing_max_range", 0.0)
	var pulses := cat.pulses as Pulses
	var landed: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 2 and cat.is_on_floor(), 240
	)
	assert_bool(landed).is_true()
	for _tick: int in 5:
		await get_tree().physics_frame
	assert_int(pulses.live_count(0.1)).is_equal(0)


## Every WaveCat instance owns its own six exported scalars; both assignment
## orders and both extremes install the exact final active config, proving
## independent cats never share one active configuration.
func test_valid_cat_threshold_pairs_round_trip_for_independent_cats() -> void:
	var packed_scenes: Array[PackedScene] = []
	var expecteds: Array = []
	for data: Dictionary in [
		{"pair": Vector2(8.0, 9.0), "silent_first": true},
		{"pair": Vector2(0.1, 0.2), "silent_first": false},
	]:
		var pair: Vector2 = data["pair"]
		var silent_first: bool = data["silent_first"]
		var authored := WaveCat.new()
		if silent_first:
			authored.set("landing_silent_speed", pair.x)
			authored.set("landing_full_speed", pair.y)
		else:
			authored.set("landing_full_speed", pair.y)
			authored.set("landing_silent_speed", pair.x)
		var packed := PackedScene.new()
		assert_int(packed.pack(authored)).is_equal(OK)
		authored.free()
		var expected: Array = CAT_CONFIG_DEFAULTS.duplicate()
		expected[2] = pair.x
		expected[3] = pair.y
		packed_scenes.append(packed)
		expecteds.append(expected)

	var cats: Array[WaveCat] = []
	for packed: PackedScene in packed_scenes:
		var cat: WaveCat = auto_free(packed.instantiate() as WaveCat)
		assert_object(cat).is_not_null()
		cat.pulses = Pulses.new()
		cat.data_mat = ShaderMaterial.new()
		add_child(cat)
		cats.append(cat)

	for i: int in cats.size():
		var active: PackedFloat64Array = cats[i].call("motion_config_snapshot")
		assert_array(active).is_equal(expecteds[i])


## Six raw authored scalars set before tree entry reach the runtime cat
## exactly, read back through the same `motion_config_snapshot` door Task 2
## defined for the player.
func test_cat_knobs_are_installed_before_tree_entry() -> void:
	var cat: WaveCat = auto_free(WaveCat.new())
	cat.pulses = Pulses.new()
	cat.data_mat = ShaderMaterial.new()
	var authored := PackedFloat64Array([12.3, 27.5, 2.0, 6.0, 0.7, 2.2])
	for index: int in CAT_CONFIG_FIELDS.size():
		cat.set(CAT_CONFIG_FIELDS[index], authored[index])
	add_child(cat)
	var active: PackedFloat64Array = cat.call("motion_config_snapshot")
	assert_array(active).is_equal(authored)


## Nonfinite or out-of-range scalars retain the cat's prior authored value.
func test_out_of_range_cat_knob_retains_the_prior_scalar() -> void:
	var refused: Array[Array] = [
		[0, 0.09],
		[0, 30.01],
		[1, 0.49],
		[1, 50.01],
		[2, -0.01],
		[2, 10.01],
		[3, 0.09],
		[3, 20.01],
		[4, -0.01],
		[4, 1.01],
		[5, -0.01],
		[5, 10.01],
	]
	for case: Array in refused:
		var cat: WaveCat = auto_free(WaveCat.new())
		var index: int = case[0]
		cat.set(CAT_CONFIG_FIELDS[index], case[1])
		assert_float(cat.get(CAT_CONFIG_FIELDS[index])).is_equal(CAT_CONFIG_DEFAULTS[index])
	for index: int in CAT_CONFIG_FIELDS.size():
		for poison: float in [NAN, INF, -INF]:
			var cat: WaveCat = auto_free(WaveCat.new())
			cat.set(CAT_CONFIG_FIELDS[index], poison)
			assert_float(cat.get(CAT_CONFIG_FIELDS[index])).is_equal(CAT_CONFIG_DEFAULTS[index])


## An invalid final six-tuple refuses before any collider, gait or brain
## construction — no silent fallback to the last active default.
func test_invalid_final_cat_threshold_pair_refuses_before_construction() -> void:
	var cat: WaveCat = auto_free(WaveCat.new())
	cat.pulses = Pulses.new()
	cat.data_mat = ShaderMaterial.new()
	cat.set("landing_silent_speed", 8.0)
	cat.set("landing_full_speed", 7.0)
	var enter := func() -> void: add_child(cat)
	await assert_error(enter).is_push_error(
		(
			"WaveCat: invalid motion configuration — landing full speed 7 m/s "
			+ "must be greater than silent speed 8 m/s"
		)
	)
	assert_bool(cat.is_physics_processing()).is_false()
	assert_bool(cat.is_processing()).is_false()
	assert_int(cat.get_child_count()).is_equal(0)


## An out-of-order threshold pair raises the SAME warning text through both
## the virtual `get_configuration_warnings()` and the registered callable
## forwarder — the dual-channel contract every warning-bearing node keeps.
func test_invalid_cat_threshold_pair_reaches_virtual_and_callable_warning_channels() -> void:
	var cat := _add_cat_direct(Vector3.ZERO)
	cat.set("landing_silent_speed", 8.0)
	cat.set("landing_full_speed", 7.0)
	var expected := "landing full speed 7 m/s must be greater than silent speed 8 m/s"
	assert_array(cat.get_configuration_warnings()).contains_exactly([expected])
	assert_array(cat.call("get_configuration_warnings")).contains_exactly([expected])


## A complementary valid edit clears the warning on both channels at once.
func test_valid_complementary_threshold_edit_clears_both_warning_channels() -> void:
	var cat := _add_cat_direct(Vector3.ZERO)
	cat.set("landing_silent_speed", 8.0)
	cat.set("landing_full_speed", 7.0)
	assert_int(cat.get_configuration_warnings().size()).is_equal(1)
	cat.set("landing_full_speed", 9.0)
	assert_array(cat.get_configuration_warnings()).is_empty()
	assert_array(cat.call("get_configuration_warnings")).is_empty()


## The eleven solver properties from the shared table, hand-asserted one at
## a time — the same contract the player's own capsule test pins.
func test_cat_solver_contract_is_explicit_on_every_property() -> void:
	var cat := _add_cat_direct(Vector3.ZERO)
	assert_int(cat.motion_mode).is_equal(CharacterBody3D.MOTION_MODE_GROUNDED)
	assert_vector(cat.up_direction).is_equal(Vector3.UP)
	assert_float(cat.floor_snap_length).is_equal_approx(0.10, 1.0e-7)
	assert_float(cat.floor_max_angle).is_equal_approx(PI / 4.0, 1.0e-7)
	assert_float(cat.safe_margin).is_equal_approx(0.001, 1.0e-7)
	assert_int(cat.max_slides).is_equal(6)
	assert_bool(cat.floor_stop_on_slope).is_true()
	assert_bool(cat.floor_constant_speed).is_false()
	assert_int(cat.platform_floor_layers).is_equal(0)
	assert_int(cat.platform_wall_layers).is_equal(0)
	assert_int(cat.platform_on_leave).is_equal(CharacterBody3D.PLATFORM_ON_LEAVE_DO_NOTHING)


## A flat-floor cat starts exactly at Y zero, settles onto the floor within
## a handful of ticks, and holds it in the controlled layer. `is_on_floor()`
## does not go true on the very first tick that lands it there (measured: a
## bare single `physics_frame` await leaves it still false), so this polls a
## short, bounded window rather than asserting on frame one.
func test_flat_root_cat_stays_exactly_y_zero_on_floor() -> void:
	auto_free(ELEVATION_FIXTURE.add_floor(self))
	var cat := _add_cat_direct(Vector3(0.0, 0.0, 0.0))
	var settled: bool = await ELEVATION_FIXTURE.poll_physics(
		get_tree(), func() -> bool: return cat.collision_layer == 2 and cat.is_on_floor(), 10
	)
	assert_bool(settled).is_true()
	assert_float(cat.global_position.y).is_equal(0.0)
	assert_bool(cat.is_on_floor()).is_true()
	assert_int(cat.collision_layer).is_equal(2)
