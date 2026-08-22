extends RefCounted
## Checked, code-built physical scenes shared by player and cat elevation
## tests. Every wait has a fixed physics-frame bound; every authored solid is
## verified through the collider the runtime node actually generated.

const DT := 1.0 / 60.0
const RAMP_SIZE := Vector3(1.4, 0.45, 1.0)
const PLATFORM_SIZE := Vector3(1.2, 0.45, 1.0)
const TABLE_SCENE := preload("res://scenes/props/table.tscn")
const TABLE_TOP_Y := 0.75  # Top centre 0.725 + half-height 0.025.
const BED_TOP_Y := 0.48  # BedFrame centre 0.42 + half-height 0.06.


static func add_box(
	parent: Node, centre: Vector3, size: Vector3, body_name: String
) -> StaticBody3D:
	var body := StaticBody3D.new()
	body.name = body_name
	body.position = centre
	var collision := CollisionShape3D.new()
	var shape := BoxShape3D.new()
	shape.size = size
	collision.shape = shape
	body.add_child(collision)
	parent.add_child(body)
	assert(collision.shape == shape, "%s must retain its checked BoxShape3D" % body_name)
	return body


static func add_floor(parent: Node, top_y := 0.0, size := Vector2(20.0, 20.0)) -> StaticBody3D:
	return add_box(parent, Vector3(0.0, top_y - 0.05, 0.0), Vector3(size.x, 0.1, size.y), "Floor")


static func add_ramp(parent: Node, datum: Vector3, size := RAMP_SIZE) -> WaveWedge:
	assert(parent.is_inside_tree(), "Ramp collider readback requires a live tree parent")
	var ramp := WaveWedge.new()
	ramp.name = "Ramp"
	ramp.size = size
	ramp.position = datum
	parent.add_child(ramp)
	var collisions := ramp.find_children("*", "CollisionShape3D", true, false)
	assert(collisions.size() == 1, "Ramp must generate exactly one collision shape")
	var collision := collisions[0] as CollisionShape3D
	var hull := collision.shape as ConvexPolygonShape3D
	assert(hull != null, "Ramp must generate its ConvexPolygonShape3D")
	assert(hull.points.size() == 6, "Ramp hull must retain all six checked wedge points")
	return ramp


static func add_ramp_platform(parent: Node, datum: Vector3) -> WaveProp:
	assert(parent.is_inside_tree(), "Platform collider readback requires a live tree parent")
	var platform := WaveProp.new()
	platform.name = "RampPlatform"
	platform.size = PLATFORM_SIZE
	platform.position = datum + Vector3(1.3, 0.225, 0.0)
	parent.add_child(platform)
	var collisions := platform.find_children("*", "CollisionShape3D", true, false)
	assert(collisions.size() == 1, "Ramp platform must generate exactly one collision shape")
	var collision := collisions[0] as CollisionShape3D
	var box := collision.shape as BoxShape3D
	assert(box != null, "Ramp platform must generate its BoxShape3D")
	assert(box.size == PLATFORM_SIZE, "Ramp platform collider must match its authored size")
	return platform


static func add_player(parent: Node, at := Vector3(0.0, 0.9, 0.0)) -> UnseeingPlayer:
	var player := UnseeingPlayer.new()
	player.pulses = Pulses.new()
	player.position = at
	parent.add_child(player)
	return player


static func add_cat(parent: Node, at := Vector3.ZERO, seed := 7) -> WaveCat:
	var cat := WaveCat.new()
	cat.pulses = Pulses.new()
	cat.data_mat = ShaderMaterial.new()
	cat.position = at
	cat.seed = seed
	parent.add_child(cat)
	return cat


static func add_table(parent: Node, at: Vector3) -> Node3D:
	var table := TABLE_SCENE.instantiate() as Node3D
	assert(table != null, "table.tscn must retain its Node3D root")
	table.position = at
	parent.add_child(table)
	return table


static func add_bed(parent: Node, at: Vector3) -> WaveProp:
	var bed := WaveProp.new()
	bed.name = "BedFrame"
	bed.size = Vector3(1.9, 0.12, 0.9)
	bed.position = at + Vector3(0.0, 0.42, 0.0)
	parent.add_child(bed)
	return bed


static func poll_physics(tree: SceneTree, predicate: Callable, max_ticks: int) -> bool:
	assert(max_ticks >= 0, "physics poll bound must not be negative")
	if predicate.call():
		return true
	for _tick: int in max_ticks:
		await tree.physics_frame
		if predicate.call():
			return true
	return false
