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
## The SHIPPED scene's own invariants live next door, in map_test.gd. The
## shipped-scene cases here are deliberate exceptions, and they are not
## invariants of the MAP but of this NODE: that a level inside every shader
## ceiling says nothing at all, and that a placement law stays silent on a
## real, fully furnished map — neither can be proved anywhere else, so each
## is held beside the law it guards rather than a file away from it.

const LEVEL_SCENE := preload("res://scenes/level_01.tscn")


## One authored wall, the way the generator and a designer both place it:
## floor position at the centerline midpoint, yaw for the axis.
func _wall(length: float, at: Vector3, vertical: bool) -> WaveWall:
	var wall := WaveWall.new()
	wall.length = length
	wall.position = at
	if vertical:
		wall.rotation.y = PI * 0.5
	return wall


func _spawn_marker(at: Vector3, yaw: float) -> WaveSpawn:
	var marker := WaveSpawn.new()
	marker.position = at
	marker.rotation.y = yaw
	return marker


## The first mesh limb a node built for itself.
func _skin(body: Node) -> MeshInstance3D:
	for child: Node in body.get_children():
		if child is MeshInstance3D:
			return child as MeshInstance3D
	return null


## The size of the box a builder actually drew — read off the mesh's own
## untransformed AABB, which works for any `Mesh` subclass. `BoxMesh.size`
## no longer exists to read: every static solid builds an `ArrayMesh` now
## (Task 5, `render::paint::labelled_box`), and its own AABB carries the
## identical numbers a `BoxMesh` of the same size always reported.
func _box(body: Node) -> Vector3:
	var skin := _skin(body)
	return Vector3.ZERO if skin == null else skin.mesh.get_aabb().size


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
## by a half-thickness at each end, laid down the middle of its short one.
## A wall's occluder and its box are the same object seen twice, so this is
## what wall_segments() has to say.
func _drawn_centerline(box: AABB) -> Vector4:
	const END_PAD := 0.15
	var lo := box.position
	var hi := box.position + box.size
	if box.size.x >= box.size.z:
		var z := (lo.z + hi.z) * 0.5
		return Vector4(lo.x + END_PAD, z, hi.x - END_PAD, z)
	var x := (lo.x + hi.x) * 0.5
	return Vector4(x, lo.z + END_PAD, x, hi.z - END_PAD)


## A room prefab, instanced the way the editor instances one: a plain
## Node3D carrying the whole room's placement, with the wall authored at
## the room's own origin, knowing nothing about where the room went.
func _room(yaw: float, scale: float, wall: WaveWall) -> Node3D:
	var room := Node3D.new()
	room.position = Vector3(10, 0, 7)
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


## A wall is its centerline padded by a half-thickness on every side —
## flanks AND run ends alike — floor to ceiling, mesh and collider the
## same box, risen from the floor node. A junction cap therefore lands
## exactly flush in a partner's flank plane; that coincidence is now the
## superface merge law's own MERGE candidate (`render::superface`), not a
## z-fight to dodge.
func test_wall_builds_box_and_collider_from_length() -> void:
	var wall: WaveWall = auto_free(_wall(7.4, Vector3(2, 0, 3), false))
	add_child(wall)
	assert_vector(_box(wall)).is_equal(Vector3(7.7, 3, 0.3))
	assert_vector(_box_shape(wall).size).is_equal(Vector3(7.7, 3, 0.3))
	assert_vector(_skin(wall).position).is_equal(Vector3(0, 1.5, 0))


