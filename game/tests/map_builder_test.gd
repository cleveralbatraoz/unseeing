extends GdUnitTestSuite
## The map builder's box math trusts axis alignment. Ported 1:1 from the
## retired custom runner.


func test_map_segments_axis_aligned() -> void:
	for s: Vector4 in MapBuilder.SEGS:
		var axis_aligned := absf(s.w - s.y) < 0.001 or absf(s.z - s.x) < 0.001
		assert_bool(axis_aligned).is_true()


func test_map_border_walls_present() -> void:
	assert_int(MapBuilder.SEGS.size()).is_greater_equal(4)
