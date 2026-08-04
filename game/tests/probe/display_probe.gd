extends Node
## Regression probe for the display defaults and the settings overlay,
## against a REAL window — the only place this can be proved. Headless has
## no screens at all, so CI can pin the overlay's state and its text but
## never that the window actually moved.
##
## It reproduces the bug that made this probe necessary: toggling full
## screen WHILE THE GAME IS STILL SLIDING INTO IT. macOS drops a full-screen
## toggle issued during another full-screen transition, and its own
## delegate then writes the old mode back over the one Godot recorded — so
## the request vanished with no error and the overlay was left describing a
## window that did not exist. The toggle here is therefore deliberately
## early, ~20 frames after boot, and the probe waits for the overlay's
## re-assertion to win.
##
## Run by tools/probe_display.sh, NOT in headless CI, and NOT under
## probe_visibility.sh — that one forces a windowed override.cfg, which is
## exactly what this probe must not have.
const MAIN := preload("res://scenes/main.tscn")

## Frames to wait for a window transition to settle. The overlay insists
## for ~240 frames; this outlasts it.
const SETTLE := 150

var _checks := 0
var _failed := 0


func _ready() -> void:
	var main: UnseeingMain = MAIN.instantiate() as UnseeingMain
	add_child(main)
	await _frames(20)
	var menu: SettingsMenu = main.settings
	var screen := DisplayServer.screen_get_size(0)
	var usable := DisplayServer.screen_get_usable_rect(0)
	print("# display: screen=%s usable=%s" % [str(screen), str(usable)])

	# 1 — the defaults: full screen, at the monitor's own resolution
	_check("boots full screen", _is_fullscreen())
	_check("boots at the monitor's own resolution", DisplayServer.window_get_size() == screen)
	_check(
		"the viewport IS the window, so native needs no scaling",
		get_viewport().get_visible_rect().size == Vector2(screen)
	)

	# 2 — the overlay opens, freezes the world and reads the truth
	_key(KEY_ESCAPE)
	await _frames(2)
	_check("Escape raises the overlay", menu.is_open())
	_check("the world freezes behind it", get_tree().paused)
	_check("the full-screen row reads ON", menu.row_value(0).contains("ON"))
	_check("the resolution row names the monitor", menu.row_value(1).contains(str(screen.x)))

	# 3 — toggling OFF actually moves the window, mid-transition and all
	_key(KEY_RIGHT)
	await _frames(SETTLE)
	print(
		(
			"# display: windowed -> mode=%d size=%s"
			% [DisplayServer.window_get_mode(), str(DisplayServer.window_get_size())]
		)
	)
	_check("the model wants windowed", not menu.wants_fullscreen())
	_check("the window really left full screen", not _is_fullscreen())
	var size := DisplayServer.window_get_size()
	_check(
		"it fits the usable area, title bar included",
		size.x <= usable.size.x and size.y <= usable.size.y
	)
	_check("it did not keep the monitor's size", size != screen)
	var frame := DisplayServer.window_get_position_with_decorations()
	_check(
		"its frame sits inside the usable area",
		frame.x >= usable.position.x and frame.y >= usable.position.y
	)

	# 4 — and back again
	_key(KEY_RIGHT)
	await _frames(SETTLE)
	_check("toggling back returns to full screen", _is_fullscreen())
	_check("and to the monitor's own resolution", DisplayServer.window_get_size() == screen)

	# 5 — Escape closes and thaws
	_key(KEY_ESCAPE)
	await _frames(3)
	_check("Escape closes the overlay", not menu.is_open())
	_check("the world thaws", not get_tree().paused)

	# 6 — and the overlay lets go: its insistence is BOUNDED, so a platform
	# that refuses is never fought forever and a settled window is left alone
	for _i: int in 400:
		if menu.enforce_left() == 0:
			break
		await get_tree().process_frame
	_check("the overlay stops insisting once the window has settled", menu.enforce_left() == 0)
	_check("and it left the window full screen", _is_fullscreen())
	_report()


func _is_fullscreen() -> bool:
	var mode := DisplayServer.window_get_mode()
	return (
		mode == DisplayServer.WINDOW_MODE_FULLSCREEN
		or mode == DisplayServer.WINDOW_MODE_EXCLUSIVE_FULLSCREEN
	)


func _key(code: Key) -> void:
	var press := InputEventKey.new()
	press.keycode = code
	press.pressed = true
	get_viewport().push_input(press)


func _frames(n: int) -> void:
	for _i: int in n:
		await get_tree().process_frame


func _check(what: String, ok: bool) -> void:
	_checks += 1
	print(("ok %d - %s" if ok else "not ok %d - %s") % [_checks, what])
	if not ok:
		_failed += 1


func _report() -> void:
	print("1..%d" % _checks)
	var verdict := (
		"PASS (%d checks)" % _checks if _failed == 0 else "FAIL (%d of %d)" % [_failed, _checks]
	)
	print("probe: %s" % verdict)
	get_tree().quit(1 if _failed > 0 else 0)
