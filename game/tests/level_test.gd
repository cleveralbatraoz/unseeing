extends GdUnitTestSuite
## The level-authoring engine nodes, held to their designer-safety laws:
## a wall builds its outline box and collider from nothing but transform
## and length knob; free-hand rotation snaps to EXACT quarter turns (walls
## are axis-aligned by law, and the snapped basis carries no trig dust);
## props stay free; and the level root is the single injection point that
## distributes material and pool, then derives the hum room from the walls
## around the fan, the spawn from its marker, and the demo tap from the
## room's west wall.
##
## The second half holds the SHIPPED scene to the validated design's
## invariants — the same laws map_builder_test held over the SEGS table,
## now read back from scenes/level_01.tscn itself.

const LEVEL_SCENE := preload("res://scenes/level_01.tscn")

## Full-strength crease separation, read off hearing_post.gdshader's
## smoothstep(0.04, 0.08, nrm) upper knee on the G channel.
const MIN_OID_SEP := 0.08

## Boxes that share a face register as touching at exactly zero overlap.
const TOUCH_EPS := 0.01


## One authored wall, the way the generator and a designer both place it:
## floor position at the centerline midpoint, yaw for the axis.
func _wall(length: float, at: Vector3, vertical: bool) -> WaveWall:
	var wall := WaveWall.new()
	wall.length = length
	wall.position = at
	if vertical:
		wall.rotation.y = PI * 0.5
	return wall


func _spawn_marker(at: Vector3, yaw: float) -> Marker3D:
	var marker := Marker3D.new()
	marker.name = "SpawnPoint"
	marker.position = at
	marker.rotation.y = yaw
	return marker


## The first mesh limb a node built for itself.
func _skin(body: Node) -> MeshInstance3D:
	for child: Node in body.get_children():
		if child is MeshInstance3D:
			return child as MeshInstance3D
	return null


func _box(body: Node) -> BoxMesh:
	var skin := _skin(body)
	return null if skin == null else skin.mesh as BoxMesh


func _box_shape(body: Node) -> BoxShape3D:
	for child: Node in body.get_children():
		if child is CollisionShape3D:
			var col := child as CollisionShape3D
			return col.shape as BoxShape3D
	return null


## The floor/ceiling slabs the level built for itself — its own direct
## StaticBody3D children that are not authored walls or props.
func _slabs(level: WaveLevel) -> Array[StaticBody3D]:
	var out: Array[StaticBody3D] = []
	for child: Node in level.get_children():
		if child is StaticBody3D and not child is WaveWall and not child is WaveProp:
			out.append(child as StaticBody3D)
	return out


## A wall is its centerline padded by a half-thickness each way, floor to
## ceiling — mesh and collider the same box, risen from the floor node.
func test_wall_builds_box_and_collider_from_length() -> void:
	var wall: WaveWall = auto_free(_wall(7.4, Vector3(2, 0, 3), false))
	add_child(wall)
	assert_vector(_box(wall).size).is_equal(Vector3(7.7, 3, 0.3))
	assert_vector(_box_shape(wall).size).is_equal(Vector3(7.7, 3, 0.3))
	assert_vector(_skin(wall).position).is_equal(Vector3(0, 1.5, 0))


## The designer-safety law: a wall rotated to any free-hand angle (tilt
## included) snaps to the nearest quarter turn on entering the tree, and
## the snapped basis is EXACT — unit columns of 0 and ±1, bit for bit.
func test_wall_snaps_freehand_rotation_to_exact_axis() -> void:
	var wall: WaveWall = auto_free(_wall(4.0, Vector3.ZERO, false))
	wall.rotation = Vector3(0.2, 1.2, -0.1)  # tilted, off-axis: unplaceable by hand
	add_child(wall)
	assert_vector(wall.basis.x).is_equal(Vector3(0, 0, -1))
	assert_vector(wall.basis.y).is_equal(Vector3(0, 1, 0))
	assert_vector(wall.basis.z).is_equal(Vector3(1, 0, 0))
	assert_float(wall.rotation.y).is_equal_approx(PI * 0.5, 0.0001)


