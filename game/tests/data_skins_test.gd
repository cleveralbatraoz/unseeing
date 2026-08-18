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


## The numeric value of `const <type> NAME = <number>;` in a shader source,
## or NAN when the declaration is missing — NAN fails every numeric assert,
## so a renamed or deleted constant is a failure rather than a skip.
func _shader_float(src: String, const_name: String) -> float:
	var pattern := "const\\s+\\w+\\s+" + const_name + "\\s*=\\s*([0-9.eE+-]+)\\s*;"
	var m := RegEx.create_from_string(pattern).search(src)
	return m.get_string(1).to_float() if m != null else NAN


## The x-ray skin: culled back faces (mandatory under an always-pass depth
## test), the always-on-top depth write, and the standing acoustic image in
## its two independent halves. Both are INSTANCE uniforms, and the world now
## has more than one source, so this is load-bearing rather than stylistic: a
## material uniform would make the quiet fan and the loud radio — which
## share this one skin — brighten and dim as a single object.
func test_xray_skin_carries_the_acoustic_image_contract() -> void:
	var src := _text(XRAY_PATH)
	assert_str(src).contains("render_mode unshaded, cull_back")
	assert_str(src).contains('#include "res://shaders/data_core.gdshaderinc"')
	assert_str(src).contains("DEPTH = source_depth(length(v_world - CAMERA_POSITION_WORLD));")
	assert_str(src).contains("instance uniform float u_source_volume = 0.0;")
	# 0.0, not 1.0: the muffle multiplies the whole acoustic image, so an
	# unpushed instance must fall to the presence floor rather than render
	# the pre-branch picture. THE BREAK this pins is the default drifting
	# back to a value that flatters an unwired source.
	assert_str(src).contains("instance uniform float u_source_muffle = 0.0;")
	# THE ORDER IS THE LAW. The muffle multiplies the WHOLE acoustic image,
	# standing silhouette and washing wave alike, so a wall can take
	# something away. Delivered pre-multiplied into one floor — which is
	# what shipped — it could only compete with reveal_at through a max(),
	# and always lost: a source's hub is unwalled from its own body by
	# construction, so reveal_at reads near 1.0 there whatever stands
	# between that source and the player, and the max handed back the 1.0.
	assert_str(src).contains("return clamp(muffle * max(wave, volume), 0.0, 1.0);")
	assert_str(src).contains(
		"float reveal = source_image(reveal_at(v_world), u_source_volume, u_source_muffle);"
	)
	assert_bool(src.contains("u_source_floor")).is_false()