## Task 5's ordinal contract, on the shape every static solid shares first:
## a wall now builds an `ArrayMesh` carrying a CUSTOM0 channel, one entry
## per vertex, holding a placeholder FACE ORDINAL (never a final label —
## Task 6's paint pass rewrites these) in `render::paint::FACE_ORDER`'s own
## −X,+X,−Y,+Y,−Z,+Z order. 24 unshared vertices, four per face, exactly the
## way `render::paint::labelled_box` builds every box-shaped solid now.
func test_wall_mesh_carries_a_bounded_custom0_ordinal_per_vertex() -> void:
	var wall: WaveWall = auto_free(_wall(4.0, Vector3.ZERO, false))
	add_child(wall)
	var mesh := _skin(wall).mesh as ArrayMesh
	assert_object(mesh).is_not_null()
	assert_int(mesh.get_surface_count()).is_equal(1)
	var arrays: Array = mesh.surface_get_arrays(0)
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var custom: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM0]
	assert_int(verts.size()).is_equal(24)
	assert_int(custom.size()).is_equal(verts.size())
	var fmt := mesh.surface_get_format(0)
	var shift := Mesh.ARRAY_FORMAT_CUSTOM0_SHIFT
	assert_int((fmt >> shift) & 7).is_equal(Mesh.ARRAY_CUSTOM_R_FLOAT)
	# every ordinal is bounded by the widest shape's own count (a box's six)
	for label: float in custom:
		assert_bool(label >= 0.0 and label < 6.0).is_true()
	# the four-vertex blocks land in FACE_ORDER's own order: −X,+X,−Y,+Y,
	# −Z,+Z — block k holds ordinal k on every one of its four vertices,
	# which a builder that fed labelled_box a shuffled or repeated ordinal
	# array would get wrong
	for face: int in 6:
		for corner: int in 4:
			assert_float(custom[face * 4 + corner]).is_equal_approx(float(face), 1e-6)


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
	# a 4 m run at x = 10, from z = 3 to z = 7
	assert_vector(level.wall_segments()[0]).is_equal_approx(
		Vector4(10, 3, 10, 7), Vector4.ONE * 0.001
	)


## Inherited SCALE is the same law seen from the other side: a wall in a 2x
## room would draw 8.58 m of box over the 4 m centerline it occludes.
## Writing the snapped basis in world space annihilates it with the same
## stroke,
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
## carries the original's own box mesh and would resize with it. A builder
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
		assert_vector(_box(copy))
		. append_failure_message("a ghost mesh is drawn ahead of the one the knob reshapes")
		. is_equal(Vector3(9.3, 3, 0.3))
	)
	assert_vector(_box(wall)).is_equal(Vector3(4.3, 3, 0.3))


## The length knob reshapes a placed wall live — mesh and collider
## together, the way a designer drags a number in the Inspector.
func test_wall_length_knob_reshapes_live() -> void:
	var wall: WaveWall = auto_free(_wall(4.0, Vector3.ZERO, false))
	add_child(wall)
	wall.length = 9.0
	assert_vector(_box(wall)).is_equal(Vector3(9.3, 3, 0.3))
	assert_vector(_box_shape(wall).size).is_equal(Vector3(9.3, 3, 0.3))


## A wall's one knob answers a minus sign the way every prop knob does.
## Left raw it splits the wall in three: the box mesh draws 3.7 m,
## BoxShape3D refuses the negative extent and keeps its default 1 m cube, and
## wall_segment sweeps the half-length BACKWARDS, handing the level a
## centerline whose ends are in the wrong order. The sign folds at the knob
## instead, so the drawn box, the collider and the occluder are one wall.
func test_a_negative_wall_length_folds_instead_of_inverting_the_wall() -> void:
	var wall := _wall(-4.0, Vector3(3, 0, 1), false)
	var level := _level_holding(wall)
	assert_float(wall.length).is_equal_approx(4.0, 0.001)
	assert_vector(_box(wall)).is_equal(Vector3(4.3, 3, 0.3))
	(
		assert_vector(_box_shape(wall).size)
		. append_failure_message("the collider kept its own size while the mesh was reshaped")
		. is_equal(Vector3(4.3, 3, 0.3))
	)
	(
		assert_vector(level.wall_segments()[0])
		. append_failure_message("centerline ends out of order: %s" % level.wall_segments()[0])
		. is_equal_approx(Vector4(1, 1, 5, 1), Vector4.ONE * 0.001)
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
	assert_vector(_box(prop)).is_equal(Vector3(0.9, 0.05, 0.6))
	assert_vector(_box_shape(prop).size).is_equal(Vector3(0.9, 0.05, 0.6))
	assert_vector(_skin(prop).position).is_equal(Vector3.ZERO)
	assert_float(prop.rotation.y).is_equal_approx(0.3, 0.0001)
	prop.size = Vector3(0.4, 0.05, 0.4)  # the knob reshapes live, like the wall's
	assert_vector(_box(prop)).is_equal(Vector3(0.4, 0.05, 0.4))


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
		_wall(4.0, Vector3(3, 0, 1), false),
		_wall(4.0, Vector3(3, 0, 5), false),
		_wall(4.0, Vector3(1, 0, 3), true),
		_wall(4.0, Vector3(5, 0, 3), true),
	]
	for wall: WaveWall in walls:
		level.add_child(wall)
	var prop := WaveProp.new()
	prop.size = Vector3(0.9, 0.72, 0.6)
	prop.position = Vector3(2.5, 0.36, 3)
	level.add_child(prop)
	var fan := SoundFan.new()
	fan.position = Vector3(4, 0, 2)
	level.add_child(fan)
	level.add_child(_spawn_marker(Vector3(2, 0, 4), -0.6))
	level.inject(mat, source_mat, pulses)
	add_child(level)
	assert_vector(level.spawn_pos()).is_equal(Vector3(2, 0.9, 4))
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
		assert_vector(_box(slab)).is_equal(Vector3(20, 0.1, 20))
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


