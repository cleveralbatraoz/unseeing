extends GdUnitTestSuite
## The shipped pulse-pool protocol crosses Rust and GLSL:
## rust/src/pulse_pool.rs packs slots and pulse_pool.gdshaderinc decodes
## them. The test-only Pulses shim mirrors that layout for gdUnit. Neither
## side detects the other drifting on its own, so this suite reads the shader
## include as TEXT and holds its constants and decode expressions against
## their shared contract.

const INC_PATH := "res://shaders/pulse_pool.gdshaderinc"
const DATA_PASS_PATH := "res://shaders/data_pass.gdshader"
const XRAY_PATH := "res://shaders/data_xray.gdshader"
const CORE_PATH := "res://shaders/data_core.gdshaderinc"
const HEARING_POST_PATH := "res://shaders/hearing_post.gdshader"
const LEVEL_SCENE := preload("res://scenes/level_01.tscn")


func _read(path: String) -> String:
	var f := FileAccess.open(path, FileAccess.READ)
	assert_object(f).is_not_null()
	return f.get_as_text() if f != null else ""


func _include_text() -> String:
	return _read(INC_PATH)


## Every shader/include file directly under res://shaders, walked rather
## than named one by one — a file added later (a 6th skin, a new include)
## is covered by construction instead of silently joining the "checked only
## 3 of 5" gap a hand-typed list leaves.
func _all_shader_files() -> Array[String]:
	var files: Array[String] = []
	var dir := DirAccess.open("res://shaders")
	assert_object(dir).is_not_null()
	dir.list_dir_begin()
	var entry := dir.get_next()
	while entry != "":
		if (
			not dir.current_is_dir()
			and (entry.ends_with(".gdshader") or entry.ends_with(".gdshaderinc"))
		):
			files.append("res://shaders/%s" % entry)
		entry = dir.get_next()
	dir.list_dir_end()
	return files


## The G channel carries a per-VERTEX face-or-role label, packed in the
## shared data CORE (every data-writing skin — the world and the
## always-on-top acoustic image — reads the same machinery). WaveLevel's
## derive-time paint pass (rust/src/render/paint.rs) assigns per-face
## superface labels to world solids; source, creature and viewmodel builders
## bake fixed role labels directly. Both skins' vertex() stages carry either
## kind through as v_label, and pack_data writes it straight into G. The
## outline post-pass diffs G, so two overlapping world solids sharing a
## label bit-for-bit melt into one silhouette while fixed-role meshes retain
## their intended creases. The OLD per-instance u_oid uniform and its
## normal-derived fallback are gone outright — no shader in the tree may
## declare or read u_oid anywhere, since a per-object override could once
## again let one wrong instance corrupt what the geometry itself already
## says. Pinned as source text so a "harmless" rewrite cannot silently
## revert the whole game's outline style, and the data pass must include the
## core rather than carry its own copy.
##
## The signature alone is not the packing: `pack_data(float reveal, vec3
## world, vec3 cam)` and `varying float v_label` both stay true even if the
## BODY quietly returned a literal in G's slot instead of v_label — so this
## also pins the exact return expression, the one line that actually wires
## the varying into the channel the outline pass diffs.
func test_data_core_reads_the_per_vertex_label_into_g() -> void:
	var core := _read(CORE_PATH)
	assert_str(core).contains("varying float v_label")
	assert_str(core).contains("vec3 pack_data(float reveal, vec3 world, vec3 cam)")
	assert_str(core).contains("clamp(reveal, 0.0, 1.0), v_label,")
	var data_pass := _read(DATA_PASS_PATH)
	var xray := _read(XRAY_PATH)
	assert_str(data_pass).contains("data_core.gdshaderinc")
	assert_str(data_pass).contains("v_label = CUSTOM0.x")
	assert_str(xray).contains("v_label = CUSTOM0.x")
	for path: String in _all_shader_files():
		var text := _read(path)
		(
			assert_bool(text.contains("u_oid"))
			. append_failure_message("%s still mentions u_oid" % path)
			. is_false()
		)


