extends SceneTree
## Editor-mode law for the level root: today `ready()` returns before
## `derive()` under `Engine.is_editor_hint()`, so a designer dragging walls
## around sees no configuration warnings and no derived wall table until
## they press play. This probe proves the fix: the level DERIVES at edit
## time too — its `get_configuration_warnings()` read-back reports the
## missing spawn, and `wall_segments()` is already populated — while a
## run-mode level, even uninjected, still derives honest geometry exactly
## as before.
##
## Godot exposes `is_editor_hint()` but no setter — not to GDScript, not in
## the gdext bindings — so a headless gdUnit4 suite is always in run mode
## and the editor branch is invisible to it. Launching the ENGINE with `-e`
## is the only way in; tools/probe_editor_level.sh runs this script in both
## modes and requires both verdicts, proving its mode before judging so a
## probe that silently ran in run mode twice would assert nothing about
## the editor at all.

const READY_FRAMES := 30

var _level: Node3D = null
var _frames := 0
var _checks := 0
var _failed := 0


func _initialize() -> void:
	if not ClassDB.class_exists("WaveLevel") or not ClassDB.class_exists("WaveWall"):
		_check("the Rust extension is loaded (see .godot/extension_list.cfg)", false)
		_report()
		return
	_level = ClassDB.instantiate("WaveLevel") as Node3D
	_level.set("extents", Vector2(20, 20))
	var wall := ClassDB.instantiate("WaveWall") as Node3D
	_level.add_child(wall)
	# deliberately no SpawnPoint marker, and no inject() call: the level
	# must derive honest geometry and complain about the missing spawn
	# either way
	root.add_child(_level)


func _process(_delta: float) -> bool:
	if _level == null:
		return true
	_frames += 1
	# poll for the condition, not for a duration: the wall table is only
	# populated once _ready's derive() has actually run, whenever that is
	var derived := (_level.call("wall_segments") as PackedVector4Array).size() > 0
	if not derived and _frames < READY_FRAMES:
		return false
	_judge(Engine.is_editor_hint())
	_report()
	return true


func _has(warnings: PackedStringArray, needle: String) -> bool:
	for warning: String in warnings:
		if warning.contains(needle):
			return true
	return false


func _warnings() -> PackedStringArray:
	# Godot's `_get_configuration_warnings` is a pure GDVIRTUAL: the editor
	# calls it directly through the C++ virtual table and never binds it to
	# ClassDB, so no script — static or dynamic — can reach the override
	# under that name (measured: has_method finds nothing for it on any
	# class, engine or extension). WaveLevel exposes the same answer back
	# through an ordinary #[func] of the same name, reached here exactly
	# like wall_segments() and rederive() below.
	return _level.call("get_configuration_warnings") as PackedStringArray


func _judge(editor: bool) -> void:
	print("# level: mode=%s" % ("editor" if editor else "run"))
	if editor:
		_check(
			"editor: the level derives and complains about the missing spawn",
			_has(_warnings(), "SpawnPoint")
		)
		_check(
			"editor: wall segments were derived at edit time",
			(_level.call("wall_segments") as PackedVector4Array).size() == 1
		)
		var fixed := Marker3D.new()
		fixed.name = "SpawnPoint"
		_level.add_child(fixed)
		_level.call("rederive")
		_check("editor: giving it a spawn clears the warning", not _has(_warnings(), "SpawnPoint"))
		_judge_solid_warning()
	else:
		_check(
			"run: an uninjected level still derives honest geometry",
			(_level.call("wall_segments") as PackedVector4Array).size() == 1
		)


## The fault-lands-on-its-node law: a WaveProp dropped on the floor plane
## (the classic half-sunk designer gesture — a box prop is CENTRED on its
## node, so y=0 buries half of it) wears the level's "sunk" complaint on
## ITS OWN warning icon, not only on the level's; lifting it clear makes
## that one icon empty again, on the very next rederive. Reached exactly
## like `wall_segments()` and `rederive()` above, through ClassDB.instantiate
## + `.call()`, never a static `WaveProp` reference: the class may not exist
## at parse time on a fresh clone with no extension loaded yet.
func _judge_solid_warning() -> void:
	var crate := ClassDB.instantiate("WaveProp") as Node3D
	crate.name = "Crate"
	crate.position = Vector3(3, 0, 3)
	_level.add_child(crate)
	_level.call("rederive")
	var crate_warnings := crate.call("get_configuration_warnings") as PackedStringArray
	_check("editor: the half-sunk crate wears its own warning", _has(crate_warnings, "sunk"))
	crate.position.y = 0.35
	_level.call("rederive")
	var lifted := crate.call("get_configuration_warnings") as PackedStringArray
	_check("editor: lifting the crate clears it", lifted.is_empty())


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
