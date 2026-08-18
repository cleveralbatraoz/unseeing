extends Node
## The platform facts the renderer's derivations rest on, answered by ONE
## screenshot so the same scene can be measured on the desktop and in a
## browser.
##
## `channel_probe` and `depth_texture_probe` already answer these on the
## desktop, by writing a pair of values, reading them back in-game with
## `get_viewport().get_texture().get_image()`, and repeating that for every
## candidate step. That shape cannot cross to the web: the browser gate
## drives the page from outside and cannot step an in-game readback loop,
## and Godot's own readback on web is a different code path from the one
## under test — measuring it would partly measure the measurement.
##
## So this scene lays the whole ladder out SPATIALLY and lets one frame
## carry every answer. `probe_platform_write` fills the screen with pairs of
## values one candidate step apart, base sweeping with y;
## `probe_platform_read` compares each pair through `hint_screen_texture`
## and paints white where they stayed distinct. Two further regions carry
## the depth texture and a control. A screenshot — taken by the harness on
## the desktop, or by Chrome's DevTools on the web — is the whole result.
##
## LAYOUT, which the decoder in tools/platform_probe.py depends on:
##   x in [0.00, 0.70)  nine bands, bit depths 8..16 left to right; white
##                      means that step survived at that base (y)
##   x in [0.70, 0.85)  the depth texture at a known surface, x60
##   x in [0.85, 1.00]  the control: the screen texture read straight back,
##                      which must be 0.5
##
## On the desktop it also prints the same verdicts, so a human running it
## by hand does not have to decode a PNG.

const READ_SHADER := preload("res://shaders/probe_platform_read.gdshader")
const WRITE_SHADER := preload("res://shaders/probe_platform_write.gdshader")

## Bit depths the ladder tests, left to right — must match NBANDS in both
## shaders.
const BANDS := 9
const FIRST_BITS := 8
## Where the write pass puts the control value, and what it must read back.
const CONTROL := 0.5
## The world quad stands this far from the eye; with near 0.05 and far 60
## Godot's reversed-Z puts it at (0.05/59.95) * (60/3 - 1) = 0.015847, which
## x60 is 0.9508 — visible in an 8-bit frame without saturating.
const WORLD_DIST := 3.0
const NEAR := 0.05
const FAR := 60.0
const DEPTH_GAIN := 60.0


func _ready() -> void:
	await get_tree().process_frame
	_build()
	await _report()
	get_tree().quit(0)


func _build() -> void:
	var camera := Camera3D.new()
	camera.position = Vector3(0, 0, 2)
	camera.near = NEAR
	camera.far = FAR
	add_child(camera)

	var write_mat := ShaderMaterial.new()
	write_mat.shader = WRITE_SHADER
	var writer := MeshInstance3D.new()
	var quad := QuadMesh.new()
	quad.size = Vector2(16, 16)
	writer.mesh = quad
	writer.material_override = write_mat
	writer.position = Vector3(0, 0, 2.0 - WORLD_DIST)
	add_child(writer)

	var read_mat := ShaderMaterial.new()
	read_mat.shader = READ_SHADER
	read_mat.set_shader_parameter("u_depth_gain", DEPTH_GAIN)
	var reader := MeshInstance3D.new()
	var post := QuadMesh.new()
	post.size = Vector2(2, 2)
	reader.mesh = post
	reader.extra_cull_margin = 16384.0
	reader.material_override = read_mat
	reader.position = Vector3(0, 0, -1)
	camera.add_child(reader)


## The same verdicts the PNG carries, printed — so a desktop run needs no
## decoder, and so a web run leaves them in the browser console too.
func _report() -> void:
	for _i: int in 8:
		await get_tree().process_frame
		await RenderingServer.frame_post_draw
	var img := get_viewport().get_texture().get_image()
	var w := img.get_width()
	var h := img.get_height()

	var levels := 0
	for band: int in BANDS:
		var bits := FIRST_BITS + band
		var x := int((float(band) + 0.5) / float(BANDS) * 0.70 * float(w))
		var survives_everywhere := true
		for row: int in 17:
			var y := int((0.05 + 0.9 * float(row) / 16.0) * float(h))
			if img.get_pixel(x, clampi(y, 0, h - 1)).r < 0.5:
				survives_everywhere = false
				break
		print("#   %d-bit step survives at every base: %s" % [bits, str(survives_everywhere)])
		if survives_everywhere:
			levels = int(pow(2.0, float(bits)))
		else:
			break

	var depth := img.get_pixel(int(0.775 * float(w)), h / 2).r
	var control := img.get_pixel(int(0.925 * float(w)), h / 2).r
	var expect := (NEAR / (FAR - NEAR)) * (FAR / WORLD_DIST - 1.0) * DEPTH_GAIN
	print(
		(
			"# platform: levels %d ; depth %.4f (reversed-Z predicts %.4f) ; control %.4f"
			% [levels, depth, expect, control]
		)
	)
	print("# CONTROL must read %.2f — anything else voids every verdict above" % CONTROL)
