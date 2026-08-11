# gdlint:ignore = max-public-methods
extends GdUnitTestSuite
## The level-authoring engine nodes, held to their designer-safety laws:
## a wall builds its outline box and collider from nothing but transform
## and length knob; free-hand rotation snaps to EXACT quarter turns (walls
## are axis-aligned by law, and the snapped basis carries no trig dust);
## props stay free; and the level root is the single injection point that
## deals the world and image skins out, then derives the spawn from its
## marker and the demo tap from the wall between the spawn and the sound
## source NEAREST it.
##
## (The directive above must sit on line 1 — gdlint keys an ignore to the
## line its problem is reported on. A gdUnit4 suite is a list of cases, not
## a class with an API: every case is a public method, so the 20-method
## ceiling counts coverage rather than surface. Suppressed in the two
## suites that outgrew it and nowhere else, so the rule keeps its teeth
## over `game/scripts/`.)
##
## The SHIPPED scene's own invariants live next door, in map_test.gd — with
## one deliberate exception below: a placement law that can only be proved
## harmless by running it against a real, fully furnished map, and so is
## held here beside the law itself rather than a file away from it.

const SHIPPED_LEVEL := preload("res://scenes/level_01.tscn")


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


## How many limbs a node has built for itself — one mesh and one collider
## is a whole shape; anything more is a ghost of an earlier build.
func _limbs(body: Node) -> int:
	var n := 0
	for child: Node in body.get_children():
		if child is MeshInstance3D or child is CollisionShape3D:
			n += 1
	return n


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


## CTRL+D IS THE AUTHORING TOOL — game/README.md tells a designer to build
## a level by duplicating walls — and Ctrl+D is `Node.duplicate()`, which
## copies the mesh and collider the ORIGINAL built for itself. Building a
## second pair on top leaves a ghost the size knob cannot reach, drawn at
## the old size forever; worse, `duplicate()` SHARES resources, so the ghost
## carries the original's own BoxMesh and would resize with it. A builder
## must therefore be idempotent: whatever limbs it finds, it owns exactly
## one pair when it is done.
func test_a_duplicated_wall_does_not_double_its_geometry() -> void:
	var wall: WaveWall = auto_free(_wall(4.0, Vector3.ZERO, false))
	add_child(wall)
	var copy: WaveWall = auto_free(wall.duplicate() as WaveWall)
	add_child(copy)
	(
		assert_int(_limbs(copy))
		. append_failure_message("the duplicate readied onto the limbs it was copied with")
		. is_equal(2)
	)
	copy.length = 9.0
	(
		assert_vector(_box(copy).size)
		. append_failure_message("a ghost mesh is drawn ahead of the one the knob reshapes")
		. is_equal(Vector3(9.3, 3, 0.3))
	)
	assert_vector(_box(wall).size).is_equal(Vector3(4.3, 3, 0.3))


## The length knob reshapes a placed wall live — mesh and collider
## together, the way a designer drags a number in the Inspector.
func test_wall_length_knob_reshapes_live() -> void:
	var wall: WaveWall = auto_free(_wall(4.0, Vector3.ZERO, false))
	add_child(wall)
	wall.length = 9.0
	assert_vector(_box(wall).size).is_equal(Vector3(9.3, 3, 0.3))
	assert_vector(_box_shape(wall).size).is_equal(Vector3(9.3, 3, 0.3))


## A wall's one knob answers a minus sign the way every prop knob does.
## Left raw it splits the wall in three: BoxMesh draws 3.7 m, BoxShape3D
## refuses the negative extent and keeps its default 1 m cube, and
## wall_segment sweeps the half-length BACKWARDS, handing the level a
## centerline whose ends are in the wrong order. The sign folds at the knob
## instead, so the drawn box, the collider and the occluder are one wall.
func test_a_negative_wall_length_folds_instead_of_inverting_the_wall() -> void:
	var wall := _wall(-4.0, Vector3(2, 0, 0), false)
	var level := _level_holding(wall)
	assert_float(wall.length).is_equal_approx(4.0, 0.001)
	assert_vector(_box(wall).size).is_equal(Vector3(4.3, 3, 0.3))
	(
		assert_vector(_box_shape(wall).size)
		. append_failure_message("the collider kept its own size while the mesh was reshaped")
		. is_equal(Vector3(4.3, 3, 0.3))
	)
	(
		assert_vector(level.wall_segments()[0])
		. append_failure_message("centerline ends out of order: %s" % level.wall_segments()[0])
		. is_equal_approx(Vector4(0, 0, 4, 0), Vector4.ONE * 0.001)
	)


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
## to plan — which the level says out loud; the message itself is held next
## door, in test_unplannable_demo_tap_reports.
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