## Removing the typed datum must produce the fallback diagnostic and keep
## the historical lifted-origin fallback.
func test_missing_spawn_reports() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		(
			"WaveLevel: no WaveSpawn stands under the level — the hero has nowhere to wake, "
			+ "so it wakes at the level's own origin, (0, 0.9, 0). Add one WaveSpawn on the "
			+ "floor, facing where the hero should look."
		)
	))
	assert_vector(level.spawn_pos()).is_equal(Vector3(0, 0.9, 0))


## Reintroducing the old name predicate must fail this case: only the typed
## datum participates, while an ordinary marker remains free for any other
## editor annotation.
func test_plain_marker_named_spawnpoint_is_not_a_spawn() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var plain := Marker3D.new()
	plain.name = "SpawnPoint"
	level.add_child(plain)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		(
			"WaveLevel: no WaveSpawn stands under the level — the hero has nowhere to wake, "
			+ "so it wakes at the level's own origin, (0, 0.9, 0). Add one WaveSpawn on the "
			+ "floor, facing where the hero should look."
		)
	))


## A duplicate is a real typed candidate regardless of its name. The first
## walk-order node wins and the loser itself receives the warning.
func test_duplicate_typed_spawn_is_reported_on_the_loser() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var first := _spawn_marker(Vector3(1, 0, 3), 0.0)
	first.name = "Start"
	level.add_child(first)
	var copy := _spawn_marker(Vector3(9, 0, 9), 1.0)
	copy.name = "Other"
	level.add_child(copy)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		(
			"WaveLevel: 2 WaveSpawn nodes stand under the level — the hero wakes at the first "
			+ "the level walk reaches, 'Start', and ignores 'Other'. Delete every extra WaveSpawn."
		)
	))
	assert_vector(level.spawn_pos()).is_equal(Vector3(1, 0.9, 3))
	(
		assert_array(copy.get_configuration_warnings())
		. contains(
			[
				(
					"WaveLevel: 2 WaveSpawn nodes stand under the level — the hero wakes at the first "
					+ "the level walk reaches, 'Start', and ignores 'Other'. Delete every extra WaveSpawn."
				)
			]
		)
	)


