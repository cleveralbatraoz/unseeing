extends SceneTree
## Editor-mode regression law for the level root: the retired implementation
## returned from `ready()` before `derive()` under `Engine.is_editor_hint()`,
## so a designer dragging walls saw no warnings or wall table until play.
## This probe holds the replacement two ways: the level DERIVES
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

enum Phase {
	WAIT_FOR_READY,
	WAIT_FOR_WALL_MOVE,
	WAIT_FOR_CRATE_WARNING,
	WAIT_FOR_EXTENTS_WARNING,
	WAIT_FOR_RUN_REBUILD,
	WAIT_FOR_WALL_ANCESTOR,
	WAIT_FOR_WALL_LENGTH,
	WAIT_FOR_WALL_INVALID_KNOBS,
	WAIT_FOR_WALL_KNOB_REPAIR,
	WAIT_FOR_WALL_IDLE,
	WAIT_FOR_SINGULAR_WALL,
	WAIT_FOR_WALL_REPAIR,
	WAIT_FOR_OWN_POISON,
	WAIT_FOR_OWN_ACKNOWLEDGMENT,
}

const READY_FRAMES := 30
## The condition-watch runs on the ENGINE's own frame cadence, not this
## script's — WaveLevel's `process()` override has to actually fire before
## a moved wall or a re-sunk crate can show up anywhere. Budgeted, not
## awaited forever, so a regression that stopped the watch fails loudly
## instead of hanging the gate.
const WATCH_FRAMES := 30

var _level: Node3D = null
var _room: Node3D = null
var _wall: Node3D = null
var _crate: Node3D = null
var _run: Node3D = null
var _old_runseg_id := 0
var _wall_writes_before_block := 0
var _body_writes_before_block := 0
var _contract_writes_before_block := 0
var _derive_count_after_knobs := 0
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
	_room = Node3D.new()
	_room.name = "Room"
	_wall = ClassDB.instantiate("WaveWall") as Node3D
	_wall.name = "LiveWall"
	_room.add_child(_wall)
	_level.add_child(_room)
	# deliberately no WaveSpawn, and no inject() call: the level
	# must derive honest geometry and complain about the missing spawn
	# either way
	root.add_child(_level)


func _process(_delta: float) -> bool:
	if _level == null:
		return true
	_frames += 1
	var done := true
	match _phase:
		Phase.WAIT_FOR_READY:
			done = _wait_for_ready()
		Phase.WAIT_FOR_WALL_MOVE:
			done = _wait_for_wall_move()
		Phase.WAIT_FOR_CRATE_WARNING:
			done = _wait_for_crate_warning()
		Phase.WAIT_FOR_EXTENTS_WARNING:
			done = _wait_for_extents_warning()
		Phase.WAIT_FOR_RUN_REBUILD:
			done = _wait_for_run_rebuild()
		Phase.WAIT_FOR_WALL_ANCESTOR:
			done = _wait_for_wall_ancestor()
		Phase.WAIT_FOR_WALL_LENGTH:
			done = _wait_for_wall_length()
		Phase.WAIT_FOR_WALL_INVALID_KNOBS:
			done = _wait_for_wall_invalid_knobs()
		Phase.WAIT_FOR_WALL_KNOB_REPAIR:
			done = _wait_for_wall_knob_repair()
		Phase.WAIT_FOR_WALL_IDLE:
			done = _wait_for_wall_idle()
		Phase.WAIT_FOR_SINGULAR_WALL:
			done = _wait_for_singular_wall()
		Phase.WAIT_FOR_WALL_REPAIR:
			done = _wait_for_wall_repair()
		Phase.WAIT_FOR_OWN_POISON:
			done = _wait_for_own_poison()
		Phase.WAIT_FOR_OWN_ACKNOWLEDGMENT:
			done = _wait_for_own_acknowledgment()
	return done


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
	_begin_run_rebuild_probe()
	return false


