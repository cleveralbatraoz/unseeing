extends SceneTree
## Headless editor proof that reusable scenes remain authored composition:
## their Rust pieces build independent, ownerless preview limbs, repack
## without serialising those limbs, recurse into the level census, and obey
## global transforms without a script on the prefab root. The scene-authoring
## extension also follows a true inherited room through MAIN instantiation,
## instantiation-preserving duplication, disk, and a nested warning repair.

enum Phase {
	WAIT_LEGACY,
	WAIT_VARIANT_READY,
	WAIT_COMPOSED_READY,
	WAIT_DUPLICATE_READY,
	WAIT_ROUNDTRIP_READY,
	WAIT_INVALID_WARNING,
	WAIT_INVALID_SETTLE,
	WAIT_REPAIR,
	WAIT_REPAIR_SETTLE,
}

const CHAIR := preload("res://scenes/props/chair.tscn")
const TABLE := preload("res://scenes/props/table.tscn")
const DOORWAY := preload("res://scenes/rooms/doorway_8m.tscn")
const ROOM := preload("res://scenes/rooms/room_16x16.tscn")
const READY_FRAMES := 30
const WATCH_FRAMES := 30
const SETTLE_FRAMES := 3
const INVALID_DERIVE_COUNT := -1
const INHERITED_PATH := "res://tests/fixtures/scene_composition/inherited_room_variant.tscn"
const COMPOSED_PATH := "res://tests/fixtures/scene_composition/composed_level.tscn"
const FLAT_PATH := "res://tests/fixtures/scene_composition/flat_level.tscn"
const BASE_PATH := "res://tests/fixtures/scene_composition/base_room.tscn"
const NESTED_PATH := "res://tests/fixtures/scene_composition/nested_prop.tscn"
const MIN_LABEL_SEP := 0.08
const BOX_FACE_NEG_Y := 2
const BOX_FACE_POS_Y := 3
const SEAM_RIGHT := NodePath("PlainGroup/InheritedRoomVariant/NestedProp/SeamRight")
const SUNKEN_WARNING := (
	"WaveLevel: 'PlainGroup/InheritedRoomVariant/NestedProp/SeamRight' is sunk through the floor "
	+ "— its box spans y -0.50..0.50, and the floor's top is at y 0.00. What is under the slab "
	+ "never draws, never sounds and cannot be walked into. A WaveProp is CENTRED on its node, "
	+ "so dropping one on the floor plane buries exactly half of it, while a wall, a column and "
	+ "a wedge STAND on theirs. Lift the node until the whole shape clears y 0.00."
)
const COMPOSED_AUTHORED: Array[NodePath] = [
	NodePath("PlainGroup"),
	NodePath("PlainGroup/InheritedRoomVariant"),
	NodePath("PlainGroup/InheritedRoomVariant/BoundaryRun"),
	NodePath("PlainGroup/InheritedRoomVariant/CrossWall"),
	NodePath("PlainGroup/InheritedRoomVariant/Fan"),
	NodePath("PlainGroup/InheritedRoomVariant/Cat"),
	NodePath("PlainGroup/InheritedRoomVariant/Spawn"),
	NodePath("PlainGroup/InheritedRoomVariant/NestedProp"),
	NodePath("PlainGroup/InheritedRoomVariant/NestedProp/MergeShelf"),
	NodePath("PlainGroup/InheritedRoomVariant/NestedProp/MergeCrate"),
	NodePath("PlainGroup/InheritedRoomVariant/NestedProp/SeamLeft"),
	NodePath("PlainGroup/InheritedRoomVariant/NestedProp/SeamRight"),
	NodePath("PlainGroup/InheritedRoomVariant/Radio"),
]
const GENERATED_ROOTS: Array[NodePath] = [
	NodePath("WaveFloor"),
	NodePath("WaveCeiling"),
	NodePath("PlainGroup/InheritedRoomVariant/BoundaryRun/RunSeg1"),
	NodePath("PlainGroup/InheritedRoomVariant/CrossWall/WaveBody"),
	NodePath("PlainGroup/InheritedRoomVariant/NestedProp/MergeShelf/WaveSkin"),
	NodePath("PlainGroup/InheritedRoomVariant/NestedProp/MergeShelf/WaveCollider"),
	NodePath("PlainGroup/InheritedRoomVariant/NestedProp/MergeCrate/WaveSkin"),
	NodePath("PlainGroup/InheritedRoomVariant/NestedProp/MergeCrate/WaveCollider"),
	NodePath("PlainGroup/InheritedRoomVariant/NestedProp/SeamLeft/WaveSkin"),
	NodePath("PlainGroup/InheritedRoomVariant/NestedProp/SeamLeft/WaveCollider"),
	NodePath("PlainGroup/InheritedRoomVariant/NestedProp/SeamRight/WaveSkin"),
	NodePath("PlainGroup/InheritedRoomVariant/NestedProp/SeamRight/WaveCollider"),
	NodePath("PlainGroup/InheritedRoomVariant/Fan/FanPedestal"),
	NodePath("PlainGroup/InheritedRoomVariant/Fan/FanPivot"),
	NodePath("PlainGroup/InheritedRoomVariant/Cat/CatCollider"),
	NodePath("PlainGroup/InheritedRoomVariant/Cat/CatSkin"),
	NodePath("PlainGroup/InheritedRoomVariant/Radio/RadioCase"),
	NodePath("PlainGroup/InheritedRoomVariant/Radio/RadioGrille"),
	NodePath("PlainGroup/InheritedRoomVariant/Radio/RadioTuner"),
	NodePath("PlainGroup/InheritedRoomVariant/Radio/RadioDialA"),
	NodePath("PlainGroup/InheritedRoomVariant/Radio/RadioDialB"),
	NodePath("PlainGroup/InheritedRoomVariant/Radio/RadioAntenna"),
]
const FORBIDDEN_GENERATED_NAMES := [
	"WaveFloor",
	"WaveCeiling",
	"WaveBody",
	"WaveSkin",
	"WaveCollider",
	"FanPedestal",
	"FanPivot",
	"RadioCase",
	"RadioGrille",
	"RadioTuner",
	"RadioDialA",
	"RadioDialB",
	"RadioAntenna",
	"CatCollider",
	"CatSkin",
]

