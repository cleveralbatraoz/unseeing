extends SceneTree
## Headless editor proof that reusable scenes remain authored composition:
## their Rust pieces build independent, ownerless preview limbs, repack
## without serialising those limbs, recurse into the level census, and obey
## global transforms without a script on the prefab root.

const CHAIR := preload("res://scenes/props/chair.tscn")
const READY_FRAMES := 30

var _level: WaveLevel
var _chair_a: Node3D
var _chair_b: Node3D
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
	_check("touching seat and back receive distinct ids", seat.oid() != back.oid())
	_check(
		"a nested spawn inherits the prefab quarter turn",
		is_equal_approx(_level.spawn_yaw(), PI * 0.5)
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


func _check(what: String, ok: bool) -> void:
	_checks += 1
	if not ok:
		_failed += 1
	print(("ok %d - %s" if ok else "not ok %d - %s") % [_checks, what])


func _report() -> void:
	print("1..%d" % _checks)
	print("probe: %s (%d checks)" % ["PASS" if _failed == 0 else "FAIL", _checks])
	quit(0 if _failed == 0 else 1)
