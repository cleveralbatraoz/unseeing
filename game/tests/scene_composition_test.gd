extends GdUnitTestSuite

const COMPOSED_SCENE := preload("res://tests/fixtures/scene_composition/composed_level.tscn")
const FLAT_SCENE := preload("res://tests/fixtures/scene_composition/flat_level.tscn")

const COMPOSED_PATHS := {
	"group": "PlainGroup",
	"room": "PlainGroup/InheritedRoomVariant",
	"run": "PlainGroup/InheritedRoomVariant/BoundaryRun",
	"run_wall": "PlainGroup/InheritedRoomVariant/BoundaryRun/RunSeg1",
	"cross_wall": "PlainGroup/InheritedRoomVariant/CrossWall",
	"fan": "PlainGroup/InheritedRoomVariant/Fan",
	"cat": "PlainGroup/InheritedRoomVariant/Cat",
	"spawn": "PlainGroup/InheritedRoomVariant/Spawn",
	"prop_root": "PlainGroup/InheritedRoomVariant/NestedProp",
	"merge_shelf": "PlainGroup/InheritedRoomVariant/NestedProp/MergeShelf",
	"merge_crate": "PlainGroup/InheritedRoomVariant/NestedProp/MergeCrate",
	"seam_left": "PlainGroup/InheritedRoomVariant/NestedProp/SeamLeft",
	"seam_right": "PlainGroup/InheritedRoomVariant/NestedProp/SeamRight",
	"radio": "PlainGroup/InheritedRoomVariant/Radio",
}

const FLAT_PATHS := {
	"run": "BoundaryRun",
	"run_wall": "BoundaryRun/RunSeg1",
	"cross_wall": "CrossWall",
	"fan": "Fan",
	"cat": "Cat",
	"spawn": "Spawn",
	"merge_shelf": "MergeShelf",
	"merge_crate": "MergeCrate",
	"seam_left": "SeamLeft",
	"seam_right": "SeamRight",
	"radio": "Radio",
}

const COMPOSED_TYPES := {
	"group": "Node3D",
	"room": "Node3D",
	"run": "WaveRun",
	"run_wall": "WaveWall",
	"cross_wall": "WaveWall",
	"fan": "SoundFan",
	"cat": "WaveCat",
	"spawn": "WaveSpawn",
	"prop_root": "Node3D",
	"merge_shelf": "WaveProp",
	"merge_crate": "WaveProp",
	"seam_left": "WaveProp",
	"seam_right": "WaveProp",
	"radio": "SoundRadio",
}

const FLAT_TYPES := {
	"run": "WaveRun",
	"run_wall": "WaveWall",
	"cross_wall": "WaveWall",
	"fan": "SoundFan",
	"cat": "WaveCat",
	"spawn": "WaveSpawn",
	"merge_shelf": "WaveProp",
	"merge_crate": "WaveProp",
	"seam_left": "WaveProp",
	"seam_right": "WaveProp",
	"radio": "SoundRadio",
}

const PAINTED_KEYS := [
	"run_wall",
	"cross_wall",
	"merge_shelf",
	"merge_crate",
	"seam_left",
	"seam_right",
	"floor",
	"ceiling",
]
const WALL_KEYS := ["run_wall", "cross_wall"]
const SOURCE_KEYS := ["fan", "radio"]
const CAT_KEYS := ["cat"]
const PROP_KEYS := ["merge_shelf", "merge_crate", "seam_left", "seam_right"]

## Metres: Godot's retained world transforms and AABB values make one f32
## round trip at the GDExtension boundary.
const WORLD_EPS_M := 0.0001
## Metres: the physics server's contact point makes a second, narrower f32
## round trip through its ray-query result.
const PHYSICS_EPS_M := 0.00001
## Dimensionless: basis columns and geometric normals are unit-vector lanes.
const BASIS_EPS := 0.0001

const EXPECTED_WALLS := {
	"run_wall":
	{
		"segment": Vector4(6, 4, 6, 10),
		"rect": Vector4(5.9, 3.9, 6.1, 10.1),
		"span": Vector2(0, 3),
	},
	"cross_wall":
	{
		"segment": Vector4(6, 7, 9, 7),
		"rect": Vector4(5.9, 6.9, 9.1, 7.1),
		"span": Vector2(0, 3),
	},
}

const EXPECTED_PROP_AABBS := {
	"merge_shelf": AABB(Vector3(10.5, 0, 8), Vector3(1, 1, 2)),
	"merge_crate": AABB(Vector3(10.7, 0, 8.1), Vector3(0.8, 1, 1)),
	"seam_left": AABB(Vector3(10.5, 0, 5.5), Vector3.ONE),
	"seam_right": AABB(Vector3(10.5, 0, 4.5), Vector3.ONE),
}


## This catches a collector which returns when it meets an untyped Node3D,
## or counts a nested/inherited typed node more than once.
func test_plain_groups_do_not_hide_or_duplicate_nested_gameplay() -> void:
	var composed := _enter_fixture(COMPOSED_SCENE)
	var flat := _enter_fixture(FLAT_SCENE)
	if composed == null or flat == null:
		return
	assert_bool(_assert_live_inventory(composed, COMPOSED_PATHS, COMPOSED_TYPES)).is_true()
	assert_bool(_assert_live_inventory(flat, FLAT_PATHS, FLAT_TYPES)).is_true()