var _level: WaveLevel
var _chair_a: Node3D
var _chair_b: Node3D
var _table: Node3D
var _doorway: Node3D
var _room: Node3D
var _variant: Node3D
var _composed: WaveLevel
var _duplicate: WaveLevel
var _roundtrip: WaveLevel
var _saved_path := ""
var _frames := 0
var _phase := Phase.WAIT_LEGACY
var _checks := 0
var _failed := 0
var _settle_minimum := INVALID_DERIVE_COUNT
var _settle_last := -1
var _settle_frames := 0


func _initialize() -> void:
	_level = WaveLevel.new()
	_level.extents = Vector2(20, 20)
	_chair_a = CHAIR.instantiate()
	_chair_a.name = "ChairA"
	_chair_a.position = Vector3(3, 0, 3)
	_level.add_child(_chair_a)
	_chair_b = CHAIR.instantiate()
	_chair_b.name = "ChairB"
	_chair_b.position = Vector3(8, 0, 3)
	_level.add_child(_chair_b)
	var turned := Node3D.new()
	turned.rotation.y = PI * 0.5
	var spawn := WaveSpawn.new()
	spawn.position = Vector3(2, 0, 2)
	turned.add_child(spawn)
	_level.add_child(turned)
	root.add_child(_level)
	_table = TABLE.instantiate()
	root.add_child(_table)
	_doorway = DOORWAY.instantiate()
	root.add_child(_doorway)
	_room = ROOM.instantiate()
	_room.position = Vector3(30, 0, 30)
	_room.rotation.y = PI * 0.5
	root.add_child(_room)


func _process(_delta: float) -> bool:
	_frames += 1
	var done := false
	match _phase:
		Phase.WAIT_LEGACY:
			done = _wait_legacy()
		Phase.WAIT_VARIANT_READY:
			done = _wait_variant_ready()
		Phase.WAIT_COMPOSED_READY:
			done = _wait_composed_ready()
		Phase.WAIT_DUPLICATE_READY:
			done = _wait_duplicate_ready()
		Phase.WAIT_ROUNDTRIP_READY:
			done = _wait_roundtrip_ready()
		Phase.WAIT_INVALID_WARNING:
			done = _wait_invalid_warning()
		Phase.WAIT_INVALID_SETTLE:
			done = _wait_invalid_settle()
		Phase.WAIT_REPAIR:
			done = _wait_repair()
		Phase.WAIT_REPAIR_SETTLE:
			done = _wait_repair_settle()
	return done


