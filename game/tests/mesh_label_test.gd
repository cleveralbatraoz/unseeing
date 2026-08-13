extends GdUnitTestSuite
## The spike this suite proves: CUSTOM0 rides an ArrayMesh built through
## gdext, per vertex, format-flagged R_FLOAT. WaveLevel.debug_labelled_box()
## builds a box with one label per face and no vertex shared across two
## faces — catches a build that silently drops the ARRAY_CUSTOM_R_FLOAT
## flag (custom would read as compressed bytes, not a float) or that welds
## a shared corner across two faces (which would blend two labels into
## one instead of keeping them apart).


func test_a_labelled_box_carries_one_label_per_face() -> void:
	var mesh: ArrayMesh = WaveLevel.debug_labelled_box(
		Vector3(2, 3, 0.3), Vector3.ZERO, PackedFloat32Array([0.25, 0.25, 0.34, 0.34, 0.43, 0.43])
	)
	assert_int(mesh.get_surface_count()).is_equal(1)
	var arrays: Array = mesh.surface_get_arrays(0)
	var custom: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM0]
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	# 4 unshared vertices per face, 6 faces — not 8 deduplicated corners
	assert_int(verts.size()).is_equal(24)
	assert_int(custom.size()).is_equal(verts.size())
	var fmt := mesh.surface_get_format(0)
	var shift := Mesh.ARRAY_FORMAT_CUSTOM0_SHIFT
	assert_int((fmt >> shift) & 7).is_equal(Mesh.ARRAY_CUSTOM_R_FLOAT)
	# the -X face owns the first unshared block of 4 (face order -X,+X,-Y,
	# +Y,-Z,+Z) and sits on the x = -1 plane. Filtering by x alone would
	# also catch the -Y/+Y/-Z/+Z faces' own corners at that same box edge
	# — every face meets its neighbours there, unshared vertex or not —
	# so the face is identified by its documented block, not by geometry
	# two other faces legitimately share.
	for i in 4:
		assert_float(verts[i].x).is_equal_approx(-1.0, 1e-5)
		assert_float(custom[i]).is_equal_approx(0.25, 1e-6)


## Godot treats CLOCKWISE triangles as front faces. Every submitted triangle
## must therefore cross opposite its stored outward normal. This
## engine-boundary witness catches mathematically outward/CCW geometry that
## the world's deliberately two-sided material can hide; under the acoustic
## image's cull_back skin it instead culls the intended exterior faces and
## may let far/interior faces win.
func test_a_labelled_box_winds_clockwise_for_godot() -> void:
	var mesh: ArrayMesh = WaveLevel.debug_labelled_box(
		Vector3(2, 3, 0.3), Vector3.ZERO, PackedFloat32Array([0.25, 0.25, 0.34, 0.34, 0.43, 0.43])
	)
	var arrays: Array = mesh.surface_get_arrays(0)
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var normals: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
	var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
	var witnessed := 0
	for triangle in indices.size() / 3:
		var i0: int = indices[triangle * 3]
		var i1: int = indices[triangle * 3 + 1]
		var i2: int = indices[triangle * 3 + 2]
		var cross: Vector3 = (verts[i1] - verts[i0]).cross(verts[i2] - verts[i0])
		if cross.length_squared() > 1e-12:
			assert_float(cross.dot(normals[i0])).is_less(0.0)
			witnessed += 1
	assert_int(witnessed).is_equal(12)