## Nested paths, not leaf names, distinguish duplicated typed data.
func test_two_typed_spawns_name_the_nested_one_that_lost() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var first := _spawn_marker(Vector3(1, 0, 3), 0.0)
	first.name = "Start"
	level.add_child(first)
	var room := Node3D.new()
	room.name = "Rooms"
	var other := _spawn_marker(Vector3(9, 0, 9), 1.0)
	other.name = "Arrival"
	room.add_child(other)
	level.add_child(room)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		(
			"WaveLevel: 2 WaveSpawn nodes stand under the level — the hero wakes at the first "
			+ "the level walk reaches, 'Start', and ignores 'Rooms/Arrival'. Delete every extra "
			+ "WaveSpawn."
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
	level.add_child(_wall(4.0, Vector3(3, 0, 1), false))
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
	level.add_child(_wall(4.0, Vector3(3, 0, 1), false))
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
		assert_vector(_box(slab)).is_equal(Vector3(8, 0.1, 6))
		assert_vector(_box_shape(slab).size).is_equal(Vector3(8, 0.1, 6))
	assert_vector(slabs[0].position).is_equal(Vector3(4, -0.05, 3))
	assert_vector(slabs[1].position).is_equal(Vector3(4, 3.05, 3))


## A slab is built through the SAME box path a wall and a prop take
## (`nodes::solid::build_box`), so it inherits the CUSTOM0 ordinal channel
## too — held here on the RESIZED slab specifically, since the resize
## rewrites the mesh's surface in place (`render::paint::resize_box_surface`)
## and a stale or doubled CUSTOM0 array from a botched `clear_surfaces` would
## show up only after a knob drag, not on the freshly built mesh.
func test_a_resized_slab_still_carries_a_bounded_custom0_ordinal() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3.ZERO, 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	level.extents = Vector2(8, 6)
	for slab: StaticBody3D in _slabs(level):
		var mesh := _skin(slab).mesh as ArrayMesh
		var arrays: Array = mesh.surface_get_arrays(0)
		var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var custom: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM0]
		assert_int(verts.size()).is_equal(24)
		assert_int(custom.size()).is_equal(verts.size())
		for label: float in custom:
			assert_bool(label >= 0.0 and label < 6.0).is_true()


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
		assert_vector(_box(slab)).is_equal(Vector3(8, 0.1, 6))


## A level of `count` z-run stub walls a metre apart, with a spawn datum so
## the only thing it can ever have to say is about its wall budget. The
## SPACING is load-bearing: a metre keeps the footprint's diagonal near
## `count` metres and well under DIST_PACK_RANGE, so these cases exercise
## the slot ceiling alone and never trip the map-size one as well.
func _stub_walls(count: int) -> WaveLevel:
	var level: WaveLevel = auto_free(WaveLevel.new())
	for i: int in count:
		level.add_child(_wall(2.0, Vector3(float(i), 0, 0), true))
	level.add_child(_spawn_marker(Vector3.ZERO, 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	return level


## The heads-up half of the wall budget, at the engine boundary. The words
## are pinned in cargo (level_plan::wall_budget); what is pinned HERE is
## that they are actually SAID — that push_wall_table still hands the
## verdict to godot_warn!. Delete that one call and every cargo test and
## every other suite stays green while a designer is never told anything.
##
## A heads-up, not an error: 29 walls of 32 still occlude, nothing is
## truncated, and the level is merely one room short of the ceiling.
func test_a_level_nearing_the_wall_slots_warns_with_its_headroom() -> void:
	var level := _stub_walls(29)
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_warning(
		(
			"WaveLevel: 29 walls against the sight shaders' 32 slots — 3 segments left, short of "
			+ "the 4 another room costs (three sides plus the doorway, which is the gap between "
			+ "two segments and so costs a segment of its own). Every wall past the last slot "
			+ "silently stops occluding. Raising MAXW (rust/src/sight.rs, mirrored in "
			+ "game/shaders/pulse_pool.gdshaderinc) is a measured decision and not a free one: "
			+ "every wall is another rect in the per-fragment sight loop, on every platform."
		)
	))
	assert_int(level.wall_segments().size()).is_equal(29)


## The other half, and the other VOLUME. Past the ceiling the drawn world no
## longer matches the authored scene — the table keeps 32 rects and the rest
## stop occluding, so waves walk through walls a designer placed — and that
## is an error, not a heads-up. A single volume for both would have taught
## the reader to scroll past the one that matters.
func test_a_level_past_the_wall_slots_errors_and_counts_the_dropped_walls() -> void:
	var level := _stub_walls(33)
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		(
			"WaveLevel: 33 walls exceed the sight shaders' 32 slots — the table keeps the first "
			+ "32 and drops 1, which stop occluding entirely: waves pass straight through them "
			+ "and no sight line counts them. Delete or merge walls, or raise MAXW "
			+ "(rust/src/sight.rs, mirrored in game/shaders/pulse_pool.gdshaderinc) — a measured "
			+ "decision and not a free one: every wall is another rect in the per-fragment sight "
			+ "loop, on every platform."
		)
	))
	assert_int(level.wall_rects().size()).is_equal(32)  # truncated, as the message says