## This catches a recursive census which skips the inherited override/addition,
## or injects source meshes after the scene has entered the tree.
func test_inherited_override_and_added_radio_reach_retained_sources_once() -> void:
	var composed := _enter_fixture(COMPOSED_SCENE)
	var flat := _enter_fixture(FLAT_SCENE)
	if composed == null or flat == null:
		return
	var composed_fan := _path_node(composed, COMPOSED_PATHS, "fan") as SoundFan
	var composed_radio := _path_node(composed, COMPOSED_PATHS, "radio") as SoundRadio
	if composed_fan == null or composed_radio == null:
		return
	assert_float(composed_fan.volume).is_equal(0.6)
	assert_bool(composed_radio is SoundRadio).is_true()
	assert_object(_mesh_limb(composed_fan)).is_not_null()
	assert_object(_mesh_limb(composed_radio)).is_not_null()

	var composed_sources := _retained_transforms(composed, composed.sources(), COMPOSED_PATHS)
	var flat_sources := _retained_transforms(flat, flat.sources(), FLAT_PATHS)
	if composed_sources.is_empty() or flat_sources.is_empty():
		return
	assert_int(composed_sources.size()).is_equal(2)
	assert_int(flat_sources.size()).is_equal(2)
	for key: String in SOURCE_KEYS:
		if not composed_sources.has(key) or not flat_sources.has(key):
			fail("the retained source tables omit semantic key '%s'" % key)
			return
		var composed_transform: Transform3D = composed_sources[key]
		var flat_transform: Transform3D = flat_sources[key]
		_assert_matching_transform(composed_transform, flat_transform)


## This catches a world-space derivation that reads an inherited room's local
## transform, or a retained-table slot that drifts away from its wall/source.
func test_composed_and_flat_fixtures_share_hand_anchored_world_outputs() -> void:
	var composed := _enter_fixture(COMPOSED_SCENE)
	var flat := _enter_fixture(FLAT_SCENE)
	if composed == null or flat == null:
		return
	var group := _path_node(composed, COMPOSED_PATHS, "group") as Node3D
	if group == null:
		return
	_assert_transform_matches(
		group.global_transform,
		Transform3D(Vector3.FORWARD, Vector3.UP, Vector3.RIGHT, Vector3(2, 0, 12)),
		"composed plain grouping frame"
	)
	var composed_outputs := _assert_hand_anchored_world_outputs(composed, COMPOSED_PATHS)
	var flat_outputs := _assert_hand_anchored_world_outputs(flat, FLAT_PATHS)
	if composed_outputs.is_empty() or flat_outputs.is_empty():
		return
	_assert_matching_world_outputs(composed_outputs, flat_outputs)


## This catches a wall which paints/collides in its inherited container frame
## rather than the normalized world frame retained by its flat equivalent.
func test_inherited_cross_wall_keeps_the_flat_collision_and_physics_verdict() -> void:
	var composed := _entered_cross_wall(COMPOSED_SCENE, COMPOSED_PATHS)
	if composed.is_empty():
		return
	await get_tree().physics_frame
	await get_tree().physics_frame
	var composed_body := composed["body"] as StaticBody3D
	var composed_wall := composed["wall"] as WaveWall
	if (
		composed_body == null
		or composed_wall == null
		or not _assert_cross_wall_ray(composed_body, composed_wall)
	):
		return
	remove_child(composed["level"] as WaveLevel)
	await get_tree().physics_frame
	var flat := _entered_cross_wall(FLAT_SCENE, FLAT_PATHS)
	if flat.is_empty():
		return
	var composed_snapshot: Dictionary = composed["snapshot"]
	var flat_snapshot: Dictionary = flat["snapshot"]
	_assert_matching_wall_snapshots(composed_snapshot, flat_snapshot)
	await get_tree().physics_frame
	await get_tree().physics_frame
	_assert_cross_wall_ray(flat["body"] as StaticBody3D, flat["wall"] as WaveWall)


func _entered_cross_wall(scene: PackedScene, paths: Dictionary) -> Dictionary:
	var level := _enter_fixture(scene)
	if level == null:
		return {}
	var wall := _path_node(level, paths, "cross_wall") as WaveWall
	if wall == null:
		return {}
	var snapshot := _private_wall_snapshot(wall)
	var body := snapshot.get("body") as StaticBody3D
	if snapshot.is_empty() or body == null:
		fail("CrossWall snapshot omitted its private body")
		return {}
	return {"level": level, "wall": wall, "snapshot": snapshot, "body": body}


## This catches a face-class map which loses a nested member, assigns a fresh
## label to a genuine overlap, or lets two face-to-face seam labels melt.
func test_nested_merges_and_touching_seams_survive_semantic_normalization() -> void:
	var composed := _enter_fixture(COMPOSED_SCENE)
	var flat := _enter_fixture(FLAT_SCENE)
	if composed == null or flat == null:
		return
	var composed_classes := _semantic_superfaces(composed, COMPOSED_PATHS)
	var flat_classes := _semantic_superfaces(flat, FLAT_PATHS)
	if composed_classes.is_empty() or flat_classes.is_empty():
		return
	assert_array(composed_classes).is_equal(flat_classes)
	assert_bool(composed_classes.has("cross_wall+run_wall")).is_true()
	assert_bool(composed_classes.has("merge_crate+merge_shelf")).is_true()
	assert_bool(flat_classes.has("cross_wall+run_wall")).is_true()
	assert_bool(flat_classes.has("merge_crate+merge_shelf")).is_true()
	_assert_fixture_mesh_labels(composed, COMPOSED_PATHS)
	_assert_fixture_mesh_labels(flat, FLAT_PATHS)