## The length knob reshapes a placed wall live — mesh and collider
## together, the way a designer drags a number in the Inspector.
func test_wall_length_knob_reshapes_live() -> void:
	var wall: WaveWall = auto_free(_wall(4.0, Vector3.ZERO, false))
	add_child(wall)
	wall.length = 9.0
	assert_vector(_box(wall).size).is_equal(Vector3(9.3, 3, 0.3))
	assert_vector(_box_shape(wall).size).is_equal(Vector3(9.3, 3, 0.3))


## A prop is a free box: its size knob is the full extent, its node the
## box center, and its rotation is NOT snapped — props carry no room
## contract, waves outline them from any angle.
func test_prop_builds_its_free_box() -> void:
	var prop: WaveProp = auto_free(WaveProp.new())
	prop.size = Vector3(0.9, 0.05, 0.6)
	prop.position = Vector3(4.6, 0.72, 4.9)
	prop.rotation.y = 0.3
	add_child(prop)
	assert_vector(_box(prop).size).is_equal(Vector3(0.9, 0.05, 0.6))
	assert_vector(_box_shape(prop).size).is_equal(Vector3(0.9, 0.05, 0.6))
	assert_vector(_skin(prop).position).is_equal(Vector3.ZERO)
	assert_float(prop.rotation.y).is_equal_approx(0.3, 0.0001)
	prop.size = Vector3(0.4, 0.05, 0.4)  # the knob reshapes live, like the wall's
	assert_vector(_box(prop).size).is_equal(Vector3(0.4, 0.05, 0.4))


## The level root end to end: one inject call distributes the material to
## every wall, prop and slab and both handles to the fan; entering the
## tree derives the hum room from the walls around the fan, the spawn
## from its marker (lifted to capsule height), and the demo tap on the
## room's west wall facing the spawn's side.
func test_level_distributes_and_derives() -> void:
	var mat := ShaderMaterial.new()
	var pulses := Pulses.new()
	var level: WaveLevel = auto_free(WaveLevel.new())
	var walls: Array[WaveWall] = [
		_wall(4.0, Vector3(2, 0, 0), false),
		_wall(4.0, Vector3(2, 0, 4), false),
		_wall(4.0, Vector3(0, 0, 2), true),
		_wall(4.0, Vector3(4, 0, 2), true),
	]
	for wall: WaveWall in walls:
		level.add_child(wall)
	var prop := WaveProp.new()
	prop.size = Vector3(0.9, 0.72, 0.6)
	prop.position = Vector3(1.5, 0.36, 2)
	level.add_child(prop)
	var fan := SoundFan.new()
	fan.position = Vector3(3, 0, 1)
	level.add_child(fan)
	level.add_child(_spawn_marker(Vector3(1, 0, 3), -0.6))
	level.inject(mat, pulses)
	add_child(level)
	assert_vector(level.hum_room()).is_equal(Vector4(0, 0, 4, 4))
	assert_vector(level.spawn_pos()).is_equal(Vector3(1, 0.9, 3))
	assert_float(level.spawn_yaw()).is_equal_approx(-0.6, 0.0001)
	assert_vector(level.demo_tap()).is_equal(Vector3(0, 0.8, 3))
	assert_vector(level.demo_tap_normal()).is_equal(Vector3(1, 0, 0))  # spawn inside the room
	assert_int(level.wall_segments().size()).is_equal(4)
	assert_object(level.fan()).is_same(fan)
	assert_object(fan.pulses).is_same(pulses)
	assert_object(fan.data_mat).is_same(mat)
	for wall: WaveWall in walls:
		assert_object(_skin(wall).material_override).is_same(mat)
	assert_object(_skin(prop).material_override).is_same(mat)
	var slabs := _slabs(level)
	assert_int(slabs.size()).is_equal(2)
	for slab: StaticBody3D in slabs:
		assert_vector(_box(slab).size).is_equal(Vector3(20, 0.1, 20))
		assert_object(_skin(slab).material_override).is_same(mat)
	assert_vector(slabs[0].position).is_equal(Vector3(10, -0.05, 10))
	assert_vector(slabs[1].position).is_equal(Vector3(10, 3.05, 10))


