extends SceneTree
## Headless editor proof that reusable scenes remain authored composition:
## their Rust pieces build independent, ownerless preview limbs, repack
## without serialising those limbs, recurse into the level census, and obey
## global transforms without a script on the prefab root.

const CHAIR := preload("res://scenes/props/chair.tscn")
const TABLE := preload("res://scenes/props/table.tscn")
const DOORWAY := preload("res://scenes/rooms/doorway_8m.tscn")
const ROOM := preload("res://scenes/rooms/room_16x16.tscn")
const READY_FRAMES := 30
const MIN_LABEL_SEP := 0.08
const BOX_FACE_NEG_Y := 2
const BOX_FACE_POS_Y := 3

var _level: WaveLevel
var _chair_a: Node3D
var _chair_b: Node3D
var _table: Node3D
var _doorway: Node3D
var _room: Node3D
var _frames := 0
var _checks := 0
var _failed := 0


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
	if _frames < READY_FRAMES and _chair_a.get_node("Seat").get_child_count() == 0:
		return false
	_judge()
	_report()
	return true


func _judge() -> void:
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
	var skins_exist := seat_skin != null and back_skin != null
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


func _pieces(node: Node) -> Array[Node]:
	return node.find_children("*", "WaveProp", true, false)


func _built_pieces(node: Node) -> int:
	var built := 0
	for piece: Node in _pieces(node):
		if piece.get_child_count() == 2:
			built += 1
	return built


func _limbs_are_ownerless(node: Node) -> bool:
	for piece: Node in _pieces(node):
		for limb: Node in piece.get_children():
			if limb.owner != null:
				return false
	return true


func _piece_has_no_limbs(piece: Node) -> bool:
	return piece.get_child_count() == 0


func _skin(piece: Node) -> MeshInstance3D:
	for child: Node in piece.get_children():
		if child is MeshInstance3D:
			return child as MeshInstance3D
	return null


func _positive_overlap(a_min: float, a_max: float, b_min: float, b_max: float) -> bool:
	return minf(a_max, b_max) - maxf(a_min, b_min) > 0.0


## A labelled box emits four unshared vertices per face in Rust's documented
## order: -X, +X, -Y, +Y, -Z, +Z. Returning NAN on a malformed/nonuniform
## block makes the seam assertion fail instead of accidentally reading a
## neighbouring face or accepting one good vertex.
func _uniform_face_label(skin: MeshInstance3D, face: int) -> float:
	if skin == null or skin.mesh == null or skin.mesh.get_surface_count() == 0:
		return NAN
	var arrays := skin.mesh.surface_get_arrays(0)
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


func _check(what: String, ok: bool) -> void:
	_checks += 1
	if not ok:
		_failed += 1
	print(("ok %d - %s" if ok else "not ok %d - %s") % [_checks, what])


func _report() -> void:
	print("1..%d" % _checks)
	print("probe: %s (%d checks)" % ["PASS" if _failed == 0 else "FAIL", _checks])
	quit(0 if _failed == 0 else 1)