## Phase 4 (editor only): an Inspector setter may rebuild a WaveRun even when
## the normalized authored geometry is unchanged. The new RunSeg walls have
## the same paths, poses and AABBs as the old generation, but start with box
## ordinals in CUSTOM0 and replace every wall handle the last derive retained.
## No explicit rederive follows the setter below: the editor condition-watch
## must notice the new Godot object identities, repaint the actual mesh bytes,
## and publish live level-relative names on its next process pass.
func _begin_run_rebuild_probe() -> void:
	_level.set("extents", Vector2(20, 20))
	_run = ClassDB.instantiate("WaveRun") as Node3D
	_run.name = "Doorway"
	_run.set("from", Vector2(4, 4))
	_run.set("to", Vector2(10, 4))
	_run.set("openings", PackedVector2Array([Vector2(6, 2)]))
	_level.add_child(_run)
	_level.call("rederive")
	var old_segment := _run.get_node_or_null("RunSeg1")
	_old_runseg_id = old_segment.get_instance_id() if old_segment != null else 0
	var same_openings: PackedVector2Array = _run.get("openings")
	_run.set("openings", same_openings.duplicate())
	_frames = 0
	_phase = Phase.WAIT_FOR_RUN_REBUILD


func _wait_for_run_rebuild() -> bool:
	var first := _run.get_node_or_null("RunSeg1")
	var recreated := first != null and first.get_instance_id() != _old_runseg_id
	var painted := recreated and _runseg_labels_are_derived()
	var current_paths := _runseg_paths_are_level_relative()
	var retained_names := _level.call("wall_names") as PackedStringArray
	var retained_paths := (
		retained_names.has("Doorway/RunSeg1") and retained_names.has("Doorway/RunSeg2")
	)
	var no_dead_generation := true
	for wall_name: String in retained_names:
		if wall_name.begins_with("<freed wall ") or wall_name.begins_with("<unnamed wall "):
			no_dead_generation = false
	if (
		(
			not recreated
			or not painted
			or not current_paths
			or not retained_paths
			or not no_dead_generation
		)
		and _frames < WATCH_FRAMES
	):
		return false
	_check("editor: an equivalent WaveRun setter creates a new segment generation", recreated)
	_check("editor: the next process pass repaints every new RunSeg CUSTOM0 label", painted)
	_check(
		"editor: rebuilt RunSeg nodes and retained table keep level-relative paths",
		current_paths and retained_paths
	)
	_check("editor: retained wall names contain no dead-generation placeholder", no_dead_generation)
	_begin_wall_ancestor_probe()
	return false


## Phase 5 (editor only): change an ANCESTOR to an oblique rotation with
## non-uniform scale. WaveLevel stages every wall through `prepare_for_derive`
## before reading any wall-owned geometry, so the first following frame's
## generated mesh/collider, canonical centerline and repaint must all agree.
## Removing that staging call makes the level derive the old canonical pose
## first and this exact one-frame witness fails.
func _begin_wall_ancestor_probe() -> void:
	# Earlier phases deliberately moved this wall. Reset the authored local
	# position so the expected composed origin below is hand-derived from the
	# room origin rather than from unrelated fixture history.
	_wall.position = Vector3.ZERO
	_room.transform = Transform3D(
		Basis.from_euler(Vector3(0.17, 0.37, -0.11)).scaled(Vector3(2.3, 3.1, 0.47)),
		Vector3(7, 0, 11)
	)
	_frames = 0
	_phase = Phase.WAIT_FOR_WALL_ANCESTOR


func _wait_for_wall_ancestor() -> bool:
	var skin := _wall_skin()
	var collider := _wall_collider()
	var expected_limb := Transform3D(Basis.IDENTITY, Vector3(7, 1.5, 11))
	var canonical_limbs := (
		skin != null
		and collider != null
		and skin.global_transform == expected_limb
		and collider.global_transform == expected_limb
	)
	var expected_segment := Vector4(5, 11, 9, 11)
	var derived_same_frame := (
		(_level.call("wall_segments") as PackedVector4Array).has(expected_segment)
		and _wall_labels_are_derived()
	)
	_check(
		"editor: an oblique ancestor snaps mesh and collider on the first frame", canonical_limbs
	)
	_check(
		"editor: wall staging lets the level derive that same canonical frame", derived_same_frame
	)
	var body := _wall_body()
	_check(
		"editor: the private physics body stays mapped to its authored WaveWall",
		body != null and body.get_parent() == _wall
	)
	# Shape and explicit physics properties are authored on WaveWall itself.
	# These setters must update the private exact body immediately, while the
	# level's condition watch notices the nested WaveSkin AABB and rederives the
	# centerline/paint without a manual call.
	_wall.set("length", 6.0)
	_wall.set("collision_layer", 4)
	_wall.set("collision_mask", 8)
	_wall.set("collision_priority", 2.5)
	_wall.set("ray_pickable", false)
	_wall.set("input_capture_on_drag", true)
	_frames = 0
	_phase = Phase.WAIT_FOR_WALL_LENGTH
	return false