## No silent nulls: a level never injected reports the miss on entering
## the tree — the world it carries could never be seen.
func test_uninjected_level_reports() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3.ZERO, 0.0))
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		"WaveLevel: data_mat/pulses not injected — the level cannot be seen"
	))


## A level without a SpawnPoint marker has nowhere to wake the hero —
## loud, with the level origin as the fallback.
func test_missing_spawn_reports() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.inject(ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		"WaveLevel: no SpawnPoint marker — the hero has nowhere to wake"
	))
	assert_vector(level.spawn_pos()).is_equal(Vector3(0, 0.9, 0))


## A fan with an open side has no room: derivation refuses loudly and
## leaves the rect ZERO — hum waves would reveal everywhere, and the
## designer is told so instead of shipping it silently.
func test_unenclosed_fan_reports() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_wall(4.0, Vector3(2, 0, 0), false))
	var fan := SoundFan.new()
	fan.position = Vector3(2, 0, 2)
	level.add_child(fan)
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		"WaveLevel: the fan is not enclosed by walls — its hum will reveal everywhere"
	))
	assert_vector(level.hum_room()).is_equal(Vector4(0, 0, 0, 0))


## A fanless level is legal silence: no hum room, no demo tap, no error.
func test_fanless_level_has_no_hum_room() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_wall(4.0, Vector3(2, 0, 0), false))
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), Pulses.new())
	add_child(level)
	assert_vector(level.hum_room()).is_equal(Vector4(0, 0, 0, 0))
	assert_vector(level.demo_tap()).is_equal(Vector3.ZERO)


## The extents knob reshapes floor and ceiling live: sizes and centers
## follow, the floor's top staying exactly at y = 0 and the ceiling's
## underside at wall height.
func test_extents_knob_resizes_slabs() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3.ZERO, 0.0))
	level.inject(ShaderMaterial.new(), Pulses.new())
	add_child(level)
	level.extents = Vector2(8, 6)
	var slabs := _slabs(level)
	assert_int(slabs.size()).is_equal(2)
	for slab: StaticBody3D in slabs:
		assert_vector(_box(slab).size).is_equal(Vector3(8, 0.1, 6))
		assert_vector(_box_shape(slab).size).is_equal(Vector3(8, 0.1, 6))
	assert_vector(slabs[0].position).is_equal(Vector3(4, -0.05, 3))
	assert_vector(slabs[1].position).is_equal(Vector3(4, 3.05, 3))


## The shipped level, instanced the way main does: injected first, then
## entered — every contract below is read back from the scene itself.
func _shipped_level() -> WaveLevel:
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), Pulses.new())
	add_child(level)
	return level


## True when the edge (x1, z1, x2, z2) lies on some wall centerline: same
## axis, same coordinate, spans overlapping.
func _edge_on_some_wall(segs: PackedVector4Array, edge: Vector4) -> bool:
	var vertical := absf(edge.z - edge.x) < 0.001
	for s: Vector4 in segs:
		if vertical:
			if absf(s.x - edge.x) < 0.001 and absf(s.z - edge.x) < 0.001:
				if minf(s.y, s.w) <= edge.w and maxf(s.y, s.w) >= edge.y:
					return true
		elif absf(s.y - edge.y) < 0.001 and absf(s.w - edge.y) < 0.001:
			if minf(s.x, s.z) <= edge.z and maxf(s.x, s.z) >= edge.x:
				return true
	return false


