class_name MapBuilder
extends RefCounted
## The map: a 20 x 20 m room with interior walls forming corridors.
## Geometry and collision are built from wall CENTERLINE segments — the same
## numbers as the validated original design, so both versions play identically.
## All segments are axis-aligned, which keeps the boxes trivial.

const WALL_H := 3.0    # walls run floor to ceiling
const WALL_T := 0.15   # half-thickness of a wall

## Wall centerlines [x1, z1, x2, z2] in meters. First four are the border.
const SEGS := [
	[0.6, 0.6, 19.4, 0.6],
	[19.4, 0.6, 19.4, 19.4],
	[19.4, 19.4, 0.6, 19.4],
	[0.6, 19.4, 0.6, 0.6],
	[6.4, 0.6, 6.4, 8.0],
	[6.4, 12.4, 6.4, 19.4],
	[6.4, 8.0, 14.0, 8.0],
	[14.0, 8.0, 14.0, 15.6],
	[9.0, 15.6, 14.0, 15.6],
	[0.6, 13.0, 4.0, 13.0],
]

static func build_world(parent: Node3D) -> void:
	# floor and ceiling as thin slabs; only their inward faces are ever seen
	_add_box(parent, Materials.Kind.ROCK, Vector3(10, -0.05, 10), Vector3(20, 0.1, 20))
	_add_box(parent, Materials.Kind.ROCK, Vector3(10, WALL_H + 0.05, 10), Vector3(20, 0.1, 20))
	_build_furniture(parent)
	for s: Array in SEGS:
		var horizontal: bool = absf(s[3] - s[1]) < 0.001
		# the box math trusts axis alignment; a diagonal segment would silently
		# produce a wrong-sized wall instead of failing
		assert(horizontal or absf(s[2] - s[0]) < 0.001,
				"map segment %s is not axis-aligned" % [s])
		var cx: float = (s[0] + s[2]) * 0.5
		var cz: float = (s[1] + s[3]) * 0.5
		var size: Vector3
		if horizontal:
			size = Vector3(absf(s[2] - s[0]) + WALL_T * 2.0, WALL_H, WALL_T * 2.0)
		else:
			size = Vector3(WALL_T * 2.0, WALL_H, absf(s[3] - s[1]) + WALL_T * 2.0)
		_add_box(parent, Materials.Kind.ROCK, Vector3(cx, WALL_H * 0.5, cz), size)

## Furniture near the spawn: waist-height obstacles the border walls can't
## teach you about. Waves outline their edges, their bodies carve bites out
## of passing wave shells, and the cane's 3D tap ray can strike them directly.
static func _build_furniture(parent: Node3D) -> void:
	# table ahead-right of spawn: four legs and a top
	var t := Vector3(4.6, 0, 4.9)
	_add_box(parent, Materials.Kind.WOOD, t + Vector3(0, 0.72, 0), Vector3(0.9, 0.05, 0.6))
	for lx: float in [-0.4, 0.4]:
		for lz: float in [-0.24, 0.24]:
			_add_box(parent, Materials.Kind.WOOD, t + Vector3(lx, 0.35, lz), Vector3(0.05, 0.7, 0.05))
	# chair beside it, backrest toward the player spawn
	var c := Vector3(3.9, 0, 5.55)
	_add_box(parent, Materials.Kind.WOOD, c + Vector3(0, 0.45, 0), Vector3(0.4, 0.05, 0.4))
	for lx: float in [-0.17, 0.17]:
		for lz: float in [-0.17, 0.17]:
			_add_box(parent, Materials.Kind.WOOD, c + Vector3(lx, 0.22, lz), Vector3(0.04, 0.45, 0.04))
	_add_box(parent, Materials.Kind.WOOD, c + Vector3(-0.18, 0.72, 0), Vector3(0.05, 0.5, 0.4))

## One box = a mesh for the data pass + a static collider for the cane rays
## and player movement. The material KIND is level data; the instance comes
## from the Materials module.
static func _add_box(parent: Node3D, kind: Materials.Kind, center: Vector3, size: Vector3) -> void:
	var mat := Materials.shared(kind)
	var body := StaticBody3D.new()
	body.position = center
	var mesh := MeshInstance3D.new()
	var box := BoxMesh.new()
	box.size = size
	mesh.mesh = box
	mesh.material_override = mat
	body.add_child(mesh)
	var col := CollisionShape3D.new()
	var shape := BoxShape3D.new()
	shape.size = size
	col.shape = shape
	body.add_child(col)
	parent.add_child(body)
