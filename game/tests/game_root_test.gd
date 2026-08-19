extends GdUnitTestSuite
## UnseeingGame's ready-side wiring contract — the migration's safety net,
## originally built and asserted before main.tscn switched away from main.gd.
## Most cases construct the registered class directly, no scene involved, the
## same way restore_test.gd and observer_test.gd build a bare
## WaveLevel/HeroBody: `UnseeingGame.new()`, `add_child`, then synchronous
## assertions. `_ready` runs INSIDE `add_child` (the parent — this suite — is
## already in the tree), so nothing here awaits a frame. Scene-loading cases
## separately prove main.tscn now boots this same Rust composition root.
##
## Pins wiring and level selection, not pixels; the rendering and perception
## suites own those downstream contracts.

const DATA_SHADER := preload("res://shaders/data_pass.gdshader")
const XRAY_SHADER := preload("res://shaders/data_xray.gdshader")
const POST_SHADER := preload("res://shaders/hearing_post.gdshader")
const WORLD_FIXTURE := preload("res://tests/world_fixture.gd")


func _game(
	with_wall: bool = false, with_source: bool = false, with_cat: bool = false
) -> UnseeingGame:
	var game: UnseeingGame = auto_free(
		WORLD_FIXTURE.game(WORLD_FIXTURE.DEFAULT_EXTENTS, with_wall, with_source, with_cat)
	)
	add_child(game)
	return game


func test_level_scene_property_is_a_packed_scene_resource_picker() -> void:
	var found := false
	for property: Dictionary in ClassDB.class_get_property_list("UnseeingGame"):
		if property["name"] == "level_scene":
			found = true
			assert_int(property["type"]).is_equal(TYPE_OBJECT)
			assert_int(property["hint"]).is_equal(PROPERTY_HINT_RESOURCE_TYPE)
			assert_str(property["hint_string"]).is_equal("PackedScene")
	assert_bool(found).is_true()


func test_empty_level_scene_keeps_the_exact_level_01_fallback() -> void:
	var game: UnseeingGame = auto_free(UnseeingGame.new())
	add_child(game)
	assert_object(game.level_scene).is_null()
	assert_str(game.level.scene_file_path).is_equal("res://scenes/level_01.tscn")


## Selection is a composition-root contract, independent of whichever objects
## a designer currently keeps in either shipped level. A valid selected level
## may deliberately contain no walls, sources, or creatures.
func test_level_scene_uses_only_the_selected_code_built_level() -> void:
	var game: UnseeingGame = auto_free(UnseeingGame.new())
	var selected: PackedScene = WORLD_FIXTURE.level_scene(Vector2(9, 7))
	game.level_scene = selected
	add_child(game)
	assert_object(game.level_scene).is_same(selected)
	assert_object(game.level).is_not_null()
	assert_vector(game.level.extents).is_equal(Vector2(9, 7))
	assert_array(game.level.wall_segments()).is_empty()
	assert_array(game.level.sources()).is_empty()
	assert_array(game.cats()).is_empty()
	assert_bool(game.observer.snapshot(0.0).has("unavailable")).is_false()


func test_wrong_level_root_refuses_before_adding_a_world() -> void:
	var game: UnseeingGame = auto_free(UnseeingGame.new())
	game.level_scene = load("res://scenes/props/chair.tscn")
	var enter := func() -> void: add_child(game)
	await (assert_error(enter).is_push_error(
		(
			"UnseeingGame: res://scenes/props/chair.tscn did not instantiate as a WaveLevel"
			+ " — check the scene's root type"
		)
	))
	assert_object(game.level).is_null()