func _wait_legacy() -> bool:
	if _frames < READY_FRAMES and _chair_a.get_node("Seat").get_child_count() == 0:
		return false
	_judge_legacy()
	_clear_legacy()
	var inherited := ResourceLoader.load(INHERITED_PATH, "PackedScene") as PackedScene
	if not is_instance_valid(inherited):
		_abort("the inherited fixture loads for MAIN instantiation")
		return true
	var state: SceneState = inherited.get_state()
	var base: SceneState = state.get_base_scene_state() if is_instance_valid(state) else null
	_check(
		"the inherited state keeps its base and every exact fixture-local record count",
		is_instance_valid(base) and _fixture_state_counts_are_exact()
	)
	_check(
		"the inherited base keeps NestedProp as one ordinary owned scene instance",
		(
			is_instance_valid(base)
			and _state_instance_path(base, NodePath("./NestedProp")) == NESTED_PATH
			and _state_owner_equals(base, NodePath("./NestedProp"), NodePath("."))
		)
	)
	_check(
		"the inherited local Fan record carries the authored volume override",
		(
			is_instance_valid(state)
			and state.get_node_count() == 3
			and _state_property_equals(state, NodePath("./Fan"), &"volume", 0.6)
		)
	)
	_check(
		"the inherited local Radio record is owned by the inherited root",
		(
			is_instance_valid(state)
			and state.get_node_count() == 3
			and _state_node_index(state, NodePath("./Radio")) >= 0
			and _state_owner_equals(state, NodePath("./Radio"), NodePath("."))
		)
	)
	_variant = inherited.instantiate(PackedScene.GEN_EDIT_STATE_MAIN) as Node3D
	if not is_instance_valid(_variant):
		_abort("the inherited fixture instantiates with GEN_EDIT_STATE_MAIN")
		return true
	root.add_child(_variant)
	_frames = 0
	_phase = Phase.WAIT_VARIANT_READY
	return false


func _wait_variant_ready() -> bool:
	if not is_instance_valid(_variant):
		_abort("the inherited MAIN remains available while its previews settle")
		return true
	var pedestal := _variant.get_node_or_null("Fan/FanPedestal")
	var radio_case := _variant.get_node_or_null("Radio/RadioCase")
	var ready := is_instance_valid(pedestal) and is_instance_valid(radio_case)
	if not ready and _frames < READY_FRAMES:
		return false
	var fan := _variant.get_node_or_null("Fan") as SoundFan
	var radio := _variant.get_node_or_null("Radio") as SoundRadio
	_check(
		"the inherited MAIN instance exposes its override, Radio, and authored transforms",
		(
			ready
			and is_instance_valid(fan)
			and is_instance_valid(radio)
			and _variant.global_transform == Transform3D.IDENTITY
			and fan.volume == 0.6
			and fan.global_position == Vector3(3, 0, 6)
			and radio.global_position == Vector3(7, 0, 7)
		)
	)
	_remove_and_free(_variant)
	_variant = null
	var composed_scene := ResourceLoader.load(COMPOSED_PATH, "PackedScene") as PackedScene
	if not is_instance_valid(composed_scene):
		_abort("the composed fixture loads for MAIN instantiation")
		return true
	_composed = composed_scene.instantiate(PackedScene.GEN_EDIT_STATE_MAIN) as WaveLevel
	if not is_instance_valid(_composed):
		_abort("the composed fixture instantiates with GEN_EDIT_STATE_MAIN")
		return true
	root.add_child(_composed)
	_frames = 0
	_phase = Phase.WAIT_COMPOSED_READY
	return false


func _wait_composed_ready() -> bool:
	if not is_instance_valid(_composed):
		_abort("the composed MAIN remains available while its inventory settles")
		return true
	var generated := _generated_inventory_is_exact(_composed)
	if not generated and _frames < READY_FRAMES:
		return false
	_check(
		"the composed MAIN keeps every authored node under its exact scene owner",
		(
			_fixture_state_counts_are_exact()
			and _authored_nodes_are_owned(_composed, COMPOSED_AUTHORED)
			and _composed_authored_owners_are_exact(_composed)
		)
	)
	_check(
		"the composed MAIN has exactly 48 recursively ownerless generated descendants",
		generated and _generated_subtrees_are_ownerless(_composed, GENERATED_ROOTS)
	)
	_duplicate = _composed.duplicate(Node.DUPLICATE_USE_INSTANTIATION) as WaveLevel
	var entered := false
	if is_instance_valid(_duplicate):
		root.add_child(_duplicate)
		entered = _duplicate.is_inside_tree()
	_check(
		"instantiation-preserving duplication returns a live tree-entered level",
		is_instance_valid(_duplicate) and entered
	)
	if not is_instance_valid(_duplicate):
		_abort("the duplicated level exists for settlement")
		return true
	_frames = 0
	_phase = Phase.WAIT_DUPLICATE_READY
	return false