## Phase 6: a length-only Inspector edit changes no path, transform or Godot
## identity. The nested wall-owned skin AABB is therefore the only watch input
## that can rederive the segment and repaint the resized ArrayMesh generation.
func _wait_for_wall_length() -> bool:
	var body := _wall_body()
	var collider := _wall_collider()
	var shape := collider.shape as BoxShape3D if collider != null else null
	var resized := (
		(_level.call("wall_segments") as PackedVector4Array).has(Vector4(4, 11, 10, 11))
		and _wall_skin().mesh.get_aabb().size == Vector3(6.3, 3, 0.3)
		and shape != null
		and shape.size == Vector3(6.3, 3, 0.3)
		and _wall_labels_are_derived()
	)
	if not resized and _frames < WATCH_FRAMES:
		return false
	_check("editor: a length-only edit rederives paint and occlusion", resized)
	var live_contract := (
		body != null
		and body.collision_layer == 4
		and body.collision_mask == 8
		and body.collision_priority == 2.5
		and not body.input_ray_pickable
		and body.input_capture_on_drag
	)
	_check("editor: live collision setters reach the exact private body", live_contract)
	_wall.set("length", -INF)
	_wall.set("collision_priority", 0.0)
	_frames = 0
	_phase = Phase.WAIT_FOR_WALL_INVALID_KNOBS
	return false


## Phase 7: malformed Inspector values never enter ArrayMesh, BoxShape or
## PhysicsServer. They retain the last valid data and file separate actionable
## warning lines on the authored WaveWall, quiet in editor output.
func _wait_for_wall_invalid_knobs() -> bool:
	var warnings := _wall.call("get_configuration_warnings") as PackedStringArray
	var body := _wall_body()
	var shape := _wall_collider().shape as BoxShape3D
	var safe: bool = (
		_has(warnings, "length was non-finite or too large")
		and _has(warnings, "collision priority was non-finite, too large, or not positive")
		and _wall.get("length") == 6.0
		and _wall_skin().mesh.get_aabb().size == Vector3(6.3, 3, 0.3)
		and shape.size == Vector3(6.3, 3, 0.3)
		and body != null
		and body.collision_priority == 2.5
	)
	_check("editor: invalid wall knobs retain finite geometry and physics", safe)
	_wall.set("length", 7.0)
	_wall.set("collision_priority", 3.5)
	_frames = 0
	_phase = Phase.WAIT_FOR_WALL_KNOB_REPAIR
	return false


## Phase 8: a genuine finite edit acknowledges both repairs and the level
## publishes the new 7 m centerline. Capture counters only after that derive;
## the next phase proves the post-staging signature was reseeded.
func _wait_for_wall_knob_repair() -> bool:
	var warnings := _wall.call("get_configuration_warnings") as PackedStringArray
	var repaired := (
		not _has(warnings, "length was non-finite or too large")
		and not _has(warnings, "collision priority was non-finite")
		and (_level.call("wall_segments") as PackedVector4Array).has(Vector4(3.5, 11, 10.5, 11))
		and _wall_labels_are_derived()
		and _wall_body().collision_priority == 3.5
	)
	if not repaired and _frames < WATCH_FRAMES:
		return false
	_check("editor: finite knob edits clear repairs and republish the wall", repaired)
	_derive_count_after_knobs = _level.call("derive_count")
	_contract_writes_before_block = _wall.call("body_contract_writes")
	_frames = 0
	_phase = Phase.WAIT_FOR_WALL_IDLE
	return false


## Phase 9: derive records the post-staging signature, and equality-guarded
## physics setters do not wake the server again on a still editor frame.
func _wait_for_wall_idle() -> bool:
	# Re-enter every explicit physics property at the value already displayed.
	# The setters still run, so this is the mutation-live witness that each
	# generated-body write (including resource identity) is equality guarded.
	_wall.set("collision_layer", 4)
	_wall.set("collision_mask", 8)
	_wall.set("collision_priority", 3.5)
	_wall.set("ray_pickable", false)
	_wall.set("input_capture_on_drag", true)
	_wall.set("physics_material_override", null)
	var settled: bool = (
		_level.call("derive_count") == _derive_count_after_knobs
		and _wall.call("body_contract_writes") == _contract_writes_before_block
	)
	_check("editor: an unchanged wall performs no second derive or physics write", settled)
	_wall_writes_before_block = _wall.call("normalization_writes")
	_body_writes_before_block = _wall.call("body_transform_writes")
	_contract_writes_before_block = _wall.call("body_contract_writes")
	_room.scale = Vector3.ZERO
	_frames = 0
	_phase = Phase.WAIT_FOR_SINGULAR_WALL
	return false


