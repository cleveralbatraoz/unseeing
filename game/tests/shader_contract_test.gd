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
## superface labels to world solids; source builders bake derived per-instance
## semantic-role labels, while creature/viewmodel builders bake fixed roles.
## Both skins' vertex() stages carry either
## kind through as v_label, and pack_data writes it straight into G. The
## outline post-pass diffs G, so two overlapping world solids sharing a
## label bit-for-bit melt into one silhouette while role-grouped meshes retain
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
	# all THREE channels clamped, G included: an unpainted solid still
	# carries nodes::solid::BOX_ORDINALS (0..5), which leaves the channel
	assert_str(core).contains("clamp(reveal, 0.0, 1.0),")
	assert_str(core).contains("clamp(v_label, 0.0, 1.0),")
	assert_str(core).contains("clamp(vd / DIST_PACK_RANGE, 0.0, 1.0));")
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


## The data core asks the wall table whether a wave could REACH the lit
## point from its own source, and extinguishes the reveal once a wall
## stands there, for every kind alike: a wall is a barrier, not a muffle,
## and no kind buys a wave passage through it.
##
## Pins the shipped gate EXPRESSION — including its polarity — as source
## text, so a silent inversion (lighting only what sits BEHIND a wall)
## cannot pass unnoticed. This does not execute the GLSL: it proves only
## that the source text still says what it must, not that the shader
## behaves; behavioural proof is the rendered probe,
## game/tests/probe/occlusion_probe.gd. Cross-referenced against
## rust/src/sight.rs::blocked_from and ::reveal_visibility, the
## cargo-pinned laws this transliterates.
func test_data_core_occludes_reveal_by_the_wall_table() -> void:
	var core := _read(CORE_PATH)
	assert_str(core).contains("float source_reveal_vis(vec3 src, vec3 world)")
	assert_str(core).contains("return wall_blocked_from(src, world) ? 0.0 : 1.0;")
	(
		assert_bool(core.contains("HUM_THROUGH"))
		. append_failure_message(
			"data_core still grants a wave kind a transmission privilege; a wall stops every sound"
		)
		. is_false()
	)
	var pool := _include_text()
	assert_str(pool).contains("bool wall_blocked_from(vec3 from, vec3 to)")
	assert_str(pool).contains("bool wall_contains(vec4 rect, vec3 p, float top)")


## THE SHELL OBEYS THE SAME LAW AS THE REVEAL, and this is the assertion
## that says so. It is deliberately NOT a restatement of the depth test:
## `if (t >= scene_d || seen_walled) { continue; }` is byte-identical to
## what shipped before 2026-08-14 (`git show origin/main` proves it), and
## `contains` is indentation-blind, so pinning that line alone is green on
## a tree where a source's ring still crosses walls — it discriminates
## nothing about this law.
##
## What must be pinned is the SOURCE-keyed cut: the ring is drawn only
## where the sound could have reached, asked of `u_ppos[i]` — the pulse's
## own origin — through the same predicate the reveal asks of it. A depth
## test against the eye answers a different question and agrees only while
## the sound was made on the camera's side of the world, which a world
## source in another room never is.
##
## Pinned alongside it: the accumulation expression, so that no second
## per-kind attenuation factor can be reintroduced there under any name
## (the deleted privilege was exactly such a factor, `env * mute * cone`).
func test_hearing_pass_cuts_every_shell_at_the_wall_that_made_it_unreachable() -> void:
	var post := _read(HEARING_POST_PATH)
	(
		assert_bool(post.contains("if (wall_blocked_from(u_ppos[i], hp)) { continue; }"))
		. append_failure_message(
			(
				"hearing_post draws a ring without asking whether the sound could reach it; "
				+ "a source in another room would be heard through the wall"
			)
		)
		. is_true()
	)
	# the ring's brightness, with nothing between `env` and `cone` to hold a
	# resurrected per-wall survival fraction
	assert_str(post).contains(
		"col += vec3(env * cone * (body + (1.0 - body) * pow(max(grz, 0.0), 4.0))"
	)
	# and the source-keyed cut must be paid AFTER the cheap rejections, or
	# it buys the per-fragment wall walk on fragments already thrown away
	(
		assert_bool(
			(
				post.find("if (cone <= 0.0) { continue; }")
				< post.find("if (wall_blocked_from(u_ppos[i], hp))")
			)
		)
		. is_true()
	)


