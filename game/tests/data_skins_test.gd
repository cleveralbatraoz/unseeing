extends GdUnitTestSuite
## The data-writing skins, held to their cross-language contracts. The
## sight math lives in three places at once — rust/src/sight.rs (the
## cargo-pinned reference), pulse_pool.gdshaderinc (its GLSL
## transliteration) and the HUM_THROUGH muffle constant shared across the
## renderer — and none of them can see another break.
##
## Most of this suite reads the shader sources as TEXT and holds the
## transliteration to the reference's constants, the way shader_contract_test
## pins the pulse-pool protocol. ONE case,
## test_explain_ray_matches_the_pinned_crossing_counts, is a second, narrower
## idiom: it instantiates the shipped level scene and calls
## WaveObserver.explain_ray on it, pinning what the RUST SIDE believes about
## a handful of sight lines. It reads no shader text, executes no GLSL, and
## proves nothing about the shader — see its own doc comment for exactly
## what it does and does not cover.

const DATA_PASS_PATH := "res://shaders/data_pass.gdshader"
const XRAY_PATH := "res://shaders/data_xray.gdshader"
const CORE_PATH := "res://shaders/data_core.gdshaderinc"
const POOL_PATH := "res://shaders/pulse_pool.gdshaderinc"
const POST_PATH := "res://shaders/hearing_post.gdshader"
const LEVEL_SCENE := preload("res://scenes/level_01.tscn")


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


## `explain_ray`, pinned against hand-derived crossing counts on the REAL
## shipped scene (level_01.tscn, the current 28x28 map, 19 walls) — not
## against sight.rs's own retired_map_rects() cargo fixture, which models a
## retired 20x20/10-wall map that predates PartyEast, RadioRoom, Store and
## Workshop. Every count below was checked against the shipped scene's own
## wall table; it agrees with the retired fixture's cargo tests only
## because these five lines happen to cross nothing but DividerNorth and
## FanRoomSouth, unchanged between the two maps.
##
## What this pins: what RUST BELIEVES about these lines, via WaveObserver
## -> rust/src/nodes/observer.rs -> sight.rs, over the level the game
## actually ships. What it does NOT pin: that the GLSL agrees.
## `explain_ray` never touches the shader, and no case in this gdUnit4
## suite executes GLSL — so a shader-only edit that leaves sight.rs
## untouched (say, pulse_pool.gdshaderinc's slab loop narrowed from
## `k < 3` to `k < 2`, turning every wall's Z bound infinite) would pass
## this case, cargo, and the whole gate, while the picture on screen is
## wrong. `test_pool_slab_test_mirrors_the_rust_reference` above catches
## PART of that risk, as a literal substring pin (it would catch a removed
## birth-wall skip or a changed graze window) but not all of it (an added
## early return, a narrowed loop bound, or the unpinned Z half of
## wall_near would all leave it intact) — see the plan's open item for the
## uncovered list. Until a rendered pixel probe or a checksum-shaped
## shader pin exists, read a pass here as "Rust still believes this", not
## as "the shader still draws this".
func test_explain_ray_matches_the_pinned_crossing_counts() -> void:
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var obs := auto_free(WaveObserver.new()) as WaveObserver
	obs.inject(level, null)
	# spawn to fan head: DividerNorth alone stands between them
	var spawn_to_fan: Dictionary = obs.explain_ray(Vector3(3.0, 0.9, 4.0), Vector3(8.6, 1.15, 4.4))
	assert_int(spawn_to_fan["camera_crossings"]).is_equal(1)
	# a same-room line: nothing stands between two points inside one room
	var same_room: Dictionary = obs.explain_ray(Vector3(8.0, 1.0, 4.0), Vector3(12.0, 1.5, 6.0))
	assert_int(same_room["camera_crossings"]).is_equal(0)
	# the diagonal into the far corridor: the divider and the fan room's
	# south wall, two crossings
	var diagonal: Dictionary = obs.explain_ray(Vector3(3.0, 0.9, 4.0), Vector3(10.0, 0.9, 10.0))
	assert_int(diagonal["camera_crossings"]).is_equal(2)
	# an endpoint landing exactly on the divider's shrunk west face: GRAZE_EPS
	# is what keeps this at zero rather than counting the touch as a crossing
	var graze: Dictionary = obs.explain_ray(Vector3(3.0, 0.9, 4.0), Vector3(6.27, 0.9, 4.0))
	assert_int(graze["camera_crossings"]).is_equal(0)
	# the birth-wall asymmetry: a source standing on the divider centerline
	# lighting an open point skips the wall it is born in (the SOURCE
	# occluder), while the CAMERA occluder still counts the same wall it
	# exits — the one pair of numbers where the two occluders diverge
	var birth_wall: Dictionary = obs.explain_ray(Vector3(6.4, 0.9, 4.0), Vector3(10.0, 0.9, 4.0))
	assert_int(birth_wall["source_crossings"]).is_equal(0)
	assert_int(birth_wall["camera_crossings"]).is_equal(1)
