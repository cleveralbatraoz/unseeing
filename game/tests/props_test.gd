# gdlint:ignore = max-public-methods
extends GdUnitTestSuite
## The prop SHAPES. A contours-only world has no material and no texture, so
## an object IS its silhouette — which is why the vocabulary is three rather
## than one. Each shape draws a line the others cannot:
##
##   WaveProp    a box       — corners
##   WaveColumn  a cylinder  — a curve
##   WaveWedge   a prism     — a diagonal
##
## What this suite holds: that each builds a body the waves can actually
## strike (mesh AND collider, reshaped together by the size knobs, so what
## is drawn is what is struck), and the ORIGIN LAW that differs on purpose —
## a box is centred on its node because a box is as often floating as
## standing, while a column and a wedge stand ON theirs, because a barrel
## resting nowhere is a mistake.
##
## (The directive above must sit on line 1 — gdlint keys an ignore to the
## line its problem is reported on. A gdUnit4 suite is a list of cases, not
## a class with an API: every case is a public method, so the 20-method
## ceiling counts coverage rather than surface. Suppressed here and in
## level_test.gd, the two suites that outgrew it, and nowhere else.)

const LIFT_EPS := 0.0001


func _spawn(at: Vector3) -> WaveSpawn:
	var spawn := WaveSpawn.new()
	spawn.position = at
	return spawn


func _run(from: Vector2, to: Vector2, openings: PackedVector2Array = []) -> WaveRun:
	var run := WaveRun.new()
	run.from = from
	run.to = to
	run.openings = openings
	return run


## A run injected before tree entry remembers the level skin, then emits the
## same two shipped divider walls as ownerless typed children at ready time.
func test_wave_run_emits_named_materialized_walls() -> void:
	var material := ShaderMaterial.new()
	var level: WaveLevel = auto_free(WaveLevel.new())
	var run := _run(Vector2(6.4, 0.6), Vector2(6.4, 19.4), [Vector2(8.0, 4.4)])
	level.add_child(run)
	level.add_child(_spawn(Vector3(2, 0, 2)))
	level.inject(material, ShaderMaterial.new(), Pulses.new())
	add_child(level)
	assert_int(run.get_child_count()).is_equal(2)
	assert_str(run.get_child(0).name).is_equal("RunSeg1")
	assert_str(run.get_child(1).name).is_equal("RunSeg2")
	assert_int(level.wall_segments().size()).is_equal(2)
	for wall: WaveWall in run.get_children():
		assert_object(_skin(wall).material_override).is_same(material)
		assert_object(wall.owner).is_null()


## Setter rebuilds clear their previous derived children before emitting the
## new residuals, so Inspector edits and Ctrl+D ghosts cannot double walls.
func test_wave_run_setters_rebuild_idempotently() -> void:
	var run: WaveRun = auto_free(_run(Vector2.ZERO, Vector2(10, 0)))
	add_child(run)
	assert_int(run.get_child_count()).is_equal(1)
	run.openings = [Vector2(4, 2)]
	assert_int(run.get_child_count()).is_equal(2)
	assert_str(run.get_child(0).name).is_equal("RunSeg1")
	assert_str(run.get_child(1).name).is_equal("RunSeg2")
	run.openings = [Vector2(4, 2)]
	assert_int(run.get_child_count()).is_equal(2)


## Openings are authored data, not a preview detail: packing and instantiating
## a scene must retain every pair exactly. This is the Ctrl+S regression gate.
func test_wave_run_openings_survive_scene_pack_and_reload() -> void:
	var root: Node3D = auto_free(Node3D.new())
	var run := _run(Vector2.ZERO, Vector2(10, 0), [Vector2(2, 1), Vector2(7, 2)])
	run.name = "Run"
	root.add_child(run)
	root.scene_file_path = "res://tests/fixtures/wave_run_pack_probe.tscn"
	run.owner = root
	var packed := PackedScene.new()
	assert_int(packed.pack(root)).is_equal(OK)
	var copy: Node3D = auto_free(packed.instantiate() as Node3D)
	var restored := copy.get_node("Run") as WaveRun
	assert_array(restored.openings).is_equal([Vector2(2, 1), Vector2(7, 2)])


## Rebuild owns only children it generated. A designer may use the same prefix
## for an annotation or authored child without the engine deleting their work.
func test_wave_run_rebuild_preserves_a_designer_owned_runseg_child() -> void:
	var run: WaveRun = auto_free(_run(Vector2.ZERO, Vector2(10, 0)))
	var note := Marker3D.new()
	note.name = "RunSegReference"
	run.add_child(note)
	add_child(run)
	assert_bool(is_instance_valid(note)).is_true()
	run.openings = [Vector2(4, 2)]
	assert_bool(is_instance_valid(note)).is_true()
	assert_object(note.get_parent()).is_same(run)


