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
const POST_PATH := "res://shaders/hearing_post.gdshader"


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
## skin reads: the slots sized MAXW and the uniforms the level fills. The
## count must equal the Rust reference's, because the level truncates its
## wall table to sight::MAXW before pushing it — a GLSL array smaller than
## that would read past its own end.
func test_pool_carries_the_shared_wall_table() -> void:
	var src := _text(POOL_PATH)
	assert_str(src).contains("const int MAXW = 32;")
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
	# ...including the exact cheap refusal in front of it (sight.rs::near),
	# which is what keeps the per-fragment sight loop affordable as a level
	# grows more walls
	assert_str(src).contains("if (!wall_near(from, to, rect)) { return false; }")
	assert_str(src).contains("min(from.x, to.x) <= rect.z && max(from.x, to.x) >= rect.x")


## One muffle vocabulary: HUM_THROUGH lives once in the pulse-pool include,
## and every wave-borne dim reads it — the surface reveal (data_core) and
## the hum shell (hearing_post). No literal 0.55 drifts out of step.
func test_hum_through_is_one_shared_constant() -> void:
	assert_str(_text(POOL_PATH)).contains("const float HUM_THROUGH = 0.55;")
	assert_str(_text(CORE_PATH)).contains("pow(HUM_THROUGH, float(blocked))")
	assert_str(_text(POST_PATH)).contains("mute = HUM_THROUGH;")


## The hearing pass never washes a player-made ring over an x-ray surface
## seen through a wall: it tests once per pixel whether the visible surface
## lies behind a wall (the always-on-top fan) and drops player shells
## there — depth alone can't, since the fan corrupts it at its own pixels.
## And the fan's OUTLINE is gated by its OWN dim reveal on BOTH sides of its
## silhouette (its pixels AND the wall pixels touching it), never by the lit
## wall behind it — so tapping that wall can't flare its edge.
func test_hearing_pass_never_washes_player_rings_on_the_xray_fan() -> void:
	var src := _text(POST_PATH)
	assert_str(src).contains("bool seen_walled = wall_crossings(cam, seen_pt) > 0;")
	assert_str(src).contains("if (t >= scene_d || seen_walled) { continue; }")
	assert_str(src).contains("if (seen_walled) { fan_r = c_c.r; }")
	assert_str(src).contains("if (fan_r >= 0.0) { reveal = min(reveal, fan_r); }")


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
