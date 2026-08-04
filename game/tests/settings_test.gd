extends GdUnitTestSuite
## The settings overlay, driven end to end against the LIVE main scene:
## Escape raises it, the world freezes behind it, the rows read the window
## that actually exists, and nothing the player presses while it is up can
## reach the hero. Pins BEHAVIOUR, not pixels — the geometry math is
## cargo-pinned in display_plan, the row text in settings_menu.
##
## Headless has no monitor (zero screens) and no mouse capture, so nothing
## here asserts on the display server or the cursor: what CI can prove is
## the overlay's own state, the pause it owns, and the input it swallows.

const MAIN_SCENE := preload("res://scenes/main.tscn")

var _main: UnseeingMain


func before_test() -> void:
	_main = auto_free(MAIN_SCENE.instantiate() as UnseeingMain)
	add_child(_main)


func after_test() -> void:
	# never strand the tree frozen for the next suite, whatever failed
	get_tree().paused = false


## Escape, the way a keyboard sends it.
func _escape() -> void:
	var key := InputEventKey.new()
	key.keycode = KEY_ESCAPE
	key.pressed = true
	get_viewport().push_input(key)


## Any of the overlay's navigation keys.
func _press(keycode: Key) -> void:
	var key := InputEventKey.new()
	key.keycode = keycode
	key.pressed = true
	get_viewport().push_input(key)


## The composition root places the overlay, and it starts out of the way:
## a game that boots into its own settings menu is a broken game.
func test_the_overlay_is_placed_and_closed_at_boot() -> void:
	assert_object(_main.settings).is_not_null()
	assert_bool(_main.settings.is_open()).is_false()
	assert_bool(get_tree().paused).is_false()


## Escape raises the overlay and freezes the world behind it.
func test_escape_raises_the_overlay_and_freezes_the_world() -> void:
	_escape()
	assert_bool(_main.settings.is_open()).is_true()
	assert_bool(get_tree().paused).is_true()


## Escape again puts it away and thaws the world.
func test_escape_again_closes_it_and_thaws_the_world() -> void:
	_escape()
	_escape()
	assert_bool(_main.settings.is_open()).is_false()
	assert_bool(get_tree().paused).is_false()


## The rows read as designed, and the resolution row names the monitor's
## own resolution — the project's viewport standing in for it headlessly.
func test_the_rows_read_the_window() -> void:
	_escape()
	assert_int(SettingsMenu.row_count()).is_equal(2)
	assert_str(_main.settings.row_label(0)).is_equal("FULLSCREEN")
	assert_str(_main.settings.row_label(1)).is_equal("RESOLUTION")
	assert_str(_main.settings.row_value(1)).contains("NATIVE")
	# the cursor opens on the first row, and only it wears the brackets
	assert_int(_main.settings.cursor_row()).is_equal(0)
	assert_str(_main.settings.row_value(0)).starts_with("<")
	assert_str(_main.settings.row_value(1)).not_contains("<")


## Down walks the cursor to the resolution row and the brackets follow.
func test_down_walks_the_cursor_to_the_resolution_row() -> void:
	_escape()
	_press(KEY_DOWN)
	assert_int(_main.settings.cursor_row()).is_equal(1)
	assert_str(_main.settings.row_value(1)).starts_with("<")
	assert_str(_main.settings.row_value(0)).not_contains("<")


## The full-screen row toggles, and the row's text says so.
func test_the_full_screen_row_toggles() -> void:
	_escape()
	var was: bool = _main.settings.wants_fullscreen()
	_press(KEY_RIGHT)
	assert_bool(_main.settings.wants_fullscreen()).is_equal(not was)
	assert_str(_main.settings.row_value(0)).contains("OFF" if was else "ON")
	_press(KEY_LEFT)
	assert_bool(_main.settings.wants_fullscreen()).is_equal(was)


## A left click, the way the mouse sends it.
func _click() -> void:
	var click := InputEventMouseButton.new()
	click.button_index = MOUSE_BUTTON_LEFT
	click.pressed = true
	get_viewport().push_input(click)


## The control for the isolation test below: with the overlay away, a
## click DOES tap the cane in this very scene. Without this, the isolation
## test could pass because clicks never worked here at all.
func test_a_click_with_the_overlay_away_taps_the_cane() -> void:
	await get_tree().physics_frame
	_click()
	await get_tree().physics_frame
	await get_tree().physics_frame
	assert_float(_main.player.last_tap).is_greater(-1.0)


## THE isolation law: while the overlay is up the world is frozen, so the
## player never hears the click at all — it is not deferred, it is never
## queued. Closing the overlay and running on proves it: the tap does not
## arrive late.
func test_a_click_while_the_overlay_is_up_never_taps_the_cane() -> void:
	await get_tree().physics_frame
	_escape()
	_click()
	await get_tree().process_frame
	assert_float(_main.player.last_tap).is_equal(-10.0)
	# and it was never merely postponed: thaw, run on, still no tap
	_escape()
	await get_tree().physics_frame
	await get_tree().physics_frame
	assert_float(_main.player.last_tap).is_equal(-10.0)


## The overlay BORROWS the pause, it does not own it. A world that was
## already frozen when the player opened the settings is still frozen when
## they close them — whatever froze it still means to.
func test_the_overlay_puts_back_the_pause_it_found() -> void:
	get_tree().paused = true
	_escape()
	assert_bool(_main.settings.is_open()).is_true()
	_escape()
	assert_bool(_main.settings.is_open()).is_false()
	assert_bool(get_tree().paused).is_true()


## An overlay freed while open must not strand the tree frozen — a suite
## tearing down its scene mid-menu would otherwise pause everything that
## runs after it.
func test_freeing_the_overlay_while_open_thaws_the_tree() -> void:
	_escape()
	assert_bool(get_tree().paused).is_true()
	_main.free()
	_main = null
	assert_bool(get_tree().paused).is_false()