## Dragging an already-readied tool node is the ordinary editor gesture. Its
## planar pose must become endpoint data and the node must return to identity.
func test_wave_run_absorbs_a_planar_drag_after_ready() -> void:
	var run: WaveRun = auto_free(_run(Vector2.ZERO, Vector2(4, 0)))
	add_child(run)
	run.position = Vector3(3, 0, 4)
	run.rotation.y = PI * 0.5
	await get_tree().process_frame
	assert_bool(run.transform.is_equal_approx(Transform3D.IDENTITY)).is_true()
	# The translation is applied first, then the quarter-turn Godot stores in
	# the local basis: (0,0)->(4,-3), (4,0)->(4,-7).
	assert_vector(run.from).is_equal_approx(Vector2(4, -3), Vector2.ONE * 0.0001)
	assert_vector(run.to).is_equal_approx(Vector2(4, -7), Vector2.ONE * 0.0001)


func test_wave_run_diagonal_warning_clears_with_the_endpoints() -> void:
	var run: WaveRun = auto_free(_run(Vector2.ZERO, Vector2(4, 4)))
	add_child(run)
	var warnings := run.get_configuration_warnings()
	assert_int(warnings.size()).is_equal(1)
	if warnings.is_empty():
		return
	assert_str(warnings[0]).contains("dominant X axis")
	run.to = Vector2(4, 0)
	assert_array(run.get_configuration_warnings()).is_empty()


## A placed run absorbs its own planar pose into parent-local endpoint data;
## the plain room above it remains free to rotate as a normal prefab ancestor.
func test_wave_run_absorbs_its_planar_transform() -> void:
	var run: WaveRun = auto_free(_run(Vector2.ZERO, Vector2(4, 0), [Vector2(1, 2)]))
	run.position = Vector3(3, 0, 4)
	run.rotation.y = PI * 0.5
	add_child(run)
	assert_bool(run.transform.is_equal_approx(Transform3D.IDENTITY)).is_true()
	assert_vector(run.from).is_equal_approx(Vector2(3, 4), Vector2.ONE * 0.0001)
	assert_vector(run.to).is_equal_approx(Vector2(3, 0), Vector2.ONE * 0.0001)
	assert_vector(run.openings[0]).is_equal_approx(Vector2(1, 2), Vector2.ONE * 0.0001)


func test_wave_run_translates_opening_coordinates_with_its_pose() -> void:
	var run: WaveRun = auto_free(_run(Vector2.ZERO, Vector2(8, 0), [Vector2(1, 2)]))
	run.position = Vector3(3, 0, 4)
	add_child(run)
	assert_vector(run.openings[0]).is_equal_approx(Vector2(4, 2), Vector2.ONE * 0.0001)


func test_wave_run_warns_when_y_scale_is_discarded() -> void:
	var run: WaveRun = auto_free(_run(Vector2.ZERO, Vector2(4, 0)))
	run.scale = Vector3(1, 2, 1)
	add_child(run)
	var warnings := run.get_configuration_warnings()
	assert_int(warnings.size()).is_equal(1)
	if warnings.is_empty():
		return
	assert_str(warnings[0]).contains("Y translation or tilt")


## The opening is a true absence in the level's occluder table: neither
## positive residual crosses its absolute axis-coordinate doorway.
func test_wave_run_doorway_leaves_no_occluder_across_its_gap() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_run(Vector2(5, 1), Vector2(5, 9), [Vector2(4, 2)]))
	level.add_child(_spawn(Vector3(2, 0, 5)))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	assert_int(level.wall_segments().size()).is_equal(2)
	for segment: Vector4 in level.wall_segments():
		var lo := minf(segment.y, segment.w)
		var hi := maxf(segment.y, segment.w)
		assert_bool(lo < 5.0 and hi > 5.0).is_false()


## Generated leaf names repeat in every run, so the level-facing table must
## expose paths. Otherwise an observer cannot tell which RunSeg1 occluded it.
func test_wave_run_walls_are_exposed_by_level_relative_path() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var divider := _run(Vector2(5, 1), Vector2(5, 4))
	divider.name = "Divider"
	level.add_child(divider)
	var east := _run(Vector2(9, 1), Vector2(9, 4))
	east.name = "PartyEast"
	level.add_child(east)
	level.add_child(_spawn(Vector3(2, 0, 2)))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var observer: WaveObserver = auto_free(WaveObserver.new())
	var camera: Camera3D = auto_free(Camera3D.new())
	observer.inject(level, camera)
	var names: Array[String] = []
	for wall: Dictionary in observer.explain_ray(Vector3.ZERO, Vector3.ONE)["walls"]:
		names.append(wall["name"])
	assert_array(names).contains(["Divider/RunSeg1", "PartyEast/RunSeg1"])