func test_shipped_walls_axis_aligned_and_bordered() -> void:
	var segs := _shipped_level().wall_segments()
	assert_int(segs.size()).is_greater_equal(4)
	for s: Vector4 in segs:
		var axis_aligned := absf(s.w - s.y) < 0.001 or absf(s.z - s.x) < 0.001
		assert_bool(axis_aligned).append_failure_message("segment %s" % s).is_true()


## The hum-room rect clips what the fan's waves may reveal. Each of its
## four edges must be collinear with (and overlapped by) a real wall
## centerline — an edge with no wall on it would clip sound in open air.
func test_shipped_hum_room_edges_lie_on_walls() -> void:
	var level := _shipped_level()
	var room := level.hum_room()
	var segs := level.wall_segments()
	var edges: Array[Vector4] = [
		Vector4(room.x, room.y, room.x, room.w),  # west
		Vector4(room.z, room.y, room.z, room.w),  # east
		Vector4(room.x, room.y, room.z, room.y),  # north
		Vector4(room.x, room.w, room.z, room.w),  # south
	]
	for edge: Vector4 in edges:
		(
			assert_bool(_edge_on_some_wall(segs, edge))
			. append_failure_message("edge %s" % edge)
			. is_true()
		)


func test_shipped_fan_inside_hum_room() -> void:
	var level := _shipped_level()
	var room := level.hum_room()
	var fan: SoundFan = level.fan()
	assert_object(fan).is_not_null()
	var at := fan.global_position
	assert_bool(at.x > room.x and at.x < room.z).is_true()
	assert_bool(at.z > room.y and at.z < room.w).is_true()


## The demo tap must land on the hum room's west wall, inside a real
## segment's span, striking toward the spawn side (-X).
func test_shipped_demo_tap_sits_on_west_hum_wall() -> void:
	var level := _shipped_level()
	var tap := level.demo_tap()
	assert_float(tap.x).is_equal_approx(level.hum_room().x, 0.001)
	var on_wall := false
	for s: Vector4 in level.wall_segments():
		if absf(s.x - tap.x) < 0.001 and absf(s.z - tap.x) < 0.001:
			if tap.z >= minf(s.y, s.w) and tap.z <= maxf(s.y, s.w):
				on_wall = true
	assert_bool(on_wall).is_true()
	assert_vector(level.demo_tap_normal()).is_equal(Vector3(-1, 0, 0))


func test_shipped_spawn_inside_bounds() -> void:
	var level := _shipped_level()
	var lo := Vector2(INF, INF)
	var hi := Vector2(-INF, -INF)
	for s: Vector4 in level.wall_segments():
		lo = Vector2(minf(lo.x, minf(s.x, s.z)), minf(lo.y, minf(s.y, s.w)))
		hi = Vector2(maxf(hi.x, maxf(s.x, s.z)), maxf(hi.y, maxf(s.y, s.w)))
	var spawn := level.spawn_pos()
	assert_bool(spawn.x > lo.x and spawn.x < hi.x).is_true()
	assert_bool(spawn.z > lo.y and spawn.z < hi.y).is_true()


## The scene IS the validated design: the numbers the retired LevelData
## carried by hand now derive from the authored nodes, unchanged.
func test_shipped_level_matches_validated_design() -> void:
	var level := _shipped_level()
	assert_int(level.wall_segments().size()).is_equal(10)
	var room := level.hum_room()
	assert_vector(room).is_equal_approx(Vector4(6.4, 0.6, 19.4, 8.0), Vector4.ONE * 0.001)
	assert_vector(level.spawn_pos()).is_equal_approx(Vector3(3, 0.9, 4), Vector3.ONE * 0.001)
	assert_float(level.spawn_yaw()).is_equal_approx(-1.9, 0.0001)
	assert_vector(level.demo_tap()).is_equal_approx(Vector3(6.4, 0.8, 4.0), Vector3.ONE * 0.001)
	assert_vector(level.demo_tap_normal()).is_equal(Vector3(-1, 0, 0))
	var fan: SoundFan = level.fan()
	assert_vector(fan.position).is_equal_approx(Vector3(8.6, 0, 4.4), Vector3.ONE * 0.001)


