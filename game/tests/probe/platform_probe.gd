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
## LAYOUT:
##   x in [0.00, 0.70)  nine bands, ascending step sizes left to right; white
##                      means that step survived at that base (y)
##   x in [0.70, 0.85)  the depth texture at a known surface, x60
##   x in [0.85, 1.00]  the control: the screen texture read straight back,
##                      which must be 0.5
##
## On the desktop it also prints the same verdicts, so a human running it
## by hand does not have to decode a PNG.

const READ_SHADER := preload("res://tests/probe/shaders/probe_platform_read.gdshader")
const WRITE_SHADER := preload("res://tests/probe/shaders/probe_platform_write.gdshader")

## Step sizes the ladder tests, left to right, as multiples of one 10-bit
## step. The only copy: pushed to the write shader as `u_steps` in `_build`,
## so the array that lays the bands out and the array that reads the verdict
## are the same array.
##
## Not powers of two. The channel's worst-case QUANTUM is what the B-channel
## reconstruction guard clears, and a power-of-two ladder rounds that quantum
## to the next whole bit: an AMD desktop GPU resolves 1/1023 at 647 of 649
## bases and the old ladder called that 512 levels, halving the number the
## guard is checked against. 0.50 and 2.00 bracket the answer and are the
## probe's own sanity checks — on a 10-bit buffer the first must fail and the
## last must survive.
const STEPS: Array[float] = [0.50, 0.90, 1.00, 1.02, 1.05, 1.10, 1.25, 1.50, 2.00]
const BANDS := 9
const TEN_BIT := 1.0 / 1023.0
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
	var failures := await _report()
	get_tree().quit(1 if failures > 0 else 0)


func _build() -> void:
	var camera := Camera3D.new()
	camera.position = Vector3(0, 0, 2)
	camera.near = NEAR
	camera.far = FAR
	add_child(camera)

	var write_mat := ShaderMaterial.new()
	write_mat.shader = WRITE_SHADER
	# ONE copy of the ladder, in the language that computes the verdict. It
	# used to exist twice — here and as a `const float STEPS[9]` in the write
	# shader — bound by nothing but a comment, while the SHADER decided which
	# band got which step and this file named the multiplier that survived.
	# A one-entry drift produced a confident wrong quantum rather than an
	# error, and that quantum is the sole source of
	# render::channel::WORST_STEP_CODES.
	write_mat.set_shader_parameter("u_steps", PackedFloat32Array(STEPS))
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


## The rows the ladder is scanned across. The write pass sweeps its base
## over the full height, but the extreme rows sit under whatever the
## platform does at the very edge of the frame, so the sweep is read across
## the middle 90% exactly as the desktop probe's base list is.
func _first_row(h: int) -> int:
	return int(0.05 * float(h))


func _last_row(h: int) -> int:
	return mini(int(0.95 * float(h)), h - 1)


## The base the write pass put at image row `y`. A fragment's SCREEN_UV.y is
## its pixel CENTRE, `(y + 0.5) / h`, and Godot's viewport image and
## SCREEN_UV share their top-down orientation. Diagnostic only: no verdict
## turns on it, so a flipped platform would misname a base, not miscount a
## level.
func _base_at(y: int, h: int) -> float:
	return lerpf(0.05, 0.95, (float(y) + 0.5) / float(h))