func _wait_duplicate_ready() -> bool:
	if not is_instance_valid(_duplicate):
		_abort("the duplicate remains available while its inventory settles")
		return true
	var generated := _generated_inventory_is_exact(_duplicate)
	if not generated and _frames < READY_FRAMES:
		return false
	_check(
		"the duplicate settles to 48 generated nodes and one slab pair and RunSeg1",
		(
			generated
			and _named_descendant_count(_duplicate, "WaveFloor") == 1
			and _named_descendant_count(_duplicate, "WaveCeiling") == 1
			and _named_descendant_count(_duplicate, "RunSeg1") == 1
		)
	)
	_check(
		"the duplicate's complete generated subtree inventory remains ownerless",
		_generated_subtrees_are_ownerless(_duplicate, GENERATED_ROOTS)
	)
	var packed := PackedScene.new()
	var pack_error := packed.pack(_duplicate)
	_check("the settled live duplicate packs", pack_error == OK)
	_saved_path = (
		"user://editor-prefab-roundtrip-%d-%d.tscn" % [OS.get_process_id(), Time.get_ticks_usec()]
	)
	var save_error := (
		ResourceSaver.save(packed, _saved_path) if pack_error == OK else ERR_CANT_CREATE
	)
	_check("the packed duplicate saves to its unique user path", save_error == OK)
	_remove_and_free(_composed)
	_composed = null
	_remove_and_free(_duplicate)
	_duplicate = null
	var loaded := (
		(
			ResourceLoader.load(_saved_path, "PackedScene", ResourceLoader.CACHE_MODE_IGNORE_DEEP)
			as PackedScene
		)
		if save_error == OK
		else null
	)
	_check("deep-cache reload returns a PackedScene", is_instance_valid(loaded))
	if not is_instance_valid(loaded):
		_abort("the saved PackedScene reloads for state and warning checks")
		return true
	_check(
		"the reloaded state graph preserves inherited-room and nested-prop instance links",
		_fixture_state_counts_are_exact() and _roundtrip_links_are_exact(loaded.get_state())
	)
	_roundtrip = loaded.instantiate(PackedScene.GEN_EDIT_STATE_MAIN) as WaveLevel
	if not is_instance_valid(_roundtrip):
		_abort("the reloaded scene instantiates with GEN_EDIT_STATE_MAIN")
		return true
	root.add_child(_roundtrip)
	_frames = 0
	_phase = Phase.WAIT_ROUNDTRIP_READY
	return false


func _wait_roundtrip_ready() -> bool:
	if not is_instance_valid(_roundtrip):
		_abort("the reloaded MAIN remains available while its inventory settles")
		return true
	var generated := _generated_inventory_is_exact(_roundtrip)
	if not generated and _frames < READY_FRAMES:
		return false
	var fan := _roundtrip.get_node_or_null("PlainGroup/InheritedRoomVariant/Fan") as SoundFan
	var radio := _roundtrip.get_node_or_null("PlainGroup/InheritedRoomVariant/Radio")
	_check(
		"the reloaded MAIN keeps authored ownership, Fan volume 0.6, and Radio",
		(
			generated
			and _fixture_state_counts_are_exact()
			and _authored_nodes_are_owned(_roundtrip, COMPOSED_AUTHORED)
			and _composed_authored_owners_are_exact(_roundtrip)
			and is_instance_valid(fan)
			and fan.volume == 0.6
			and is_instance_valid(radio)
			and radio is SoundRadio
		)
	)
	var loaded_scene := (
		ResourceLoader.load(_saved_path, "PackedScene", ResourceLoader.CACHE_MODE_IGNORE_DEEP)
		as PackedScene
	)
	_check(
		"the complete saved SceneState graph contains no generated name",
		(
			_fixture_state_counts_are_exact()
			and is_instance_valid(loaded_scene)
			and not _state_graph_has_forbidden(loaded_scene.get_state(), {})
		)
	)
	var seam := _roundtrip.get_node_or_null(SEAM_RIGHT) as Node3D
	if not is_instance_valid(seam):
		_abort("the nested SeamRight exists for warning watching")
		return true
	var before := _derive_count(_roundtrip)
	if before == INVALID_DERIVE_COUNT:
		_abort("derive_count is valid before sinking nested SeamRight")
		return true
	seam.position.y = 0.0
	_begin_settle(_roundtrip, before + 1)
	_frames = 0
	_phase = Phase.WAIT_INVALID_WARNING
	return false