## THE SILENCE GATE, and the most valuable of the three: the shipped map is
## inside both shader ceilings — 19 walls against 32 slots, a 38.02 m
## wall-centerline diagonal against a 40 m DIST_PACK_RANGE — so entering the
## tree must produce NO engine output whatsoever.
##
## Silence is the hard half of a diagnostic to hold. A budget that shouted
## on a healthy level would be worse than no budget at all: a designer who
## sees a wall of text on every run stops reading it, and the message that
## matters arrives inside noise they have already learned to skip. Nothing
## else in the suite would notice a threshold nudged the wrong way; this is
## the assertion that would.
##
## This is ALSO the wall-merge voice's own silence counterpart — "no engine
## output whatsoever" already covers "no wall-merge warning" for the same
## 125-solid shipped map `test_a_solid_merged_into_a_wall_warns_naming_it`
## proves the warning fires on; a second, narrower silence test asserting
## the identical thing on the identical map would only ever go red in
## lockstep with this one.
func test_the_shipped_level_says_nothing_about_either_shader_ceiling() -> void:
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await assert_error(enter).is_success()
	# non-vacuity: a level that silently dropped its own wall table would
	# pass the silence gate above just as cleanly as a healthy one
	assert_array(level.wall_segments()).is_not_empty()


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
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	assert_int(level.unfloored_solids()).is_equal(0)


## THE authoring gesture: drag a WaveProp onto the floor plane, which is
## where the editor's grid puts it. A box prop is CENTRED on its node — the
## origin law is right, since a shelf or a beam floats as often as it stands
## — so exactly half the crate ends up under the slab, where nothing draws,
## nothing sounds and nothing can be walked into. Until now the only thing
## that noticed was a CI assertion in the shipped map's own suite.
func test_a_prop_dropped_on_the_floor_plane_reports() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.extents = Vector2(20, 20)
	var crate := WaveProp.new()
	crate.name = "DesignerCrate"
	crate.size = Vector3(0.5, 0.5, 0.5)
	crate.position = Vector3(4, 0, 4)
	level.add_child(crate)
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		(
			"WaveLevel: 'DesignerCrate' is sunk through the floor — its box spans y "
			+ "-0.25..0.25, and the floor's top is at y 0.00. What is under the slab never "
			+ "draws, never sounds and cannot be walked into. A WaveProp is CENTRED on its "
			+ "node, so dropping one on the floor plane buries exactly half of it, while a "
			+ "wall, a column and a wedge STAND on theirs. Lift the node until the whole "
			+ "shape clears y 0.00."
		)
	))
	assert_int(level.sunken_solids()).is_equal(1)


## And the half that keeps the law readable: the SHIPPED map says nothing.
## Every one of its 125 solids stands on the floor or above it — the walls
## resting their undersides exactly on y = 0, which is the boundary case a
## sloppy predicate would report as sunk on every wall in the level.
func test_the_shipped_map_keeps_every_solid_above_its_floor() -> void:
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	assert_int(level.sunken_solids()).is_equal(0)


## THE WALL-MERGE VOICE: a crate meant to stand flush against a wall's own
## south face, but built 2 cm deeper than the wall is thick — so instead of
## stopping at the wall's near flank it reaches exactly through to the far
## one and pokes 2 cm out the other side. Its south face lands EXACTLY on
## the wall's own south face (same plane the T-junction cap merge uses:
## `render::superface`'s merge law needs millimetre precision, not a bare
## "2 cm into the wall" — a face merely nudged 2 cm off a wall's plane is
## no longer coplanar with anything and would NOT merge at all), which is
## what actually shares its cluster with the wall and triggers the voice.
##
## Wall: length 4, non-vertical (runs along X, position (0,0,0)) — its
## thickness spans z −0.15..0.15 (WALL_T = 0.15, rust/src/level_plan.rs).
## Crate: size (0.4, 0.4, 0.32), position (0, 0.5, 0.01) — z spans
## −0.15..0.17, its own south face landing exactly at z = −0.15.
func test_a_solid_merged_into_a_wall_warns_naming_it() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_wall(4.0, Vector3.ZERO, false))
	var crate := WaveProp.new()
	crate.name = "WallCrate"
	crate.size = Vector3(0.4, 0.4, 0.32)
	crate.position = Vector3(0.0, 0.5, 0.01)
	level.add_child(crate)
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_warning(
		(
			"WaveLevel: 'WallCrate' overlaps the wall structure and is drawn as part of it — "
			+ "its faces take the walls' labels and its pierce lines draw. Pull it clear of the "
			+ "wall if that was a nudge, or leave it if the bump is authored."
		)
	))


