extends GdUnitTestSuite
## The composition root's perceptual ladder, read back from a code-built live
## world: five wave materials sharing one pool and the same per-frame
## globals, render priorities stacking the world under the acoustic image
## of sources, the level's wall table delivered to every skin that occludes
## by it, and each skin wearing the shader it should. Pins WIRING, not
## pixels — the rendered probes own the pixels.

const WORLD_FIXTURE := preload("res://tests/world_fixture.gd")
const DATA_SHADER := preload("res://shaders/data_pass.gdshader")
const XRAY_SHADER := preload("res://shaders/data_xray.gdshader")
const POST_SHADER := preload("res://shaders/hearing_post.gdshader")


func _main() -> UnseeingGame:
	# The two census-dependent cases ask for exactly their collaborators;
	# shipped level walls and sources remain freely authorable.
	var main: UnseeingGame = auto_free(
		WORLD_FIXTURE.game(WORLD_FIXTURE.DEFAULT_EXTENTS, true, true)
	)
	add_child(main)
	return main


## Five materials, one pool: membership in wave_mats is what delivers
## u_time/u_flick and the pulse arrays to every skin each frame.
func test_five_wave_materials_share_pool_and_globals() -> void:
	var main := _main()
	var mats := main.wave_mats()
	assert_int(mats.size()).is_equal(5)
	assert_object(mats[0]).is_same(main.data_mat)
	assert_object(mats[1]).is_same(main.source_mat)
	assert_object(mats[2]).is_same(main.cane_mat)
	assert_object(mats[3]).is_same(main.body_mat)
	assert_object(mats[4]).is_same(main.post_mat)


## Draw order IS the perceptual layering once the source image fakes its
## depth: the world at real depth (priority 0), the acoustic image of
## sources on top of it (priority 20). The hero's cane and body ride at
## real depth with the world, unlayered.
func test_priorities_stack_the_perceptual_ladder() -> void:
	var main := _main()
	assert_int(main.data_mat.render_priority).is_equal(0)
	assert_int(main.source_mat.render_priority).is_equal(20)


## The wall table reaches EVERY skin that occludes by it, and the list is
## the whole point: the world (reveal occlusion), the sources (per-object
## silhouette muffle), the hearing pass (the shell cut, which is every
## ring's and not just the hero's, plus the x-rayed-source ring guard) —
## AND the hero's own cane and body.
##
## Those last two are where this assertion was previously short, and the
## gap was not cosmetic. Both wear `data_pass.gdshader`, which runs
## `reveal_at` -> `source_reveal_vis` -> `wall_blocked_from` exactly like
## the world does; but nothing ever pushed them a table, so `u_wall_count`
## kept its shader default of 0, the wall loop broke on its first
## iteration, and the barrier law was a no-op on the two surfaces the
## player is guaranteed to be looking at. A source humming in the next
## room lit the hero's own legs through the wall while the same frame
## painted the room around them black.
##
## The cane keeps its own standing floor (u_base 0.85); the source skin has
## NO material-wide floor at all, because each source carries its own per
## instance.
func test_wall_table_reaches_every_occluding_skin() -> void:
	var main := _main()
	var wall_segs := main.level.wall_segments()
	# non-vacuity: an empty wall table would let every read-back count
	# below agree with itself trivially, at zero
	assert_array(wall_segs).is_not_empty()
	var walls := wall_segs.size()
	assert_float(main.cane_mat.get_shader_parameter("u_base")).is_equal(0.85)
	for m: ShaderMaterial in [
		main.data_mat, main.source_mat, main.post_mat, main.cane_mat, main.body_mat
	]:
		var rects: PackedVector4Array = m.get_shader_parameter("u_walls")
		assert_int(rects.size()).is_equal(walls)
		assert_int(m.get_shader_parameter("u_wall_count")).is_equal(walls)
		# each wall's own sweep, in the SAME slot order — both are
		# projections of one Vec<Occluder>, so a length that disagrees means
		# the two tables were built separately again
		var spans: PackedVector2Array = m.get_shader_parameter("u_wall_y")
		assert_int(spans.size()).is_equal(walls)
		for span: Vector2 in spans:
			assert_float(span.x).is_equal(0.0)
			assert_float(span.y).is_equal(3.0)


