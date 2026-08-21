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
	if not explained.has("superfaces"):
		fail("observer refused the injected fixture census: %s" % explained)
		return false
	var members := {}
	for superface: Dictionary in explained["superfaces"]:
		for value: Variant in superface.get("members", []):
			var path := str(value)
			if not reverse.has(path):
				fail("observer member '%s' is absent from the fixture map" % path)
				return false
			var key: String = str(reverse[path])
			members[key] = true
	return _has_exact_keys(members, PAINTED_KEYS, "observer membership")


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
	var epsilon := Vector3.ONE * 0.0001
	assert_vector(actual.basis.x).is_equal_approx(expected.basis.x, epsilon)
	assert_vector(actual.basis.y).is_equal_approx(expected.basis.y, epsilon)
	assert_vector(actual.basis.z).is_equal_approx(expected.basis.z, epsilon)
	assert_vector(actual.origin).is_equal_approx(expected.origin, epsilon)
