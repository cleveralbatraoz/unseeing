extends SceneTree
## Editor-mode law for sound sources and the cat: placed in the editor
## they BUILD their blueprint limbs with no injection; placed at run
## time uninjected they build NOTHING (the runtime guard still holds).
## Runs twice from tools/probe_editor_sources.sh: once with -e, once
## without. Each run proves its mode before judging.

const READY_FRAMES := 30

var _fan: Node3D = null
var _radio: Node3D = null
var _cat: CharacterBody3D = null
var _cat_born_at := Vector3.ZERO
## The physics clock reading right after the cat is born — the mark the
## settle wait polls against, so "several physics ticks elapsed" is a
## measured fact rather than a guessed frame count.
var _cat_physics_mark := 0
var _frames := 0
var _checks := 0
var _failed := 0


func _initialize() -> void:
	if (
		not ClassDB.class_exists("SoundFan")
		or not ClassDB.class_exists("SoundRadio")
		or not ClassDB.class_exists("WaveCat")
	):
		_check("the Rust extension is loaded (see .godot/extension_list.cfg)", false)
		_report()
		return
	_fan = ClassDB.instantiate("SoundFan") as Node3D
	root.add_child(_fan)
	_radio = ClassDB.instantiate("SoundRadio") as Node3D
	root.add_child(_radio)
	_cat = ClassDB.instantiate("WaveCat") as CharacterBody3D
	root.add_child(_cat)
	_cat_born_at = _cat.position
	_cat_physics_mark = Engine.get_physics_frames()


func _process(_delta: float) -> bool:
	_frames += 1
	var editor := Engine.is_editor_hint()
	var still_building := (
		editor
		and (
			(_fan != null and _fan.get_child_count() == 0)
			or (_radio != null and _radio.get_child_count() == 0)
			or (_cat != null and _cat.get_child_count() == 0)
		)
	)
	# Condition-based, not a guessed duration: keep polling the REAL physics
	# clock until it has actually advanced since the cat was born, so a cat
	# that still ticks in the editor gets the CHANCE to move before its
	# stillness checks are judged — counting idle _process() iterations as a
	# proxy would prove nothing about physics frames actually elapsing.
	# READY_FRAMES bounds the whole wait (build + settle), so a stuck build
	# or a stalled physics loop fails loudly instead of spinning forever.
	var cat_unsettled := (
		editor and _cat != null and Engine.get_physics_frames() - _cat_physics_mark < 2
	)
	if (still_building or cat_unsettled) and _frames < READY_FRAMES:
		return false
	if _fan == null or _radio == null or _cat == null:
		return true
	_judge()
	_report()
	return true


func _judge() -> void:
	var editor := Engine.is_editor_hint()
	print("# sources: mode=%s" % ("editor" if editor else "run"))
	_judge_fan(editor)
	_judge_radio(editor)
	_judge_cat(editor)


func _judge_fan(editor: bool) -> void:
	var pedestal := _fan.get_node_or_null("FanPedestal")
	var pivot := _fan.get_node_or_null("FanPivot")
	if editor:
		_check("editor: the fan builds its pedestal", pedestal != null)
		_check("editor: the fan builds its pivot head", pivot != null)
	else:
		_check("run uninjected: the fan builds nothing", _fan.get_child_count() == 0)


func _judge_radio(editor: bool) -> void:
	var case := _radio.get_node_or_null("RadioCase")
	var grille := _radio.get_node_or_null("RadioGrille")
	var antenna := _radio.get_node_or_null("RadioAntenna")
	if editor:
		_check("editor: the radio builds its case", case != null)
		_check("editor: the radio builds its grille", grille != null)
		_check("editor: the radio builds its antenna", antenna != null)
	else:
		_check("run uninjected: the radio builds nothing", _radio.get_child_count() == 0)


func _judge_cat(editor: bool) -> void:
	var collider := _cat.get_node_or_null("CatCollider")
	var skin := _cat.get_node_or_null("CatSkin") as MeshInstance3D
	if editor:
		_check("editor: the cat builds its collider", collider != null)
		_check("editor: the cat builds its skin", skin != null)
		if skin != null:
			_check("editor: the cat skin has a mesh surface", skin.mesh.get_surface_count() >= 1)
			_check("editor: the cat skin rides the node, not the world", not skin.top_level)
		_check("editor: the cat does not tick", not _cat.is_physics_processing())
		# Defense in depth, not the primary guard: physics_process()
		# (cat.rs) bails immediately whenever brain/gait/tail are None,
		# which is exactly what build_editor_pose() leaves them as — so
		# today this check would pass even with processing left on. It
		# catches the day a future change BOTH previews something (e.g.
		# mood) by populating those fields in the editor build AND leaves
		# processing enabled; "does not tick" above is what actually holds
		# the line today.
		_check("editor: the cat has not moved", _cat.position.is_equal_approx(_cat_born_at))
	else:
		_check("run uninjected: the cat builds nothing", _cat.get_child_count() == 0)


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
