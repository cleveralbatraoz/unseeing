extends GdUnitTestSuite
## The pulse-pool protocol lives in two languages at once: pulses.gd packs
## the slots, pulse_pool.gdshaderinc decodes them. Neither side can see the
## other break, so this suite reads the shader include as TEXT and holds its
## constants and decode expressions against the GDScript they must mirror.

const INC_PATH := "res://shaders/pulse_pool.gdshaderinc"
const DATA_PASS_PATH := "res://shaders/data_pass.gdshader"
const CORE_PATH := "res://shaders/data_core.gdshaderinc"
const LEVEL_SCENE := preload("res://scenes/level_01.tscn")


func _read(path: String) -> String:
	var f := FileAccess.open(path, FileAccess.READ)
	assert_object(f).is_not_null()
	return f.get_as_text() if f != null else ""


func _include_text() -> String:
	return _read(INC_PATH)


## The G channel carries a per-object id, packed in the shared data CORE
## now (every data-writing skin — the world and the always-on-top acoustic
## image — reads the same machinery): pack_data declares a per-instance
## u_oid and writes it into G when set, falling back to the normal-id
## crease encoding when unset (u_oid < 0). The outline post-pass diffs G,
## so one flat id per object draws one unified silhouette with no interior
## component seams. Pinned as source text so a "harmless" rewrite cannot
## silently revert the whole game's outline style, and the data pass must
## include the core rather than carry its own copy.
func test_data_core_writes_object_id_into_g() -> void:
	var core := _read(CORE_PATH)
	assert_str(core).contains("instance uniform float u_oid")
	assert_str(core).contains("u_oid >= 0.0")
	assert_str(_read(DATA_PASS_PATH)).contains("data_core.gdshaderinc")


## The numeric value of `const <type> NAME = <number>;` in the include, or
## NAN when the declaration is missing — NAN fails every numeric assert.
func _shader_const(const_name: String) -> float:
	var pattern := "const\\s+\\w+\\s+" + const_name + "\\s*=\\s*([0-9.]+)\\s*;"
	var m := RegEx.create_from_string(pattern).search(_include_text())
	return m.get_string(1).to_float() if m != null else NAN


## One number, THREE homes: the Rust core owns it (rust/src/pulse_pool.rs,
## MAXP), the GDScript shim mirrors it as Pulses.MAXP, and the include pins
## it for both shaders' uniform arrays. This assertion holds the include
## against the shim; a drift in the core itself is caught by pulses_test's
## eviction suite, which counts real slots.
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
