extends GdUnitTestSuite
## The data-writing skins, held to their cross-language contracts. The
## sight math lives in two places at once — rust/src/sight.rs (the
## cargo-pinned reference) and pulse_pool.gdshaderinc (its GLSL
## transliteration) — and neither can see the other break. A wall is now an
## absolute barrier for every wave, so no shader speaks a muffle vocabulary
## at all.
##
## Most of this suite reads the shader sources as TEXT and holds the
## transliteration to the reference's constants, the way shader_contract_test
## pins the pulse-pool protocol. ONE case,
## test_explain_ray_matches_the_pinned_crossing_counts, is a second, narrower
## idiom: it instantiates a one-wall level built in code and calls
## WaveObserver.explain_ray on it, pinning what the RUST SIDE believes about
## a handful of sight lines. It reads no shader text, executes no GLSL, and
## proves nothing about the shader — see its own doc comment for exactly
## what it does and does not cover.

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
## skip of the SOURCE occluder (blocked_from) — a "harmless" rewrite of
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


## The muffle vocabulary is gone outright: a wall stops a wave dead, whether
## the wave reveals geometry (data_core) or carries a shell through the air
## (hearing_post's player rings and a standing source's hum alike).
##
## Absence of the identifier `HUM_THROUGH` is the WEAK half of this and is
## kept only to catch a literal revert; on its own it is satisfied by the
## same constant under any other name. The load-bearing half is that BOTH
## wave readers reach the wall table through the one kind-free predicate,
## and that neither multiplies its result by anything: a surviving fraction
## has nowhere to live once the only wall answer is a bool.
func test_no_shader_lets_a_wave_through_a_wall() -> void:
	for path: String in [POOL_PATH, CORE_PATH, POST_PATH]:
		(
			assert_bool(_text(path).contains("HUM_THROUGH"))
			. append_failure_message(
				"%s still speaks the muffle vocabulary; a wall stops a wave outright" % path
			)
			. is_false()
		)
	# the wall answer is a BOOL in both languages, so there is no fraction
	# to raise to a power and no exponent to tune
	assert_str(_text(POOL_PATH)).contains("bool wall_blocked_from(vec3 from, vec3 to)")
	assert_str(_text(CORE_PATH)).contains("wall_blocked_from(src, world) ? 0.0 : 1.0")
	assert_str(_text(POST_PATH)).contains("if (wall_blocked_from(u_ppos[i], hp)) { continue; }")


## The CAMERA half of the hearing pass's occlusion — a different law from
## the wall barrier, and this case covers only this half (the barrier is
## test_no_shader_lets_a_wave_through_a_wall above, and
## shader_contract_test.gd's shell case).
##
## No ring washes an x-ray surface seen through a wall: the pass tests once
## per pixel whether the visible surface lies behind a wall (any
## always-on-top source) and drops shells there — depth alone can't, since a
## source corrupts it at its own pixels. And a source's OUTLINE is gated by
## its OWN dim reveal on BOTH sides of its silhouette (its pixels AND the
## wall pixels touching it), never by the lit wall behind it — so tapping
## that wall can't flare its edge.
##
## NOTE the depth/seen_walled line below is byte-identical to what shipped
## before the barrier law, so it discriminates nothing about that law; it
## is here for the camera half only.
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


## The reveal loop's EXACT early-out. The accumulator is a max and
## source_reveal_vis only ever returns a value in [0, 1], so the bound
## computed with that factor at its maximum is an upper bound on the pulse's
## contribution: a pulse that cannot beat what is already accumulated is
## dropped BEFORE the per-fragment wall walk instead of after it. No pixel
## differs; only the cost.
##
## It is NOT "the loop's affordability", and this docstring used to say it
## was. The barrier campaign made source_reveal_vis a 0/1 gate, and against a
## `reveal` still sitting at exactly 0.0 — which is every fragment that every
## in-range pulse is walled off from — `bound <= reveal` can never fire. The
## fragments that pay the full walk are precisely the ones in the room next
## door. What bounds this loop is the radius gate and the death gate above it.
##
## `min(flare, 1.0)` is gone from the expression because pulse_flare returns
## an already-clamped value: the clamp moved into the law
## (rust/src/render/reveal.rs::flare) so the cargo-pinned reference and the
## rendered number are the same one, rather than the reference describing a
## shape the shader then clamped on its own.
func test_reveal_loop_bounds_a_pulse_before_walking_the_walls() -> void:
	var src := _text(CORE_PATH)
	assert_str(src).contains("float bound = flare * atten * cone * gain * peak;")
	assert_str(src).contains("if (bound <= reveal) { continue; }")
	assert_str(src).contains("bound * source_reveal_vis(u_ppos[i], world)")
	# the bound must be formed BEFORE the wall walk, or it buys nothing
	assert_bool(src.find("float bound =") < src.find("bound * source_reveal_vis")).is_true()
	# and the flare it multiplies must be the pinned law's own clamped
	# output, not a raw shape this line then bounds for itself
	assert_str(src).contains("float flare = pulse_flare(ga, tail);")


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