## THE WHOLE LABEL UNIVERSE IS ONE LADDER, and this holds the Godot side to
## it. Every value that can reach the G channel in a shipped level — the
## floor, the five palette entries every wall and prop is coloured from, the
## cat, the hero's body, the ceiling, the cane — must be a rung of
## render::labels' ladder, and every pair of them must be able to draw a
## seam.
##
## Cargo proves the law over the Rust table; this proves the table the ENGINE
## serves is that same table. They are the same numbers only because one of
## them is computed from the other, and this is the assertion that says so —
## the palette in particular had lived in the level node, where nothing could
## compare it against the creature and viewmodel labels standing either side
## of it in the same band.
func test_every_shipped_label_is_a_rung_of_the_one_ladder() -> void:
	var core: WaveCore = auto_free(WaveCore.new())
	var roles: Dictionary = core.role_labels()
	var palette: PackedFloat64Array = core.world_palette()
	var sep: float = core.min_label_separation()
	assert_int(palette.size()).is_equal(5)

	# the population that can share one rendered frame
	var floor_label: float = roles["Floor"]
	var coexisting := PackedFloat64Array([floor_label])
	coexisting.append_array(palette)
	for name: String in ["Cat", "HeroBody", "Ceiling", "HeroCane"]:
		var label: float = roles[name]
		coexisting.append(label)
	assert_int(coexisting.size()).is_equal(10)

	# ...every pair of which must draw a seam
	for i: int in coexisting.size():
		for j: int in range(i + 1, coexisting.size()):
			var gap: float = absf(coexisting[i] - coexisting[j])
			(
				assert_float(gap)
				. append_failure_message(
					(
						"%s and %s land %s apart, under MIN_SEP %s"
						% [str(coexisting[i]), str(coexisting[j]), str(gap), str(sep)]
					)
				)
				. is_greater_equal(sep)
			)

	# ...and every one is a rung: base 0.15, step 0.09, ten of them, filling
	# the sRGB-safe band to exactly 0.96
	for label: float in coexisting:
		var rung: float = (label - 0.15) / 0.09
		(
			assert_float(absf(rung - roundf(rung)))
			. append_failure_message("%s is not a rung of the ladder" % str(label))
			. is_less(1e-6)
		)
		assert_float(label).is_between(0.15, 0.96)


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
	# each wall's OWN sweep, defaulted to a ZERO-HEIGHT span rather than the
	# shipped (0, 3): a material nobody pushed must lose the barrier law
	# loudly, where a default that happened to be right on this map would
	# ship the bug invisibly
	assert_str(src).contains("uniform vec2 u_wall_y[MAXW];")
	# the UNIFORM is gone; the comment explaining why it went still names it
	assert_bool(src.contains("uniform float u_wall_top")).is_false()


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
	assert_str(src).contains("wall_contains(u_walls[i], from, u_wall_y[i])")
	# an occluder describing no volume stops nothing, and the guard is
	# written !(a <= b) so a NaN lane reads as EMPTY — `a > b` would read it
	# as ordered and hand it to a slab test where t0 > t1 is also false
	assert_str(src).contains("bool wall_empty(vec4 rect, vec2 yspan)")
	assert_str(src).contains("return !(rect.x <= rect.z) || !(rect.y <= rect.w)")
	# ...including the exact cheap refusal in front of it (sight.rs::near),
	# which is what keeps the per-fragment sight loop affordable now that
	# the map holds nineteen walls instead of ten
	# the slab arithmetic now lives in wall_entry and REPORTS its own t0;
	# wall_crosses is a reading of it, so the two cannot disagree about
	# whether a wall is in the way while disagreeing about where it is
	assert_str(src).contains("if (!wall_near(from, to, rect)) { return WALL_MISS; }")
	# THE NEGATION IS LOAD-BEARING and this pin is why it cannot be
	# quietly flattened to `<= 1.0`: the two differ on NaN, and GLSL leaves
	# max/min with a NaN operand implementation-defined. `!(x > 1.0)` reads
	# NaN as BLOCKING, which is the direction settled law 3 needs, because
	# wall_blocked_from calls this to stop waves.
	assert_str(src).contains("return !(wall_entry(from, to, rect, yspan) > 1.0);")
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
	# TWO tests, ORed. The depth read is exact — a fragment in the
	# always-on-top band IS a source — and finally covers a source hidden
	# behind a PROP, which is in no occluder table anywhere. The wall-table
	# inference stays behind it because the depth texture is measured on
	# desktop GL and unmeasured on WebGL2: if it is dead there the first
	# term is false everywhere and the pass degrades to exactly its former
	# behaviour, never to worse.
	assert_str(src).contains("uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;")
	assert_str(_text(POOL_PATH)).contains("bool depth_is_acoustic_image(float depth)")
	assert_str(_text(POOL_PATH)).contains("return depth >= ALWAYS_ON_TOP - SOURCE_BAND;")
	# TWO DIFFERENT QUESTIONS, and the split is the assertion. Conflating
	# them shipped for exactly one commit: feeding the ring cut "is this an
	# acoustic image" instead of "is something in front of it" drops player
	# rings over EVERY source pixel in the game, including a fan standing in
	# the open, because every source fragment is an acoustic image by
	# definition. A depth read cannot answer the ring cut's question at all —
	# at an x-rayed pixel the depth buffer holds the SOURCE's faked
	# always-on-top value, not the occluder's.
	assert_str(src).contains("float wall_t = wall_first_entry(cam, seen_pt);")
	assert_str(src).contains("bool seen_walled = !(wall_t > 1.0);")
	assert_str(src).contains(
		"bool seen_image = depth_is_acoustic_image(texture(depth_tex, uv).r) || seen_walled;"
	)
	# A CUT IS A DISTANCE, NEVER A FLAG. The OR shipped and was a defect:
	# a boolean is fragment-constant, so it killed every root at the pixel
	# including rings physically NEARER than the wall that set it — and
	# because an x-rayed source's skin takes the pixel from the wall behind
	# it, that flag was true across the source's whole silhouette. A source
	# seen through a wall punched a source-shaped hole in the hero's own air.
	assert_str(src).contains("if (t >= air_d) { continue; }")
	# the NEGATIVE half, which is what catches the regression rewritten in
	# any other spelling: no OR may return to that compare, because any
	# boolean folded in there is fragment-constant by construction
	assert_str(src).not_contains("t >= scene_d ||")
	# and the NaN barrier, which Rust does not need and GLSL does
	assert_str(src).contains(
		"float air_d = seen_walled ? (wall_t >= 0.0 ? wall_t * scene_d : 0.0) : scene_d;"
	)
	assert_str(src).contains("if (seen_image) { src_r = c_c.r; }")
	assert_str(src).contains("if (src_r >= 0.0) { reveal = min(reveal, src_r); }")


