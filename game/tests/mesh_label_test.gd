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
