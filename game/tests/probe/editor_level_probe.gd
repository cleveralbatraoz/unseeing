extends SceneTree
## Editor-mode law for the level root: today `ready()` returns before
## `derive()` under `Engine.is_editor_hint()`, so a designer dragging walls
## around sees no configuration warnings and no derived wall table until
## they press play. This probe proves the fix two ways: the level DERIVES
## at edit time too — its `get_configuration_warnings()` read-back reports
## the missing spawn, and `wall_segments()` is already populated — and,
## once in the tree, it WATCHES the scene and keeps deriving on its own:
## moving the wall, re-sinking the crate, or shrinking the level's own
## `extents` knob — with no `rederive()` call anywhere near any of the
## three — still reaches `wall_segments()` and the crate's own warning icon
## a few editor frames later. `extents` is not a censused node's property
## at all (it lives on the level itself), which is exactly why it gets its
## own phase here: `derive` genuinely reads it (`report_placement` measures
## the floor slab's own world box), so the watch has to see a resize as a
## real change too, not only a moved or reshaped child. A run-mode level,
## even uninjected, still derives honest geometry exactly as before, and
## pays no per-frame cost doing it — `process()` is off outside the editor.
##
## Godot exposes `is_editor_hint()` but no setter — not to GDScript, not in
## the gdext bindings — so a headless gdUnit4 suite is always in run mode
## and the editor branch is invisible to it. Launching the ENGINE with `-e`
## is the only way in; tools/probe_editor_level.sh runs this script in both
## modes and requires both verdicts, proving its mode before judging so a
## probe that silently ran in run mode twice would assert nothing about
## the editor at all.

enum Phase { WAIT_FOR_READY, WAIT_FOR_WALL_MOVE, WAIT_FOR_CRATE_WARNING, WAIT_FOR_EXTENTS_WARNING }

const READY_FRAMES := 30
## The condition-watch runs on the ENGINE's own frame cadence, not this
## script's — WaveLevel's `process()` override has to actually fire before
## a moved wall or a re-sunk crate can show up anywhere. Budgeted, not
## awaited forever, so a regression that stopped the watch fails loudly
## instead of hanging the gate.
const WATCH_FRAMES := 30

var _level: Node3D = null
var _wall: Node3D = null
var _crate: Node3D = null
var _frames := 0
var _phase := Phase.WAIT_FOR_READY
var _checks := 0
var _failed := 0


func _initialize() -> void:
	if not ClassDB.class_exists("WaveLevel") or not ClassDB.class_exists("WaveWall"):
		_check("the Rust extension is loaded (see .godot/extension_list.cfg)", false)
		_report()
		return
	_level = ClassDB.instantiate("WaveLevel") as Node3D
	_level.set("extents", Vector2(20, 20))
	_wall = ClassDB.instantiate("WaveWall") as Node3D
	_level.add_child(_wall)
	# deliberately no WaveSpawn, and no inject() call: the level
	# must derive honest geometry and complain about the missing spawn
	# either way
	root.add_child(_level)


func _process(_delta: float) -> bool:
	if _level == null:
		return true
	_frames += 1
	match _phase:
		Phase.WAIT_FOR_READY:
			return _wait_for_ready()
		Phase.WAIT_FOR_WALL_MOVE:
			return _wait_for_wall_move()
		Phase.WAIT_FOR_CRATE_WARNING:
			return _wait_for_crate_warning()
		Phase.WAIT_FOR_EXTENTS_WARNING:
			return _wait_for_extents_warning()
		_:
			return true


## Phase 0: poll for the condition, not for a duration — the wall table is
## only populated once `_ready`'s `derive()` has actually run, whenever
## that is. Once it has, run every check that needs no further engine
## frame ([`_judge`]). A run-mode level has nothing left to watch —
## `process()` is off there — so it reports and quits on the spot; an
## editor-mode level moves the wall with no `rederive()` call anywhere
## near it and moves on to prove the condition-watch across real engine
## frames.
func _wait_for_ready() -> bool:
	var derived := (_level.call("wall_segments") as PackedVector4Array).size() > 0
	if not derived and _frames < READY_FRAMES:
		return false
	var editor := Engine.is_editor_hint()
	print("# level: mode=%s" % ("editor" if editor else "run"))
	_judge(editor)
	if not editor:
		_report()
		return true
	_wall.position.x += 2.0
	_frames = 0
	_phase = Phase.WAIT_FOR_WALL_MOVE
	return false


