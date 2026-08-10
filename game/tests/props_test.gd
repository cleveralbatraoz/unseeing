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

const LIFT_EPS := 0.0001


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


## A column is a real cylinder to the eye AND to the rays: mesh and collider
## carry the same radius and height, and both follow the knobs live.
func test_column_builds_a_cylinder_the_rays_can_strike() -> void:
	var column: WaveColumn = auto_free(WaveColumn.new())
	column.radius = 0.3
	column.height = 0.9
	add_child(column)
	var mesh := _skin(column).mesh as CylinderMesh
	var shape := _shape(column) as CylinderShape3D
	assert_object(mesh).is_not_null()
	assert_object(shape).is_not_null()
	assert_float(mesh.top_radius).is_equal_approx(0.3, LIFT_EPS)
	assert_float(mesh.bottom_radius).is_equal_approx(0.3, LIFT_EPS)
	assert_float(mesh.height).is_equal_approx(0.9, LIFT_EPS)
	assert_float(shape.radius).is_equal_approx(0.3, LIFT_EPS)
	assert_float(shape.height).is_equal_approx(0.9, LIFT_EPS)
	column.radius = 0.5
	column.height = 2.0
	assert_float(mesh.top_radius).is_equal_approx(0.5, LIFT_EPS)
	assert_float(shape.radius).is_equal_approx(0.5, LIFT_EPS)
	assert_float(mesh.height).is_equal_approx(2.0, LIFT_EPS)
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
## STRUCK. BoxMesh takes a negative extent happily; BoxShape3D REFUSES it
## and silently keeps whatever it had — its default 1 x 1 x 1 — so the box
## a designer sees and the box the waves and the cane hit are different
## objects. The only engine diagnostic ("BoxShape3D size cannot be
## negative") names no node, and the bad value survives a save. So the sign
## folds away at the knob, and the Inspector reads back what was built.
func test_a_negative_size_cannot_split_the_drawn_box_from_its_collider() -> void:
	var prop: WaveProp = auto_free(WaveProp.new())
	prop.size = Vector3(-0.8, 0.4, -0.6)
	add_child(prop)
	assert_vector(prop.size).is_equal(Vector3(0.8, 0.4, 0.6))
	assert_vector((_skin(prop).mesh as BoxMesh).size).is_equal(Vector3(0.8, 0.4, 0.6))
	(
		assert_vector((_shape(prop) as BoxShape3D).size)
		. append_failure_message("the collider kept its own size while the mesh was reshaped")
		. is_equal(Vector3(0.8, 0.4, 0.6))
	)
	prop.size = Vector3(1.0, -2.0, 1.0)  # and live, on a dragged knob
	assert_vector((_skin(prop).mesh as BoxMesh).size).is_equal(Vector3(1, 2, 1))
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


## All three shapes reach the level through ONE door: the level hands out
## the world skin and the flat object id without knowing which shape it has
## — that is the whole point of the solid abstraction. A shape that answered
## only some of it would silently lose its outline or its seams.
func test_every_shape_answers_the_same_solid_door() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var marker := Marker3D.new()
	marker.name = "SpawnPoint"
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
		assert_bool(oid >= 0.0).append_failure_message("%s took no id" % solid.name).is_true()


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
