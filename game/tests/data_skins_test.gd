extends GdUnitTestSuite
## The data-writing skins, held to their cross-language contracts. The
## sight math lives in three places at once — rust/src/sight.rs (the
## cargo-pinned reference), pulse_pool.gdshaderinc (its GLSL
## transliteration) and the HUM_THROUGH muffle constant shared across the
## renderer — and none of them can see another break. This suite reads the
## shader sources as TEXT and holds the transliteration to the reference's
## constants, the way shader_contract_test pins the pulse-pool protocol.

const DATA_PASS_PATH := "res://shaders/data_pass.gdshader"
const XRAY_PATH := "res://shaders/data_xray.gdshader"
const CORE_PATH := "res://shaders/data_core.gdshaderinc"
const POOL_PATH := "res://shaders/pulse_pool.gdshaderinc"


func _text(path: String) -> String:
	var f := FileAccess.open(path, FileAccess.READ)
	assert_object(f).is_not_null()
	return f.get_as_text() if f != null else ""


## The x-ray skin: culled back faces (mandatory under an always-pass depth
## test), the always-on-top depth write, and the per-object source muffle
## on its standing floor — the acoustic image's own contract. It shares the
## reveal loop, the object id and the wall table through the data core; only
## the faked depth and the muffled floor are its own.
func test_xray_skin_carries_the_acoustic_image_contract() -> void:
	var src := _text(XRAY_PATH)
	assert_str(src).contains("render_mode unshaded, cull_back")
	assert_str(src).contains('#include "res://shaders/data_core.gdshaderinc"')
	assert_str(src).contains("DEPTH = ALWAYS_ON_TOP;")
	assert_str(src).contains("uniform float u_source_muffle = 1.0;")
	assert_str(src).contains("u_base * u_source_muffle")


## The shared pulse-pool include carries the wall table every occluding
## skin reads: the slots sized MAXW=16 and the uniforms the level fills.
func test_pool_carries_the_shared_wall_table() -> void:
	var src := _text(POOL_PATH)
	assert_str(src).contains("const int MAXW = 16;")
	assert_str(src).contains("uniform int u_wall_count = 0;")
	assert_str(src).contains("uniform vec4 u_walls[MAXW];")
	assert_str(src).contains("uniform float u_wall_top = 3.0;")


## The GLSL slab test is a literal transliteration of sight.rs, pinned in
## the pulse-pool include: the same graze window (t strictly inside
## 0.001..0.999), the same axis-parallel degeneration, and the birth-wall
## skip of the SOURCE occluder (crossings_from) — a "harmless" rewrite of
## either side trips the contract.
func test_pool_slab_test_mirrors_the_rust_reference() -> void:
	var src := _text(POOL_PATH)
	assert_str(src).contains("float t0 = 0.001;")
	assert_str(src).contains("float t1 = 0.999;")
	assert_str(src).contains("abs(d[k]) < 1e-6")
	assert_str(src).contains("t0 = max(t0, min(ta, tb));")
	assert_str(src).contains("t1 = min(t1, max(ta, tb));")
	assert_str(src).contains("wall_contains(u_walls[i], from, u_wall_top)")


## One muffle vocabulary: HUM_THROUGH lives once in the pulse-pool include,
## and every wave-borne dim reads it — the surface reveal (data_core) most
## of all. No literal 0.55 drifts out of step across the renderer.
func test_hum_through_is_one_shared_constant() -> void:
	assert_str(_text(POOL_PATH)).contains("const float HUM_THROUGH = 0.55;")
	assert_str(_text(CORE_PATH)).contains("pow(HUM_THROUGH, float(blocked))")


## The depth hack lives once, in the core, and is a FRAGMENT depth write
## (the near plane's window depth ~1.0 under reversed-Z: GEQUAL passes
## everywhere, draw order layers the image). Never a vertex POSITION
## override — that would defeat near-plane clipping, and geometry crossing
## the camera plane would rasterize as full-screen projective sheets.
func test_core_defines_the_depth_hack_once_as_fragment_depth() -> void:
	assert_str(_text(CORE_PATH)).contains("const float ALWAYS_ON_TOP = 0.999999;")
	for path: String in [DATA_PASS_PATH, XRAY_PATH]:
		assert_bool(_text(path).contains("0.999999")).is_false()
		assert_bool(_text(path).contains("POSITION =")).is_false()