## The depth hack lives once, in the core, and is a FRAGMENT depth write
## (the near plane's window depth ~1.0 under reversed-Z: GEQUAL passes
## everywhere, draw order layers the image). Never a vertex POSITION
## override — that would defeat near-plane clipping, and geometry crossing
## the camera plane would rasterize as full-screen projective sheets.
func test_core_defines_the_depth_hack_once_as_fragment_depth() -> void:
	# the band constants live in the POOL include now: data_xray writes the
	# always-on-top depth and hearing_post reads it back to identify a
	# source, so it is a protocol two shaders share
	assert_str(_text(POOL_PATH)).contains("const float ALWAYS_ON_TOP = 0.999999;")
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
	# the band moved into the POOL include, which is where a protocol two
	# shaders share belongs: data_xray writes it and hearing_post reads it
	assert_str(_text(POOL_PATH)).contains("const float SOURCE_BAND = 1.0e-3;")
	assert_str(core).contains(
		"return ALWAYS_ON_TOP - SOURCE_BAND * clamp(dist / DIST_PACK_RANGE, 0.0, 1.0);"
	)
	# The width is DERIVED, and this holds the shipped GLSL literal against
	# the derivation rather than against itself. What stood here before was
	# `assert_bool(1.0e-5 < 1.0 - 0.999999 + 1.0e-5)`, which reduces to
	# `x < 1e-6 + x` — true for every x, so it measured nothing at all and
	# passed while the band was a hundred times too narrow to order a fan's
	# own blades against its own housing.
	var band: WaveCore = auto_free(WaveCore.new())
	assert_float(_shader_float(_text(POOL_PATH), "SOURCE_BAND")).is_equal(band.source_band())
	# and both bounds the derivation rests on, read back from Rust:
	# coarse enough to resolve the tightest gap a shipped source has
	assert_float(band.source_band_resolution()).is_less(band.min_source_limb_gap())
	# and still out of reach of any world fragment past the near plane
	assert_float(band.deepest_world_fragment_in_band()).is_less(band.camera_near() + 0.001)
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
	# an endpoint landing exactly on the wall's shrunk west face, 6.4 -
	# (WALL_T - RECT_SHRINK) = 6.28: GRAZE_EPS is what keeps this at zero
	# rather than counting the touch as a crossing. The literal has to track
	# the shrink -- a centimetre off the face and the line simply misses the
	# rect, and this reads zero for a reason that has nothing to do with the
	# window it is named for.
	var graze: Dictionary = obs.explain_ray(Vector3(3.0, 0.9, 4.0), Vector3(6.28, 0.9, 4.0))
	assert_int(graze["camera_crossings"]).is_equal(0)
	# the birth-wall asymmetry: a source standing on the wall's own
	# centerline lighting an open point skips the wall it is born in (the
	# SOURCE occluder), while the CAMERA occluder still counts the same
	# wall it exits — the one pair of numbers where the two occluders diverge
	var birth_wall: Dictionary = obs.explain_ray(Vector3(6.4, 0.9, 4.0), Vector3(10.0, 0.9, 4.0))
	assert_int(birth_wall["source_crossings"]).is_equal(0)
	assert_int(birth_wall["camera_crossings"]).is_equal(1)