## A level without a SpawnPoint marker has nowhere to wake the hero, and
## the fallback is worse than "somewhere else": the level's own origin is
## the corner outside the border walls, so the hero wakes sealed into the
## sliver there. The message says where it was put and that it is very
## likely unreachable — "nowhere to wake" alone never did.
func test_missing_spawn_reports() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		(
			"WaveLevel: no Marker3D named exactly 'SpawnPoint' under the level — the hero has "
			+ "nowhere to wake, so it wakes at the level's own origin, (0, 0.9, 0). That is the "
			+ "corner of the map, outside the border walls: the hero is very likely sealed into "
			+ "the sliver there and cannot reach the level at all. Add a Marker3D named "
			+ "'SpawnPoint', standing on the floor, facing where the hero should look."
		)
	))
	assert_vector(level.spawn_pos()).is_equal(Vector3(0, 0.9, 0))


## THE Ctrl+D case, the one issue #19 is named for: duplicating the marker
## in the editor leaves 'SpawnPoint2', which the exact-name test never
## matched — so the copy was not even collected, a designer who dragged it
## across the map moved nothing, and the hero woke at the original without
## a word. The winner is unchanged; the silence is not.
func test_auto_numbered_spawn_copy_is_reported_and_never_promoted() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	var copy := _spawn_marker(Vector3(9, 0, 9), 1.0)
	copy.name = "SpawnPoint2"
	level.add_child(copy)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		(
			"WaveLevel: auto-numbered spawn copies IGNORED: 'SpawnPoint2'. Only a Marker3D "
			+ "named exactly 'SpawnPoint' wakes the hero, and Ctrl+D renames the copy — so "
			+ "moving the copy moves nothing. Rename the one you want to 'SpawnPoint' and "
			+ "delete the rest."
		)
	))
	assert_vector(level.spawn_pos()).is_equal(Vector3(1, 0.9, 3))


## Two markers named EXACTLY 'SpawnPoint' is legal in Godot under two
## different parents, and used to be settled in silence by whichever the
## walk reached first. The first still wins — nothing that is valid today
## moves — and the loser is named by its PATH, the only thing that
## separates two nodes carrying one name.
func test_two_exact_spawn_markers_name_the_one_that_lost() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	var room := Node3D.new()
	room.name = "Rooms"
	room.add_child(_spawn_marker(Vector3(9, 0, 9), 1.0))
	level.add_child(room)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		(
			"WaveLevel: 2 markers are named exactly 'SpawnPoint' — the hero wakes at the first "
			+ "the level walk reaches, 'SpawnPoint', and ignores 'Rooms/SpawnPoint'. Delete or "
			+ "rename every spawn marker but one."
		)
	))
	assert_vector(level.spawn_pos()).is_equal(Vector3(1, 0.9, 3))


## An open-sided fan room is legal now: the fan's waves are stopped by the
## walls that ARE there (source→surface sight), not clipped to an enclosing
## rectangle — so derivation no longer refuses. The level enters the tree
## and reads back its walls, fan and spawn. (The one wall does not stand
## between the two, so the tap goes unplanned and the level says so.)
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
## demo tap, and no error. Nothing was authored wrong here, so nothing is
## said — a level that complained about its own quiet would teach a
## designer to stop reading the log.
##
## The one wall stands clear of the floor's edge on purpose. A wall laid ON
## the boundary hangs its padded box a half-thickness over the void, which
## IS a fault and is now reported as one — so a fixture that meant "a level
## with nothing wrong with it" has to be a level with nothing wrong with it.
func test_fanless_level_has_no_demo_tap() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_wall(4.0, Vector3(3, 0, 1), false))
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await assert_error(enter).is_success()
	assert_vector(level.demo_tap()).is_equal(Vector3.ZERO)


## Scene order is not a contract. The demo tap aims at the source whose hub
## is NEAREST the spawn, so dragging a row in a 129-sibling Scene dock no
## longer re-aims the opening strike in silence. Here the far source is
## listed first and the near one second: the tap lands on the wall between
## the spawn and the NEAR one, which is not the wall first-in-scene-order
## would have struck.
func test_demo_tap_aims_at_the_nearest_source_not_the_first() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_wall(4.0, Vector3(5, 0, 7), false))  # x-run, spans x 3..7
	level.add_child(_wall(4.0, Vector3(9, 0, 5), true))  # z-run, spans z 3..7
	var far := SoundFan.new()
	far.name = "FarFan"
	far.position = Vector3(13, 0, 5)
	level.add_child(far)
	var near := SoundFan.new()
	near.name = "NearFan"
	near.position = Vector3(5, 0, 9)
	level.add_child(near)
	level.add_child(_spawn_marker(Vector3(5, 0, 5), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	assert_vector(level.demo_tap()).is_equal_approx(Vector3(5, 0.8, 6.85), Vector3.ONE * 0.001)
	assert_vector(level.demo_tap_normal()).is_equal(Vector3(0, 0, -1))


## An unplannable tap used to be a silent wrong result: the `if let` simply
## did not fire, tap_point kept its zeroed default, and main.gd fired the
## opening strike at the world origin — under the floor in the corner of
## the map. One drag of a source into the spawn's own room reaches that, so
## the level now names the source it could not reach past and says what
## happens instead.
func test_unplannable_demo_tap_reports() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_wall(4.0, Vector3(2, 0, 0), false))
	var fan := SoundFan.new()
	fan.name = "Fan"
	fan.position = Vector3(2, 0, 2.5)
	level.add_child(fan)
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		(
			"WaveLevel: no wall stands between the spawn at (1, 0.9, 3) and 'Fan', the sound "
			+ "source nearest it, at (2, 1.15, 2.4) — the dev demo tap cannot be planned and "
			+ "stays at the world origin, where an input-less run (UNSEEING_DEMO=1, or ?demo "
			+ "in the URL) strikes instead of on a wall."
		)
	))
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