## A reusable prop is composition, not framework: its plain root lets the
## level recurse into every typed piece in each independent instance.
func test_chair_prefab_instances_are_recursively_censused() -> void:
	assert_bool(ResourceLoader.exists("res://scenes/props/chair.tscn")).is_true()
	if not ResourceLoader.exists("res://scenes/props/chair.tscn"):
		return
	var scene := load("res://scenes/props/chair.tscn") as PackedScene
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.extents = Vector2(20, 20)
	for at: Vector3 in [Vector3(3, 0, 3), Vector3(8, 0, 3)]:
		var chair := scene.instantiate() as Node3D
		chair.position = at
		level.add_child(chair)
	level.add_child(_spawn(Vector3(1, 0, 1)))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var pieces := level.find_children("*", "WaveProp", true, false)
	assert_int(pieces.size()).is_equal(12)
	assert_array(level.get_configuration_warnings()).is_empty()
	for piece: WaveProp in pieces:
		assert_bool(piece.oid() > 0.0).is_true()


## Heading is world data: a zero-yaw spawn inside a turned prefab faces the
## prefab's global quarter turn without any code or duplicated angle knob.
func test_spawn_under_a_rotated_prefab_uses_global_yaw() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var room := Node3D.new()
	room.rotation.y = PI * 0.5
	room.add_child(_spawn(Vector3(2, 0, 3)))
	level.add_child(room)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	assert_float(level.spawn_yaw()).is_equal_approx(PI * 0.5, 0.0001)


func _skin(body: Node) -> MeshInstance3D:
	for child: Node in body.get_children():
		if child is MeshInstance3D:
			return child as MeshInstance3D
	return null


func _shape(body: Node) -> Shape3D:
	for child: Node in body.get_children():
		if child is CollisionShape3D:
			return (child as CollisionShape3D).shape
	return null


func _collider(body: Node) -> CollisionShape3D:
	for child: Node in body.get_children():
		if child is CollisionShape3D:
			return child as CollisionShape3D
	return null


## The world box a shape actually draws — what the eye is shown, and the
## only place the origin law can be read once a node is turned.
func _world_box(body: Node) -> AABB:
	var skin := _skin(body)
	return skin.global_transform * skin.get_aabb()


## A column is a real cylinder to the eye AND to the rays: mesh and collider
## carry the same radius and height, and both follow the knobs live. The
## mesh is an `ArrayMesh` now (Task 5), generated rather than a `CylinderMesh`
## primitive, so its radius/height are read off its own AABB — diameter
## across X/Z, height along Y — instead of a `top_radius`/`height` property
## that no longer exists; the resize is still IN PLACE (matching the
## primitive's own live-reshape behaviour), so the same captured `mesh`
## handle reflects a later knob drag with no re-fetch.
func test_column_builds_a_cylinder_the_rays_can_strike() -> void:
	var column: WaveColumn = auto_free(WaveColumn.new())
	column.radius = 0.3
	column.height = 0.9
	add_child(column)
	var mesh := _skin(column).mesh as ArrayMesh
	var shape := _shape(column) as CylinderShape3D
	assert_object(mesh).is_not_null()
	assert_object(shape).is_not_null()
	assert_float(mesh.get_aabb().size.x).is_equal_approx(0.6, LIFT_EPS)
	assert_float(mesh.get_aabb().size.z).is_equal_approx(0.6, LIFT_EPS)
	assert_float(mesh.get_aabb().size.y).is_equal_approx(0.9, LIFT_EPS)
	assert_float(shape.radius).is_equal_approx(0.3, LIFT_EPS)
	assert_float(shape.height).is_equal_approx(0.9, LIFT_EPS)
	column.radius = 0.5
	column.height = 2.0
	assert_float(mesh.get_aabb().size.x).is_equal_approx(1.0, LIFT_EPS)
	assert_float(shape.radius).is_equal_approx(0.5, LIFT_EPS)
	assert_float(mesh.get_aabb().size.y).is_equal_approx(2.0, LIFT_EPS)
	assert_float(shape.height).is_equal_approx(2.0, LIFT_EPS)


