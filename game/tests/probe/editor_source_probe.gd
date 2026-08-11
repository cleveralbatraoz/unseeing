extends SceneTree
## Editor-mode law for sound sources and the cat: placed in the editor
## they BUILD their blueprint limbs with no injection; placed at run
## time uninjected they build NOTHING (the runtime guard still holds).
## Runs twice from tools/probe_editor_sources.sh: once with -e, once
## without. Each run proves its mode before judging.

const READY_FRAMES := 30

var _fan: Node3D = null
var _radio: Node3D = null
var _frames := 0
var _checks := 0
var _failed := 0


func _initialize() -> void:
	if not ClassDB.class_exists("SoundFan") or not ClassDB.class_exists("SoundRadio"):
		_check("the Rust extension is loaded (see .godot/extension_list.cfg)", false)
		_report()
		return
	_fan = ClassDB.instantiate("SoundFan") as Node3D
	root.add_child(_fan)
	_radio = ClassDB.instantiate("SoundRadio") as Node3D
	root.add_child(_radio)


func _process(_delta: float) -> bool:
	_frames += 1
	if _frames < READY_FRAMES and Engine.is_editor_hint():
		var fan_empty := _fan != null and _fan.get_child_count() == 0
		var radio_empty := _radio != null and _radio.get_child_count() == 0
		if fan_empty or radio_empty:
			return false
	if _fan == null or _radio == null:
		return true
	_judge()
	_report()
	return true


func _judge() -> void:
	var editor := Engine.is_editor_hint()
	print("# sources: mode=%s" % ("editor" if editor else "run"))
	_judge_fan(editor)
	_judge_radio(editor)


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
