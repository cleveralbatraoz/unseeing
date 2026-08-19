extends Node
## What the screen texture actually preserves — the platform fact two
## rendering laws rest on and nobody had measured.
##
## `hearing_post` reconstructs a world point from the B channel and asks the
## wall table about it, and the only thing keeping that point outside the
## wall it stands on is that `sight::RECT_SHRINK` (0.05 m) exceeds B's own
## half-gap. That half-gap is divided by the BAND distance is packed into
## and not by the whole channel —
## `DIST_PACK_RANGE * WORST_STEP_CODES / (2 * (levels - 1) * BAND)` — because
## the pipeline's transfer destroys the bottom 45% of the codes. So the guard
## turns on a number the project had two contradictory stories about: the
## brief said 8-bit LDR, and one measurement claimed RGB10_A2. At 8 bits the
## guard is broken outright; at the measured 1.25 nominal codes over the
## measured band it is 44.3 mm and the shipped build clears it by 5.7.
##
## METHOD. Write two values into the data channels that differ by exactly
## one 10-bit step — 1/1023 — on either side of the screen. Sample both
## through `hint_screen_texture` and amplify their difference by 4000. If
## the buffer holds 10 bits the two survive as distinct codes and the result
## saturates white; if it holds 8, they quantise onto the same code and the
## result is black. The amplification is what makes an 8-bit SCREENSHOT
## able to report a 10-bit buffer: the difference is measured inside the
## shader, where the precision still exists, and only the verdict is
## screenshotted.
##
## The control is the same measurement at a step of 1/255, which every
## candidate format preserves — without it a black reading cannot be told
## from a probe that measured nothing at all.
##
## THE LADDER LIVES ELSEWHERE NOW. This probe used to walk nine bit depths
## across seventeen bases and end in a level count, and both halves of that
## were wrong: a subsample of the base column reports a step the buffer does
## not really preserve everywhere (it missed the two bases in 649 where a
## nominal 10-bit step collapses on Mesa/AMD), and a power-of-two ladder can
## only ever answer 512 or 1024. `platform_probe.tscn` does that measurement
## properly, at every base, in ONE frame — nine spatial bands instead of
## nine hundred readbacks. What is left here is the FORMAT question this
## probe is uniquely shaped for, plus a gate on the one step the renderer's
## tolerance is actually derived against.

const READ_SHADER := preload("res://tests/probe/shaders/probe_channel_read.gdshader")
const WRITE_SHADER := preload("res://tests/probe/shaders/probe_channel_write.gdshader")

var _failures := 0
var _write_mat: ShaderMaterial
var _read_mat: ShaderMaterial


func _ready() -> void:
	await get_tree().process_frame
	_build()
	await _measure()
	get_tree().quit(1 if _failures > 0 else 0)


## A camera, a quad that writes the pair, and a post quad that reads the
## screen texture back — the same three-part shape the game itself has.
func _build() -> void:
	var camera := Camera3D.new()
	camera.position = Vector3(0, 0, 2)
	add_child(camera)

	_write_mat = ShaderMaterial.new()
	_write_mat.shader = WRITE_SHADER
	var writer := MeshInstance3D.new()
	var quad := QuadMesh.new()
	quad.size = Vector2(8, 8)
	writer.mesh = quad
	writer.material_override = _write_mat
	writer.position = Vector3(0, 0, -1)
	add_child(writer)

	_read_mat = ShaderMaterial.new()
	_read_mat.shader = READ_SHADER
	var reader := MeshInstance3D.new()
	var post := QuadMesh.new()
	post.size = Vector2(2, 2)
	reader.mesh = post
	reader.extra_cull_margin = 16384.0
	reader.material_override = _read_mat
	reader.position = Vector3(0, 0, -1)
	camera.add_child(reader)


## The verdict pixel, read at the centre of the screen after a settle.
func _read_verdict() -> float:
	var peak := 0.0
	for _i: int in 8:
		await get_tree().process_frame
		await RenderingServer.frame_post_draw
		var img := get_viewport().get_texture().get_image()
		peak = maxf(peak, img.get_pixel(img.get_width() / 2, img.get_height() / 2).r)
	return peak


func _measure() -> void:
	var base := 0.5

	# CONTROL: a step every candidate format preserves. A black reading here
	# means the probe is measuring nothing and every verdict below is void.
	_write_mat.set_shader_parameter("u_lo", base)
	_write_mat.set_shader_parameter("u_hi", base + 1.0 / 255.0)
	var eight := await _read_verdict()
	_check("an 8-bit step SURVIVES the screen texture (%.3f > 0.5)" % eight, eight > 0.5)

	# THE MEASUREMENT: one 10-bit step.
	_write_mat.set_shader_parameter("u_lo", base)
	_write_mat.set_shader_parameter("u_hi", base + 1.0 / 1023.0)
	var ten := await _read_verdict()

	# and a NULL: no step at all must read black, or the amplifier is
	# reporting noise rather than a difference
	_write_mat.set_shader_parameter("u_lo", base)
	_write_mat.set_shader_parameter("u_hi", base)
	var none := await _read_verdict()
	_check("no step at all reads BLACK (%.3f < 0.5)" % none, none < 0.5)

	print("# channel: 8-bit step %.3f ; 10-bit step %.3f ; no step %.3f" % [eight, ten, none])

	# THE PRECISION, measured as a WORST CASE over the whole channel rather
	# than at one convenient value.
	#
	# THE GATE: the step the renderer's tolerance is derived against must
	# separate at every base this probe can reach.
	#
	# A single base lies at one arbitrary place on the quantisation grid,
	# and that alone can move a verdict: 0.5 x 1023 = 511.5 sits exactly
	# between two 10-bit codes, so HALF a code still crosses a boundary
	# there, while 0.25 x 1023 = 255.75 does not. Measured at fixed bases
	# this channel reported 2^-11 at 0.5 and 2^-10 at 0.25 — the same
	# buffer, two answers. So the step is swept across the channel and
	# every base must separate.
	#
	# The step itself comes from Rust rather than a literal here, because a
	# platform gate that carries its own copy of the number it is gating on
	# can drift from the renderer while both keep passing.
	var worst_step: float = WaveCore.new().channel_worst_step()
	var nominal := 1.0 / 1023.0
	var bases: PackedFloat64Array = PackedFloat64Array()
	for i: int in 17:
		bases.append(0.05 + 0.9 * float(i) / 16.0)
	var separated := 0
	for probe_base: float in bases:
		_write_mat.set_shader_parameter("u_lo", probe_base)
		_write_mat.set_shader_parameter("u_hi", probe_base + worst_step)
		if await _read_verdict() > 0.5:
			separated += 1
	print(
		(
			"# channel: the assumed worst step %.8f (%.3f nominal codes) separated at %d/%d bases"
			% [worst_step, worst_step / nominal, separated, bases.size()]
		)
	)
	_check(
		(
			"the step render::channel derives its tolerance from separates at every base (%d/%d)"
			% [separated, bases.size()]
		),
		separated == bases.size()
	)
	print("1..%d" % 3)
	if _failures > 0:
		print("channel-probe: FAIL (%d)" % _failures)
	else:
		print("channel-probe: PASS")


func _check(label: String, ok: bool) -> void:
	if ok:
		print("ok - %s" % label)
	else:
		_failures += 1
		print("not ok - %s" % label)