## The origin law: a column STANDS on its node. y = 0 puts a barrel on the
## floor with no arithmetic from the designer, so both limbs ride half a
## height up — and they re-ride it when the height knob moves.
func test_column_stands_on_its_node() -> void:
	var column: WaveColumn = auto_free(WaveColumn.new())
	column.height = 0.9
	add_child(column)
	assert_vector(_skin(column).position).is_equal_approx(
		Vector3(0, 0.45, 0), Vector3.ONE * LIFT_EPS
	)
	assert_vector(_collider(column).position).is_equal_approx(
		Vector3(0, 0.45, 0), Vector3.ONE * LIFT_EPS
	)
	column.height = 3.0
	assert_vector(_skin(column).position).is_equal_approx(
		Vector3(0, 1.5, 0), Vector3.ONE * LIFT_EPS
	)
	assert_vector(_collider(column).position).is_equal_approx(
		Vector3(0, 1.5, 0), Vector3.ONE * LIFT_EPS
	)


## THE ORIGIN LAW, generalised — because "a column stands on its node" is a
## law about the FLOOR, and the floor is a world thing. The lift that
## implements it is a local offset of half the height, so the moment the
## node is turned off vertical the lift points sideways and the barrel sinks:
## laid on its side it went a full radius through the floor. Standing means
## the shape's LOWEST point in world space rests at the node's own y —
## which for a tipped cylinder is the RADIUS, not half the height.
func test_a_tipped_column_still_stands_on_its_node() -> void:
	var column: WaveColumn = auto_free(WaveColumn.new())
	column.radius = 0.3
	column.height = 0.9
	column.rotation.z = PI * 0.5
	add_child(column)
	var box := _world_box(column)
	(
		assert_float(box.position.y)
		. append_failure_message("the barrel sank to y = %.3f" % box.position.y)
		. is_equal_approx(0.0, 0.001)
	)
	# lying down it is two radii tall and a height long, and it rests on
	# the floor rather than hovering a half-height over it
	assert_float(box.size.y).is_equal_approx(0.6, 0.001)
	assert_float(box.size.x).is_equal_approx(0.9, 0.001)


## The same law on the wedge, whose underside is a flat face rather than a
## curve: tipped a quarter turn it stands on what used to be its tall end,
## and its lowest point is still the node's own y.
func test_a_tipped_wedge_still_stands_on_its_node() -> void:
	var wedge: WaveWedge = auto_free(WaveWedge.new())
	wedge.size = Vector3(1.2, 0.6, 0.8)
	wedge.rotation.z = PI * 0.5
	add_child(wedge)
	var box := _world_box(wedge)
	(
		assert_float(box.position.y)
		. append_failure_message("the ramp sank to y = %.3f" % box.position.y)
		. is_equal_approx(0.0, 0.001)
	)
	assert_float(box.size.y).is_equal_approx(1.2, 0.001)


## A shape hangs under whatever transform reaches it, its own and its
## ancestors' alike, so the law has to be read in the space the floor is in.
## A barrel dropped into a prefab that has been tipped over would otherwise
## stand on the node only while the prefab was upright.
func test_an_inherited_tip_sinks_nothing_either() -> void:
	var room: Node3D = auto_free(Node3D.new())
	room.position = Vector3(3, 0, 5)
	room.rotation.z = PI * 0.5
	var column := WaveColumn.new()
	column.radius = 0.3
	column.height = 0.9
	room.add_child(column)
	add_child(room)
	var box := _world_box(column)
	(
		assert_float(box.position.y)
		. append_failure_message("the barrel sank to y = %.3f" % box.position.y)
		. is_equal_approx(0.0, 0.001)
	)


## Turning a node after it is built has to re-lift it, or the law holds
## only for shapes a designer never touches again: rotating a barrel in the
## viewport fires no knob setter, and the editor would show it sinking with
## no way to get it back except reloading the scene.
func test_turning_a_placed_column_re_lifts_it() -> void:
	var column: WaveColumn = auto_free(WaveColumn.new())
	column.radius = 0.3
	column.height = 0.9
	add_child(column)
	assert_float(_world_box(column).position.y).is_equal_approx(0.0, 0.001)
	column.rotation.z = PI * 0.5
	# the engine accumulates transform changes and notifies once, so ask for
	# the update the frame boundary would have brought
	column.force_update_transform()
	(
		assert_float(_world_box(column).position.y)
		. append_failure_message("a turned barrel kept the lift it was built with")
		. is_equal_approx(0.0, 0.001)
	)
	column.rotation.z = 0.0
	column.force_update_transform()
	assert_float(_world_box(column).position.y).is_equal_approx(0.0, 0.001)
	assert_float(_world_box(column).size.y).is_equal_approx(0.9, 0.001)