## Editor watching makes derive repeat after every authored change. A
## second paint must use the mesh builder's immutable vertex layout, not
## mistake the first pass's real CUSTOM0 labels (all below 1.0) for face
## ordinal zero. This merge fixture deliberately gives WallCrate several
## face labels; the retired CUSTOM0-as-ordinal implementation collapsed
## all 24 vertices to one label on the replay.
func test_rederive_keeps_a_multilabel_solid_byte_for_byte() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_wall(4.0, Vector3.ZERO, false))
	var crate := WaveProp.new()
	crate.name = "WallCrate"
	crate.size = Vector3(0.4, 0.4, 0.32)
	crate.position = Vector3(0.0, 0.5, 0.01)
	level.add_child(crate)
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var warning := (
		"WaveLevel: 'WallCrate' overlaps the wall structure and is drawn as part of it — "
		+ "its faces take the walls' labels and its pierce lines draw. Pull it clear of the "
		+ "wall if that was a nudge, or leave it if the bump is authored."
	)
	var enter := func() -> void: add_child(level)
	await assert_error(enter).is_push_warning(warning)

	var first: PackedFloat32Array = _skin(crate).mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	var has_more_than_one := false
	for label: float in first:
		if not is_equal_approx(label, first[0]):
			has_more_than_one = true
			break
	assert_bool(has_more_than_one).is_true()

	var replay := func() -> void: level.rederive()
	await assert_error(replay).is_push_warning(warning)
	var second: PackedFloat32Array = _skin(crate).mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	assert_int(second.size()).is_equal(first.size())
	for vertex: int in range(first.size()):
		assert_float(second[vertex]).is_equal(first[vertex])


## THE ORDINAL GUARD: a degenerate box (one extent flattened to zero)
## folds four of its six faces away — `render::faces::face_from_poly`
## refuses each collapsed polygon (its own two first corners coincide
## once the flattened axis zeroes their only distinguishing component),
## leaving only the pair whose corners never depended on that axis. Only
## 2 of the 6 CUSTOM0 ordinals `render::paint::face_count` promises this
## mesh therefore have a real face behind them — painting positionally
## anyway would slide every later ordinal onto the wrong face. The level
## refuses this ONE solid loudly, naming it, and leaves its mesh
## unpainted rather than risk that — CUSTOM0 still holds the six
## placeholder ordinals its builder wrote, not a label and not the
## all-zero fill an entry with no face in the census would otherwise
## bake. A healthy neighbour in the SAME derive still paints correctly,
## proving the refusal is scoped to the one degenerate solid and does not
## poison the derive.
func test_a_degenerate_solid_is_refused_not_mislabelled() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var flat := WaveProp.new()
	flat.name = "FlatCrate"
	flat.size = Vector3(0, 1, 1)
	flat.position = Vector3(0, 0.5, 0)
	level.add_child(flat)
	var healthy := WaveProp.new()
	healthy.name = "HealthyCrate"
	healthy.size = Vector3(1, 1, 1)
	healthy.position = Vector3(3, 0.5, 0)
	level.add_child(healthy)
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await (assert_error(enter).is_push_error(
		(
			"WaveLevel: 'FlatCrate' built 2 planar face(s) from its shape, not the 6 it should "
			+ "— a degenerate size folded one or more away. Its own seams cannot be painted "
			+ "correctly this derive; skipping it rather than mislabeling by position. Give "
			+ "every extent a real size."
		)
	))
	# the healthy neighbour, painted in the SAME derive, still carries
	# real, in-range labels — the refusal above did not poison it
	var custom: PackedFloat32Array = _skin(healthy).mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	for label: float in custom:
		assert_float(label).is_between(0.15, 0.96)
	# and the REFUSED solid's own mesh is untouched: still the six
	# placeholder ordinals `nodes::solid::BOX_ORDINALS` built it with, four
	# vertices each, never overwritten by a label the paint pass never
	# chose for it. Hand-derived from the builder, not read back from the
	# paint pass: `render::paint::labelled_box` walks FACE_ORDER emitting
	# four corners per face, so ordinal k occupies vertices 4k..4k+3.
	var refused: PackedFloat32Array = _skin(flat).mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	assert_int(refused.size()).is_equal(24)
	for vertex: int in range(24):
		assert_float(refused[vertex]).is_equal(float(vertex / 4))