## A WAVE MUST END, and it must end where the CPU says it does.
##
## `reveal_at` had no end condition at all. Its only radius gate,
## `dist > min(radius, d.y)`, freezes into the static `dist > max_r` the
## moment the front has run its course, and the decay it applied there was a
## sum of exponentials, which never reaches zero — so every point a wave
## ever reached kept 0.068 of peak (a tap) or 0.257 (a hum) until some later,
## unrelated sound reused the pool slot and switched it off in one frame. The
## visible life of a sound was a property of the slot allocator.
##
## Two things are pinned, and the pair is the law. First the death gate
## itself, as source text, ahead of the cone gate and the exponentials so a
## dead pulse costs a compare rather than a trace. Second — and this is the
## part a substring cannot fake — the GLSL tail table is read back branch by
## branch and held against rust/src/render/reveal.rs's own numbers through
## WaveCore. Editing one arm of the shader chain without editing Rust gives
## that kind of sound a different life on screen than its slot was budgeted
## for, and nothing else in the tree compares the two.
func test_a_wave_stops_revealing_when_its_pool_slot_expires() -> void:
	var core := _read(CORE_PATH)
	(
		assert_bool(core.contains("if (ga >= tail) { continue; }"))
		. append_failure_message(
			(
				"reveal_at has no death gate; a surface a wave once lit stays lit "
				+ "until an unrelated sound reuses the slot"
			)
		)
		. is_true()
	)
	assert_str(core).contains("float pulse_flare(float since_front, float tail)")
	# and the gate must be paid BEFORE the cone test, or a dead pulse still
	# buys a normalize, a dot and a smoothstep on every fragment it reaches
	(
		assert_bool(core.find("if (ga >= tail) { continue; }") < core.find("float cone ="))
		. append_failure_message("the death gate is paid after the cone gate")
		. is_true()
	)

	var wave_core: WaveCore = auto_free(WaveCore.new())
	var chain := _glsl_fade_tail_chain()
	(
		assert_array(chain)
		. append_failure_message("pulse_fade_tail's guarded arms are missing from the pool include")
		. is_not_empty()
	)
	# every kind emit() packs today, plus the two outside that range the
	# i32/float domain admits and the wildcard arm must still answer
	for kind: int in [0, 1, 2, 3, 4, -1]:
		var glsl := _evaluate_glsl_chain(chain, float(kind))
		(
			assert_float(glsl)
			. append_failure_message(
				(
					"GLSL grants kind %d a %s s tail while Rust budgets its slot for %s s"
					% [kind, str(glsl), str(wave_core.wave_fade_tail(kind))]
				)
			)
			. is_equal(wave_core.wave_fade_tail(kind))
		)


## THE SHAPE ITSELF, evaluated rather than spelled.
##
## `pulse_flare`'s four constants are the whole look of the game: the weight
## and time constant of the strike flash, and of the lingering half. A
## `contains()` assertion cannot tell 1.3 from 1.0, or a 3.0 time constant
## from a 4.0 one — change either and the shipped decay is a different law
## from render::reveal::flare while every substring pin in this suite still
## matches. So the constants are read OUT of the shipped GLSL, the shape is
## rebuilt from them here, and the result is compared against Rust across the
## whole domain, per kind.
##
## The closing window rides with them, because it is what ends the wave:
## flat at 1.0 for the first three quarters of every kind's life — so a cane
## tap and its own echo decay identically on one surface until each nears its
## own end — then smoothstepping to exactly zero at the tail.
func test_the_rendered_decay_shape_is_the_cargo_pinned_one() -> void:
	var core := _read(CORE_PATH)
	var wave_core: WaveCore = auto_free(WaveCore.new())
	var shape := _glsl_flare_constants(core)
	(
		assert_array(shape)
		. append_failure_message("pulse_flare's shape constants are unreadable from the GLSL")
		. is_not_empty()
	)
	var fast_w: float = shape[0]
	var fast_t: float = shape[1]
	var slow_w: float = shape[2]
	var slow_t: float = shape[3]
	var close := _shader_close_fraction(core)
	assert_float(close).is_equal(wave_core.wave_close_fraction())
	for kind: int in [0, 1, 2, 3]:
		var tail: float = wave_core.wave_fade_tail(kind)
		var opens: float = tail * (1.0 - close)
		for step: int in 40:
			var since: float = tail * float(step) / 39.0
			var raw: float = fast_w * exp(-since / fast_t) + slow_w * exp(-since / slow_t)
			var window: float = 1.0 - smoothstep(opens, tail, since)
			var glsl: float = clampf(raw * window, 0.0, 1.0) if since < tail else 0.0
			var rust: float = wave_core.wave_flare(since, tail)
			(
				assert_float(glsl)
				. append_failure_message(
					(
						"kind %d at %s s: the shipped GLSL decays to %s, Rust to %s"
						% [kind, str(since), str(glsl), str(rust)]
					)
				)
				. is_equal_approx(rust, 1e-6)
			)