## A wedge is GENERATED — the engine ships neither a triangular prism mesh
## nor a prism collider. Eight triangles of surface for the eye, a six-point
## convex hull for the rays, and both rebuilt when the size knob moves,
## because every vertex of a prism shifts when any extent changes.
func test_wedge_generates_surface_and_hull_together() -> void:
	var wedge: WaveWedge = auto_free(WaveWedge.new())
	wedge.size = Vector3(1.2, 0.6, 0.8)
	add_child(wedge)
	var mesh := _skin(wedge).mesh as ArrayMesh
	var hull := _shape(wedge) as ConvexPolygonShape3D
	assert_object(mesh).is_not_null()
	assert_object(hull).is_not_null()
	assert_int(mesh.get_surface_count()).is_equal(1)
	assert_int(mesh.surface_get_array_len(0)).is_equal(24)  # 8 triangles
	assert_int(hull.points.size()).is_equal(6)
	# the hull fills exactly the box the designer sized, and only its two
	# tall corners rise: that is what makes it a wedge and not a box
	var tall := 0
	for p: Vector3 in hull.points:
		assert_bool(absf(p.x) <= 0.6 + LIFT_EPS).is_true()
		assert_bool(absf(p.y) <= 0.3 + LIFT_EPS).is_true()
		assert_bool(absf(p.z) <= 0.4 + LIFT_EPS).is_true()
		if p.y > 0.0:
			tall += 1
	assert_int(tall).is_equal(2)
	wedge.size = Vector3(2.0, 1.0, 1.0)
	var grown := _shape(wedge) as ConvexPolygonShape3D
	var reach := 0.0
	for p: Vector3 in grown.points:
		reach = maxf(reach, p.x)
	assert_float(reach).is_equal_approx(1.0, LIFT_EPS)


## Task 5's ordinal contract on the wedge: each of its eight triangles
## carries the CUSTOM0 ordinal of the one face it belongs to, in
## `render::faces::wedge_faces`'s own order — floor (0, two triangles), tall
## back wall (1, two), slope (2, two), −Z end (3, one), +Z end (4, one) —
## matching `prop_shape::WEDGE_TRIANGLE_ORDINALS` and
## `prop_shape::wedge_triangles`'s identical triangle order.
func test_wedge_mesh_carries_custom0_ordinals_matching_its_five_faces() -> void:
	var wedge: WaveWedge = auto_free(WaveWedge.new())
	wedge.size = Vector3(1.2, 0.6, 0.8)
	add_child(wedge)
	var mesh := _skin(wedge).mesh as ArrayMesh
	var arrays: Array = mesh.surface_get_arrays(0)
	var custom: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM0]
	assert_int(custom.size()).is_equal(24)
	var want := PackedFloat32Array(
		[0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 3, 3, 4, 4, 4]
	)
	for i: int in custom.size():
		assert_float(custom[i]).is_equal_approx(want[i], 1e-6)


## Task 5's ordinal contract on the column: its two flat rims take ordinals
## 0 (bottom) and 1 (top), matching `render::faces::column_faces`'s own
## bottom-then-top order, and the curved flank — which has no plane and so
## no entry in `faces()` at all — takes ordinal 2. `column_triangles` builds
## one SEGMENT at a time (32 of them, `render::faces::RIM_SEGMENTS`), each
## contributing its own bottom-cap triangle (3 verts, ordinal 0), top-cap
## triangle (3 verts, ordinal 1) and two flank triangles (6 verts, ordinal
## 2) before moving to the next segment — so the 384 vertices run in 32
## repeats of that 3/3/6 block, never as three long global runs.
func test_column_mesh_carries_custom0_ordinals_for_rims_and_flank() -> void:
	var column: WaveColumn = auto_free(WaveColumn.new())
	column.radius = 0.3
	column.height = 0.9
	add_child(column)
	var mesh := _skin(column).mesh as ArrayMesh
	var arrays: Array = mesh.surface_get_arrays(0)
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var custom: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM0]
	assert_int(verts.size()).is_equal(384)
	assert_int(custom.size()).is_equal(verts.size())
	for segment: int in 32:
		var base := segment * 12
		for i: int in range(base, base + 3):
			assert_float(custom[i]).is_equal_approx(0.0, 1e-6)
		for i: int in range(base + 3, base + 6):
			assert_float(custom[i]).is_equal_approx(1.0, 1e-6)
		for i: int in range(base + 6, base + 12):
			assert_float(custom[i]).is_equal_approx(2.0, 1e-6)