## This catches a healthy nested or inherited authoring path that silently
## loses its floor, mesh build, warning forwarding, or paint census.
func test_composed_and_flat_fixtures_derive_without_faults() -> void:
	var composed := _enter_fixture(COMPOSED_SCENE)
	var flat := _enter_fixture(FLAT_SCENE)
	if composed == null or flat == null:
		return
	_assert_fixture_is_healthy(composed, COMPOSED_PATHS)
	_assert_fixture_is_healthy(flat, FLAT_PATHS)


## Injecting before tree entry is the one supported construction path: it lets
## WaveLevel deliver source and solid materials before each node builds limbs.
func _enter_fixture(scene: PackedScene) -> WaveLevel:
	if scene == null:
		fail("fixture PackedScene is null")
		return null
	var level := scene.instantiate() as WaveLevel
	if level == null:
		fail("fixture did not instantiate as WaveLevel")
		return null
	level = auto_free(level) as WaveLevel
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	return level


## Normalize the retained wall table through the explicit fixture paths. An
## unrecognized path or duplicate semantic key invalidates the whole result.
func _wall_rows(level: WaveLevel, paths: Dictionary) -> Dictionary:
	var reverse := _reverse_paths(paths)
	if reverse.is_empty():
		return {}
	var names := level.call("wall_names") as PackedStringArray
	var segments := level.wall_segments()
	if names.size() != segments.size():
		fail("retained wall names and segments differ in length")
		return {}
	var rows := {}
	for index: int in names.size():
		var path: String = names[index]
		if not reverse.has(path):
			fail("retained wall path '%s' is absent from the fixture map" % path)
			return {}
		var key: String = str(reverse[path])
		if rows.has(key):
			fail("retained walls duplicate semantic key '%s'" % key)
			return {}
		rows[key] = {"path": path, "segment": segments[index]}
	return rows


## Read all four retained wall arrays as one row. A row is invalid when any
## parallel array is malformed: accepting a partial table would hide a shader
## slot drift behind matching centerlines.
func _complete_wall_rows(level: WaveLevel, paths: Dictionary) -> Dictionary:
	var reverse := _reverse_paths(paths)
	if reverse.is_empty():
		return {}
	var names := level.call("wall_names") as PackedStringArray
	var segments := level.wall_segments()
	var rects := level.wall_rects()
	var spans := level.wall_spans()
	if names.size() != 2 or segments.size() != 2 or rects.size() != 2 or spans.size() != 2:
		fail("retained wall arrays must each contain the two authored wall rows")
		return {}
	var rows := {}
	for index: int in names.size():
		var path: String = names[index]
		if not reverse.has(path) or not EXPECTED_WALLS.has(str(reverse.get(path, ""))):
			fail("retained wall path '%s' is not one of the two semantic wall paths" % path)
			return {}
		var key: String = str(reverse[path])
		if rows.has(key):
			fail("retained walls duplicate semantic key '%s'" % key)
			return {}
		var segment: Vector4 = segments[index]
		var rect: Vector4 = rects[index]
		var span: Vector2 = spans[index]
		rows[key] = {"path": path, "segment": segment, "rect": rect, "span": span}
	if not _has_exact_keys(rows, WALL_KEYS, "complete retained walls"):
		return {}
	return rows


## This total helper returns no partial output after an unknown, duplicate, or
## malformed retained member.
func _assert_hand_anchored_world_outputs(level: WaveLevel, paths: Dictionary) -> Dictionary:
	var rows := _complete_wall_rows(level, paths)
	if rows.is_empty():
		return {}
	for key: String in WALL_KEYS:
		var row: Dictionary = rows[key]
		var expected: Dictionary = EXPECTED_WALLS[key]
		var segment: Vector4 = row["segment"]
		var expected_segment: Vector4 = expected["segment"]
		var rect: Vector4 = row["rect"]
		var expected_rect: Vector4 = expected["rect"]
		var span: Vector2 = row["span"]
		var expected_span: Vector2 = expected["span"]
		_assert_vector4_approx(segment, expected_segment, WORLD_EPS_M, "%s centerline" % key)
		_assert_vector4_approx(rect, expected_rect, WORLD_EPS_M, "%s occluder" % key)
		assert_vector(span).is_equal_approx(expected_span, Vector2.ONE * WORLD_EPS_M)
		var raw_path: String = str(paths[key])
		assert_str(row["path"]).is_equal(raw_path)

	var creatures_and_sources := _anchored_sources_and_cats(level, paths)
	var props := _anchored_prop_aabbs(level, paths)
	if (
		creatures_and_sources.is_empty()
		or props.is_empty()
		or not _assert_spawn_anchor(level, paths)
	):
		return {}
	return {
		"walls": rows,
		"sources": creatures_and_sources["sources"],
		"cats": creatures_and_sources["cats"],
		"props": props,
		"spawn": level.spawn_pos(),
	}


func _anchored_sources_and_cats(level: WaveLevel, paths: Dictionary) -> Dictionary:
	var sources := _retained_transforms(level, level.sources(), paths)
	var cats := _retained_transforms(level, level.cats(), paths)
	if sources.is_empty() or cats.is_empty():
		return {}
	var expected := _has_exact_keys(sources, SOURCE_KEYS, "retained sources")
	expected = _has_exact_keys(cats, CAT_KEYS, "retained cats") and expected
	if not expected:
		return {}
	var fan := _path_node(level, paths, "fan") as SoundFan
	var radio := _path_node(level, paths, "radio") as Node3D
	var cat := _path_node(level, paths, "cat") as Node3D
	if fan == null or radio == null or cat == null:
		return {}
	var anchored := _assert_node_anchor(level, paths, "fan", Vector3(8, 0, 9), PI * 0.5)
	anchored = _assert_node_anchor(level, paths, "radio", Vector3(9, 0, 5), PI * 0.5) and anchored
	anchored = _assert_node_anchor(level, paths, "cat", Vector3(4, 0, 5), PI * 0.5) and anchored
	if not anchored:
		return {}
	assert_float(fan.volume).is_equal_approx(0.6, WORLD_EPS_M)
	return {"sources": sources, "cats": cats}


