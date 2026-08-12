extends SceneTree
## The slab-drawing law (level_plan::slab_drawn), proved against a REAL
## engine in BOTH modes — the only instrument that can reach the editor
## branch at all.
##
## WaveLevel builds a floor and a ceiling slab, and hides the ceiling when
## Engine.is_editor_hint() is true, so a designer can lay a map out from
## directly above instead of through a 28 x 28 m opaque lid. The gdUnit4
## suite cannot see that branch: Godot exposes is_editor_hint() but no
## setter — not to GDScript, not in the gdext bindings — so a headless
## suite is always in run mode, and deleting the line that applies the law
## left every test green.
##
## Launching the ENGINE in editor mode is the way in, and it works
## headlessly with the stock pinned binary:
##
##     godot --headless --path game -e -s res://tests/probe/editor_slab_probe.gd
##
## Engine STATE, not pixels — the bodies the level built, their order, their
## placement and their visibility — so unlike the windowed probes next door
## this one is headless, deterministic, and a real ci/pipeline.sh gate. Run
## by tools/probe_editor_slabs.sh, which runs it in both modes and requires
## both verdicts.
##
## The law is asserted for whichever mode the probe finds itself in, so one
## script covers both directions. The runner checks the mode line it prints,
## because a probe that silently ran in run mode twice would assert nothing
## about the editor at all.

## The level's ground plan for this probe. The placements below are derived
## from it by hand: center (10, 10) from the 20 x 20 extents, the floor's
## top exactly at y = 0 (SLAB_T 0.1 -> center -0.05) and the ceiling's
## underside exactly at wall height (WALL_H 3.0 -> center 3.05).
const EXTENTS := Vector2(20, 20)
const FLOOR_AT := Vector3(10, -0.05, 10)
const CEILING_AT := Vector3(10, 3.05, 10)
## Frames to allow the level's deferred _ready to build its limbs. A
## SceneTree script's _initialize() runs BEFORE the _ready of a node it
## added, so the slabs are not there on frame one. Polled for, never waited
## out: the budget only bounds how long a MISSING body takes to be reported.
const READY_FRAMES := 30

var _level: Node3D = null
var _frames := 0
var _checks := 0
var _failed := 0


func _initialize() -> void:
	if not ClassDB.class_exists("WaveLevel"):
		_check("the Rust extension is loaded (see .godot/extension_list.cfg)", false)
		_report()
		return
	_level = ClassDB.instantiate("WaveLevel") as Node3D
	_level.set("extents", EXTENTS)
	var marker := WaveSpawn.new()
	_level.add_child(marker)
	_level.call("inject", ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	root.add_child(_level)


func _process(_delta: float) -> bool:
	if _level == null:
		return true
	_frames += 1
	var slab_floor := _level.get_node_or_null("WaveFloor") as Node3D
	var lid := _level.get_node_or_null("WaveCeiling") as Node3D
	# poll for the condition, not for a duration — the slabs appear on the
	# frame the level's _ready runs, whenever that is
	if (slab_floor == null or lid == null) and _frames < READY_FRAMES:
		return false
	_judge(slab_floor, lid)
	_report()
	return true


## The whole law, asserted for the mode this process is actually in.
func _judge(slab_floor: Node3D, lid: Node3D) -> void:
	var editor := Engine.is_editor_hint()
	print("# slabs: mode=%s" % ("editor" if editor else "run"))
	# hidden, never skipped: the pair is what the extents knob reshapes,
	# what the object-id colouring anchors on and what the seam census
	# reports, so a level carrying one slab in the editor and two in the
	# game would answer all three differently without saying so
	_check("the level built a floor slab", slab_floor != null)
	_check("the level built a ceiling slab (hidden in the editor, never skipped)", lid != null)
	if slab_floor == null or lid == null:
		return
	print(
		(
			"# slabs: WaveFloor visible=%s pos=%s ; WaveCeiling visible=%s pos=%s"
			% [slab_floor.visible, slab_floor.position, lid.visible, lid.position]
		)
	)
	_check("floor and ceiling are built in that order", slab_floor.get_index() < lid.get_index())
	_check("the floor draws — it is the ground plane a map is placed against", slab_floor.visible)
	_check(
		(
			"the ceiling draws in the game and not in the editor (here: %s)"
			% ("hidden" if not lid.visible else "drawn")
		),
		lid.visible == (not editor)
	)
	# geometry is mode-independent: hiding the lid must not move or resize
	# anything, or the editor would be laying out a different map
	_check("the floor stands at %s" % FLOOR_AT, slab_floor.position.is_equal_approx(FLOOR_AT))
	_check("the ceiling stands at %s" % CEILING_AT, lid.position.is_equal_approx(CEILING_AT))


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
	quit(1 if _failed > 0 else 0)
