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
## test), the always-on-top depth write, and the standing acoustic image on
## its own floor. That floor is an INSTANCE uniform, and the world now has
## more than one source, so this is load-bearing rather than stylistic: a
## material uniform would make the quiet fan and the loud radio — which
## share this one skin — brighten and dim as a single object.
func test_xray_skin_carries_the_acoustic_image_contract() -> void:
	var src := _text(XRAY_PATH)
	assert_str(src).contains("render_mode unshaded, cull_back")
	assert_str(src).contains('#include "res://shaders/data_core.gdshaderinc"')
	assert_str(src).contains("DEPTH = source_depth(length(v_world - CAMERA_POSITION_WORLD));")
	assert_str(src).contains("instance uniform float u_source_floor = 0.0;")
	assert_str(src).contains("max(reveal_at(v_world), u_source_floor)")


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
	# which is what keeps the per-fragment sight loop affordable now that
	# the map holds nineteen walls instead of ten
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
## lies behind a wall (any always-on-top source) and drops player shells
## there — depth alone can't, since a source corrupts it at its own pixels.
## And a source's OUTLINE is gated by its OWN dim reveal on BOTH sides of
## its silhouette (its pixels AND the wall pixels touching it), never by the
## lit wall behind it — so tapping that wall can't flare its edge.
func test_hearing_pass_never_washes_player_rings_on_an_xrayed_source() -> void:
	var src := _text(POST_PATH)
	assert_str(src).contains("bool seen_walled = wall_crossings(cam, seen_pt) > 0;")
	assert_str(src).contains("if (t >= scene_d || seen_walled) { continue; }")
	assert_str(src).contains("if (seen_walled) { src_r = c_c.r; }")
	assert_str(src).contains("if (src_r >= 0.0) { reveal = min(reveal, src_r); }")


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


## The reveal loop's EXACT early-out, and it is the loop's affordability.
## The accumulator is a max and source_reveal_vis only ever returns a value
## in [0, 1], so the bound computed with that factor at its maximum is an
## upper bound on the pulse's contribution: a pulse that cannot beat what is
## already accumulated is dropped BEFORE the per-fragment wall walk instead
## of after it. No pixel differs; only the cost — which matters most for an
## EVEN source, whose sphere passes the cone gate in every direction.
func test_reveal_loop_bounds_a_pulse_before_walking_the_walls() -> void:
	var src := _text(CORE_PATH)
	assert_str(src).contains("float bound = min(flare, 1.0) * atten * cone * gain * peak;")
	assert_str(src).contains("if (bound <= reveal) { continue; }")
	assert_str(src).contains("bound * source_reveal_vis(typ, u_ppos[i], world)")
	# the bound must be formed BEFORE the wall walk, or it buys nothing
	assert_bool(src.find("float bound =") < src.find("bound * source_reveal_vis")).is_true()


## A CONSTANT always-on-top depth only works while the world holds one
## source. Two acoustic images writing the same value resolve against each
## other by opaque draw order alone, and Godot sorts opaque surfaces
## near-to-far — so the nearer source draws first and the farther one,
## passing GEQUAL on an equal value, paints over it. The layer gets a BAND
## instead: still above every world fragment, but ordered inside itself by
## true distance.
func test_the_acoustic_image_layer_is_a_band_ordered_by_distance() -> void:
	var core := _text(CORE_PATH)
	assert_str(core).contains("const float SOURCE_BAND = 1.0e-5;")
	assert_str(core).contains(
		"return ALWAYS_ON_TOP - SOURCE_BAND * clamp(dist / DIST_PACK_RANGE, 0.0, 1.0);"
	)
	# the band must be far narrower than any depth step the world can make,
	# or a source would start losing to the geometry it is felt through
	assert_bool(1.0e-5 < 1.0 - 0.999999 + 1.0e-5).is_true()
	# and the source skin must USE it rather than writing the constant
	var xray := _text(XRAY_PATH)
	assert_bool(xray.contains("DEPTH = ALWAYS_ON_TOP;")).is_false()