func _wait_invalid_warning() -> bool:
	if not is_instance_valid(_roundtrip):
		_abort("the reloaded MAIN remains available while its warning settles")
		return true
	var seam := _roundtrip.get_node_or_null(SEAM_RIGHT)
	var warnings := _warning_snapshot(seam)
	var changed := (
		is_instance_valid(seam)
		and _warning_snapshot_is_valid(warnings)
		and _settle_minimum != INVALID_DERIVE_COUNT
		and _derive_count(_roundtrip) >= _settle_minimum
		and _warning_snapshot_has(warnings, SUNKEN_WARNING)
	)
	if not changed and _frames < WATCH_FRAMES:
		return false
	_check("sinking nested SeamRight raises derive_count and its exact full-path warning", changed)
	_frames = 0
	_settle_frames = 0
	_settle_last = -1
	_phase = Phase.WAIT_INVALID_SETTLE
	return false


func _wait_invalid_settle() -> bool:
	if not is_instance_valid(_roundtrip):
		_abort("the reloaded MAIN remains available during invalid settlement")
		return true
	var stable := _settled(_roundtrip)
	if not stable and _frames < WATCH_FRAMES:
		return false
	_check("the invalid authored state holds one derive count for three frames", stable)
	var seam := _roundtrip.get_node_or_null(SEAM_RIGHT) as Node3D
	if not is_instance_valid(seam):
		_abort("the nested SeamRight remains available for repair")
		return true
	var before := _derive_count(_roundtrip)
	if before == INVALID_DERIVE_COUNT:
		_abort("derive_count is valid before repairing nested SeamRight")
		return true
	seam.position.y = 0.5
	_begin_settle(_roundtrip, before + 1)
	_frames = 0
	_phase = Phase.WAIT_REPAIR
	return false


func _wait_repair() -> bool:
	if not is_instance_valid(_roundtrip):
		_abort("the reloaded MAIN remains available while its repair settles")
		return true
	var seam := _roundtrip.get_node_or_null(SEAM_RIGHT)
	var warnings := _warning_snapshot(seam)
	var repaired := (
		is_instance_valid(seam)
		and _warning_snapshot_is_valid(warnings)
		and _settle_minimum != INVALID_DERIVE_COUNT
		and _derive_count(_roundtrip) >= _settle_minimum
		and _warning_snapshot_is_empty(warnings)
	)
	if not repaired and _frames < WATCH_FRAMES:
		return false
	_check("repairing nested SeamRight raises the count and clears its warning", repaired)
	_frames = 0
	_settle_frames = 0
	_settle_last = -1
	_phase = Phase.WAIT_REPAIR_SETTLE
	return false


func _wait_repair_settle() -> bool:
	if not is_instance_valid(_roundtrip):
		_abort("the reloaded MAIN remains available during repaired settlement")
		return true
	var stable := _settled(_roundtrip)
	if not stable and _frames < WATCH_FRAMES:
		return false
	_check(
		"the repaired state stays stable for three frames and removes the temporary scene",
		stable and _remove_temp_scene()
	)
	_report()
	return true


func _judge_legacy() -> void:
	print("# prefabs: mode=%s" % ("editor" if Engine.is_editor_hint() else "run"))
	_check("the probe is in editor mode", Engine.is_editor_hint())
	_check("two instances keep independent typed pieces", _pieces(_level).size() == 12)
	_check("the first instance builds all six previews", _built_pieces(_chair_a) == 6)
	_check("the second instance builds all six previews", _built_pieces(_chair_b) == 6)
	_check("generated preview limbs stay ownerless", _limbs_are_ownerless(_chair_a))
	var packed := PackedScene.new()
	_check("an instantiated chair repacks", packed.pack(_chair_a) == OK)
	var copy := packed.instantiate()
	_check("repacking preserves six authored pieces", _pieces(copy).size() == 6)
	_check("repacking leaks no generated limbs", _pieces(copy).all(_piece_has_no_limbs))
	var seat := _chair_a.get_node("Seat") as WaveProp
	var back := _chair_a.get_node("Back") as WaveProp
	var seat_skin := _skin(seat)
	var back_skin := _skin(back)
	var seat_box := AABB()
	var back_box := AABB()
	var seat_touch_label := NAN
	var back_touch_label := NAN
	var skins_exist := is_instance_valid(seat_skin) and is_instance_valid(back_skin)
	if skins_exist:
		seat_box = seat_skin.global_transform * seat_skin.get_aabb()
		back_box = back_skin.global_transform * back_skin.get_aabb()
		seat_touch_label = _uniform_face_label(seat_skin, BOX_FACE_POS_Y)
		back_touch_label = _uniform_face_label(back_skin, BOX_FACE_NEG_Y)
	_check(
		"the chair seat top meets the back bottom",
		skins_exist and is_equal_approx(seat_box.position.y + seat_box.size.y, back_box.position.y)
	)
	_check(
		"the touching chair faces overlap across both planar axes",
		(
			skins_exist
			and _positive_overlap(
				seat_box.position.x, seat_box.end.x, back_box.position.x, back_box.end.x
			)
			and _positive_overlap(
				seat_box.position.z, seat_box.end.z, back_box.position.z, back_box.end.z
			)
		)
	)
	_check(
		"the actual touching face labels keep full crease separation",
		(
			not is_nan(seat_touch_label)
			and not is_nan(back_touch_label)
			and absf(seat_touch_label - back_touch_label) >= MIN_LABEL_SEP
		)
	)
	_check(
		"a nested spawn inherits the prefab quarter turn",
		is_equal_approx(_level.spawn_yaw(), PI * 0.5)
	)
	_check("the table is a plain draggable root with five pieces", _pieces(_table).size() == 5)
	_check(
		"the doorway prefab emits two residual walls",
		_doorway.find_children("*", "WaveWall", true, false).size() == 2
	)
	_check(
		"the room prefab emits five border segments",
		_room.find_children("*", "WaveWall", true, false).size() == 5
	)
	var north := _room.get_node("North/RunSeg1") as WaveWall
	_check(
		"a rotated room composes its ancestor transform",
		not north.global_position.is_equal_approx(north.position)
	)
	copy.free()