func _anchored_prop_aabbs(level: WaveLevel, paths: Dictionary) -> Dictionary:
	var props := {}
	for key: String in PROP_KEYS:
		var prop := _path_node(level, paths, key) as Node3D
		if prop == null:
			return {}
		var actual := _world_aabb(prop)
		if actual.size == Vector3.ZERO:
			return {}
		var expected: AABB = EXPECTED_PROP_AABBS[key]
		assert_vector(actual.position).is_equal_approx(expected.position, Vector3.ONE * WORLD_EPS_M)
		assert_vector(actual.size).is_equal_approx(expected.size, Vector3.ONE * WORLD_EPS_M)
		props[key] = actual
	return props


func _assert_spawn_anchor(level: WaveLevel, paths: Dictionary) -> bool:
	if not _path_node(level, paths, "spawn") is WaveSpawn:
		return false
	assert_vector(level.spawn_pos()).is_equal_approx(Vector3(4, 0.9, 9), Vector3.ONE * WORLD_EPS_M)
	assert_float(level.spawn_yaw()).is_equal_approx(PI * 0.5, WORLD_EPS_M)
	return true


func _assert_node_anchor(
	level: WaveLevel, paths: Dictionary, key: String, origin: Vector3, yaw: float
) -> bool:
	var node := _path_node(level, paths, key) as Node3D
	if node == null:
		return false
	assert_vector(node.global_position).is_equal_approx(origin, Vector3.ONE * WORLD_EPS_M)
	assert_float(node.global_rotation.y).is_equal_approx(yaw, WORLD_EPS_M)
	return true


func _world_aabb(node: Node3D) -> AABB:
	var skin := _mesh_limb(node)
	if skin == null or skin.mesh == null:
		fail("%s has no world mesh AABB" % node.get_path())
		return AABB()
	return skin.global_transform * skin.mesh.get_aabb()


func _assert_matching_world_outputs(composed: Dictionary, flat: Dictionary) -> void:
	for key: String in WALL_KEYS:
		var composed_row: Dictionary = composed["walls"][key]
		var flat_row: Dictionary = flat["walls"][key]
		var composed_segment: Vector4 = composed_row["segment"]
		var flat_segment: Vector4 = flat_row["segment"]
		var composed_rect: Vector4 = composed_row["rect"]
		var flat_rect: Vector4 = flat_row["rect"]
		var composed_span: Vector2 = composed_row["span"]
		var flat_span: Vector2 = flat_row["span"]
		_assert_vector4_approx(composed_segment, flat_segment, WORLD_EPS_M, "%s centerline" % key)
		_assert_vector4_approx(composed_rect, flat_rect, WORLD_EPS_M, "%s occluder" % key)
		assert_vector(composed_span).is_equal_approx(flat_span, Vector2.ONE * WORLD_EPS_M)
	for key: String in SOURCE_KEYS:
		var composed_source: Transform3D = composed["sources"][key]
		var flat_source: Transform3D = flat["sources"][key]
		_assert_matching_transform(composed_source, flat_source)
	for key: String in CAT_KEYS:
		var composed_cat: Transform3D = composed["cats"][key]
		var flat_cat: Transform3D = flat["cats"][key]
		_assert_matching_transform(composed_cat, flat_cat)
	for key: String in PROP_KEYS:
		var composed_box: AABB = composed["props"][key]
		var flat_box: AABB = flat["props"][key]
		assert_vector(composed_box.position).is_equal_approx(
			flat_box.position, Vector3.ONE * WORLD_EPS_M
		)
		assert_vector(composed_box.size).is_equal_approx(flat_box.size, Vector3.ONE * WORLD_EPS_M)
	var composed_spawn: Vector3 = composed["spawn"]
	var flat_spawn: Vector3 = flat["spawn"]
	assert_vector(composed_spawn).is_equal_approx(flat_spawn, Vector3.ONE * WORLD_EPS_M)


