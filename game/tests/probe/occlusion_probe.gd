extends Node
## Regression probe for the wave-through-wall law, reproducing the report:
## walk UP TO the divider the fan is behind and tap it. Boots main and
## checks:
##   1. SOURCE REVEAL (hearing pass OFF), measured FIRST and before any
##      player sound exists: the fan hums untouched behind the divider and
##      the hero stands in the room beyond it — no tap, no footstep, so a
##      wall must stop the fan's own wave dead. Run before cases 2 and 3
##      on purpose: a player tap fired near the sample points would light
##      them legitimately and swamp a real leak;
##   2. FINAL IMAGE (hearing pass ON): tapping the wall must not brighten
##      the fan behind it — not its shell wash, not its OUTLINE borrowing
##      the lit wall. Sampled on the fan's OUTLINE (guard ring), where the
##      flare showed;
##   3. REVEAL (hearing pass OFF): the same tap — its direct pulse AND its
##      echoes — must not reach the fan's own data reveal. Sampled on the
##      fan's SOLID interior only; the outline points sit at the fan/wall
##      boundary and would read the wall the tap rightly lights.
## Windowed, real GPU; run by tools/probe_visibility.sh, not in headless.
const MAIN := preload("res://scenes/main.tscn")
## The hero right at the divider, looking at the fan behind it.
const AT_WALL := Vector3(5.8, 0.9, 4.0)
const FAN := Vector3(8.6, 1.15, 4.4)
## The fan's OUTLINE — the guard ring is where the borrowed-outline flare
## showed; used for the final-image check (hearing on).
const FAN_EDGE: Array[Vector3] = [
	Vector3(8.6, 1.58, 4.35),  # guard ring top
	Vector3(8.6, 0.72, 4.35),  # guard ring bottom
	Vector3(8.6, 1.15, 4.4),  # motor hub
	Vector3(8.6, 0.6, 4.4),  # pole
]
## The fan's SOLID motor box — reading this in the DATA pass gives the
## fan's own reveal, never the divider around it (the thin pole/base would
## let the pixel tolerance spill onto the lit wall); used for reveal.
const FAN_CORE: Array[Vector3] = [
	Vector3(8.6, 1.15, 4.4),  # motor hub — a solid box, tolerance stays on it
]
const WALL_TAP := Vector3(6.25, 1.5, 4.06)  # the aimed strike on the divider face

## The hero at the spawn marker, in the room WEST of the Divider (x = 6.4,
## whose doorway spans z in [8, 12.4]). The fan at (8.6, 4.4) is east of
## it, and the fan's sight line to every point below crosses SOLID divider
## well clear of that doorway. Nothing the fan emits may light any of them.
const AT_SPAWN := Vector3(3.0, 0.9, 4.0)
## What the hero looks at from the spawn: the divider face, north of the
## tap point so cases 2 and 3's later strike cannot be mistaken for a leak.
const SPAWN_AIM := Vector3(6.25, 1.0, 6.5)
## Spawn-room surfaces the fan must never reveal. Both are chosen so the
## fan-to-point line pierces the divider: (8.6, 4.4) -> (5.0, 6.0) crosses
## x = 6.4 at z ~= 5.38, inside the divider's z in [0.6, 8] run.
const SPAWN_SIDE: Array[Vector3] = [
	Vector3(6.25, 1.5, 6.5),  # the divider's WEST face, fan directly behind it
	Vector3(5.0, 0.0, 6.0),  # spawn-room floor
]

var _checks := 0
var _failed := 0


func _ready() -> void:
	# keep the window on top and foregrounded — an occluded window is
	# throttled by the OS to a frame a second, and the per-frame GPU
	# readback below then starves; on top it renders at full rate.
	DisplayServer.window_set_flag(DisplayServer.WINDOW_FLAG_ALWAYS_ON_TOP, true)
	DisplayServer.window_move_to_foreground()
	var main: UnseeingGame = MAIN.instantiate() as UnseeingGame
	add_child(main)
	await _settle(35)
	Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
	main.hero.visible = false
	# 1 — SOURCE REVEAL, measured before any player sound exists. The fan
	# hums behind the divider and the hero stands in the room beyond it;
	# the only waves in flight are the sources' own. Absolute, not a delta:
	# a continuous source has no "before" to subtract. This runs FIRST
	# because a tap of the hero's own would light this room legitimately.
	main.player.position = AT_SPAWN
	main.player.camera.look_at(SPAWN_AIM, Vector3.UP)
	await _settle(30)
	_set_quad(main, false)
	var leak := await _peak_r(main, SPAWN_SIDE, 26)
	print("# occlusion @spawn: fan lifts spawn-room reveal %.3f" % leak)
	_check(
		"the fan does NOT reveal the spawn room through the divider (%.3f < 0.08)" % leak,
		leak < 0.08
	)
	_set_quad(main, true)
	main.player.position = AT_WALL
	main.player.camera.look_at(FAN, Vector3.UP)
	await _settle(20)
	# 2 — FINAL IMAGE: tapping the wall must not FLARE the fan's outline
	# (the borrowed-outline + shell-wash bug the reporter saw)
	var base_img := await _peak_r(main, FAN_EDGE, 12)
	main.player.queue_wave(0, WALL_TAP, 6.0, 5.5, 1.0, 6, Vector3(-1, 0, 0))
	var flare := await _peak_r(main, FAN_EDGE, 26) - base_img
	await _settle(40)
	# 3 — REVEAL: the same tap (its direct pulse AND its echoes) must not
	# light the fan's own data reveal — read on the fan's solid interior,
	# hearing quad out of the way
	_set_quad(main, false)
	var base_r := await _peak_r(main, FAN_CORE, 12)
	main.player.queue_wave(0, WALL_TAP, 6.0, 5.5, 1.0, 6, Vector3(-1, 0, 0))
	var reveal := await _peak_r(main, FAN_CORE, 26) - base_r
	print(
		"# occlusion @wall: tap flares outline %.3f ; tap lifts fan reveal %.3f" % [flare, reveal]
	)
	_check("tapping the wall does NOT flare the fan behind it (%.3f < 0.12)" % flare, flare < 0.12)
	_check(
		"the tap's reveal (with echoes) does NOT reach the fan (%.3f < 0.08)" % reveal,
		reveal < 0.08
	)
	_report()


## Peak brightness over `pts` (each with a small pixel tolerance, to catch
## a thin outline) across `frames` frames.
func _peak_r(main: UnseeingGame, pts: Array[Vector3], frames: int) -> float:
	var cam := main.player.camera
	var view := cam.get_viewport().get_visible_rect().size
	var peak := 0.0
	for _i: int in frames:
		await get_tree().process_frame
		await RenderingServer.frame_post_draw
		var img := get_viewport().get_texture().get_image()
		for pt: Vector3 in pts:
			var px := cam.unproject_position(pt)
			var cx := roundi(px.x * img.get_width() / view.x)
			var cy := roundi(px.y * img.get_height() / view.y)
			for dy: int in range(-2, 3):
				for dx: int in range(-2, 3):
					var x := clampi(cx + dx, 0, img.get_width() - 1)
					var y := clampi(cy + dy, 0, img.get_height() - 1)
					peak = maxf(peak, img.get_pixel(x, y).r)
	return peak


func _set_quad(main: UnseeingGame, shown: bool) -> void:
	for c: Node in main.player.camera.get_children():
		if c is MeshInstance3D and (c as MeshInstance3D).material_override == main.post_mat:
			(c as MeshInstance3D).visible = shown


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


func _settle(n: int) -> void:
	for _i: int in n:
		await get_tree().process_frame