## The data core occludes a source's REVEAL by the shared wall table now,
## not a room rectangle: source_reveal_vis counts the walls between the
## source and the lit point (wall_crossings_from, the birth wall skipped)
## and cuts player sounds crisp, muffling only the hum by HUM_THROUGH per
## wall. Pinned as source text so the GLSL cannot drift from its
## cargo-pinned reference, rust/src/sight.rs.
func test_data_core_occludes_reveal_by_the_wall_table() -> void:
	var core := _read(CORE_PATH)
	assert_str(core).contains("float source_reveal_vis(float typ, vec3 src, vec3 world)")
	assert_str(core).contains("wall_crossings_from(src, world)")
	assert_str(core).contains("pow(HUM_THROUGH, float(blocked))")
	var pool := _include_text()
	assert_str(pool).contains("int wall_crossings_from(vec3 from, vec3 to)")
	assert_str(pool).contains("bool wall_contains(vec4 rect, vec3 p, float top)")


## The numeric value of `const <type> NAME = <number>;` in the include, or
## NAN when the declaration is missing — NAN fails every numeric assert.
func _shader_const(const_name: String) -> float:
	var pattern := "const\\s+\\w+\\s+" + const_name + "\\s*=\\s*([0-9.]+)\\s*;"
	var m := RegEx.create_from_string(pattern).search(_include_text())
	return m.get_string(1).to_float() if m != null else NAN


## One number, THREE homes: the Rust core owns it (rust/src/pulse_pool.rs,
## MAXP), the test-only GDScript shim mirrors it as Pulses.MAXP, and the
## include pins it for both shaders' uniform arrays. This assertion holds
## the include against the shim; a drift in the core itself is caught by
## pulses_test's eviction suite, which counts real slots.
func test_maxp_matches_the_pool() -> void:
	assert_float(_shader_const("MAXP")).is_equal(float(Pulses.MAXP))


## The WaveCore instance mirrors the pulse pool's MAXP constant through a
## #[func] so that Godot code can query the pool size without keeping a
## duplicate constant. This assertion verifies the new WaveCore.max_pulses()
## method returns the same value as the shader constant.
func test_wavecore_max_pulses_mirrors_the_pool() -> void:
	assert_float(_shader_const("MAXP")).is_equal(float(WaveCore.new().max_pulses()))


## MAXW lives in two languages: rust/src/sight.rs owns it and the include
## pins it for the two occluding skins' uniform arrays. The level now READS
## the Rust copy and tells a designer how many free slots are left, so a
## drift between the two would be a lie in the most expensive direction —
## a level reporting room it does not have while the shaders have already
## dropped its newest walls. Nothing else compares the two numbers:
## data_skins_test pins the include's literal, and pins Rust's to nothing.
func test_maxw_matches_the_rust_sight_reference() -> void:
	assert_int(int(_shader_const("MAXW"))).is_equal(WaveLevel.wall_slots())


## The wall budget as a law about HEADROOM, not a census. A level that
## outgrows the sight shaders' slots is a level-breaking fault — every wall
## past the last slot silently stops occluding — and until now the only
## thing that noticed was map_test's frozen wall count, which fails at the
## twentieth wall and reads like a bug in the census rather than a map that
## outgrew a shader constant.
##
## So this asserts what is LEFT: a room costs about four segments (three
## sides plus the doorway, which is the gap between two segments), and the
## shipped 19-wall map keeps 13 of 32 slots free — about three more rooms.
## It goes red one room short of the ceiling, at the same count where
## WaveLevel itself starts warning, and its message names the constant.
func test_the_shipped_map_leaves_room_for_more_walls() -> void:
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var slots := WaveLevel.wall_slots()
	var walls := level.wall_segments().size()
	var room := WaveLevel.room_segments()
	var left := slots - walls
	var told := "%d walls of %d slots: %d segments left, under the %d " % [walls, slots, left, room]
	told += "another room costs. Past the last slot a wall silently stops occluding."
	told += " Shrink the map, or raise MAXW (rust/src/sight.rs) — a measured decision."
	assert_int(left).append_failure_message(told).is_greater_equal(room)