## THE TIME COORDINATE, pinned by the wall-clock death it implies.
##
## The whole reveal law is written against seconds-since-the-front-passed —
## `ga = age - dist / speed` — and nothing else in the tree says so. Replace
## it with `ga = age` and every cargo test and every substring pin stays
## green, while the fan (reach 9 m at speed 4.5, so a ring time of exactly
## 2.0 s, against kind 3's tail of exactly 2.0 s) stops revealing the outer
## metre of its own wash at the instant its front arrives there — a ring
## still drawn in the air over surfaces that never light.
##
## Asserted at two distances, because at zero the two coordinates agree.
func test_a_waves_reveal_dies_later_the_farther_it_has_travelled() -> void:
	var wave_core: WaveCore = auto_free(WaveCore.new())
	assert_str(_read(CORE_PATH)).contains("float ga = age - dist / d.z;")
	# hand-derived for a kind-3 hum at 4.5 m/s: at the source the front
	# arrives at once and the reveal ends one tail later (2.0 s); nine metres
	# out the front takes 9/4.5 = 2.0 s and the reveal ends at 4.0 s
	assert_float(wave_core.wave_death_time(3, 0.0, 4.5)).is_equal_approx(2.0, 1e-9)
	assert_float(wave_core.wave_death_time(3, 9.0, 4.5)).is_equal_approx(4.0, 1e-9)
	# and the pool holds the slot exactly that long for the farthest point,
	# so a wave's reveal and its data die together
	assert_float(wave_core.wave_death_time(3, 9.0, 4.5)).is_equal_approx(
		9.0 / 4.5 + wave_core.wave_fade_tail(3), 1e-9
	)
	# total at the door: a speed that cannot carry a wave answers NAN
	assert_bool(is_nan(wave_core.wave_death_time(3, 1.0, 0.0))).is_true()
	assert_bool(is_nan(wave_core.wave_death_time(3, -1.0, 4.5))).is_true()


## `pulse_flare`'s four shape constants in source order — fast weight, fast
## time, slow weight, slow time — or an empty array when the expression has
## been rewritten past recognition, which fails the caller rather than
## silently checking nothing.
func _glsl_flare_constants(core: String) -> PackedFloat64Array:
	var pattern := (
		"float shape = ([0-9.]+) \\* exp\\(-since_front / ([0-9.]+)\\)"
		+ " \\+ ([0-9.]+) \\* exp\\(-since_front / ([0-9.]+)\\);"
	)
	var m := RegEx.create_from_string(pattern).search(core)
	if m == null:
		return PackedFloat64Array()
	return PackedFloat64Array(
		[
			m.get_string(1).to_float(),
			m.get_string(2).to_float(),
			m.get_string(3).to_float(),
			m.get_string(4).to_float(),
		]
	)


## CLOSE_FRACTION as the shipped shader declares it, or NAN — which fails
## every numeric assert rather than skipping the check.
func _shader_close_fraction(core: String) -> float:
	var m := RegEx.create_from_string("const float CLOSE_FRACTION = ([0-9.]+);").search(core)
	return m.get_string(1).to_float() if m != null else NAN


## `pulse_fade_tail`'s guarded arms as (threshold, seconds) pairs, in source
## order, with the unguarded fallthrough appended as an arm no `typ` can
## miss. Read out of the shipped GLSL rather than retyped, so this suite
## compares the shader against Rust instead of comparing two of its own
## transcriptions against each other.
func _glsl_fade_tail_chain() -> Array[PackedFloat64Array]:
	var arms: Array[PackedFloat64Array] = []
	var body := _include_text()
	var start := body.find("float pulse_fade_tail(")
	if start < 0:
		return arms
	var end := body.find("\n}", start)
	if end < 0:
		return arms
	body = body.substr(start, end - start)
	for m: RegExMatch in (
		RegEx
		. create_from_string("if \\(typ < ([0-9.]+)\\) \\{ return ([0-9.]+); \\}")
		. search_all(body)
	):
		arms.append(PackedFloat64Array([m.get_string(1).to_float(), m.get_string(2).to_float()]))
	var last := RegEx.create_from_string("\\n\\treturn ([0-9.]+);").search(body)
	if last == null:
		arms.clear()
		return arms
	arms.append(PackedFloat64Array([INF, last.get_string(1).to_float()]))
	return arms


## The chain evaluated exactly as GLSL evaluates it: first arm whose
## threshold `typ` falls under wins.
func _evaluate_glsl_chain(chain: Array[PackedFloat64Array], typ: float) -> float:
	for arm: PackedFloat64Array in chain:
		if typ < arm[0]:
			return arm[1]
	return NAN


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
## shader range in silence) — derived from the default authored level.
##
## Derived HERE from `level.extents` and the slab placement law
## (`rust/src/nodes/level.rs`'s `slab_center`: the floor's top sits at
## y = 0, the ceiling's underside at y = WALL_H, each slab SLAB_T thick on
## the far side of that face) rather than from a new WaveLevel accessor,
## which would just mirror `rust/src/level_plan.rs`'s own arithmetic and
## pass whatever that arithmetic did, including a wrong one. This assertion
## is independent of the scene's object census and wall layout.
func test_dist_pack_range_covers_the_default_level_diagonal() -> void:
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	const SLAB_T := 0.1  # rust/src/level_plan.rs SLAB_T, hand-transcribed
	var height := WaveLevel.wall_height() + 2.0 * SLAB_T
	var diagonal := Vector3(level.extents.x, height, level.extents.y).length()
	assert_float(_shader_const("DIST_PACK_RANGE")).is_greater(diagonal)