func _state_node_index(state: SceneState, path: NodePath) -> int:
	if not is_instance_valid(state):
		return -1
	for index: int in range(state.get_node_count()):
		if state.get_node_path(index) == path:
			return index
	return -1


func _state_property_equals(
	state: SceneState, path: NodePath, property: StringName, expected: Variant
) -> bool:
	var index := _state_node_index(state, path)
	if not is_instance_valid(state) or index < 0:
		return false
	for property_index: int in range(state.get_node_property_count(index)):
		if state.get_node_property_name(index, property_index) == property:
			return state.get_node_property_value(index, property_index) == expected
	return false


func _state_instance_path(state: SceneState, path: NodePath) -> String:
	var index := _state_node_index(state, path)
	if not is_instance_valid(state) or index < 0:
		return ""
	var instance: Variant = state.get_node_instance(index)
	var packed := instance as PackedScene if instance is PackedScene else null
	return packed.resource_path if is_instance_valid(packed) else ""


func _state_graph_has_forbidden(state: SceneState, seen: Dictionary) -> bool:
	if not is_instance_valid(state):
		return true
	var state_id := state.get_instance_id()
	if seen.has(state_id):
		return false
	seen[state_id] = true
	for index: int in range(state.get_node_count()):
		var path := state.get_node_path(index)
		var name_count := path.get_name_count()
		if name_count > 0 and _is_forbidden_generated_name(path.get_name(name_count - 1)):
			return true
		var instance: Variant = state.get_node_instance(index)
		if instance is PackedScene:
			var packed := instance as PackedScene
			if (
				not is_instance_valid(packed)
				or _state_graph_has_forbidden(packed.get_state(), seen)
			):
				return true
	var base: SceneState = state.get_base_scene_state()
	return base != null and (not is_instance_valid(base) or _state_graph_has_forbidden(base, seen))


func _is_forbidden_generated_name(name: String) -> bool:
	return name.begins_with("RunSeg") or name in FORBIDDEN_GENERATED_NAMES


func _authored_nodes_are_owned(node: Node, paths: Array[NodePath]) -> bool:
	if not is_instance_valid(node):
		return false
	for path: NodePath in paths:
		var authored := node.get_node_or_null(path)
		if not is_instance_valid(authored) or not is_instance_valid(authored.owner):
			return false
	return true


func _generated_subtrees_are_ownerless(node: Node, paths: Array[NodePath]) -> bool:
	if not is_instance_valid(node):
		return false
	var count := 0
	for path: NodePath in paths:
		var generated := node.get_node_or_null(path)
		if not is_instance_valid(generated):
			return false
		var subtree: Array[Node] = [generated]
		while not subtree.is_empty():
			var current: Node = subtree.pop_back()
			if not is_instance_valid(current):
				return false
			if current.owner != null:
				return false
			count += 1
			subtree.append_array(current.get_children())
	var every_ownerless := 0
	for descendant: Node in node.find_children("*", "", true, false):
		if not is_instance_valid(descendant):
			return false
		if descendant.owner == null:
			every_ownerless += 1
	return count == 48 and every_ownerless == 48