## Phase 10: zero ancestor scale is an Inspector-representable edit for which
## Godot has no affine inverse. Godot itself reports its inverse failure at
## assignment before extension code can observe it; on the following frame
## WaveWall must add no doomed write, keep the last exact generated body and
## store one actionable editor warning without logging another line.
func _wait_for_singular_wall() -> bool:
	var warnings := _wall.call("get_configuration_warnings") as PackedStringArray
	var blocked := _has(warnings, "ancestor transform is singular, non-finite, or too large")
	var no_writes: bool = (
		_wall.call("normalization_writes") == _wall_writes_before_block
		and _wall.call("body_transform_writes") == _body_writes_before_block
		and _wall.call("body_contract_writes") == _contract_writes_before_block
	)
	_check("editor: a singular ancestor stores its repair warning", blocked)
	_check("editor: a singular ancestor performs no doomed geometry write", no_writes)
	_room.transform = Transform3D(Basis.IDENTITY, Vector3(8, 0, 12))
	_frames = 0
	_phase = Phase.WAIT_FOR_WALL_REPAIR
	return false


## Phase 11: once the ancestor is invertible again, the next editor frame
## repairs the exact generated pose, rederives the level and clears the icon.
func _wait_for_wall_repair() -> bool:
	var warnings := _wall.call("get_configuration_warnings") as PackedStringArray
	var repaired := warnings.is_empty() and _wall_body_is_exact_and_finite()
	_check("editor: repairing the ancestor clears and resynchronizes the wall", repaired)
	_wall.position = Vector3(NAN, 5, INF)
	_frames = 0
	_phase = Phase.WAIT_FOR_OWN_POISON
	return false


## Phase 12: poisoned own lanes recover from the last canonical placement,
## retain every finite lane and keep their warning across an idle frame.
func _wait_for_own_poison() -> bool:
	var warnings := _wall.call("get_configuration_warnings") as PackedStringArray
	var body := _wall_body()
	var recovered := (
		_has(warnings, "its transform contained NaN or infinity")
		and body != null
		and body.global_position.y == 5.0
		and body.global_transform.is_finite()
	)
	_check("editor: a poisoned wall recovers finite geometry and stores its warning", recovered)
	_wall.position = Vector3(2, 0, 0)
	_frames = 0
	_phase = Phase.WAIT_FOR_OWN_ACKNOWLEDGMENT
	return false


## Phase 13: one valid authored move is the explicit acknowledgment promised
## by the warning text. It clears the icon and publishes a finite exact body.
func _wait_for_own_acknowledgment() -> bool:
	var warnings := _wall.call("get_configuration_warnings") as PackedStringArray
	_check(
		"editor: a valid move acknowledges and clears the recovered wall warning",
		warnings.is_empty() and _wall_body_is_exact_and_finite()
	)
	_report()
	return true


func _wall_body() -> StaticBody3D:
	for child: Node in _wall.get_children():
		if child is StaticBody3D:
			return child as StaticBody3D
	return null


func _wall_skin() -> MeshInstance3D:
	var skins := _wall.find_children("*", "MeshInstance3D", true, false)
	return skins[0] as MeshInstance3D if not skins.is_empty() else null


func _wall_collider() -> CollisionShape3D:
	var colliders := _wall.find_children("*", "CollisionShape3D", true, false)
	return colliders[0] as CollisionShape3D if not colliders.is_empty() else null


func _wall_labels_are_derived() -> bool:
	var skin := _wall_skin()
	if skin == null or skin.mesh == null or skin.mesh.get_surface_count() == 0:
		return false
	var arrays := skin.mesh.surface_get_arrays(0)
	if arrays.size() <= Mesh.ARRAY_CUSTOM0 or not arrays[Mesh.ARRAY_CUSTOM0] is PackedFloat32Array:
		return false
	var custom: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM0]
	if custom.is_empty():
		return false
	for label: float in custom:
		if not is_finite(label) or label < 0.15 or label > 0.96:
			return false
	return true