func _private_wall_snapshot(wall: WaveWall) -> Dictionary:
	var bodies: Array[StaticBody3D] = []
	for child: Node in wall.get_children():
		if (
			child is StaticBody3D
			and child.has_meta("_unseeing_wave_wall_body")
			and child.get_meta("_unseeing_wave_wall_body") == true
		):
			bodies.append(child as StaticBody3D)
	if bodies.size() != 1:
		fail("CrossWall must own exactly one private StaticBody3D")
		return {}
	var body := bodies[0]
	if body.owner != null:
		fail("CrossWall private StaticBody3D must remain ownerless")
		return {}
	var skins := body.find_children("WaveSkin", "MeshInstance3D", true, false)
	var colliders := body.find_children("WaveCollider", "CollisionShape3D", true, false)
	if skins.size() != 1 or colliders.size() != 1:
		fail("CrossWall private body must have exactly one WaveSkin and one WaveCollider")
		return {}
	var skin := skins[0] as MeshInstance3D
	var collider := colliders[0] as CollisionShape3D
	if (
		skin == null
		or collider == null
		or skin.mesh == null
		or skin.owner != null
		or collider.owner != null
	):
		fail("CrossWall private limbs are malformed or have an authored owner")
		return {}
	var shape := collider.shape as BoxShape3D
	if shape == null:
		fail("CrossWall WaveCollider must carry a BoxShape3D")
		return {}
	var paint_frame: Transform3D = wall.call("paint_frame")
	var expected_body := Transform3D(Vector3.LEFT, Vector3.UP, Vector3.FORWARD, Vector3(7.5, 0, 7))
	var expected_frame := Transform3D(
		Vector3.LEFT, Vector3.UP, Vector3.FORWARD, Vector3(7.5, 1.5, 7)
	)
	_assert_transform_matches(body.global_transform, expected_body, "CrossWall private body")
	_assert_transform_matches(skin.global_transform, expected_frame, "CrossWall WaveSkin")
	_assert_transform_matches(collider.global_transform, expected_frame, "CrossWall WaveCollider")
	_assert_transform_matches(paint_frame, expected_frame, "CrossWall paint_frame")
	assert_vector(skin.mesh.get_aabb().size).is_equal_approx(
		Vector3(3.3, 3, 0.3), Vector3.ONE * WORLD_EPS_M
	)
	assert_vector(shape.size).is_equal_approx(Vector3(3.3, 3, 0.3), Vector3.ONE * WORLD_EPS_M)
	return {
		"body": body,
		"body_transform": body.global_transform,
		"skin_transform": skin.global_transform,
		"collider_transform": collider.global_transform,
		"paint_frame": paint_frame,
		"mesh_size": skin.mesh.get_aabb().size,
		"shape_size": shape.size,
	}


func _assert_matching_wall_snapshots(composed: Dictionary, flat: Dictionary) -> void:
	for key: String in ["body_transform", "skin_transform", "collider_transform", "paint_frame"]:
		var composed_frame: Transform3D = composed[key]
		var flat_frame: Transform3D = flat[key]
		_assert_transform_matches(composed_frame, flat_frame, "CrossWall %s" % key)
	var composed_mesh_size: Vector3 = composed["mesh_size"]
	var flat_mesh_size: Vector3 = flat["mesh_size"]
	var composed_shape_size: Vector3 = composed["shape_size"]
	var flat_shape_size: Vector3 = flat["shape_size"]
	assert_vector(composed_mesh_size).is_equal_approx(flat_mesh_size, Vector3.ONE * WORLD_EPS_M)
	assert_vector(composed_shape_size).is_equal_approx(flat_shape_size, Vector3.ONE * WORLD_EPS_M)


func _assert_cross_wall_ray(body: StaticBody3D, wall: WaveWall) -> bool:
	if body == null:
		fail("CrossWall physics body is null")
		return false
	var query := PhysicsRayQueryParameters3D.create(Vector3(8, 1.5, 6), Vector3(8, 1.5, 8))
	var hit := get_viewport().world_3d.direct_space_state.intersect_ray(query)
	if not hit.has("collider") or not hit.has("position") or not hit.has("normal"):
		fail("CrossWall ray produced no complete real-physics hit")
		return false
	var collider := hit["collider"] as StaticBody3D
	if collider == null:
		fail("CrossWall ray collider is not a StaticBody3D")
		return false
	assert_object(collider).is_same(body)
	assert_object(collider.get_parent()).is_same(wall)
	var position: Vector3 = hit["position"]
	var normal: Vector3 = hit["normal"]
	assert_vector(position).is_equal_approx(Vector3(8, 1.5, 6.85), Vector3.ONE * PHYSICS_EPS_M)
	assert_vector(normal).is_equal_approx(Vector3.FORWARD, Vector3.ONE * BASIS_EPS)
	return true


## Normalise class membership, never its numeric id or label: independently
## authored fixture order is allowed to choose a different lawful palette.
func _semantic_superfaces(level: WaveLevel, paths: Dictionary) -> Array[String]:
	var reverse := _reverse_paths(paths)
	if reverse.is_empty():
		return []
	reverse["Floor"] = "floor"
	reverse["Ceiling"] = "ceiling"
	var observer: WaveObserver = auto_free(WaveObserver.new())
	observer.inject(level, null)
	var explained: Dictionary = observer.explain_oids()
	if (
		explained.has("unavailable")
		or not explained.has("superfaces")
		or not explained["superfaces"] is Array
	):
		fail("observer refused or malformed semantic superfaces: %s" % explained)
		return []
	var classes: Array[String] = []
	for value: Variant in explained["superfaces"]:
		var raw := _superface_member_paths(value)
		if raw.is_empty():
			return []
		var members: Array[String] = []
		var seen := {}
		for path: String in raw["members"]:
			if not reverse.has(path):
				fail("observer member '%s' is absent from the fixture map" % path)
				return []
			var key: String = str(reverse[path])
			if seen.has(key):
				fail("observer superface duplicates semantic key '%s'" % key)
				return []
			seen[key] = true
			members.append(key)
		members.sort()
		classes.append(_encode_members(members))
	classes.sort()
	return classes


func _encode_members(members: Array[String]) -> String:
	if members.is_empty():
		fail("observer superface has no semantic members")
		return ""
	var encoded := members[0]
	for index: int in range(1, members.size()):
		encoded += "+%s" % members[index]
	return encoded