## Both reusable prop resources must survive the real runtime loader, keep a
## plain composition root, and instantiate every typed piece that gives the
## silhouette its shape. Covering both paths catches a missing/renamed table
## resource that the chair-only level census cannot see.
func test_reusable_prop_scenes_load_and_instantiate_at_runtime() -> void:
	var fixtures: Array[Dictionary] = [
		{"path": "res://scenes/props/chair.tscn", "root": "Chair", "pieces": 6},
		{"path": "res://scenes/props/table.tscn", "root": "Table", "pieces": 5},
	]
	for fixture: Dictionary in fixtures:
		var path: String = fixture["path"]
		var exists := ResourceLoader.exists(path)
		assert_bool(exists).append_failure_message(path).is_true()
		if not exists:
			continue
		var packed := load(path) as PackedScene
		assert_object(packed).append_failure_message(path).is_not_null()
		if packed == null:
			continue
		var loaded: Node = auto_free(packed.instantiate())
		assert_object(loaded).append_failure_message(path).is_not_null()
		if loaded == null:
			continue
		var instance := loaded as Node3D
		assert_object(instance).append_failure_message(path).is_not_null()
		if instance == null:
			continue
		add_child(instance)
		assert_str(instance.get_class()).append_failure_message(path).is_equal("Node3D")
		assert_str(instance.name).append_failure_message(path).is_equal(fixture["root"])
		var pieces := instance.find_children("*", "WaveProp", true, false)
		assert_int(pieces.size()).append_failure_message(path).is_equal(fixture["pieces"])
		for piece: WaveProp in pieces:
			(
				assert_int(piece.get_child_count())
				. append_failure_message(str(piece.get_path()))
				. is_equal(2)
			)


## Five materials wearing the shaders `main.gd` assigned them, stacked at
## the same perceptual-ladder priorities, the cane's standing floor intact.
func test_five_materials_wear_the_right_shaders_and_priorities() -> void:
	var game := _game()
	assert_object(game.data_mat.shader).is_same(DATA_SHADER)
	assert_object(game.source_mat.shader).is_same(XRAY_SHADER)
	assert_object(game.cane_mat.shader).is_same(DATA_SHADER)
	assert_object(game.body_mat.shader).is_same(DATA_SHADER)
	assert_object(game.post_mat.shader).is_same(POST_SHADER)
	assert_int(game.data_mat.render_priority).is_equal(0)
	assert_int(game.source_mat.render_priority).is_equal(20)
	assert_float(game.cane_mat.get_shader_parameter("u_base")).is_equal(0.85)


## `wave_mats()` and the five named properties are the SAME five objects,
## in the perceptual-ladder order — two ways of reading one set of fields,
## never two copies of it.
func test_wave_mats_is_the_named_materials_in_order() -> void:
	var game := _game()
	var mats := game.wave_mats()
	assert_int(mats.size()).is_equal(5)
	assert_object(mats[0]).is_same(game.data_mat)
	assert_object(mats[1]).is_same(game.source_mat)
	assert_object(mats[2]).is_same(game.cane_mat)
	assert_object(mats[3]).is_same(game.body_mat)
	assert_object(mats[4]).is_same(game.post_mat)


## Zero authored walls, sources, and cats is a complete valid level, not an
## uninitialized one. The selected fixture still reaches the observer through
## the same inject-before-add path while every optional census is empty.
func test_zero_content_level_is_wired_and_observable() -> void:
	var game := _game()
	assert_object(game.level).is_not_null()
	assert_array(game.level.wall_segments()).is_empty()
	assert_array(game.level.sources()).is_empty()
	assert_array(game.cats()).is_empty()
	assert_bool(game.observer.snapshot(0.0).has("unavailable")).is_false()


## The player built its eye in its own `_ready` — the camera the hero's
## viewmodel and the post quad both anchor to.
func test_player_camera_is_live() -> void:
	var game := _game()
	assert_object(game.player).is_not_null()
	assert_object(game.player.camera).is_not_null()
	assert_bool(game.player.camera.is_inside_tree()).is_true()


## The hero was wired before it entered the tree too: it dresses the SAME
## player and rides the SAME camera, not a second one nobody plays through.
func test_hero_is_wired_to_the_same_player_and_camera() -> void:
	var game := _game()
	assert_object(game.hero).is_not_null()
	assert_object(game.hero.player).is_same(game.player)
	assert_object(game.hero.camera).is_same(game.player.camera)


## Added LAST, on purpose: unhandled input walks the tree bottom-up, so the
## settings overlay must see Escape before anything else in the tree does.
func test_settings_is_the_last_child() -> void:
	var game := _game()
	var last := game.get_child(game.get_child_count() - 1)
	assert_object(last).is_same(game.settings)