## The origin law again: a wedge stands on its node, so a ramp placed at
## y = 0 rests on the floor and rises from there.
func test_wedge_stands_on_its_node() -> void:
	var wedge: WaveWedge = auto_free(WaveWedge.new())
	wedge.size = Vector3(1.2, 0.6, 0.8)
	add_child(wedge)
	assert_vector(_skin(wedge).position).is_equal_approx(Vector3(0, 0.3, 0), Vector3.ONE * LIFT_EPS)
	assert_vector(_collider(wedge).position).is_equal_approx(
		Vector3(0, 0.3, 0), Vector3.ONE * LIFT_EPS
	)
	wedge.size = Vector3(1.2, 1.4, 0.8)
	assert_vector(_skin(wedge).position).is_equal_approx(Vector3(0, 0.7, 0), Vector3.ONE * LIFT_EPS)


## A minus sign in a size knob must never split what is DRAWN from what is
## STRUCK. The generated box mesh takes a negative extent happily; BoxShape3D
## REFUSES it and silently keeps whatever it had — its default 1 x 1 x 1 —
## so the box a designer sees and the box the waves and the cane hit are
## different objects. The only engine diagnostic ("BoxShape3D size cannot be
## negative") names no node, and the bad value survives a save. So the sign
## folds away at the knob, and the Inspector reads back what was built.
func test_a_negative_size_cannot_split_the_drawn_box_from_its_collider() -> void:
	var prop: WaveProp = auto_free(WaveProp.new())
	prop.size = Vector3(-0.8, 0.4, -0.6)
	add_child(prop)
	assert_vector(prop.size).is_equal(Vector3(0.8, 0.4, 0.6))
	assert_vector(_skin(prop).mesh.get_aabb().size).is_equal(Vector3(0.8, 0.4, 0.6))
	(
		assert_vector((_shape(prop) as BoxShape3D).size)
		. append_failure_message("the collider kept its own size while the mesh was reshaped")
		. is_equal(Vector3(0.8, 0.4, 0.6))
	)
	prop.size = Vector3(1.0, -2.0, 1.0)  # and live, on a dragged knob
	assert_vector(_skin(prop).mesh.get_aabb().size).is_equal(Vector3(1, 2, 1))
	assert_vector((_shape(prop) as BoxShape3D).size).is_equal(Vector3(1, 2, 1))


## The same law on the round and the sloped shape: one vocabulary, one
## answer to a minus sign. CylinderShape3D refuses a negative radius or
## height exactly as the box shape does, and a wedge's generated hull would
## simply mirror itself — so every knob folds the sign away and reads back
## the magnitude it built.
func test_a_negative_knob_folds_on_every_shape() -> void:
	var column: WaveColumn = auto_free(WaveColumn.new())
	column.radius = -0.3
	column.height = -0.9
	add_child(column)
	assert_float(column.radius).is_equal_approx(0.3, LIFT_EPS)
	assert_float(column.height).is_equal_approx(0.9, LIFT_EPS)
	assert_float((_shape(column) as CylinderShape3D).radius).is_equal_approx(0.3, LIFT_EPS)
	assert_float((_shape(column) as CylinderShape3D).height).is_equal_approx(0.9, LIFT_EPS)
	var wedge: WaveWedge = auto_free(WaveWedge.new())
	wedge.size = Vector3(-1.2, 0.6, -0.8)
	add_child(wedge)
	assert_vector(wedge.size).is_equal(Vector3(1.2, 0.6, 0.8))


## THE S KEY, answered the same way twice. A wall discards its node scale
## and says so; the three prop shapes ABSORBED it silently — a prop of
## 0.5 under scale (4, 1, 2) drew, collided and coloured as 2.0 x 0.5 x 1.0,
## which is self-consistent (the world was right) while the Inspector went
## on reporting 0.5. Two halves of one vocabulary behaving oppositely on the
## same key is the trap; a knob that lies is worse than either. So the scale
## folds INTO the knob and the node comes back at 1: the geometry does not
## move, and the number a designer reads is the number that was built.
func test_a_scaled_prop_folds_the_scale_into_its_size_knob() -> void:
	var prop: WaveProp = auto_free(WaveProp.new())
	prop.size = Vector3(0.5, 0.5, 0.5)
	prop.scale = Vector3(4, 1, 2)
	add_child(prop)
	assert_vector(prop.size).is_equal_approx(Vector3(2, 0.5, 1), Vector3.ONE * LIFT_EPS)
	assert_vector(prop.scale).is_equal_approx(Vector3.ONE, Vector3.ONE * LIFT_EPS)
	assert_vector(_skin(prop).mesh.get_aabb().size).is_equal_approx(
		Vector3(2, 0.5, 1), Vector3.ONE * LIFT_EPS
	)
	assert_vector((_shape(prop) as BoxShape3D).size).is_equal_approx(
		Vector3(2, 0.5, 1), Vector3.ONE * LIFT_EPS
	)
	# the fold is a change of vocabulary, not of geometry: the world box is
	# exactly the one the scaled node drew
	assert_vector(_world_box(prop).size).is_equal_approx(Vector3(2, 0.5, 1), Vector3.ONE * 0.001)