func _assert_fixture_mesh_labels(level: WaveLevel, paths: Dictionary) -> void:
	var run_wall := _path_node(level, paths, "run_wall") as Node3D
	var cross_wall := _path_node(level, paths, "cross_wall") as Node3D
	var shelf := _path_node(level, paths, "merge_shelf") as Node3D
	var crate := _path_node(level, paths, "merge_crate") as Node3D
	var seam_left := _path_node(level, paths, "seam_left") as Node3D
	var seam_right := _path_node(level, paths, "seam_right") as Node3D
	if (
		run_wall == null
		or cross_wall == null
		or shelf == null
		or crate == null
		or seam_left == null
		or seam_right == null
	):
		return
	var run_label := _face_at_plane_and_normal(run_wall, 0, 5.85, Vector3.LEFT)
	var cross_label := _face_at_plane_and_normal(cross_wall, 0, 5.85, Vector3.LEFT)
	var shelf_top := _face_at_plane_and_normal(shelf, 1, 1.0, Vector3.UP)
	var crate_top := _face_at_plane_and_normal(crate, 1, 1.0, Vector3.UP)
	var shelf_side := _face_at_plane_and_normal(shelf, 0, 11.5, Vector3.RIGHT)
	var crate_side := _face_at_plane_and_normal(crate, 0, 11.5, Vector3.RIGHT)
	var left_seam := _face_at_plane_and_normal(seam_left, 2, 5.5, Vector3.FORWARD)
	var right_seam := _face_at_plane_and_normal(seam_right, 2, 5.5, Vector3.BACK)
	if (
		run_label.is_empty()
		or cross_label.is_empty()
		or shelf_top.is_empty()
		or crate_top.is_empty()
		or shelf_side.is_empty()
		or crate_side.is_empty()
		or left_seam.is_empty()
		or right_seam.is_empty()
	):
		return
	_assert_face_labels_match(run_label, cross_label, "run-wall/cross-wall merge")
	_assert_face_labels_match(shelf_top, crate_top, "nested shelf/crate top merge")
	_assert_face_labels_match(shelf_side, crate_side, "nested shelf/crate side merge")
	var left_label: float = left_seam["label"]
	var right_label: float = right_seam["label"]
	assert_float(absf(left_label - right_label)).is_greater_equal(
		WaveCore.new().min_label_separation()
	)


## The plane coordinate is a hand-derived world coordinate (0=x, 1=y, 2=z).
## The face is selected before CUSTOM0 is read, so an ordinal or label write
## cannot steer its own test toward a convenient face.
func _face_at_plane_and_normal(
	node: Node3D, axis: int, plane: float, normal: Vector3
) -> Dictionary:
	if axis < 0 or axis > 2:
		fail("face plane axis %d is outside the Vector3 domain" % axis)
		return {}
	var mesh_data := _box_mesh_arrays(node)
	if mesh_data.is_empty():
		return {}
	var skin: MeshInstance3D = mesh_data["skin"]
	var vertices: PackedVector3Array = mesh_data["vertices"]
	var custom: PackedFloat32Array = mesh_data["custom"]
	var matches: Array[PackedFloat32Array] = []
	for first: int in range(0, vertices.size(), 4):
		var a := skin.global_transform * vertices[first]
		var b := skin.global_transform * vertices[first + 1]
		var c := skin.global_transform * vertices[first + 2]
		var d := skin.global_transform * vertices[first + 3]
		var centroid := (a + b + c + d) * 0.25
		var geometric_normal := (b - a).cross(c - a).normalized()
		if absf(centroid[axis] - plane) > WORLD_EPS_M:
			continue
		if geometric_normal.dot(normal) < 1.0 - BASIS_EPS:
			continue
		var lanes := PackedFloat32Array()
		for index: int in range(first, first + 4):
			lanes.append(custom[index])
		matches.append(lanes)
	if matches.size() != 1:
		fail("%s has %d faces at the requested plane/normal" % [node.get_path(), matches.size()])
		return {}
	var lanes := matches[0]
	var label: float = lanes[0]
	for lane: float in lanes:
		if lane != label:
			fail("%s selected face CUSTOM0 lanes are not bit-equal" % node.get_path())
			return {}
	if label < 0.15 or label > 0.96:
		fail("%s selected face label %.8f is outside the sRGB-safe band" % [node.get_path(), label])
		return {}
	return {"label": label, "lanes": lanes}


func _box_mesh_arrays(node: Node3D) -> Dictionary:
	var skin := _mesh_limb(node)
	if skin == null or not skin.mesh is ArrayMesh:
		fail("%s has no ArrayMesh WaveSkin" % node.get_path())
		return {}
	var mesh := skin.mesh as ArrayMesh
	if mesh.get_surface_count() != 1:
		fail("%s WaveSkin must expose exactly one surface" % node.get_path())
		return {}
	var arrays := mesh.surface_get_arrays(0)
	if arrays.size() <= Mesh.ARRAY_CUSTOM0:
		fail("%s WaveSkin omits mesh arrays" % node.get_path())
		return {}
	var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var custom: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM0]
	if vertices.size() != 24 or custom.size() != 24:
		fail("%s WaveSkin must carry 24 vertices and 24 CUSTOM0 lanes" % node.get_path())
		return {}
	return {"skin": skin, "vertices": vertices, "custom": custom}


func _assert_face_labels_match(a: Dictionary, b: Dictionary, label: String) -> void:
	var a_lanes: PackedFloat32Array = a["lanes"]
	var b_lanes: PackedFloat32Array = b["lanes"]
	if a_lanes.size() != 4 or b_lanes.size() != 4:
		fail("%s lacks four selected CUSTOM0 lanes" % label)
		return
	for index: int in 4:
		assert_float(a_lanes[index]).append_failure_message(label).is_equal(b_lanes[index])