## Phase 1 (editor only): the wall was just dragged 2 m along X with NO
## `rederive()` call anywhere near the edit — the level's own `process()`
## override, polling its scene signature, is the only thing left that
## could notice. Poll `wall_segments()` for the moved centerline (was
## (-2, 0, 2, 0); now (0, 0, 4, 0)) until it shows up or the budget lapses,
## then plant the crate half-sunk again — `_crate.position.y = 0.0`, once
## more with no manual refresh — and move on to watch its warning icon.
func _wait_for_wall_move() -> bool:
	var segs := _level.call("wall_segments") as PackedVector4Array
	var moved := segs.size() == 1 and segs[0].is_equal_approx(Vector4(0.0, 0.0, 4.0, 0.0))
	if not moved and _frames < WATCH_FRAMES:
		return false
	_check("editor: moving the wall re-derives the table with no rederive() call", moved)
	_crate.position.y = 0.0
	_frames = 0
	_phase = Phase.WAIT_FOR_CRATE_WARNING
	return false


## Phase 2 (editor only): the crate was just re-sunk to the floor plane —
## the exact half-sunk gesture `_judge_solid_warning` already lifted clear
## — with no `rederive()` call. Poll the crate's OWN warning icon until the
## "sunk" complaint reappears or the budget lapses, then shrink the
## level's own `extents` knob — the one condition `derive` reads that is
## NOT a censused node's property — and move on to watch its effect.
func _wait_for_crate_warning() -> bool:
	var warnings := _crate.call("get_configuration_warnings") as PackedStringArray
	var sunk_again := _has(warnings, "sunk")
	if not sunk_again and _frames < WATCH_FRAMES:
		return false
	_check(
		"editor: re-sinking the crate re-derives its warning with no rederive() call", sunk_again
	)
	_level.set("extents", Vector2(2.0, 20.0))
	_frames = 0
	_phase = Phase.WAIT_FOR_EXTENTS_WARNING
	return false


## Phase 3 (editor only): `extents` was just shrunk from (20, 20) to
## (2, 20) — through `.set()`, exactly as the Inspector would — with no
## `rederive()` call and no further touch on the crate. Hand-derived
## geometry: the crate sits at (3, 0, 3) with its default 0.5 m cube skin,
## so its world footprint is x [2.75, 3.25], z [2.75, 3.25]. Against the
## ORIGINAL 20x20 floor (x [0, 20]) that footprint is fully inside — no
## unfloored complaint, only the "sunk" one this phase already found.
## Against the SHRUNK floor (x [0, 2]) it is entirely outside — neither
## edge overlaps — so `unfloored()` reports it as standing off the floor
## entirely, and that message is the only one in the level's placement
## vocabulary that names the shape's "footprint". `set_extents` itself
## resizes the floor mesh synchronously, but the WARNING can only move
## through `derive`, which nothing here calls by hand — poll the crate's
## icon for "footprint" until it appears or the budget lapses, then report.
func _wait_for_extents_warning() -> bool:
	var warnings := _crate.call("get_configuration_warnings") as PackedStringArray
	var off_the_shrunk_floor := _has(warnings, "footprint")
	if not off_the_shrunk_floor and _frames < WATCH_FRAMES:
		return false
	_check(
		(
			"editor: shrinking extents re-derives the crate's floor warning with no "
			+ "rederive() call"
		),
		off_the_shrunk_floor
	)
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
	if editor:
		_check(
			"editor: the level derives and complains about the missing spawn",
			_has(_warnings(), "WaveSpawn")
		)
		_check(
			"editor: wall segments were derived at edit time",
			(_level.call("wall_segments") as PackedVector4Array).size() == 1
		)
		var fixed := WaveSpawn.new()
		_level.add_child(fixed)
		_level.call("rederive")
		_check("editor: giving it a spawn clears the warning", not _has(_warnings(), "WaveSpawn"))
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
## at parse time on a fresh clone with no extension loaded yet. `_crate` is
## kept on the script, not a local: the next two phases resink it with no
## further help from this function.
func _judge_solid_warning() -> void:
	_crate = ClassDB.instantiate("WaveProp") as Node3D
	_crate.name = "Crate"
	_crate.position = Vector3(3, 0, 3)
	_level.add_child(_crate)
	_level.call("rederive")
	var crate_warnings := _crate.call("get_configuration_warnings") as PackedStringArray
	_check("editor: the half-sunk crate wears its own warning", _has(crate_warnings, "sunk"))
	_crate.position.y = 0.35
	_level.call("rederive")
	var lifted := _crate.call("get_configuration_warnings") as PackedStringArray
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