## A wedge's size is three components against a three-component scale, so
## the fold is exact there too — every vertex it generates lies on its own
## local axes.
func test_a_scaled_wedge_folds_the_scale_into_its_size_knob() -> void:
	var wedge: WaveWedge = auto_free(WaveWedge.new())
	wedge.size = Vector3(1.2, 0.5, 0.8)
	wedge.scale = Vector3(2, 1, 3)
	add_child(wedge)
	assert_vector(wedge.size).is_equal_approx(Vector3(2.4, 0.5, 2.4), Vector3.ONE * LIFT_EPS)
	assert_vector(wedge.scale).is_equal_approx(Vector3.ONE, Vector3.ONE * LIFT_EPS)
	assert_vector(_world_box(wedge).size).is_equal_approx(
		Vector3(2.4, 0.5, 2.4), Vector3.ONE * 0.001
	)
	assert_float(_world_box(wedge).position.y).is_equal_approx(0.0, 0.001)


## A column carries ONE radius and ONE height against a three-component
## scale. Uniform, that is exactly representable and the fold loses nothing.
func test_a_uniformly_scaled_column_folds_exactly() -> void:
	var column: WaveColumn = auto_free(WaveColumn.new())
	column.radius = 0.3
	column.height = 0.9
	column.scale = Vector3(2, 2, 2)
	add_child(column)
	assert_float(column.radius).is_equal_approx(0.6, LIFT_EPS)
	assert_float(column.height).is_equal_approx(1.8, LIFT_EPS)
	assert_vector(column.scale).is_equal_approx(Vector3.ONE, Vector3.ONE * LIFT_EPS)
	assert_float(_world_box(column).size.y).is_equal_approx(1.8, 0.001)
	assert_float(_world_box(column).position.y).is_equal_approx(0.0, 0.001)


## Pulled by different amounts across X and Z, a cylinder is an ELLIPTIC
## cylinder — a shape neither the generated column mesh's circular ring nor
## CylinderShape3D can be, and one this vocabulary deliberately does not own.
## The fold takes the LARGER of the two, so the barrel a designer ends up
## with CONTAINS the one they drew: erring inwards would leave drawn
## geometry outside the collider, and refusing the scale outright would
## throw away the axial stretch they can perfectly well have.
func test_a_non_uniformly_scaled_column_grows_to_contain_what_was_drawn() -> void:
	var column: WaveColumn = auto_free(WaveColumn.new())
	column.radius = 0.3
	column.height = 0.9
	column.scale = Vector3(2, 3, 1)
	add_child(column)
	assert_float(column.height).is_equal_approx(2.7, LIFT_EPS)
	assert_float(column.radius).is_equal_approx(0.6, LIFT_EPS)
	assert_vector(column.scale).is_equal_approx(Vector3.ONE, Vector3.ONE * LIFT_EPS)
	assert_float(_world_box(column).size.y).is_equal_approx(2.7, 0.001)


## A mirrored axis is not a size, so only its magnitude survives the fold —
## and the shape must not silently keep drawing at the scaled extent it had.
func test_a_mirrored_scale_folds_to_its_magnitude() -> void:
	var prop: WaveProp = auto_free(WaveProp.new())
	prop.size = Vector3(0.5, 0.5, 0.5)
	prop.scale = Vector3(-2, 1, 1)
	add_child(prop)
	assert_vector(prop.size).is_equal_approx(Vector3(1, 0.5, 0.5), Vector3.ONE * LIFT_EPS)
	assert_vector(_world_box(prop).size).is_equal_approx(Vector3(1, 0.5, 0.5), Vector3.ONE * 0.001)


## An unscaled node must come through untouched — the shipped map is 129
## nodes of scale exactly 1, and a fold that fired on float dust would move
## every one of them.
func test_an_unscaled_shape_is_left_alone() -> void:
	var prop: WaveProp = auto_free(WaveProp.new())
	prop.size = Vector3(0.9, 0.05, 0.6)
	prop.rotation.y = 0.3
	add_child(prop)
	assert_vector(prop.size).is_equal(Vector3(0.9, 0.05, 0.6))
	var column: WaveColumn = auto_free(WaveColumn.new())
	column.radius = 0.28
	column.height = 0.9
	column.rotation.y = 2.1
	add_child(column)
	assert_float(column.radius).is_equal(0.28)
	assert_float(column.height).is_equal(0.9)