## The observer reads a fully wired world at t=0 — no "unavailable" key,
## which is what an uninjected or torn-down system would answer with
## instead.
func test_observer_snapshot_is_available_at_boot() -> void:
	var game := _game()
	var snapshot: Dictionary = game.observer.snapshot(0.0)
	assert_bool(snapshot.has("unavailable")).is_false()


## The clock starts at zero, exactly as `main.gd`'s `now := 0.0` field
## initializer did, and stays writable — the contract a restore's env
## application leans on.
func test_now_starts_at_zero_and_is_writable() -> void:
	var game := _game()
	assert_float(game.now).is_equal(0.0)
	game.now = 1.0
	assert_float(game.now).is_equal(1.0)


## One process frame and one physics frame — the pair every clock in the
## game needs to see a change. Duplicated from
## `restore_transaction_test.gd::_one_frame`, which is where this idiom
## lives; this suite's own composition root has no scene of its own to
## share a helper file with.
##
## WARNING: `await physics_frame` after `await process_frame` spans a SECOND
## `process()` call — any guard testing behavior that must land in exactly
## one frame should await only `process_frame` directly. See
## `test_process_pushes_this_frames_new_source_waves_into_the_materials` for
## the guard that depends on this distinction.
func _one_frame() -> void:
	await get_tree().process_frame
	await get_tree().physics_frame


## Whether a source's FIRST-EVER appointment has already arrived by `now`.
## `next_emit()` itself is a trait method with no `#[func]` surface, so this
## derives from the cadence knob instead, which is exported and readable off
## any source's class: a fresh `Cadence` gate books its first wave exactly
## one interval out (`Cadence::every`, sound_source.rs, pinned there by
## `one_beat_per_cadence`), and a non-positive cadence never fires at all
## (the same gate's own refusal). Valid only for a level's FIRST tick ever —
## which is exactly what the test below drives.
func _due_at(source: Node3D, now: float) -> bool:
	var cadence: float = source.get("cadence")
	return cadence > 0.0 and cadence <= now


## Two real process frames: `now` strictly increases both times, and every
## one of the five wave materials carries `u_time` equal to `now` — the
## push `process()` makes each frame, not a value set once at boot and
## never touched again.
func test_two_process_frames_advance_now_and_set_u_time_on_all_five_mats() -> void:
	var game := _game()
	assert_float(game.now).is_equal(0.0)
	await _one_frame()
	var after_first: float = game.now
	assert_bool(after_first > 0.0).is_true()
	for mat: ShaderMaterial in game.wave_mats():
		assert_float(mat.get_shader_parameter("u_time")).is_equal(after_first)
	await _one_frame()
	assert_bool(game.now > after_first).is_true()
	for mat: ShaderMaterial in game.wave_mats():
		assert_float(mat.get_shader_parameter("u_time")).is_equal(game.now)


## `capture_env()` carries exactly the nine fields `main.gd::capture_env`
## did, at the values a freshly booted, never-ticked world actually holds
## — the flicker's and the demo tap's own constructor defaults, read back
## rather than mirrored.
func test_capture_env_returns_exactly_the_nine_keys_with_plausible_values() -> void:
	var game := _game()
	var env: Dictionary = game.capture_env()
	assert_int(env.size()).is_equal(9)
	for key: String in [
		"now",
		"demo_checked",
		"demo_armed",
		"demo_next",
		"flicker_t",
		"flicker_level",
		"flicker_drop_until",
		"flicker_next_drop",
		"flicker_rng_state",
	]:
		assert_bool(env.has(key)).is_true()
	assert_float(env["now"]).is_equal(0.0)
	assert_bool(env["demo_checked"]).is_false()
	assert_bool(env["demo_armed"]).is_false()
	assert_float(env["demo_next"]).is_equal(0.6)
	assert_float(env["flicker_t"]).is_equal(0.0)
	assert_float(env["flicker_level"]).is_equal(1.0)
	assert_float(env["flicker_drop_until"]).is_equal(-1.0)
	assert_float(env["flicker_next_drop"]).is_equal(9.0)
	assert_int(typeof(env["flicker_rng_state"])).is_equal(TYPE_INT)


