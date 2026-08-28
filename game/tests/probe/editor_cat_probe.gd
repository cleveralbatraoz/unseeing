extends SceneTree
## Editor-mode regression law for WaveCat's ancestor-placement warning
## (deterministic-rotation-wire review follow-up, issue #64/#82):
## `check_ancestor_placement` used to run only once, from `ready()`. A
## designer who rotated an ancestor while editing saw the warning triangle
## on the NEXT scene load, but fixing the rotation never cleared it without
## one — exactly the live-tracking gap `WaveWall`'s own ancestor law never
## had, because `WaveWall::process()` (`wall.rs:136-140,197-199`) already
## polls every editor frame. This probe proves the fix closes the same gap
## for the cat: rotating an ancestor LIVE (no `ready()` re-entry) raises the
## warning within a few editor frames, and un-rotating it clears the warning
## just as live. It also proves the poll's scope: a run-mode cat never
## re-checks after `ready()`, so the same live rotation leaves a run-mode
## cat's warning untouched — `check_ancestor_placement` in `process()` is
## gated on `Engine.is_editor_hint()`, not "always on".
##
## Godot exposes `is_editor_hint()` but no setter — not to GDScript, not in
## the gdext bindings — so a headless gdUnit4 suite is always in run mode
## and the editor branch is invisible to it. Launching the ENGINE with `-e`
## is the only way in; tools/probe_editor_cat.sh runs this script in both
## modes and requires both verdicts.

enum Phase { WAIT_FOR_READY, WAIT_FOR_WARNING, WAIT_FOR_CLEAR, WAIT_FOR_RUN_SILENCE }

const READY_FRAMES := 30
## The condition-watch runs on the ENGINE's own frame cadence: WaveCat's
## `process()` override has to actually fire before a rotated ancestor can
## show up anywhere. Budgeted, not awaited forever, so a regression that
## stopped the poll fails loudly instead of hanging the gate.
const WATCH_FRAMES := 30

var _room: Node3D = null
var _cat: Node3D = null
var _frames := 0
var _phase := Phase.WAIT_FOR_READY
var _checks := 0
var _failed := 0


func _initialize() -> void:
	if not ClassDB.class_exists("WaveCat"):
		_check("the Rust extension is loaded (see .godot/extension_list.cfg)", false)
		_report()
		return
	_room = Node3D.new()
	_room.name = "Room"
	_cat = ClassDB.instantiate("WaveCat") as Node3D
	_cat.name = "LiveCat"
	_cat.set("pulses", ClassDB.instantiate("Pulses"))
	_cat.set("data_mat", ShaderMaterial.new())
	_room.add_child(_cat)
	# Identity ancestor at scene load: `ready()`'s own check must stay
	# silent, exactly like every shipped cat (level_01.tscn's cat sits at
	# level root).
	root.add_child(_room)


func _process(_delta: float) -> bool:
	if _cat == null:
		return true
	_frames += 1
	var done := true
	match _phase:
		Phase.WAIT_FOR_READY:
			done = _wait_for_ready()
		Phase.WAIT_FOR_WARNING:
			done = _wait_for_warning()
		Phase.WAIT_FOR_CLEAR:
			done = _wait_for_clear()
		Phase.WAIT_FOR_RUN_SILENCE:
			done = _wait_for_run_silence()
	return done


## Phase 0: poll for `ready()` having actually run (its generated collider
## is the cheapest observable witness), then judge the identity-ancestor
## silence every mode must start from before rotating the ancestor live —
## with no further help from `ready()`, no `rederive()`-equivalent call,
## and no scene reload.
func _wait_for_ready() -> bool:
	var built := _cat.get_child_count() > 0
	if not built and _frames < READY_FRAMES:
		return false
	var editor := Engine.is_editor_hint()
	print("# cat: mode=%s" % ("editor" if editor else "run"))
	var silent_at_load := (_cat.call("get_configuration_warnings") as PackedStringArray).is_empty()
	_check(
		(
			"%s: an untransformed ancestor stays silent at scene load"
			% ("editor" if editor else "run")
		),
		silent_at_load
	)
	_room.rotation = Vector3(0.0, PI / 2.0, 0.0)
	_frames = 0
	if editor:
		_phase = Phase.WAIT_FOR_WARNING
	else:
		_phase = Phase.WAIT_FOR_RUN_SILENCE
	return false


## Phase 1 (editor only): the ancestor was just rotated PI/2 about Y with NO
## `ready()` re-entry and no scene reload — `process()`'s own poll, gated on
## `is_editor_hint()`, is the only thing left that could notice. Poll the
## warning icon until the ancestor-rotation message appears or the budget
## lapses, then repair the ancestor and move on to watch the CLEAR.
func _wait_for_warning() -> bool:
	var warnings := _cat.call("get_configuration_warnings") as PackedStringArray
	var raised := _has(warnings, "ancestor 'Room' does not")
	if not raised and _frames < WATCH_FRAMES:
		return false
	_check("editor: rotating an ancestor live raises the warning with no ready() re-entry", raised)
	_room.rotation = Vector3.ZERO
	_frames = 0
	_phase = Phase.WAIT_FOR_CLEAR
	return false


## Phase 2 (editor only): the ancestor was just repaired back to identity,
## again with no `ready()` re-entry. Poll for the warning to disappear or
## the budget to lapse, then report.
func _wait_for_clear() -> bool:
	var cleared := (_cat.call("get_configuration_warnings") as PackedStringArray).is_empty()
	if not cleared and _frames < WATCH_FRAMES:
		return false
	_check(
		"editor: repairing the ancestor live clears the warning with no ready() re-entry", cleared
	)
	_report()
	return true


## Phase 1 (run only): the ancestor was rotated exactly the same way, but a
## run-mode cat's `process()` never calls `check_ancestor_placement` at all
## — only `ready()` did, once, before the rotation happened. Wait out the
## SAME budget the editor phase polls against and require the warning to
## have stayed silent the whole time, proving the poll is scoped to the
## editor rather than merely slower to react.
func _wait_for_run_silence() -> bool:
	if _frames < WATCH_FRAMES:
		return false
	var stayed_silent := (_cat.call("get_configuration_warnings") as PackedStringArray).is_empty()
	_check("run: a live ancestor rotation is never polled outside the editor", stayed_silent)
	_report()
	return true


func _has(warnings: PackedStringArray, needle: String) -> bool:
	for warning: String in warnings:
		if warning.contains(needle):
			return true
	return false


func _check(what: String, ok: bool) -> void:
	_checks += 1
	if not ok:
		_failed += 1
	print(("ok %d - %s" if ok else "not ok %d - %s") % [_checks, what])


func _report() -> void:
	print("1..%d" % _checks)
	if _failed > 0:
		print("probe: FAIL (%d of %d)" % [_failed, _checks])
	else:
		print("probe: PASS (%d checks)" % _checks)
	quit(1 if _failed > 0 else 0)
