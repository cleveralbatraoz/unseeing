extends Node
## Can the hearing pass tell an acoustic image from a real surface by ASKING
## the depth buffer, instead of inferring it?
##
## It infers it today: `hearing_post` rebuilds a world point from the B
## channel and asks the wall table whether a wall stands there. That
## inference is incomplete — `WaveProp`, `WaveColumn` and `WaveWedge` are in
## no occluder table anywhere — so a source hidden behind a pillar is
## misclassified, its rings are not cut and its outline borrows the pillar's
## brightness.
##
## A depth read would settle it exactly, with no table and no
## reconstruction: the source skin is opaque and writes `DEPTH` inside the
## always-on-top band, so any fragment whose depth is in that band IS a
## source. `data_core.gdshaderinc` has carried the claim that the depth
## texture is "unreliable on WebGL2/Compatibility" since before this suite
## existed, and the claim is structural — the whole B-channel distance
## packing exists because of it — so it deserves a measurement rather than
## another citation.
##
## MEASURED HERE, on desktop GL:
##   1. the depth texture is LIVE, and its value matches the analytic
##      reversed-Z mapping to four decimal places;
##   2. an always-on-top fragment reads back inside the acoustic-image band,
##      so the two layers are separable by a single comparison;
##   3. declaring it beside `hint_screen_texture` costs the screen read
##      nothing — which had to be checked, because `hearing_post` IS the
##      screen texture and could not adopt the depth texture otherwise.
##
## STILL UNMEASURED: WebGL2. This is a desktop measurement and the web is a
## first-class target, which is why the shipped fix ORs this test with the
## old wall-table inference rather than replacing it — see
## `hearing_post.gdshader`. Where the depth texture works the layer test is
## exact; where it is dead the pass degrades to exactly its former
## behaviour, and never to worse.

const DEPTH_SHADER := preload("res://shaders/probe_depth_read.gdshader")
const COMBINED_SHADER := preload("res://shaders/probe_depth_combined.gdshader")
const SCREEN_ONLY_SHADER := preload("res://shaders/probe_screen_read.gdshader")
const WRITE_SHADER := preload("res://shaders/probe_channel_write.gdshader")
const XRAY_WRITE_SHADER := preload("res://shaders/probe_depth_xray.gdshader")

## The camera the analytic expectation below is derived against.
const NEAR := 0.05
const FAR := 60.0
## Where the world quad stands, in metres from the eye.
const WORLD_DIST := 3.0
## `data_core.gdshaderinc`, hand-transcribed: the probe must not read its
## expectation out of the same file the game reads it from.
const ALWAYS_ON_TOP := 0.999999
const SOURCE_BAND := 1.0e-3
## The depth read is amplified by this before it is screenshotted, because
## a world fragment's depth is ~0.016 and an 8-bit frame cannot resolve it.
const DEPTH_GAIN := 60.0

var _failures := 0
var _read_mat: ShaderMaterial


func _ready() -> void:
	await get_tree().process_frame
	_build()
	await _measure()
	get_tree().quit(1 if _failures > 0 else 0)


func _build() -> void:
	var camera := Camera3D.new()
	camera.position = Vector3(0, 0, 2)
	camera.near = NEAR
	camera.far = FAR
	add_child(camera)

	# the WORLD quad: real depth, 3 m from the eye
	_add_quad(WRITE_SHADER, Vector3(0, 0, 2.0 - WORLD_DIST), 8.0)

	_read_mat = ShaderMaterial.new()
	var reader := MeshInstance3D.new()
	var post := QuadMesh.new()
	post.size = Vector2(2, 2)
	reader.mesh = post
	reader.extra_cull_margin = 16384.0
	reader.material_override = _read_mat
	reader.position = Vector3(0, 0, -1)
	camera.add_child(reader)


func _add_quad(shader: Shader, at: Vector3, size: float) -> MeshInstance3D:
	var mat := ShaderMaterial.new()
	mat.shader = shader
	mat.set_shader_parameter("u_lo", 0.75)
	mat.set_shader_parameter("u_hi", 0.75)
	var mi := MeshInstance3D.new()
	var quad := QuadMesh.new()
	quad.size = Vector2(size, size)
	mi.mesh = quad
	mi.material_override = mat
	mi.position = at
	add_child(mi)
	return mi


