extends GdUnitTestSuite
## UnseeingGame's ready-side wiring contract — the migration's safety net,
## built and asserted BEFORE any old main.gd suite is touched. Constructs
## the class directly, no scene involved, the same way restore_test.gd and
## observer_test.gd build a bare WaveLevel/HeroBody: `UnseeingGame.new()`,
## `add_child`, then synchronous assertions. `_ready` runs INSIDE
## `add_child` (the parent — this suite — is already in the tree), so
## nothing here awaits a frame.
##
## Pins WIRING, not pixels or behaviour: `main.tscn` still boots `main.gd`
## and every existing suite still runs against it unchanged. This suite is
## the only thing in the tree that has ever instantiated `UnseeingGame`.

const DATA_SHADER := preload("res://shaders/data_pass.gdshader")
const XRAY_SHADER := preload("res://shaders/data_xray.gdshader")
const POST_SHADER := preload("res://shaders/hearing_post.gdshader")


func _game() -> UnseeingGame:
	var game: UnseeingGame = auto_free(UnseeingGame.new())
	add_child(game)
	return game


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


## The level is present, injected BEFORE it entered the tree, and derived:
## real wall segments, not an empty table nobody ever handed geometry to
## walk.
func test_level_is_wired_and_derived() -> void:
	var game := _game()
	assert_object(game.level).is_not_null()
	assert_int(game.level.wall_segments().size()).is_greater(0)


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
