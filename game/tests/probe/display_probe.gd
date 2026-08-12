extends Node
## Regression probe for the display defaults and the settings overlay,
## against a REAL window — the only place this can be proved. Headless has
## no screens at all, so CI can pin the overlay's state and its text but
## never that the window actually moved.
##
## It reproduces the bug that made this probe necessary: a full-screen
## toggle sent WHILE ANOTHER FULL-SCREEN TRANSITION IS STILL ANIMATING.
## macOS drops it, and its own delegate then writes the old mode back over
## the one Godot recorded — so the request vanishes with no error and the
## overlay is left describing a window that does not exist. Step 4 provokes
## exactly that, three toggles five frames apart, and then waits for the
## overlay's re-assertion to win. Every other step waits for the window to
## come to REST first: racing the animation measures the animation, and a
## probe whose verdict depends on how busy the window server was is a probe
## that fails for reasons nobody can act on.
##
## Run by tools/probe_display.sh, NOT in headless CI, and NOT under
## probe_visibility.sh — that one forces a windowed override.cfg, which is
## exactly what this probe must not have.
##
## ENVIRONMENT TRAP, measured the hard way: launching a dozen full-screen
## apps back to back wedges the macOS window server. It keeps ACCEPTING the
## request — window_get_mode() answers FULLSCREEN — while the frame never
## grows, so the size checks below fail on a machine that was fine an hour
## earlier, and fail identically on code that passed then. If steps 1 and 4
## report full screen at the OLD window size, suspect the machine before
## the game: close the leftover full-screen spaces (or log out) and run
## again. The probe says so out loud when it sees that shape.
const MAIN := preload("res://scenes/main.tscn")

## Frames to wait for a window transition to settle. The overlay insists
## for ~240 frames; this outlasts it.
const SETTLE := 150

var _checks := 0
var _failed := 0


func _ready() -> void:
	var main: UnseeingGame = MAIN.instantiate() as UnseeingGame
	add_child(main)
	var menu: SettingsMenu = main.settings
	var screen := DisplayServer.screen_get_size(0)
	var usable := DisplayServer.screen_get_usable_rect(0)
	# The macOS full-screen space animates, and how long it takes depends on
	# the machine and on what the window server is already doing. Wait for
	# the boot transition to COME TO REST before judging the defaults —
	# racing it would only measure the animation. The mid-transition case
	# this probe exists for is provoked deliberately in step 4 instead.
	await _settled(screen)
	print("# display: screen=%s usable=%s" % [str(screen), str(usable)])

	# 1 — the defaults: full screen, at the monitor's own resolution
	_check("boots full screen", _is_fullscreen())
	if _is_fullscreen() and DisplayServer.window_get_size() != screen:
		print(
			(
				(
					"# display: NOTE the window server accepted full screen but never "
					+ "resized the frame (%s, wanted %s). That is the wedged-window-server "
					+ "state, not the game — close leftover full-screen spaces and re-run."
				)
				% [str(DisplayServer.window_get_size()), str(screen)]
			)
		)
	_check(
		"boots at the monitor's own resolution (%s)" % str(DisplayServer.window_get_size()),
		DisplayServer.window_get_size() == screen
	)
	_check(
		(
			"the viewport IS the window, so native needs no scaling (%s)"
			% str(get_viewport().get_visible_rect().size)
		),
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

	# 4 — THE REGRESSION: toggle again while the last transition is still
	# animating. macOS drops a full-screen toggle sent during another
	# full-screen transition and then writes the old mode back behind us, so
	# a menu that asked once would be left describing a window that does not
	# exist. Ask three times, five frames apart, and the window must still
	# end where the model says it does.
	_key(KEY_RIGHT)
	await _frames(5)
	_key(KEY_RIGHT)
	await _frames(5)
	_key(KEY_RIGHT)
	await _quiet(menu)
	await _settled(screen)
	print(
		(
			"# display: after churn -> mode=%d size=%s wants_fs=%s"
			% [
				DisplayServer.window_get_mode(),
				str(DisplayServer.window_get_size()),
				str(menu.wants_fullscreen())
			]
		)
	)
	_check("rapid toggling leaves the window where the model says", _is_fullscreen())
	_check(
		"and at the monitor's own resolution (%s)" % str(DisplayServer.window_get_size()),
		DisplayServer.window_get_size() == screen
	)

	# 5 — Escape closes and thaws. The overlay has already let go by now:
	# _quiet above waited for it, which is the BOUNDED-insistence pin — a
	# platform that refuses is never fought forever, and a settled window is
	# left alone for the player to drag.
	_check("the overlay stops insisting once the window has settled", menu.enforce_left() == 0)
	_key(KEY_ESCAPE)
	await _frames(3)
	_check("Escape closes the overlay", not menu.is_open())
	_check("the world thaws", not get_tree().paused)
	_report()


## Wait for the window to stop moving: full screen at the monitor's size.
## The macOS transition is animated and window_get_mode() flips to
## full screen the instant it is ASKED, while window_get_size() only catches
## up when the animation ends — so the size is what says "at rest".
func _settled(screen: Vector2i) -> void:
	for i: int in 600:
		if _is_fullscreen() and DisplayServer.window_get_size() == screen:
			return
		if i % 120 == 119:
			print(
				(
					"# display: still settling after %d frames — mode=%d size=%s"
					% [i + 1, DisplayServer.window_get_mode(), str(DisplayServer.window_get_size())]
				)
			)
		await get_tree().process_frame


## Wait for the overlay to stop re-asserting its plan — bounded, so a stuck
## platform ends the wait rather than hanging the probe.
func _quiet(menu: SettingsMenu) -> void:
	for _i: int in 900:
		if menu.enforce_left() == 0:
			return
		await get_tree().process_frame


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