## All three shapes reach the level through ONE door: the level hands out
## the world skin and the flat object id without knowing which shape it has
## — that is the whole point of the solid abstraction. A shape that answered
## only some of it would silently lose its outline or its seams.
func test_every_shape_answers_the_same_solid_door() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var marker := WaveSpawn.new()
	level.add_child(marker)
	var box := WaveProp.new()
	box.position = Vector3(1, 0.25, 1)
	level.add_child(box)
	var column := WaveColumn.new()
	column.position = Vector3(5, 0, 5)
	level.add_child(column)
	var wedge := WaveWedge.new()
	wedge.position = Vector3(9, 0, 9)
	level.add_child(wedge)
	var world := ShaderMaterial.new()
	level.inject(world, ShaderMaterial.new(), Pulses.new())
	add_child(level)
	for solid: Node in [box, column, wedge]:
		assert_object(_skin(solid).material_override).is_same(world)
		var oid: float = solid.call("oid")
		# `oid >= 0.0` alone is vacuous post-flip: a solid's mesh carries a
		# real, non-negative CUSTOM0 ordinal (0..5, BOX_ORDINALS) from the
		# moment it is BUILT, before the derive-time paint pass ever runs —
		# so "never painted" and "painted for real" both clear that bar.
		# Real labels live in [0.15, 0.96] (the perception law), strictly
		# outside the placeholder ordinals' own range, so this is the check
		# that actually tells "painted" from "never painted" apart.
		(
			assert_float(oid)
			. append_failure_message("%s took no real label (read back %.3f)" % [solid.name, oid])
			. is_between(0.15, 0.96)
		)


## How many limbs a shape has built for itself — one mesh and one collider
## is a whole shape; anything more is a ghost of an earlier build.
func _limbs_of(body: Node) -> int:
	var n := 0
	for child: Node in body.get_children():
		if child is MeshInstance3D or child is CollisionShape3D:
			n += 1
	return n


## Furniture is authored by duplicating furniture (game/README.md), and
## Ctrl+D is `Node.duplicate()`: the copy arrives already carrying the mesh
## and collider the original built for itself. A builder that adds a pair
## unconditionally therefore gives the copy TWO — the size knob reaches only
## the newest, so the ghost is drawn at the size it was copied at, forever,
## and its collider is struck there too. All three shapes answer alike,
## because a designer duplicates all three alike.
func test_a_duplicated_shape_does_not_double_its_geometry() -> void:
	for shape: Node3D in [WaveProp.new(), WaveColumn.new(), WaveWedge.new()]:
		var solid: Node3D = auto_free(shape)
		add_child(solid)
		var copy: Node3D = auto_free(solid.duplicate() as Node3D)
		add_child(copy)
		(
			assert_int(_limbs_of(copy))
			. append_failure_message(
				"%s readied onto the limbs it was copied with" % copy.get_class()
			)
			. is_equal(2)
		)


## A column and a wedge ride their own limbs up onto their lift — and ONLY
## their own. Walking the node's children instead would teleport whatever a
## designer nested under it, which is the natural way to group props in the
## editor: a lid on a barrel would be silently buried inside it, invisible
## from every side, with no engine system able to notice.
func test_a_shape_lifts_its_own_limbs_and_nothing_a_designer_nested() -> void:
	for shape: Node3D in [WaveColumn.new(), WaveWedge.new()]:
		var solid: Node3D = auto_free(shape)
		var guest := Marker3D.new()
		guest.name = "Guest"
		guest.position = Vector3(0, 0.95, 0)
		solid.add_child(guest)
		add_child(solid)
		(
			assert_vector(guest.position)
			. append_failure_message("%s moved a nested child on _ready" % solid.get_class())
			. is_equal(Vector3(0, 0.95, 0))
		)
		if solid is WaveColumn:
			(solid as WaveColumn).height = 2.4
		else:
			(solid as WaveWedge).size = Vector3(1.0, 2.4, 1.0)
		(
			assert_vector(guest.position)
			. append_failure_message("%s moved a nested child on a knob drag" % solid.get_class())
			. is_equal(Vector3(0, 0.95, 0))
		)
		# ...while its OWN limb did ride up
		assert_vector(_skin(solid).position).is_equal_approx(
			Vector3(0, 1.2, 0), Vector3.ONE * LIFT_EPS
		)