## `apply_env` is the exact write side of `capture_env`: capturing, moving
## `now` far away, applying the FIRST capture back and capturing again
## lands on the same nine values — nothing in the pair loses or invents a
## field.
func test_apply_env_of_capture_env_round_trips() -> void:
	var game := _game()
	var first: Dictionary = game.capture_env()
	game.now = 42.0
	game.apply_env(first)
	var second: Dictionary = game.capture_env()
	assert_dict(second).is_equal(first)


## A native caller can bypass the blob parser and hand `apply_env` every f64
## directly. The boundary repairs the whole temporal group before capture can
## expose poison, warns once, and suppresses repeats from a persistently bad
## caller rather than flooding every frame.
func test_apply_env_repairs_poison_and_warns_only_once() -> void:
	var game := _game()
	var poisoned: Dictionary = game.capture_env()
	poisoned["now"] = NAN
	poisoned["demo_next"] = INF
	poisoned["flicker_t"] = -INF
	poisoned["flicker_level"] = NAN
	poisoned["flicker_drop_until"] = INF
	poisoned["flicker_next_drop"] = -1.0
	var warning := "UnseeingGame: repaired invalid temporal input; further temporal repairs are silent"
	var first := func() -> void: game.apply_env(poisoned)
	await assert_error(first).is_push_warning(warning)

	var repaired: Dictionary = game.capture_env()
	assert_float(repaired["now"]).is_equal(0.0)
	assert_float(repaired["demo_next"]).is_equal(0.6)
	assert_float(repaired["flicker_t"]).is_equal(0.0)
	assert_float(repaired["flicker_level"]).is_equal(1.0)
	assert_float(repaired["flicker_drop_until"]).is_equal(-1.0)
	assert_float(repaired["flicker_next_drop"]).is_equal(9.0)

	var repeated := func() -> void: game.apply_env(poisoned)
	await assert_error(repeated).is_success()


## A world with a life behind it: ticked, looked, tapped, and a wave still
## queued — `restore_transaction_test.gd`'s `_boot_ticked`/`_lively`
## fixture recipe, adapted for the bare `UnseeingGame` this suite
## constructs directly rather than through `main.tscn`. The hero looks
## DOWN before tapping (a level swing strikes nothing) and the wave is
## queued LAST, with no frame after it, for the same reasons that suite's
## own doc comment gives.
func _lively_game() -> UnseeingGame:
	var game := _game()
	await _one_frame()
	game.now += 1.0
	await _one_frame()
	game.player.look(Vector2(0.0, 100.0))
	game.player.tap()
	for _i in 2:
		await _one_frame()
	game.player.queue_wave(2, Vector3(2.5, 0.5, 3.25), 6.25, 5.5, 0.75, 3, Vector3.UP)
	return game


## `restore_blob` of a blob captured from a live world restores cleanly —
## its own freshly-computed hash echoed back in the verdict — and the same
## blob with a doctored `hash` key is refused with a one-key `unavailable`
## naming both numbers. `restore_transaction_test.gd`'s round-trip and
## lying-hash assertions, against a directly-built root rather than one
## loaded through `main.tscn`.
func test_restore_blob_restores_a_fresh_capture_and_refuses_a_doctored_hash() -> void:
	var game := await _lively_game()
	var blob: Dictionary = game.observer.capture(game.now, game.capture_env())
	assert_bool(blob.has("unavailable")).is_false()
	for _i in 5:
		await _one_frame()
	var verdict: Dictionary = game.restore_blob(blob)
	assert_str(str(verdict.get("unavailable", ""))).is_empty()
	assert_str(verdict["hash"]).is_equal(blob["hash"])

	var doctored: Dictionary = blob.duplicate(true)
	var honest: String = doctored["hash"]
	doctored["hash"] = "0000000000000000"
	var refused: Dictionary = game.restore_blob(doctored)
	assert_bool(refused.has("unavailable")).is_true()
	assert_str(refused["unavailable"]).contains("stored 0000000000000000")
	assert_str(refused["unavailable"]).contains("restored %s" % honest)