## A box has exactly six faces, and no reading of a five- or seven-entry
## array is "close enough" — it would silently assign some face a label
## meant for another. The guard refuses loudly (checked via the exact
## `assert_error`/`is_push_error` message) and hands back an empty mesh —
## zero surfaces, checked by a second, unwrapped call, since GDScript
## lambdas capture their outer locals BY VALUE: an assignment made inside
## the `assert_error` callable would never reach a variable read after it.
func test_wrong_length_face_labels_is_refused() -> void:
	var five := func() -> void:
		WaveLevel.debug_labelled_box(
			Vector3.ONE, Vector3.ZERO, PackedFloat32Array([0.1, 0.2, 0.3, 0.4, 0.5])
		)
	await (assert_error(five).is_push_error(
		(
			"WaveLevel.debug_labelled_box: face_labels had 5 entries, not the 6 a box's "
			+ "faces need (−X,+X,−Y,+Y,−Z,+Z) — returning an empty mesh rather than "
			+ "guessing which face a wrong-length array meant."
		)
	))
	var five_mesh: ArrayMesh = WaveLevel.debug_labelled_box(
		Vector3.ONE, Vector3.ZERO, PackedFloat32Array([0.1, 0.2, 0.3, 0.4, 0.5])
	)
	assert_int(five_mesh.get_surface_count()).is_equal(0)

	var seven := func() -> void:
		WaveLevel.debug_labelled_box(
			Vector3.ONE, Vector3.ZERO, PackedFloat32Array([0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7])
		)
	await (assert_error(seven).is_push_error(
		(
			"WaveLevel.debug_labelled_box: face_labels had 7 entries, not the 6 a box's "
			+ "faces need (−X,+X,−Y,+Y,−Z,+Z) — returning an empty mesh rather than "
			+ "guessing which face a wrong-length array meant."
		)
	))
	var seven_mesh: ArrayMesh = WaveLevel.debug_labelled_box(
		Vector3.ONE, Vector3.ZERO, PackedFloat32Array([0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7])
	)
	assert_int(seven_mesh.get_surface_count()).is_equal(0)


## THE PER-FRAME REBUILD MUST NOT INHERIT THE LAST FRAME'S LABEL.
## `render::paint` exposes separate doors for two populations that want
## opposite things. A column and a wedge are STATIC solids rebuilt only when
## a designer drags a knob, and must keep the label the level's derive already
## baked — that is
## `resize_outward_triangle_surface_preserving_labels`, pinned in level_test.gd.
## The direct `resize_triangle_surface` door serves the hero's cane/body and
## the cat's whole mesh, rebuilt EVERY frame from a label their builder chose
## this frame. Carrying old CUSTOM0 there would freeze frame one's labels
## forever, because the fixed-resolution tessellations always match in length
## and the carry always fires.
##
## No shipped direct-door caller varies its label today (the cat and hero
## layers each bake one fixed role label), which is exactly why this needs a
## door rather than a node: the trap is silent until the day one does.
func test_a_rebuilt_triangle_surface_takes_the_label_it_was_just_given() -> void:
	var mesh := ArrayMesh.new()
	mesh = WaveLevel.debug_triangle_surface(mesh, 0.25)
	var first: PackedFloat32Array = mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	assert_int(first.size()).is_equal(3)
	for label: float in first:
		assert_float(label).is_equal_approx(0.25, 1e-6)

	# same mesh, same vertex count, a DIFFERENT label
	mesh = WaveLevel.debug_triangle_surface(mesh, 0.63)
	var second: PackedFloat32Array = mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	assert_int(second.size()).is_equal(3)
	for label: float in second:
		assert_float(label).is_equal_approx(0.63, 1e-6)


## The direct unindexed triangle-list door obeys the same clockwise-front-face
## engine convention as the indexed box path. The debug triangle is already
## Godot-clockwise; its stored normal is +Y, so its submitted vertex cross
## product must point -Y without passing through the outward adapter.
func test_a_rebuilt_triangle_surface_winds_clockwise_for_godot() -> void:
	var mesh := WaveLevel.debug_triangle_surface(ArrayMesh.new(), 0.25)
	var arrays: Array = mesh.surface_get_arrays(0)
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var normals: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
	var cross: Vector3 = (verts[1] - verts[0]).cross(verts[2] - verts[0])
	assert_float(cross.dot(normals[0])).is_less(0.0)