func _assert_fixture_is_healthy(level: WaveLevel, paths: Dictionary) -> void:
	assert_int(level.unfloored_solids()).is_equal(0)
	assert_int(level.sunken_solids()).is_equal(0)
	assert_array(level.get_configuration_warnings()).is_empty()
	for key: String in [
		"run",
		"run_wall",
		"cross_wall",
		"merge_shelf",
		"merge_crate",
		"seam_left",
		"seam_right",
		"fan",
		"radio",
		"spawn",
	]:
		var node := _path_node(level, paths, key)
		if node == null:
			return
		var warnings: PackedStringArray = node.call("get_configuration_warnings")
		assert_array(warnings).append_failure_message("%s warning forwarder" % key).is_empty()
	var cats := _retained_transforms(level, level.cats(), paths)
	if cats.is_empty() or not _has_exact_keys(cats, CAT_KEYS, "healthy retained cats"):
		return
	var cat := _path_node(level, paths, "cat") as WaveCat
	if cat == null:
		return
	var retained_cat: Transform3D = cats["cat"]
	_assert_matching_transform(retained_cat, cat.global_transform)
	assert_object(_mesh_limb(cat)).is_not_null()
	var observer: WaveObserver = auto_free(WaveObserver.new())
	observer.inject(level, null)
	var explained: Dictionary = observer.explain_oids()
	if (
		explained.has("unavailable")
		or not explained.has("faults")
		or not explained["faults"] is Array
	):
		fail("observer refused healthy fault census: %s" % explained)
		return
	assert_array(explained["faults"]).is_empty()


## Normalize retained Node3D arrays by fixture path. Returning an empty map on
## a bad row stops a caller from accepting a partial census as a valid one.
func _retained_transforms(level: WaveLevel, retained: Array, paths: Dictionary) -> Dictionary:
	var reverse := _reverse_paths(paths)
	if reverse.is_empty():
		return {}
	var out := {}
	for value: Variant in retained:
		var node := value as Node3D
		if node == null:
			fail("retained output contains a non-Node3D value")
			return {}
		var path := str(level.get_path_to(node))
		if not reverse.has(path):
			fail("retained node path '%s' is absent from the fixture map" % path)
			return {}
		var key: String = str(reverse[path])
		if out.has(key):
			fail("retained nodes duplicate semantic key '%s'" % key)
			return {}
		out[key] = node.global_transform
	return out


## The fixture inventory deliberately comes from literal paths and classes,
## never from a discovery helper that could bless the census it is meant to
## check.
func _assert_live_inventory(level: WaveLevel, paths: Dictionary, types: Dictionary) -> bool:
	if not _assert_recursive_counts(level):
		return false
	if not _assert_authored_nodes(level, paths, types):
		return false
	if not _assert_retained_inventory(level, paths):
		return false
	return _assert_observer_membership(level, paths)


func _assert_authored_nodes(level: WaveLevel, paths: Dictionary, types: Dictionary) -> bool:
	for key: String in paths:
		var node := _path_node(level, paths, key)
		if node == null:
			return false
		var expected_type: String = str(types.get(key, ""))
		if expected_type.is_empty() or not _is_expected_type(node, expected_type):
			fail(
				(
					"fixture path '%s' does not resolve to %s"
					% [paths[key], types.get(key, "a known type")]
				)
			)
			return false
		if key == "run_wall":
			if node.owner != null:
				fail("generated RunSeg1 must remain ownerless")
				return false
		elif node.owner == null:
			fail("authored fixture node '%s' has no owner" % paths[key])
			return false
	return true


func _assert_retained_inventory(level: WaveLevel, paths: Dictionary) -> bool:
	var walls := _wall_rows(level, paths)
	var sources := _retained_transforms(level, level.sources(), paths)
	var cats := _retained_transforms(level, level.cats(), paths)
	if walls.is_empty() or sources.is_empty() or cats.is_empty():
		return false
	var has_expected_keys := _has_exact_keys(walls, WALL_KEYS, "walls")
	has_expected_keys = _has_exact_keys(sources, SOURCE_KEYS, "sources") and has_expected_keys
	has_expected_keys = _has_exact_keys(cats, CAT_KEYS, "cats") and has_expected_keys
	if not has_expected_keys:
		return false
	var spawn := _path_node(level, paths, "spawn") as WaveSpawn
	if spawn == null:
		return false
	var expected_spawn := spawn.global_position + Vector3(0.0, 0.9, 0.0)
	if not level.spawn_pos().is_equal_approx(expected_spawn):
		fail("selected spawn output did not retain the authored WaveSpawn")
		return false
	return true


func _assert_recursive_counts(level: WaveLevel) -> bool:
	var counts := {"run": 0, "wall": 0, "prop": 0, "fan": 0, "radio": 0, "cat": 0, "spawn": 0}
	_count_gameplay_nodes(level, counts)
	var expected := {"run": 1, "wall": 2, "prop": 4, "fan": 1, "radio": 1, "cat": 1, "spawn": 1}
	for key: String in expected:
		if counts[key] != expected[key]:
			fail(
				(
					"recursive census expected %d %s node(s), found %d"
					% [expected[key], key, counts[key]]
				)
			)
			return false
	return true