## A RE-DERIVE REACHES ALL FIVE SKINS, which is the property that used to be
## supplied by luck rather than by design.
##
## The level rebuilds its wall table on every `derive()`, but three of the
## five occluding skins are owned by the composition root, which pushed the
## table to them ONCE — correct only because a runtime level happens to
## derive exactly once, before that push. `WaveLevel::rederive` is a `#[func]`
## and anything may call it; afterwards the hearing pass and the hero's own
## cane and body would have been carrying the previous derivation's walls
## while the level's own two carried the current one's.
##
## The break this catches is a return to that shape: the three root-owned
## skins are REGISTERED with the level now, so one owner refreshes all five.
func test_a_rederive_refreshes_every_occluding_skin_not_only_the_levels_own() -> void:
	var main := _main()
	var walls := main.level.wall_segments().size()
	assert_int(walls).is_greater(0)
	var skins: Array[ShaderMaterial] = [
		main.data_mat, main.source_mat, main.post_mat, main.cane_mat, main.body_mat
	]
	# Scribble a table that cannot coincide with the right answer. Size
	# alone is not enough: this fixture holds ONE wall, so a one-entry
	# scribble compares 1 against 1 and passes whether or not the re-derive
	# ever reached the skin — which is exactly how the first version of this
	# test passed against a deliberately broken build.
	var scribble := PackedVector4Array(
		[Vector4(9, 9, 9, 9), Vector4(9, 9, 9, 9), Vector4(9, 9, 9, 9)]
	)
	for m: ShaderMaterial in skins:
		m.set_shader_parameter("u_walls", scribble)
		m.set_shader_parameter("u_wall_y", PackedVector2Array([Vector2(9, 9)]))
		m.set_shader_parameter("u_wall_count", 99)

	main.level.rederive()

	var truth: PackedVector4Array = main.level.wall_rects()
	for i: int in skins.size():
		var m: ShaderMaterial = skins[i]
		var rects: PackedVector4Array = m.get_shader_parameter("u_walls")
		var spans: PackedVector2Array = m.get_shader_parameter("u_wall_y")
		(
			assert_int(rects.size())
			. append_failure_message("skin %d kept a stale wall table across a re-derive" % i)
			. is_equal(walls)
		)
		# ...and the VALUES, so a table of the right length but the wrong
		# contents cannot pass either
		assert_vector(rects[0]).is_equal(truth[0])
		assert_int(spans.size()).is_equal(walls)
		assert_int(m.get_shader_parameter("u_wall_count")).is_equal(walls)


## THE ALLOCATOR AND THE RENDERER ARE ONE LAW, and this is where the two
## ends meet on the engine side.
##
## `render::labels::MIN_SEP` decides how far apart two labels must be before
## a seam may be drawn between them; the hearing pass's
## `smoothstep(lo, hi, nrm)` decides how brightly that gap actually draws.
## They used to be two literals — one in Rust, one in GLSL — with nothing
## comparing them, so lowering MIN_SEP to fit a starved label band kept every
## cargo test green while the shader went on fading over a knee it no longer
## matched, rendering the seams the allocator had just approved at a fraction
## of full strength.
##
## The knee is now derived in Rust and pushed. This asserts the composition
## root actually pushed it, that it equals the derivation, and that the
## shader READS the uniform rather than a literal — three things, because
## any one of them alone is satisfiable while the law is broken.
func test_the_crease_knee_reaches_the_post_pass_from_the_one_separation() -> void:
	var main := _main()
	var core: WaveCore = auto_free(WaveCore.new())
	var pushed: Vector2 = main.post_mat.get_shader_parameter("u_crease_knee")
	var derived: Vector2 = core.crease_knee()
	(
		assert_vector(pushed)
		. append_failure_message(
			"the post pass was pushed %s, but MIN_SEP derives %s" % [str(pushed), str(derived)]
		)
		. is_equal(derived)
	)
	# the derivation itself: full strength at MIN_SEP, half a knee below it
	var sep: float = core.min_label_separation()
	assert_float(derived.y).is_equal_approx(sep, 0.0001)
	assert_float(derived.x).is_equal_approx(sep * 0.5, 0.0001)
	# ...and it must be a knee GLSL can evaluate at all: smoothstep divides
	# by (hi - lo)
	assert_bool(derived.x < derived.y).is_true()
	# the shader reads the uniform, not the literal it used to carry
	var post := FileAccess.open("res://shaders/hearing_post.gdshader", FileAccess.READ)
	var src := post.get_as_text() if post != null else ""
	assert_str(src).contains("smoothstep(u_crease_knee.x, u_crease_knee.y, nrm)")
	assert_bool(src.contains("smoothstep(0.04, 0.08")).is_false()