## The label lives in the MESH now, not in a per-instance uniform, so
## every path that rewrites a solid's surface has to carry it across — a
## knob dragged after the level derived once used to overwrite the whole
## CUSTOM0 array with `nodes::solid::BOX_ORDINALS`, handing the shader G
## values of 1..5 where a label in [0.15, 0.96] belongs. Every internal
## face boundary of that solid would then crease, and its seam with the
## floor it stands on would be judged against a number no colouring ever
## chose.
##
## The wall stands well inside the default 20x20 extents so the placement
## laws stay silent and the only behaviour under test is the resize.
## 0.15 is the floor slab's own fixed role label
## (`render::labels::role_label(Role::Floor)`) and 0.08 is MIN_SEP, both
## written out here rather than read back from the code that chose them.
func test_a_resize_after_the_derive_keeps_the_painted_labels() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var wall := _wall(4.0, Vector3(5, 0, 5), false)
	wall.name = "ResizedWall"
	level.add_child(wall)
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var painted: PackedFloat32Array = _skin(wall).mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	assert_int(painted.size()).is_equal(24)
	for label: float in painted:
		assert_float(label).is_between(0.15, 0.96)

	wall.length = 6.0
	var resized: PackedFloat32Array = _skin(wall).mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	assert_int(resized.size()).is_equal(24)
	for vertex: int in range(24):
		assert_float(resized[vertex]).is_equal(painted[vertex])
	# and the seam with the floor it stands on still draws
	for label: float in resized:
		assert_float(absf(label - 0.15)).is_greater_equal(0.08)


## The same law on the OTHER resize path: a column and a wedge rebuild
## their whole triangle list rather than resizing a box surface
## (`render::paint::resize_triangle_surface`), and the vertex count of
## both is fixed by their tessellation, never by the knob — so the labels
## carry across position for position exactly as a box's do.
func test_a_rebuilt_triangle_solid_keeps_the_painted_labels() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var column := WaveColumn.new()
	column.name = "ResizedColumn"
	column.radius = 0.3
	column.height = 1.0
	column.position = Vector3(10, 0, 10)
	level.add_child(column)
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)

	var painted: PackedFloat32Array = _skin(column).mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	assert_int(painted.size()).is_equal(384)  # COLUMN_SEGMENTS * 12
	for label: float in painted:
		assert_float(label).is_between(0.15, 0.96)

	column.radius = 0.5
	var resized: PackedFloat32Array = _skin(column).mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	assert_int(resized.size()).is_equal(384)
	for vertex: int in range(384):
		assert_float(resized[vertex]).is_equal(painted[vertex])