## MUTATION GUARD: `process()` must feed `level.tick_sources()` the
## CAMERA's position, never the player's own BODY — the two differ by the
## head-bob offset, and here, deliberately, by much more. The code-built
## fixture puts one divider between its spawn and fan. Relocating the camera
## into the fan's side removes that wall for the eye alone. Both multipliers are
## computed through the level's own `source_muffle` oracle rather than
## assumed, so the fixture proves it discriminates before the real
## assertion leans on it. If `process()` ever fed the body's position
## instead, the fan would render exactly as muffled as it does from the
## spawn, and the final assertion would fail.
func test_process_feeds_tick_sources_the_camera_not_the_body() -> void:
	var game := _game(true, true)
	# The fixture asks for exactly one fan; class lookup keeps this assertion
	# about the source contract rather than its authored node name.
	var fan: Node3D = null
	for source: Node3D in game.level.sources():
		if source.is_class("SoundFan"):
			fan = source
			break
	if fan == null:
		fail("the code-built signal fixture carries no SoundFan")
		return
	var hub: Vector3 = fan.global_position
	var body_at: Vector3 = game.player.global_position
	game.player.camera.global_position = hub + Vector3(0.3, 0.0, 0.0)  # the fan's own room
	var eye_at: Vector3 = game.player.camera.global_position
	var eye_muffle: float = game.level.source_muffle(eye_at, hub)
	var body_muffle: float = game.level.source_muffle(body_at, hub)
	assert_bool(body_muffle < eye_muffle).is_true()  # the fixture must actually discriminate
	await _one_frame()
	var fan_entry: Dictionary
	for s: Dictionary in game.observer.snapshot(game.now)["sources"]:
		if s["name"] == str(fan.name):
			fan_entry = s
	# the muffle the level pushed must be measured from the EYE, not from
	# the body: the fixture above proves the two differ, so a muffle taken
	# from the wrong point reads body_muffle here and fails
	var pushed_muffle: float = fan_entry["source_muffle"]
	var pushed_volume: float = fan_entry["source_volume"]
	var voice_volume: float = fan_entry["volume"]
	assert_float(pushed_muffle).is_equal_approx(eye_muffle, 0.001)
	assert_float(pushed_volume).is_equal_approx(voice_volume, 0.001)


## MUTATION GUARD: the apply loop (`WaveCore.tick` plus the u_count/u_ppos/
## u_pdat/u_pdir push) must run AFTER `level.tick_sources` and the cat
## loop, in the SAME `process()` frame — a source whose appointment first
## comes due THIS frame must already be counted in this frame's materials,
## not next frame's. Adapted from `source_test.gd`'s
## `test_one_tick_drives_every_source_on_its_own_voice`: the pool is
## provably empty going into this one frame — boot alone never ticks
## anything — so `u_count` must equal exactly the sources whose first
## appointment (its own cadence — `_due_at` above) has already arrived by
## the time this frame's `now` lands. The explicit fixture source makes the
## guard non-vacuous without borrowing the shipped map's current census.
##
## Deliberately NOT `_one_frame()`: that helper awaits `process_frame` THEN
## `physics_frame`, and measured here (`game.now` moves between the two
## awaits) the second wait spans a SECOND `process()` call — which would
## silently absorb a one-frame-stale apply loop into a merely one-frame-
## late-but-still-eventually-2 count and defeat this exact guard. One
## `process_frame` await is exactly one `process()` call, pinned by
## `test_two_process_frames_advance_now_and_set_u_time_on_all_five_mats`
## already showing `now`/`u_time` moving once per await.
func test_process_pushes_this_frames_new_source_waves_into_the_materials() -> void:
	var game := _game(true, true)
	game.now = 1.0
	await get_tree().process_frame
	var due := 0
	for source: Node3D in game.level.sources():
		if _due_at(source, game.now):
			due += 1
	# non-vacuity: a missing fixture source would pass a u_count of 0 just as
	# cleanly as a healthy one
	assert_int(due).is_greater(0)
	for mat: ShaderMaterial in game.wave_mats():
		var count: int = mat.get_shader_parameter("u_count")
		assert_int(count).is_equal(due)
