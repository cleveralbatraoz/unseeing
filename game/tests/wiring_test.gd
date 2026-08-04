extends GdUnitTestSuite
## The composition root's perceptual ladder, read back from the LIVE main
## scene: five wave materials sharing one pool and the same per-frame
## globals, render priorities stacking the world under the acoustic image
## of sources, the level's wall table delivered to every skin that occludes
## by it, and each skin wearing the shader it should. Pins WIRING, not
## pixels — the rendered probes own the pixels.

const MAIN_SCENE := preload("res://scenes/main.tscn")
const DATA_SHADER := preload("res://shaders/data_pass.gdshader")
const XRAY_SHADER := preload("res://shaders/data_xray.gdshader")
const POST_SHADER := preload("res://shaders/hearing_post.gdshader")


func _main() -> UnseeingMain:
	var main: UnseeingMain = auto_free(MAIN_SCENE.instantiate() as UnseeingMain)
	add_child(main)
	return main


## Five materials, one pool: membership in wave_mats is what delivers
## u_time/u_flick and the pulse arrays to every skin each frame.
func test_five_wave_materials_share_pool_and_globals() -> void:
	var main := _main()
	var mats := main.wave_mats
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


## The wall table reaches every skin that occludes by it — the world
## (reveal occlusion), the source (per-object silhouette muffle) and the
## hearing pass (player-shell cut). The source keeps its standing floor
## (u_base 0.9) and the cane its own (0.85).
func test_wall_table_reaches_every_occluding_skin() -> void:
	var main := _main()
	assert_float(main.source_mat.get_shader_parameter("u_base")).is_equal(0.9)
	assert_float(main.cane_mat.get_shader_parameter("u_base")).is_equal(0.85)
	for m: ShaderMaterial in [main.data_mat, main.source_mat, main.post_mat]:
		var rects: PackedVector4Array = m.get_shader_parameter("u_walls")
		assert_int(rects.size()).is_equal(10)
		assert_int(m.get_shader_parameter("u_wall_count")).is_equal(10)
		assert_float(m.get_shader_parameter("u_wall_top")).is_equal(3.0)


## Skin identities: the level hands the fan the source material, and the
## source image is LIVE — the fan wears the XRAY skin (always heard,
## muffled through walls); the world, the props, the cat and the hero's
## own body all wear the WORLD shader at real depth; the hearing pass wears
## the post shader.
func test_skin_identities() -> void:
	var main := _main()
	assert_object(main.fan.data_mat).is_same(main.source_mat)
	assert_object(main.data_mat.shader).is_same(DATA_SHADER)
	assert_object(main.source_mat.shader).is_same(XRAY_SHADER)
	assert_object(main.cane_mat.shader).is_same(DATA_SHADER)
	assert_object(main.body_mat.shader).is_same(DATA_SHADER)
	assert_object(main.post_mat.shader).is_same(POST_SHADER)