## emit() packs dat.w as type * 10 + gain * 9; the shaders must decode with
## exactly floor(w / 10) and mod(w, 10) / 9 — pinned as literal source text,
## so a "harmless" rewrite of either expression trips the contract.
func test_decode_expressions_are_literal() -> void:
	var src := _include_text()
	assert_str(src).contains("floor(d.w / 10.0)")
	assert_str(src).contains("mod(d.w, 10.0) / 9.0")


## DIST_PACK_RANGE lives in the include, which is the copy that renders, and
## now also in rust/src/level_plan.rs, because WaveLevel measures the map it
## derived against it and says so out loud. A drift between the two would
## make that report worthless in the quiet direction — a level checking
## itself against 40 while the shaders pack against 30 calls a broken map
## fine — and nothing else compares them.
func test_dist_pack_range_matches_the_level_budget() -> void:
	assert_float(_shader_const("DIST_PACK_RANGE")).is_equal(WaveLevel.pack_range())


## The hearing pass's screen texture is sampled `filter_nearest`, and that is
## load-bearing, not decorative: the data pass writes flat per-vertex
## face-or-role labels with a hard step at every intended seam or crease (two
## overlapping world faces melt to the SAME bit pattern; two separate
## touching faces sit at least MIN_SEP = 0.08 apart), and a bilinear tap at an
## unlucky sub-pixel phase would blend neighbouring labels, halving a genuine
## 0.08 diff onto the dead crease floor the hearing pass's `nrm` threshold
## (hearing_post.gdshader:75) never crosses — the seam the label law exists to
## draw would vanish exactly where a wave revealed it. Pinned as source text
## so a "harmless" filter cleanup cannot silently reopen it.
func test_hearing_pass_reads_the_screen_texture_nearest() -> void:
	var post := _read(HEARING_POST_PATH)
	assert_str(post).contains("uniform sampler2D screen_tex : hint_screen_texture, filter_nearest;")


## Camera distance is packed into one color channel divided by
## DIST_PACK_RANGE, CLAMPED rather than wrapped (data_core.gdshaderinc:150),
## so a point past the range does not alias — it saturates, and everything
## out there reads a flat 1.0. That is worse than it sounds: the silhouette
## outline is a Laplacian of that channel (hearing_post.gdshader:75) and the
## Laplacian of a plateau is zero, so far geometry draws no outline at all,
## and the hearing pass recovers scene depth as c_c.b * DIST_PACK_RANGE
## (hearing_post.gdshader:58), which pins at the range and cuts player-sound
## rings against a world that is not there.
##
## The range must therefore exceed the longest sight line the map allows:
## the full 3D diagonal of the SLAB PAIR'S union, floor to ceiling — the
## drawn world's own outer shell, not just the wall centerlines standing on
## it (issue #45: a large, sparsely walled room's short wall centerlines
## measured a tiny footprint while the slab underfoot, which is what every
## silhouette and every footstep actually draws against, reached far past
## shader range in silence) — derived from the shipped level scene, the one
## map that ever renders.
##
## Derived HERE from `level.extents` and the slab placement law
## (`rust/src/nodes/level.rs`'s `slab_center`: the floor's top sits at
## y = 0, the ceiling's underside at y = WALL_H, each slab SLAB_T thick on
## the far side of that face) rather than from a new WaveLevel accessor,
## which would just mirror `rust/src/level_plan.rs`'s own arithmetic and
## pass whatever that arithmetic did, including a wrong one. The shipped
## map's walls border fully inside its slab, so the wall table WaveLevel
## unions in belt-and-braces never widens the footprint past it — extents
## alone are already the whole story here.
func test_dist_pack_range_covers_the_map_diagonal() -> void:
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	const SLAB_T := 0.1  # rust/src/level_plan.rs SLAB_T, hand-transcribed
	var height := WaveLevel.wall_height() + 2.0 * SLAB_T
	var diagonal := Vector3(level.extents.x, height, level.extents.y).length()
	assert_float(_shader_const("DIST_PACK_RANGE")).is_greater(diagonal)