func _generated_inventory_is_exact(node: Node) -> bool:
	if not is_instance_valid(node) or not _generated_subtrees_are_ownerless(node, GENERATED_ROOTS):
		return false
	var run_segments := 0
	for descendant: Node in node.find_children("*", "", true, false):
		if String(descendant.name).begins_with("RunSeg"):
			run_segments += 1
	return (
		run_segments == 1
		and _named_descendant_count(node, "WaveFloor") == 1
		and _named_descendant_count(node, "WaveCeiling") == 1
		and _named_descendant_count(node, "RunSeg1") == 1
	)


func _warning_has(node: Node, needle: String) -> bool:
	return _warning_snapshot_has(_warning_snapshot(node), needle)


func _warning_snapshot(node: Node) -> Dictionary:
	if not is_instance_valid(node) or not node.has_method("get_configuration_warnings"):
		return {}
	var result: Variant = node.call("get_configuration_warnings")
	return {"warnings": result} if result is PackedStringArray else {}


func _warning_snapshot_is_valid(snapshot: Dictionary) -> bool:
	return snapshot.has("warnings") and snapshot["warnings"] is PackedStringArray


func _warning_snapshot_has(snapshot: Dictionary, needle: String) -> bool:
	if not _warning_snapshot_is_valid(snapshot):
		return false
	var warnings: PackedStringArray = snapshot["warnings"]
	for warning: String in warnings:
		if warning == needle:
			return true
	return false


func _warning_snapshot_is_empty(snapshot: Dictionary) -> bool:
	if not _warning_snapshot_is_valid(snapshot):
		return false
	var warnings: PackedStringArray = snapshot["warnings"]
	return warnings.is_empty()


func _begin_settle(level: WaveLevel, minimum_count: int) -> void:
	var current := _derive_count(level)
	_settle_minimum = (
		maxi(minimum_count, current)
		if minimum_count >= 0 and current != INVALID_DERIVE_COUNT
		else INVALID_DERIVE_COUNT
	)
	_settle_last = -1
	_settle_frames = 0


func _settled(level: WaveLevel) -> bool:
	var count := _derive_count(level)
	if (
		_settle_minimum == INVALID_DERIVE_COUNT
		or count == INVALID_DERIVE_COUNT
		or count < _settle_minimum
	):
		return false
	if count != _settle_last:
		_settle_last = count
		_settle_frames = 1
	else:
		_settle_frames += 1
	return _settle_frames >= SETTLE_FRAMES


func _derive_count(level: WaveLevel) -> int:
	if not is_instance_valid(level) or not level.has_method("derive_count"):
		return INVALID_DERIVE_COUNT
	var result: Variant = level.call("derive_count")
	if not result is int or result < 0:
		return INVALID_DERIVE_COUNT
	return result


func _remove_temp_scene() -> bool:
	if _saved_path.is_empty():
		return true
	var absolute := ProjectSettings.globalize_path(_saved_path)
	if not FileAccess.file_exists(absolute):
		_saved_path = ""
		return true
	var removed := DirAccess.remove_absolute(absolute) == OK
	if removed and not FileAccess.file_exists(absolute):
		_saved_path = ""
		return true
	return false


func _fixture_state_counts_are_exact() -> bool:
	var expected := {
		NESTED_PATH: 5, BASE_PATH: 7, INHERITED_PATH: 3, COMPOSED_PATH: 3, FLAT_PATH: 11
	}
	for path: String in expected:
		var scene := ResourceLoader.load(path, "PackedScene") as PackedScene
		var state: SceneState = scene.get_state() if is_instance_valid(scene) else null
		if not is_instance_valid(state) or state.get_node_count() != expected[path]:
			return false
	return true


func _state_owner_equals(state: SceneState, path: NodePath, expected: NodePath) -> bool:
	var index := _state_node_index(state, path)
	return is_instance_valid(state) and index >= 0 and state.get_node_owner_path(index) == expected


func _roundtrip_links_are_exact(state: SceneState) -> bool:
	if not is_instance_valid(state) or state.get_node_count() != 3:
		return false
	var room_index := _state_node_index(state, NodePath("./PlainGroup/InheritedRoomVariant"))
	if room_index < 0:
		return false
	var room_variant: Variant = state.get_node_instance(room_index)
	if not room_variant is PackedScene:
		return false
	var room := room_variant as PackedScene
	if not is_instance_valid(room) or room.resource_path != INHERITED_PATH:
		return false
	var room_state: SceneState = room.get_state()
	if not is_instance_valid(room_state):
		return false
	var base: SceneState = room_state.get_base_scene_state()
	return (
		room_state.get_node_count() == 3
		and is_instance_valid(base)
		and base.get_node_count() == 7
		and _state_instance_path(base, NodePath("./NestedProp")) == NESTED_PATH
	)


