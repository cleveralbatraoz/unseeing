extends GdUnitTestSuite
## The map builder's laws: box math trusts axis alignment, and every placement
## in LevelData must agree with the wall centerlines it was derived from.


func test_map_segments_axis_aligned() -> void:
	for s: Vector4 in MapBuilder.SEGS:
		var axis_aligned := absf(s.w - s.y) < 0.001 or absf(s.z - s.x) < 0.001
		assert_bool(axis_aligned).is_true()


func test_map_border_walls_present() -> void:
	assert_int(MapBuilder.SEGS.size()).is_greater_equal(4)


## The hum-room rect clips what the fan's waves may reveal. Each of its four
## edges must be collinear with (and overlapped by) a real wall centerline —
## an edge with no wall on it would clip sound in open air.
func test_hum_room_edges_lie_on_walls() -> void:
	var room := MapBuilder.level().hum_room
	var edges: Array[Vector4] = [
		Vector4(room.x, room.y, room.x, room.w),  # west
		Vector4(room.z, room.y, room.z, room.w),  # east
		Vector4(room.x, room.y, room.z, room.y),  # north
		Vector4(room.x, room.w, room.z, room.w),  # south
	]
	for edge: Vector4 in edges:
		assert_bool(_edge_on_some_wall(edge)).append_failure_message("edge %s" % edge).is_true()


func test_fan_spawn_inside_hum_room() -> void:
	var lvl := MapBuilder.level()
	var room := lvl.hum_room
	assert_bool(lvl.fan_spawn.x > room.x and lvl.fan_spawn.x < room.z).is_true()
	assert_bool(lvl.fan_spawn.z > room.y and lvl.fan_spawn.z < room.w).is_true()


## The demo tap must land on the west hum-room wall (x = 6.4), inside a real
## segment's span, striking toward the spawn side (-X).
func test_demo_tap_sits_on_west_hum_wall() -> void:
	var lvl := MapBuilder.level()
	assert_float(lvl.demo_tap.x).is_equal_approx(6.4, 0.001)
	var on_wall := false
	for s: Vector4 in MapBuilder.SEGS:
		if absf(s.x - 6.4) < 0.001 and absf(s.z - 6.4) < 0.001:
			if lvl.demo_tap.z >= minf(s.y, s.w) and lvl.demo_tap.z <= maxf(s.y, s.w):
				on_wall = true
	assert_bool(on_wall).is_true()
	assert_vector(lvl.demo_tap_normal).is_equal(Vector3(-1, 0, 0))


func test_spawn_inside_map_bounds() -> void:
	var lvl := MapBuilder.level()
	var lo := Vector2(INF, INF)
	var hi := Vector2(-INF, -INF)
	for s: Vector4 in MapBuilder.SEGS:
		lo = Vector2(minf(lo.x, minf(s.x, s.z)), minf(lo.y, minf(s.y, s.w)))
		hi = Vector2(maxf(hi.x, maxf(s.x, s.z)), maxf(hi.y, maxf(s.y, s.w)))
	assert_bool(lvl.spawn_pos.x > lo.x and lvl.spawn_pos.x < hi.x).is_true()
	assert_bool(lvl.spawn_pos.z > lo.y and lvl.spawn_pos.z < hi.y).is_true()


## True when the edge (x1, z1, x2, z2) lies on some SEGS centerline: same
## axis, same coordinate, spans overlapping.
func _edge_on_some_wall(edge: Vector4) -> bool:
	var vertical := absf(edge.z - edge.x) < 0.001
	for s: Vector4 in MapBuilder.SEGS:
		if vertical:
			if absf(s.x - edge.x) < 0.001 and absf(s.z - edge.x) < 0.001:
				if minf(s.y, s.w) <= edge.w and maxf(s.y, s.w) >= edge.y:
					return true
		elif absf(s.y - edge.y) < 0.001 and absf(s.w - edge.y) < 0.001:
			if minf(s.x, s.z) <= edge.z and maxf(s.x, s.z) >= edge.x:
				return true
	return false