## Issue #35 (nesting inflation): mesh_world_box used to union EVERY
## descendant MeshInstance3D under a node, including a CENSUSED child's own
## skin — so grouping a prop under a crate for editor convenience silently
## grew the crate's OWN colouring box past what it actually draws.
##
## Hand-derived: Crate (WaveProp, size 1x1x1) at (4, 0.5, 4) draws x
## 3.50..4.50, y 0.00..1.00, z 3.50..4.50 — resting exactly on the floor.
## NestedProp (size 0.2 cubed) is Crate's own CHILD, at LOCAL (3, 0, 0):
## world (7, 0.5, 4), box x 6.90..7.10, y 0.40..0.60, z 3.90..4.10 — well
## past Crate's own footprint on x. FarProp (size 0.2 cubed) sits at (6.5,
## 0.5, 4), box x 6.40..6.60, y 0.40..0.60, z 3.90..4.10: clear of Crate's
## OWN box by 1.90 m on x, and of NestedProp's own box by 0.30 m on x
## (Box3::touches only grows a box by TOUCH_EPS = 0.01, so neither margin is
## close) — but squarely inside the UNION of the two, x 3.50..7.10. Pre-fix
## that union is exactly what the buggy recursion reported as Crate's own
## box, so Crate and FarProp came back as a touching pair; post-fix they do
## not.
func test_mesh_world_box_stops_unioning_a_nested_censused_child() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.extents = Vector2(20, 20)
	var crate := WaveProp.new()
	crate.name = "Crate"
	crate.size = Vector3.ONE
	crate.position = Vector3(4, 0.5, 4)
	var nested := WaveProp.new()
	nested.name = "NestedProp"
	nested.size = Vector3(0.2, 0.2, 0.2)
	nested.position = Vector3(3, 0, 0)  # local to Crate: world (7, 0.5, 4)
	crate.add_child(nested)
	level.add_child(crate)
	var far := WaveProp.new()
	far.name = "FarProp"
	far.size = Vector3(0.2, 0.2, 0.2)
	far.position = Vector3(6.5, 0.5, 4)
	level.add_child(far)
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var obs: WaveObserver = auto_free(WaveObserver.new())
	obs.inject(level, null)
	var e: Dictionary = obs.explain_oids()
	var touching := false
	for pair: Dictionary in e["pairs"]:
		var a: String = pair["name_a"]
		var b: String = pair["name_b"]
		if (a == "Crate" and b == "FarProp") or (a == "FarProp" and b == "Crate"):
			touching = true
	(
		assert_bool(touching)
		. append_failure_message(
			(
				"'Crate' and 'FarProp' share a touch pair — a nested child that "
				+ "neither box reaches is still inflating the crate's own colouring box"
			)
		)
		. is_false()
	)


## The other consumer mesh_world_box feeds: placed_solids/report_placement.
## Same law (issue #35), the placement half.
##
## ParentCrate (WaveProp, size 1x1x1) at (4, 0.5, 4) draws y 0.00..1.00 —
## its underside exactly on the floor, the same boundary the shipped map's
## own walls rest on without being flagged (test_the_shipped_map_keeps_
## every_solid_above_its_floor). ChildProp (size 0.2 cubed) is ParentCrate's
## own CHILD, at LOCAL (0, -0.5, 0): world (4, 0.0, 4), box y -0.10..0.10 —
## straddling the floor on its own, regardless of any fix. Pre-fix,
## mesh_world_box(parent) unioned the child's y range into the parent's OWN
## report too (y -0.10..1.00), spuriously sinking a crate that never moved;
## post-fix the parent reports only its own box and stays clean, while the
## child keeps its fault — blame moves to whichever node actually sits
## wrong, never to the parent for where the child sits.
func test_a_nested_prop_keeps_its_own_placement_fault_off_its_parent() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.extents = Vector2(20, 20)
	var crate := WaveProp.new()
	crate.name = "ParentCrate"
	crate.size = Vector3.ONE
	crate.position = Vector3(4, 0.5, 4)
	var child := WaveProp.new()
	child.name = "ChildProp"
	child.size = Vector3(0.2, 0.2, 0.2)
	child.position = Vector3(0, -0.5, 0)  # local to the crate: world (4, 0.0, 4)
	crate.add_child(child)
	level.add_child(crate)
	level.add_child(_spawn_marker(Vector3(1, 0, 3), 0.0))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	# exactly one solid may report a sunk fault here (the child); an
	# unmatched second push_error (the parent's, wrongly inflated by the
	# child's own y-range) fails the suite at teardown as a leftover Godot
	# runtime error the two assertions below name precisely.
	await assert_error(enter).is_push_error(any_string())
	assert_int(level.sunken_solids()).is_equal(1)
	(
		assert_array(crate.get_configuration_warnings())
		. append_failure_message(
			"'ParentCrate' still carries a fault that belongs to its nested child"
		)
		. is_empty()
	)
	var child_warnings := child.get_configuration_warnings()
	assert_int(child_warnings.size()).is_equal(1)
	assert_str(child_warnings[0]).contains("is sunk through the floor")