## THE BREAK: the silhouette knee derived in Rust and never pushed, which is
## exactly how u_grain_amp lived for months — the Rust doc claiming ownership
## while the live value was the GLSL default.
##
## Here the default draws NOTHING at all, so an unpushed material is a black
## screen rather than a slightly different picture. That is deliberate: it is
## the one uniform on this pass whose wrong value is unmissable, and it must
## stay that way.
func test_the_silhouette_knee_reaches_the_post_pass_in_metres() -> void:
	var main := _main()
	var core: WaveCore = auto_free(WaveCore.new())
	var pushed: Vector2 = main.post_mat.get_shader_parameter("u_sil_knee")
	var derived: Vector2 = core.silhouette_knee()
	(
		assert_vector(pushed)
		. append_failure_message(
			"the post pass was pushed %s, but Rust derives %s" % [str(pushed), str(derived)]
		)
		. is_equal(derived)
	)
	# and it is NOT the shader default, which would mean nobody pushed
	assert_bool(pushed.x < 1.0e3).is_true()
	# the shader reads the uniform, not the literal it used to carry
	var post := FileAccess.open("res://shaders/hearing_post.gdshader", FileAccess.READ)
	var src := post.get_as_text() if post != null else ""
	assert_str(src).contains("smoothstep(u_sil_knee.x, u_sil_knee.y, lap)")
	assert_bool(src.contains("smoothstep(0.012, 0.03")).is_false()


## THE BREAK: `render::grain::GRAIN_AMP` and the shader's `u_grain_amp`
## drifting apart. The Rust doc claimed the constant was "owned here, pushed
## from the composition root" — it was not pushed at all, and the live value
## was the GLSL default, so a maintainer changing GRAIN_AMP in Rust got an
## unchanged picture. `reveal::PRESENCE` is derived FROM this constant, and
## settled law 1 (a sound source is always visible) rests on that derivation,
## so the ownership direction is load-bearing rather than cosmetic.
func test_the_film_grain_amplitude_reaches_the_post_pass_from_rust() -> void:
	var main := _main()
	var pushed: float = main.post_mat.get_shader_parameter("u_grain_amp")
	var owned: float = WaveLevel.grain_amp()
	(
		assert_float(pushed)
		. append_failure_message(
			"the post pass holds %f, but render::grain::GRAIN_AMP is %f" % [pushed, owned]
		)
		. is_equal_approx(owned, 0.000001)
	)
	# NOT asserted here: that PRESENCE is twice this. `reveal::PRESENCE` is
	# DEFINED as 2 * GRAIN_AMP in Rust, so restating it in GDScript would be
	# a mirror that no edit can break. What this case can honestly catch is
	# a push that is missing or points at the wrong constant, and it does.


## Skin identities: the level hands EVERY sound source the source material,
## and the source image is LIVE — a source wears the XRAY skin (always
## heard, muffled through walls); the world, the props, the cat and the
## hero's own body all wear the WORLD shader at real depth; the hearing pass
## wears the post shader.
func test_skin_identities() -> void:
	var main := _main()
	var sources := main.level.sources()
	assert_array(sources).is_not_empty()
	# Read through the shared exported material surface: the point is that the
	# level dressed its source without knowing its concrete class. The
	# abstraction is a Rust trait, so GDScript has no common base type here.
	for source: Node3D in sources:
		assert_object(source.get("data_mat")).is_same(main.source_mat)
	assert_object(main.data_mat.shader).is_same(DATA_SHADER)
	assert_object(main.source_mat.shader).is_same(XRAY_SHADER)
	assert_object(main.cane_mat.shader).is_same(DATA_SHADER)
	assert_object(main.body_mat.shader).is_same(DATA_SHADER)
	assert_object(main.post_mat.shader).is_same(POST_SHADER)