## The shipped level carries the companion cat, exposes it for the root to
## tick, and has injected it the same way it injects the fan — so it can
## both sound (pulse pool) and be seen (data-pass material).
func test_shipped_level_exposes_and_injects_the_cat() -> void:
	var level := _shipped_level()
	var cats := level.cats()
	assert_int(cats.size()).is_equal(1)
	var cat: WaveCat = cats[0]
	assert_object(cat.pulses).is_not_null()
	assert_object(cat.data_mat).is_not_null()
	# it wakes in the west room a few steps south of the hero, on the floor
	assert_vector(cat.position).is_equal_approx(Vector3(2.8, 0, 7.6), Vector3.ONE * 0.001)


## Every authored box in the level that carries a flat object id, paired
## with the world box it fills. The fan and the cat are deliberately absent:
## each is a MULTI-box object whose limbs SHARE one id on purpose, so that
## it reads as a single silhouette — a pairwise "must differ" law is exactly
## wrong for them.
func _painted_boxes(node: Node, out: Array[Dictionary]) -> void:
	for child: Node in node.get_children():
		var skin := _skin(child)
		if skin != null:
			var oid := -1.0
			if child is WaveWall:
				oid = (child as WaveWall).oid()
			elif child is WaveProp:
				oid = (child as WaveProp).oid()
			if oid >= 0.0:
				(
					out
					. append(
						{
							"name": str(child.name),
							"box": skin.global_transform * skin.get_aabb(),
							"oid": oid,
						}
					)
				)
		_painted_boxes(child, out)


## Where two boxes interpenetrate there is no depth step, so the silhouette
## Laplacian on B has nothing to bite on — only the G-channel crease can
## draw their seam, and the shader fades it over smoothstep(0.04, 0.08).
## Two touching boxes closer than 0.08 in id therefore draw a weak seam, and
## IDENTICAL ids draw none at all: the pair melts into one silhouette. The
## shipped level must clear the knee on every touching pair.
func test_shipped_touching_boxes_draw_their_seam() -> void:
	var boxes: Array[Dictionary] = []
	_painted_boxes(_shipped_level(), boxes)
	assert_int(boxes.size()).is_equal(21)
	var melted: Array[String] = []
	for i: int in boxes.size():
		for j: int in range(i + 1, boxes.size()):
			var near: Dictionary = boxes[i]
			var far: Dictionary = boxes[j]
			var near_box: AABB = near["box"]
			var far_box: AABB = far["box"]
			if not near_box.grow(TOUCH_EPS).intersects(far_box):
				continue
			var near_oid: float = near["oid"]
			var far_oid: float = far["oid"]
			if absf(near_oid - far_oid) < MIN_OID_SEP:
				melted.append(
					"%s(%.2f) touches %s(%.2f)" % [near["name"], near_oid, far["name"], far_oid]
				)
	(
		assert_array(melted)
		. append_failure_message("touching boxes with no seam between them: %s" % str(melted))
		. is_empty()
	)


## Ids are reused wherever no pixel shows two boxes meeting — that reuse is
## what lets a five-entry palette dress a level of any size. A run that gave
## every box its own id would pass the seam law above while proving nothing.
func test_shipped_level_reuses_ids_between_distant_boxes() -> void:
	var boxes: Array[Dictionary] = []
	_painted_boxes(_shipped_level(), boxes)
	var distinct := {}
	for box: Dictionary in boxes:
		distinct[box["oid"]] = true
	assert_int(distinct.size()).is_less(boxes.size())