func _composed_authored_owners_are_exact(node: Node) -> bool:
	if not is_instance_valid(node):
		return false
	var group := node.get_node_or_null("PlainGroup")
	var room := node.get_node_or_null("PlainGroup/InheritedRoomVariant")
	var nested := node.get_node_or_null("PlainGroup/InheritedRoomVariant/NestedProp")
	if not is_instance_valid(group) or not is_instance_valid(room) or not is_instance_valid(nested):
		return false
	if group.owner != node or room.owner != node:
		return false
	for child_name: String in [
		"BoundaryRun", "CrossWall", "Fan", "Cat", "Spawn", "NestedProp", "Radio"
	]:
		var child := room.get_node_or_null(child_name)
		if not is_instance_valid(child) or child.owner != room:
			return false
	for prop_name: String in ["MergeShelf", "MergeCrate", "SeamLeft", "SeamRight"]:
		var prop := nested.get_node_or_null(prop_name)
		if not is_instance_valid(prop) or prop.owner != nested:
			return false
	return true


func _named_descendant_count(node: Node, wanted: String) -> int:
	if not is_instance_valid(node):
		return 0
	var count := 0
	for descendant: Node in node.find_children("*", "", true, false):
		if not is_instance_valid(descendant):
			return 0
		if descendant.name == wanted:
			count += 1
	return count


func _clear_legacy() -> void:
	for node: Node in [_level, _table, _doorway, _room]:
		_remove_and_free(node)
	_level = null
	_chair_a = null
	_chair_b = null
	_table = null
	_doorway = null
	_room = null


func _remove_and_free(node: Node) -> void:
	if not is_instance_valid(node):
		return
	var parent := node.get_parent()
	if is_instance_valid(parent):
		parent.remove_child(node)
	node.free()


func _pieces(node: Node) -> Array[Node]:
	if not is_instance_valid(node):
		return []
	return node.find_children("*", "WaveProp", true, false)


func _built_pieces(node: Node) -> int:
	var built := 0
	for piece: Node in _pieces(node):
		if is_instance_valid(piece) and piece.get_child_count() == 2:
			built += 1
	return built


func _limbs_are_ownerless(node: Node) -> bool:
	for piece: Node in _pieces(node):
		if not is_instance_valid(piece):
			return false
		for limb: Node in piece.get_children():
			if not is_instance_valid(limb):
				return false
			if limb.owner != null:
				return false
	return true


func _piece_has_no_limbs(piece: Node) -> bool:
	return is_instance_valid(piece) and piece.get_child_count() == 0


func _skin(piece: Node) -> MeshInstance3D:
	if not is_instance_valid(piece):
		return null
	for child: Node in piece.get_children():
		if is_instance_valid(child) and child is MeshInstance3D:
			return child as MeshInstance3D
	return null


func _positive_overlap(a_min: float, a_max: float, b_min: float, b_max: float) -> bool:
	return minf(a_max, b_max) - maxf(a_min, b_min) > 0.0


## A labelled box emits four unshared vertices per face in Rust's documented
## order: -X, +X, -Y, +Y, -Z, +Z. Returning NAN on a malformed/nonuniform
## block makes the seam assertion fail instead of accidentally reading a
## neighbouring face or accepting one good vertex.
func _uniform_face_label(skin: MeshInstance3D, face: int) -> float:
	var mesh := skin.mesh if is_instance_valid(skin) else null
	if not is_instance_valid(skin) or not is_instance_valid(mesh) or mesh.get_surface_count() == 0:
		return NAN
	var arrays := mesh.surface_get_arrays(0)
	if arrays.size() <= Mesh.ARRAY_CUSTOM0:
		return NAN
	var encoded: Variant = arrays[Mesh.ARRAY_CUSTOM0]
	if not encoded is PackedFloat32Array:
		return NAN
	var custom: PackedFloat32Array = encoded
	var first := face * 4
	if first < 0 or first + 4 > custom.size():
		return NAN
	var label := custom[first]
	for vertex: int in range(first + 1, first + 4):
		if custom[vertex] != label:
			return NAN
	return label


func _abort(what: String) -> void:
	while _checks < 36:
		_check(what, false)
	_report()


func _check(what: String, ok: bool) -> void:
	_checks += 1
	if not ok:
		_failed += 1
	print(("ok %d - %s" if ok else "not ok %d - %s") % [_checks, what])


func _report() -> void:
	_remove_temp_scene()
	print("1..%d" % _checks)
	if _failed > 0:
		print("probe: FAIL (%d of %d)" % [_failed, _checks])
	else:
		print("probe: PASS (%d checks)" % _checks)
	quit(0 if _failed == 0 else 1)


func _finalize() -> void:
	_remove_temp_scene()