func _count_gameplay_nodes(node: Node, counts: Dictionary) -> void:
	for child: Node in node.get_children():
		if child is WaveRun:
			counts["run"] += 1
		elif child is WaveWall:
			counts["wall"] += 1
		elif child is WaveProp:
			counts["prop"] += 1
		elif child is SoundFan:
			counts["fan"] += 1
		elif child is SoundRadio:
			counts["radio"] += 1
		elif child is WaveCat:
			counts["cat"] += 1
		elif child is WaveSpawn:
			counts["spawn"] += 1
		_count_gameplay_nodes(child, counts)


func _assert_observer_membership(level: WaveLevel, paths: Dictionary) -> bool:
	var reverse := _reverse_paths(paths)
	if reverse.is_empty():
		return false
	reverse["Floor"] = "floor"
	reverse["Ceiling"] = "ceiling"
	var observer: WaveObserver = auto_free(WaveObserver.new())
	observer.inject(level, null)
	var explained: Dictionary = observer.explain_oids()
	if not explained.has("superfaces") or not explained["superfaces"] is Array:
		fail("observer refused with malformed superfaces: %s" % explained)
		return false
	var raw_superfaces := explained["superfaces"] as Array
	var members := {}
	for superface_value: Variant in raw_superfaces:
		var normalized := _superface_member_paths(superface_value)
		if normalized.is_empty():
			return false
		var class_members := {}
		for path: String in normalized["members"]:
			if not reverse.has(path):
				fail("observer member '%s' is absent from the fixture map" % path)
				return false
			var key: String = str(reverse[path])
			if class_members.has(key):
				fail("observer superface duplicates semantic key '%s'" % key)
				return false
			class_members[key] = true
			members[key] = true
	return _has_exact_keys(members, PAINTED_KEYS, "observer membership")


func _superface_member_paths(superface_value: Variant) -> Dictionary:
	if not superface_value is Dictionary:
		fail("observer superface entry is not a Dictionary")
		return {}
	var superface: Dictionary = superface_value
	if not superface.has("members"):
		fail("observer superface entry omits members")
		return {}
	var raw_members: Variant = superface["members"]
	if not raw_members is Array:
		fail("observer superface members are not an Array")
		return {}
	var paths: Array[String] = []
	for value: Variant in raw_members:
		if not value is String:
			fail("observer member is not a String")
			return {}
		paths.append(str(value))
	return {"members": paths}


func _reverse_paths(paths: Dictionary) -> Dictionary:
	var reverse := {}
	for key: String in paths:
		var path: String = paths[key]
		if reverse.has(path):
			fail("fixture paths duplicate '%s' for %s and %s" % [path, reverse[path], key])
			return {}
		reverse[path] = key
	return reverse


func _path_node(level: WaveLevel, paths: Dictionary, key: String) -> Node:
	if not paths.has(key):
		fail("fixture map has no semantic key '%s'" % key)
		return null
	var path: String = str(paths[key])
	var node := level.get_node_or_null(NodePath(path))
	if node == null:
		fail("fixture path '%s' is missing" % path)
		return null
	return node


func _is_expected_type(node: Node, expected: String) -> bool:
	var matches := false
	match expected:
		"Node3D":
			matches = node is Node3D
		"WaveRun":
			matches = node is WaveRun
		"WaveWall":
			matches = node is WaveWall
		"WaveProp":
			matches = node is WaveProp
		"SoundFan":
			matches = node is SoundFan
		"SoundRadio":
			matches = node is SoundRadio
		"WaveCat":
			matches = node is WaveCat
		"WaveSpawn":
			matches = node is WaveSpawn
		_:
			fail("unknown expected fixture type '%s'" % expected)
	return matches


func _has_exact_keys(actual: Dictionary, expected: Array, label: String) -> bool:
	if actual.size() != expected.size():
		fail("%s expected %d semantic entries, found %d" % [label, expected.size(), actual.size()])
		return false
	for key: String in expected:
		if not actual.has(key):
			fail("%s omit semantic key '%s'" % [label, key])
			return false
	return true


func _mesh_limb(node: Node) -> MeshInstance3D:
	for limb: Node in node.find_children("*", "MeshInstance3D", true, false):
		return limb as MeshInstance3D
	fail("%s did not build a mesh limb" % node.get_path())
	return null


func _assert_matching_transform(actual: Transform3D, expected: Transform3D) -> void:
	_assert_transform_matches(actual, expected, "normalized fixture transform")


func _assert_transform_matches(actual: Transform3D, expected: Transform3D, label: String) -> void:
	var basis_epsilon := Vector3.ONE * BASIS_EPS
	var world_epsilon := Vector3.ONE * WORLD_EPS_M
	assert_vector(actual.basis.x).append_failure_message(label).is_equal_approx(
		expected.basis.x, basis_epsilon
	)
	assert_vector(actual.basis.y).append_failure_message(label).is_equal_approx(
		expected.basis.y, basis_epsilon
	)
	assert_vector(actual.basis.z).append_failure_message(label).is_equal_approx(
		expected.basis.z, basis_epsilon
	)
	assert_vector(actual.origin).append_failure_message(label).is_equal_approx(
		expected.origin, world_epsilon
	)


func _assert_vector4_approx(
	actual: Vector4, expected: Vector4, epsilon: float, label: String
) -> void:
	assert_float(actual.x).append_failure_message(label).is_equal_approx(expected.x, epsilon)
	assert_float(actual.y).append_failure_message(label).is_equal_approx(expected.y, epsilon)
	assert_float(actual.z).append_failure_message(label).is_equal_approx(expected.z, epsilon)
	assert_float(actual.w).append_failure_message(label).is_equal_approx(expected.w, epsilon)