## One wall, built in code rather than pinned to level_01.tscn's own
## layout, which is free to change census underneath this law. The
## centerline (6.4, 0.6)-(6.4, 8.0) and every point below are lifted
## unchanged from sight.rs's own cargo fixtures —
## `endpoint_grazes_are_not_crossings` for the graze case and
## `source_is_not_blocked_by_the_wall_it_is_born_in` for the birth-wall
## one — so the geometry each line proves is already hand-checked on the
## Rust side; this only pins that the boundary carries the same verdict.
func _one_wall_level() -> WaveLevel:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var marker := WaveSpawn.new()
	level.add_child(marker)
	var wall := WaveWall.new()
	wall.name = "TheWall"
	wall.length = 7.4
	wall.position = Vector3(6.4, 0, 4.3)
	wall.rotation.y = PI * 0.5  # a z-run wall at x = 6.4, spanning z 0.6..8.0
	level.add_child(wall)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	return level


## `explain_ray`, pinned against hand-derived crossing counts on a one-wall
## level built in code — not the shipped scene, whose census is free to
## change underneath this law, and not sight.rs's own retired_map_rects()
## cargo fixture either, kept a deliberate duplicate there for its own
## reason (see that module's doc comment).
##
## What this pins: what RUST BELIEVES about these lines, via WaveObserver
## -> rust/src/nodes/observer.rs -> sight.rs. What it does NOT pin: that
## the GLSL agrees. `explain_ray` never touches the shader, and no case in
## this gdUnit4 suite executes GLSL — so a shader-only edit that leaves
## sight.rs untouched (say, pulse_pool.gdshaderinc's slab loop narrowed
## from `k < 3` to `k < 2`, turning every wall's Z bound infinite) would
## pass this case, cargo, and the whole gate, while the picture on screen
## is wrong. `test_pool_slab_test_mirrors_the_rust_reference` above catches
## PART of that risk, as a literal substring pin (it would catch a removed
## birth-wall skip or a changed graze window) but not all of it (an added
## early return, a narrowed loop bound, or the unpinned Z half of
## wall_near would all leave it intact) — see the plan's open item for the
## uncovered list. Until a rendered pixel probe or a checksum-shaped
## shader pin exists, read a pass here as "Rust still believes this", not
## as "the shader still draws this".
func test_explain_ray_matches_the_pinned_crossing_counts() -> void:
	var level := _one_wall_level()
	var obs := auto_free(WaveObserver.new()) as WaveObserver
	obs.inject(level, null)
	# a straight line through the wall, born well clear of it: one crossing
	# either occluder counts the same way
	var through: Dictionary = obs.explain_ray(Vector3(3.0, 0.9, 4.0), Vector3(10.0, 0.9, 4.0))
	assert_int(through["camera_crossings"]).is_equal(1)
	assert_int(through["source_crossings"]).is_equal(1)
	# an endpoint landing exactly on the wall's shrunk west face: GRAZE_EPS
	# is what keeps this at zero rather than counting the touch as a crossing
	var graze: Dictionary = obs.explain_ray(Vector3(3.0, 0.9, 4.0), Vector3(6.27, 0.9, 4.0))
	assert_int(graze["camera_crossings"]).is_equal(0)
	# the birth-wall asymmetry: a source standing on the wall's own
	# centerline lighting an open point skips the wall it is born in (the
	# SOURCE occluder), while the CAMERA occluder still counts the same
	# wall it exits — the one pair of numbers where the two occluders diverge
	var birth_wall: Dictionary = obs.explain_ray(Vector3(6.4, 0.9, 4.0), Vector3(10.0, 0.9, 4.0))
	assert_int(birth_wall["source_crossings"]).is_equal(0)
	assert_int(birth_wall["camera_crossings"]).is_equal(1)
