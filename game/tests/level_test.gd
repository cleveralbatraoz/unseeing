extends GdUnitTestSuite
## The level-authoring engine nodes, held to their designer-safety laws:
## a wall builds its outline box and collider from nothing but transform
## and length knob; free-hand rotation snaps to EXACT quarter turns (walls
## are axis-aligned by law, and the snapped basis carries no trig dust);
## props stay free; and the level root is the single injection point that
## deals the world and image skins out, then derives the spawn from its
## marker and the demo tap from the wall between the spawn and the first
## sound source.
##
## The SHIPPED scene's own invariants live next door, in map_test.gd.


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


## The world box a solid actually draws — what the eye is shown, against
## which every derived contract is held.
func _world_box(body: Node) -> AABB:
	var skin := _skin(body)
	return skin.global_transform * skin.get_aabb()


## The centerline the DRAWN box implies: its long horizontal axis pulled in
## by a wall half-thickness at each end, laid down the middle of its short
## one. A wall's occluder and its box are the same object seen twice, so
## this is what wall_segments() has to say.
func _drawn_centerline(box: AABB) -> Vector4:
	const HALF_T := 0.15
	var lo := box.position
	var hi := box.position + box.size
	if box.size.x >= box.size.z:
		var z := (lo.z + hi.z) * 0.5
		return Vector4(lo.x + HALF_T, z, hi.x - HALF_T, z)
	var x := (lo.x + hi.x) * 0.5
	return Vector4(x, lo.z + HALF_T, x, hi.z - HALF_T)


## A room prefab, instanced the way the editor instances one: a plain
## Node3D carrying the whole room's placement, with the wall authored at
## the room's own origin, knowing nothing about where the room went.
func _room(yaw: float, scale: float, wall: WaveWall) -> Node3D:
	var room := Node3D.new()
	room.position = Vector3(10, 0, 4)
	room.rotation.y = yaw
	room.scale = Vector3.ONE * scale
	room.add_child(wall)
	return room


## A level holding one authored subtree, injected and entered the way main
## does it — so wall_segments() is the real derived contract, not a plan.
func _level_holding(node: Node3D) -> WaveLevel:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(node)
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	return level


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


## THE PREFAB LAW: a wall must not care what it hangs under. A room dropped
## into a level at a quarter turn carries its walls around with it, and the
## centerline the sight shaders occlude by is derived in WORLD space — so it
## has to be SNAPPED in world space too. Snap the local basis instead and a
## wall draws down one axis while occluding down the other, with nothing in
## the game able to notice: sound passes through walls the eye is shown, and
## stops at air it is not.
func test_wall_in_a_rotated_room_occludes_where_it_draws() -> void:
	var wall := _wall(4.0, Vector3(2, 0, 0), false)
	var level := _level_holding(_room(PI * 0.5, 1.0, wall))
	var box := _world_box(wall)
	assert_int(level.wall_segments().size()).is_equal(1)
	(
		assert_vector(level.wall_segments()[0])
		. append_failure_message("segment %s vs drawn box %s" % [level.wall_segments()[0], box])
		. is_equal_approx(_drawn_centerline(box), Vector4.ONE * 0.001)
	)
	# the room's quarter turn turned the wall's length axis onto world Z:
	# a 4 m run at x = 10, from z = 0 to z = 4
	assert_vector(level.wall_segments()[0]).is_equal_approx(
		Vector4(10, 0, 10, 4), Vector4.ONE * 0.001
	)


## Inherited SCALE is the same law seen from the other side: a wall in a 2x
## room would draw 8.6 m of box and occlude 4.26 m of centerline. Writing
## the snapped basis in world space annihilates it with the same stroke,
## because a quadrant basis has unit columns — length stays the one size
## knob however deep the prefab is nested, and the wall still runs floor to
## ceiling.
func test_a_scaled_room_cannot_stretch_a_wall_past_its_occluder() -> void:
	var wall := _wall(4.0, Vector3(2, 0, 0), false)
	var level := _level_holding(_room(PI * 0.5, 2.0, wall))
	var box := _world_box(wall)
	(
		assert_vector(level.wall_segments()[0])
		. append_failure_message("segment %s vs drawn box %s" % [level.wall_segments()[0], box])
		. is_equal_approx(_drawn_centerline(box), Vector4.ONE * 0.001)
	)
	assert_vector(box.size).is_equal_approx(Vector3(0.3, 3, 4.3), Vector3.ONE * 0.001)
	assert_float(box.position.y).is_equal_approx(0.0, 0.001)


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


