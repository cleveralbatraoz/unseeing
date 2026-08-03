extends GdUnitTestSuite
## The level-authoring engine nodes, held to their designer-safety laws:
## a wall builds its outline box and collider from nothing but transform
## and length knob; free-hand rotation snaps to EXACT quarter turns (walls
## are axis-aligned by law, and the snapped basis carries no trig dust);
## props stay free; and the level root is the single injection point that
## distributes material and pool, then derives the hum room from the walls
## around the fan, the spawn from its marker, and the demo tap from the
## room's west wall.


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