## The same verdicts the PNG carries, printed — so a desktop run needs no
## decoder, and so a web run leaves them in the browser console too.
##
## Returns the number of failed checks, because this scene is run by
## tools/probe_visibility.sh under `set -eu`: a measurement that cannot fail
## is a line of output, not a gate.
func _report() -> int:
	for _i: int in 8:
		await get_tree().process_frame
		await RenderingServer.frame_post_draw
	var img := get_viewport().get_texture().get_image()
	var w := img.get_width()
	var h := img.get_height()

	# EVERY ROW, not a sample of seventeen. The write pass sweeps the base
	# with y and the whole point of that sweep is the WORST base, so any
	# subsample can only report a level the buffer does not really hold.
	# Seventeen rows also made the verdict depend on the window size, since
	# `int(fraction * h)` picks different bases at 600 rows than at 720:
	# the same export on the same Apple GPU answered 1024 at one size and
	# 512 at another. A dense scan costs one image read either way.
	print(
		"# viewport: %d x %d (%d bases swept per band)" % [w, h, _last_row(h) - _first_row(h) + 1]
	)
	# The verdict: the SMALLEST step that survived at every base. Reading
	# upward and stopping at the first clean band would be wrong if the
	# ladder is not monotone, so every band is scanned and the smallest
	# clean one wins.
	var quantum := 0.0
	var dirtiest_clean := 0.0
	for band: int in BANDS:
		var mult := STEPS[band]
		var x := int((float(band) + 0.5) / float(BANDS) * 0.70 * float(w))
		var collapsed_at := -1
		var collapsed_last := -1
		var collapsed := 0
		for y: int in range(_first_row(h), _last_row(h) + 1):
			if img.get_pixel(x, y).r < 0.5:
				collapsed += 1
				collapsed_last = y
				if collapsed_at < 0:
					collapsed_at = y
		if collapsed_at < 0:
			print("#   step %.2f x 1/1023 survives at every base" % mult)
			if quantum == 0.0:
				quantum = mult * TEN_BIT
		else:
			# Where it broke is the diagnosis: a collapse only at bright
			# bases is a transfer curve, a handful scattered is the
			# quantisation grid one code coarser than the step, and one at
			# every base is a narrower buffer.
			dirtiest_clean = maxf(dirtiest_clean, mult)
			print(
				(
					"#   step %.2f x 1/1023 collapses at %d of %d bases, %.5f..%.5f"
					% [
						mult,
						collapsed,
						_last_row(h) - _first_row(h) + 1,
						_base_at(collapsed_at, h),
						_base_at(collapsed_last, h),
					]
				)
			)
	if quantum > 0.0 and dirtiest_clean > quantum / TEN_BIT:
		# A larger step failing where a smaller one passed cannot happen in
		# a quantiser. It means the ladder is measuring something else.
		print("# WARNING: the ladder is not monotone; a larger step failed where a smaller passed")

	var depth := img.get_pixel(int(0.775 * float(w)), h / 2).r
	var control := img.get_pixel(int(0.925 * float(w)), h / 2).r
	var expect := (NEAR / (FAR - NEAR)) * (FAR / WORLD_DIST - 1.0) * DEPTH_GAIN
	# NO LEVEL COUNT. The old ladder ended in one and it was the wrong
	# shape of answer twice over: a buffer depth cannot express a gap of
	# 1.25 codes, and "819 levels" is neither the format (1024) nor
	# anything `render::channel::CHANNEL_LEVELS` could be set from. What
	# the reconstruction guard clears is the WORST STEP, in nominal codes,
	# and that is what this reports.
	print(
		(
			(
				"# platform: worst step %.3f nominal codes (%.8f of full scale)"
				+ " ; depth %.4f (reversed-Z predicts %.4f) ; control %.4f"
			)
			% [quantum / TEN_BIT, quantum, depth, expect, control]
		)
	)
	print("# CONTROL must read %.2f — anything else voids every verdict above" % CONTROL)

	# THE GATE. The control decides whether anything above is admissible at
	# all — Godot's own readback is how this scene reaches its own pixels,
	# so a control that is not 0.5 voids the run rather than failing it.
	var failures := 0
	if absf(control - CONTROL) > 0.02:
		failures += 1
		print(
			(
				"not ok - the control reads %.4f, not %.2f: the readback is not working here"
				% [control, CONTROL]
			)
		)
		return failures
	print("ok - the control reads %.4f, so the readback is trustworthy" % control)

	# and the measurement itself, against what the renderer assumes. A
	# platform needing a WIDER step than render::channel derives its
	# tolerance from is one where a lit wall can read as a source seen
	# through one, which is a level-breaking fault rather than a note.
	var assumed: float = WaveCore.new().channel_worst_step()
	if quantum > 0.0 and quantum <= assumed:
		print(
			(
				"ok - the channel separates at %.3f nominal codes, inside the %.3f assumed"
				% [quantum / TEN_BIT, assumed / TEN_BIT]
			)
		)
	else:
		failures += 1
		print(
			(
				"not ok - the channel needs %.3f nominal codes but WORST_STEP_CODES assumes %.3f"
				% [quantum / TEN_BIT, assumed / TEN_BIT]
			)
		)
	print("1..2")
	if failures > 0:
		print("platform-probe: FAIL (%d)" % failures)
	else:
		print("platform-probe: PASS")
	return failures
