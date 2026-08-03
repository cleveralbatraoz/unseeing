extends GdUnitTestSuite
## The pulse-pool protocol lives in two languages at once: pulses.gd packs
## the slots, pulse_pool.gdshaderinc decodes them. Neither side can see the
## other break, so this suite reads the shader include as TEXT and holds its
## constants and decode expressions against the GDScript they must mirror.

const INC_PATH := "res://shaders/pulse_pool.gdshaderinc"
const LEVEL_SCENE := preload("res://scenes/level_01.tscn")


func _include_text() -> String:
	var f := FileAccess.open(INC_PATH, FileAccess.READ)
	assert_object(f).is_not_null()
	return f.get_as_text() if f != null else ""


## The numeric value of `const <type> NAME = <number>;` in the include, or
## NAN when the declaration is missing — NAN fails every numeric assert.
func _shader_const(const_name: String) -> float:
	var pattern := "const\\s+\\w+\\s+" + const_name + "\\s*=\\s*([0-9.]+)\\s*;"
	var m := RegEx.create_from_string(pattern).search(_include_text())
	return m.get_string(1).to_float() if m != null else NAN


## Both shaders size their uniform arrays from the include's MAXP; the CPU
## pool sizes its packed arrays from Pulses.MAXP. One number, two homes.
func test_maxp_matches_the_pool() -> void:
	assert_float(_shader_const("MAXP")).is_equal(float(Pulses.MAXP))


## emit() packs dat.w as type * 10 + gain * 9; the shaders must decode with
## exactly floor(w / 10) and mod(w, 10) / 9 — pinned as literal source text,
## so a "harmless" rewrite of either expression trips the contract.
func test_decode_expressions_are_literal() -> void:
	var src := _include_text()
	assert_str(src).contains("floor(d.w / 10.0)")
	assert_str(src).contains("mod(d.w, 10.0) / 9.0")


## Camera distance is packed into one color channel divided by
## DIST_PACK_RANGE; any visible point packing above 1.0 would alias. The
## range must therefore exceed the longest sight line the map allows: the
## full 3D diagonal of the wall-centerline extents, floor to ceiling —
## derived from the shipped level scene, the one map that ever renders.
func test_dist_pack_range_covers_the_map_diagonal() -> void:
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var lo := Vector2(INF, INF)
	var hi := Vector2(-INF, -INF)
	for s: Vector4 in level.wall_segments():
		lo = Vector2(minf(lo.x, minf(s.x, s.z)), minf(lo.y, minf(s.y, s.w)))
		hi = Vector2(maxf(hi.x, maxf(s.x, s.z)), maxf(hi.y, maxf(s.y, s.w)))
	var diagonal := Vector3(hi.x - lo.x, WaveLevel.wall_height(), hi.y - lo.y).length()
	assert_float(_shader_const("DIST_PACK_RANGE")).is_greater(diagonal)