## The running half of the slab-drawing law (level_plan::slab_drawn). The
## editor hides the ceiling — a lid over the whole extents is one opaque
## quad across the top-down view a map is laid out in — and the risk in
## that fix is that it leaks into the game, or that the lid is dropped
## outright instead of hidden. Either one opens the hero's closed room to
## the sky, silently: nothing else in this suite looks at whether a slab
## draws. So both slabs are still BUILT, in floor-then-ceiling order, and
## both are still drawn.
##
## Asked of the skin's visibility IN THE TREE, not of one node's flag, so
## it holds however the lid is hidden — body, skin, or a parent above them.
func test_both_slabs_draw_in_the_running_game() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3.ZERO, 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var slabs := _slabs(level)
	assert_int(slabs.size()).is_equal(2)
	(
		assert_bool(_skin(slabs[0]).is_visible_in_tree())
		. append_failure_message("the floor does not draw in the running game")
		. is_true()
	)
	(
		assert_bool(_skin(slabs[1]).is_visible_in_tree())
		. append_failure_message("the ceiling does not draw in the running game")
		. is_true()
	)


## The same law on the level root, whose limbs are whole slab BODIES. A
## `_ready` runs again whenever a node re-enters the tree after
## `request_ready()` — which is what the editor does to a reloaded scene —
## and an unconditional build would leave a second floor and a second
## ceiling inside the first, invisible and uncollidable.
func test_a_second_ready_does_not_double_the_slabs() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3.ZERO, 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	assert_int(_slabs(level).size()).is_equal(2)
	remove_child(level)
	level.request_ready()
	add_child(level)
	(
		assert_int(_slabs(level).size())
		. append_failure_message("the second _ready built a second floor and ceiling")
		. is_equal(2)
	)
	# and the extents knob still drives the slabs that are actually there
	level.extents = Vector2(8, 6)
	for slab: StaticBody3D in _slabs(level):
		assert_vector(_box(slab).size).is_equal(Vector3(8, 0.1, 6))


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


## A crate a designer dragged to a negative coordinate. The level's slabs
## span 0..extents from its own origin and NEVER grow to meet stray
## geometry, so there is no floor under it: the box still draws, the waves
## still outline it, and the hero who walks there falls with gravity and
## nothing underfoot. Before this the level said not one word about it.
func test_a_solid_outside_the_extents_reports() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.extents = Vector2(20, 20)
	var crate := WaveProp.new()
	crate.name = "StrayCrate"
	crate.size = Vector3.ONE
	crate.position = Vector3(-10, 0.5, -10)
	level.add_child(crate)
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		(
			"WaveLevel: 'StrayCrate' stands off the floor entirely — its footprint is x "
			+ "-10.50..-9.50, z -10.50..-9.50, and the floor covers x 0.00..20.00, z "
			+ "0.00..20.00. There is no slab under any of it: it draws where nothing holds it "
			+ "up, and the hero who walks there falls out of the world. Move it inside the "
			+ "extents, or grow the level's extents to cover it — the slabs span 0..extents "
			+ "from the level's own origin and never move to meet stray geometry."
		)
	))
	assert_int(level.unfloored_solids()).is_equal(1)


## The milder half of the same fault, and the one an authored map actually
## reaches: a crate pushed past the last centimetre of the extents. Most of
## it is supported, so it is not told the hero falls through it — but the
## overhang has no slab, and the count sees it.
func test_a_solid_hanging_over_the_floor_edge_reports() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.extents = Vector2(20, 20)
	var crate := WaveProp.new()
	crate.name = "LedgeCrate"
	crate.size = Vector3.ONE
	crate.position = Vector3(20, 0.5, 4.5)
	level.add_child(crate)
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		(
			"WaveLevel: 'LedgeCrate' hangs over the edge of the floor — its footprint is x "
			+ "19.50..20.50, z 4.00..5.00, and the floor covers x 0.00..20.00, z 0.00..20.00. "
			+ "The part outside has no slab under it. Move it inside the extents, or grow the "
			+ "level's extents to cover it — the slabs span 0..extents from the level's own "
			+ "origin and never move to meet stray geometry."
		)
	))
	assert_int(level.unfloored_solids()).is_equal(1)


## The other half of a diagnostic, and the half that decides whether anyone
## keeps reading it: the SHIPPED map says nothing. 125 authored solids, 19
## of them walls whose padded boxes reach 0.45 m of the extents' edge, and
## not one of them trips the law. A footprint test that fired here would be
## noise from the first run.
func test_the_shipped_map_stands_on_its_own_floor() -> void:
	var level: WaveLevel = auto_free(SHIPPED_LEVEL.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	assert_int(level.unfloored_solids()).is_equal(0)