## The level root end to end: one inject call deals the materials out — the
## world skin to every solid, the cat and the slabs, the acoustic image plus
## the pool to every sound source; entering the tree derives the spawn from
## its marker (lifted to capsule height). Here the fan and the spawn share
## one room, so the spawn→fan line crosses no wall and there is no demo tap
## to plan.
func test_level_distributes_and_derives() -> void:
	var mat := ShaderMaterial.new()
	var source_mat := ShaderMaterial.new()
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
	level.inject(mat, source_mat, pulses)
	add_child(level)
	assert_vector(level.spawn_pos()).is_equal(Vector3(1, 0.9, 3))
	assert_float(level.spawn_yaw()).is_equal_approx(-0.6, 0.0001)
	assert_vector(level.demo_tap()).is_equal(Vector3.ZERO)  # no wall between spawn and fan
	assert_vector(level.demo_tap_normal()).is_equal(Vector3.UP)  # default, no tap planned
	assert_int(level.wall_segments().size()).is_equal(4)
	assert_int(level.sources().size()).is_equal(1)
	assert_object(level.sources()[0]).is_same(fan)
	assert_object(fan.pulses).is_same(pulses)
	assert_object(fan.data_mat).is_same(source_mat)
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
		"WaveLevel: materials/pulses not injected — the level cannot be seen"
	))


## A level without a SpawnPoint marker has nowhere to wake the hero —
## loud, with the level origin as the fallback.
func test_missing_spawn_reports() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		"WaveLevel: no SpawnPoint marker — the hero has nowhere to wake"
	))
	assert_vector(level.spawn_pos()).is_equal(Vector3(0, 0.9, 0))


## An open-sided fan room is legal now: the fan's waves are stopped by the
## walls that ARE there (source→surface sight), not clipped to an enclosing
## rectangle — so derivation no longer refuses. The level enters the tree
## and reads back its walls, fan and spawn.
func test_open_fan_room_is_legal() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_wall(4.0, Vector3(2, 0, 0), false))
	var fan := SoundFan.new()
	fan.position = Vector3(2, 0, 2)
	level.add_child(fan)
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	assert_int(level.wall_segments().size()).is_equal(1)
	assert_object(level.sources()[0]).is_same(fan)
	assert_vector(level.spawn_pos()).is_equal(Vector3(1, 0.9, 3))


## A fanless level is legal silence: no source to strike toward, so no
## demo tap, and no error.
func test_fanless_level_has_no_demo_tap() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_wall(4.0, Vector3(2, 0, 0), false))
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	assert_vector(level.demo_tap()).is_equal(Vector3.ZERO)


## The extents knob reshapes floor and ceiling live: sizes and centers
## follow, the floor's top staying exactly at y = 0 and the ceiling's
## underside at wall height.
func test_extents_knob_resizes_slabs() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3.ZERO, 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	level.extents = Vector2(8, 6)
	var slabs := _slabs(level)
	assert_int(slabs.size()).is_equal(2)
	for slab: StaticBody3D in slabs:
		assert_vector(_box(slab).size).is_equal(Vector3(8, 0.1, 6))
		assert_vector(_box_shape(slab).size).is_equal(Vector3(8, 0.1, 6))
	assert_vector(slabs[0].position).is_equal(Vector3(4, -0.05, 3))
	assert_vector(slabs[1].position).is_equal(Vector3(4, 3.05, 3))


## Injection is ordered, and the order is the contract: by the time the
## level is in the tree it has already pushed an empty wall table and
## coloured every id without the sources' anchors. A late inject cannot
## repair either, so it is refused loudly rather than half-honoured — the
## alternative is a world that renders with seams that silently do not draw.
func test_late_injection_reports_rather_than_limping() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3.ZERO, 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var late := func() -> void:
		level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	await (assert_error(late).is_push_error(
		(
			"WaveLevel: inject() after the level entered the tree — the wall table and the "
			+ "object-id colouring were already derived without it. Inject BEFORE add_child()."
		)
	))