func _read() -> float:
	var peak := 0.0
	for _i: int in 8:
		await get_tree().process_frame
		await RenderingServer.frame_post_draw
		var img := get_viewport().get_texture().get_image()
		peak = maxf(peak, img.get_pixel(img.get_width() / 2, img.get_height() / 2).r)
	return peak


func _measure() -> void:
	# 1 — the depth texture is live, and it is REAL depth.
	#
	# Hand-derived from Godot's reversed-Z mapping, z = (n / (f - n)) *
	# (f / d - 1): with near 0.05, far 60 and a surface 3 m away that is
	# (0.05 / 59.95) * (60 / 3 - 1) = 0.015847. Amplified by 60 it lands at
	# 0.9508, which an 8-bit frame can carry. Matching a derived value to
	# this tolerance is what separates "the buffer holds something" from
	# "the buffer holds depth".
	_read_mat.shader = DEPTH_SHADER
	_read_mat.set_shader_parameter("u_gain", DEPTH_GAIN)
	var world_depth := await _read()
	var expected := (NEAR / (FAR - NEAR)) * (FAR / WORLD_DIST - 1.0) * DEPTH_GAIN
	print(
		(
			"# depth at a world surface %.1f m away: %.4f (reversed-Z predicts %.4f)"
			% [WORLD_DIST, world_depth, expected]
		)
	)
	_check(
		"the depth texture is LIVE and carries real depth (%.4f ~ %.4f)" % [world_depth, expected],
		absf(world_depth - expected) < 0.02
	)

	# 2 — the two layers are separable by ONE comparison, read at UNIT gain.
	#
	# The gain above exists to make a world fragment's 0.016 visible in an
	# 8-bit frame, and it must be dropped here: at any gain above 1 every
	# value over 1/gain saturates, so an always-on-top fragment and a world
	# fragment 3 m away would both read 1.0 and the check would prove
	# nothing. At unit gain the world reads ~0.016 (8-bit code 4) and a
	# source reads ~1.0 (code 255).
	_read_mat.set_shader_parameter("u_gain", 1.0)
	var world_unit := await _read()
	_add_quad(XRAY_WRITE_SHADER, Vector3(0, 0, 1.0), 8.0)
	var source_unit := await _read()
	var band_floor := ALWAYS_ON_TOP - SOURCE_BAND
	print(
		(
			"# at unit gain: world %.4f, acoustic image %.4f, band floor %.4f"
			% [world_unit, source_unit, band_floor]
		)
	)
	_check(
		(
			"an acoustic-image fragment reads INSIDE the always-on-top band (%.4f >= %.4f)"
			% [source_unit, band_floor]
		),
		source_unit >= band_floor
	)
	_check(
		"...and a world fragment reads nowhere near it (%.4f < %.4f)" % [world_unit, band_floor],
		world_unit < band_floor * 0.5
	)

	# 3 — and the pass that must use it can. hearing_post IS the screen
	# texture; if declaring a depth uniform beside it cost the screen read,
	# the whole approach would be closed however well the depth read worked.
	#
	# TWO SHADERS, not two branches of one. This compared `c` against
	# `min(c + d, c)` at first — an algebraic identity, so the two readings
	# were computed to be equal and the check could not fail whatever the
	# platform did. The comparison only means something across a shader that
	# declares the depth texture and one that does not.
	_read_mat.shader = SCREEN_ONLY_SHADER
	var colour_alone := await _read()
	_read_mat.shader = COMBINED_SHADER
	var colour_beside_depth := await _read()
	print(
		(
			"# screen texture: %.3f alone, %.3f with a depth uniform declared"
			% [colour_alone, colour_beside_depth]
		)
	)
	_check("the CONTROL sees the geometry at all (%.3f > 0.5)" % colour_alone, colour_alone > 0.5)
	_check(
		(
			"declaring hint_depth_texture costs the screen read NOTHING (%.3f == %.3f)"
			% [colour_beside_depth, colour_alone]
		),
		absf(colour_beside_depth - colour_alone) < 0.02
	)

	print("1..5")
	if _failures > 0:
		print("depth-texture-probe: FAIL (%d)" % _failures)
	else:
		print("depth-texture-probe: PASS")


func _check(label: String, ok: bool) -> void:
	if ok:
		print("ok - %s" % label)
	else:
		_failures += 1
		print("not ok - %s" % label)