func _wall_body_is_exact_and_finite() -> bool:
	var body := _wall_body()
	return (
		body != null
		and body.global_transform.is_finite()
		and body.global_transform.basis.x in [Vector3.RIGHT, Vector3.LEFT]
		and body.global_transform.basis.y == Vector3.UP
		and body.global_transform.basis.z in [Vector3.FORWARD, Vector3.BACK]
	)


func _runseg_labels_are_derived() -> bool:
	var walls := _run.find_children("*", "WaveWall", true, false)
	if walls.size() != 2:
		return false
	for wall: Node in walls:
		var skins := wall.find_children("*", "MeshInstance3D", true, false)
		var skin: MeshInstance3D = skins[0] as MeshInstance3D if not skins.is_empty() else null
		if skin == null or skin.mesh == null or skin.mesh.get_surface_count() == 0:
			return false
		var arrays := skin.mesh.surface_get_arrays(0)
		if (
			arrays.size() <= Mesh.ARRAY_CUSTOM0
			or not arrays[Mesh.ARRAY_CUSTOM0] is PackedFloat32Array
		):
			return false
		var custom: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM0]
		if custom.is_empty():
			return false
		for label: float in custom:
			if not is_finite(label) or label < 0.15 or label > 0.96:
				return false
	return true


func _runseg_paths_are_level_relative() -> bool:
	var paths: Array[String] = []
	for wall: Node in _run.find_children("*", "WaveWall", true, false):
		paths.append(str(_level.get_path_to(wall)))
	return paths.has("Doorway/RunSeg1") and paths.has("Doorway/RunSeg2")


func _has(warnings: PackedStringArray, needle: String) -> bool:
	for warning: String in warnings:
		if warning.contains(needle):
			return true
	return false


func _has_exact(warnings: PackedStringArray, expected: String) -> bool:
	for warning: String in warnings:
		if warning == expected:
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
		_judge_paint_warnings()
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


## Paint-time faults follow the same editor contract as placement faults:
## store the exact runtime words on the authored node, never on the level,
## print nothing in editor mode, and clear the node on the next healthy
## derive. Dynamic ClassDB construction keeps this editor-only probe
## parseable before a fresh checkout has built its extension.
func _judge_paint_warnings() -> void:
	var flat := ClassDB.instantiate("WaveProp") as Node3D
	flat.name = "FlatCrate"
	flat.set("size", Vector3(0, 1, 1))
	flat.position = Vector3(3, 0.5, 3)
	_level.add_child(flat)
	_level.call("rederive")
	var degenerate := (
		"WaveLevel: 'FlatCrate' built 2 planar face(s) from its shape, not the 6 it should "
		+ "— a degenerate size folded one or more away. Its own seams cannot be painted "
		+ "correctly this derive; skipping it rather than mislabeling by position. Give "
		+ "every extent a real size."
	)
	_check(
		"editor: a degenerate paint fault belongs only to its solid",
		(
			_has_exact(flat.call("get_configuration_warnings") as PackedStringArray, degenerate)
			and not _has_exact(_warnings(), degenerate)
		)
	)
	flat.set("size", Vector3.ONE)
	_level.call("rederive")
	_check(
		"editor: repairing degenerate geometry clears its paint fault",
		not _has_exact(flat.call("get_configuration_warnings") as PackedStringArray, degenerate)
	)
	_level.remove_child(flat)
	flat.free()

	_wall.position = Vector3(4, 0, 4)
	var merged := ClassDB.instantiate("WaveProp") as Node3D
	merged.name = "WallCrate"
	merged.set("size", Vector3(0.4, 0.4, 0.32))
	merged.position = Vector3(4, 0.5, 4.01)
	_level.add_child(merged)
	_level.call("rederive")
	var overlap := (
		"WaveLevel: 'WallCrate' overlaps the wall structure and is drawn as part of it — "
		+ "its faces take the walls' labels and its pierce lines draw. Pull it clear of the "
		+ "wall if that was a nudge, or leave it if the bump is authored."
	)
	_check(
		"editor: a wall-merge paint fault belongs only to its solid",
		(
			_has_exact(merged.call("get_configuration_warnings") as PackedStringArray, overlap)
			and not _has_exact(_warnings(), overlap)
		)
	)
	merged.position.z = 6.0
	_level.call("rederive")
	_check(
		"editor: pulling a solid clear removes its wall-merge paint fault",
		not _has_exact(merged.call("get_configuration_warnings") as PackedStringArray, overlap)
	)
	_level.remove_child(merged)
	merged.free()
	_wall.position = Vector3.ZERO
	_level.call("rederive")


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
