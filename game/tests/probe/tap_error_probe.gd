extends Node
## How far a single value moves on its way through one data channel — the
## number `render::channel::recon_eps` asserts rather than measures.
##
## `platform_probe` measures a gap WIDTH: the smallest step two values must
## be apart to come back distinct, 1.25 nominal codes on Mesa/AMD desktop
## GL. `recon_eps` halves that and calls it the worst error in a single
## value, which holds for a quantiser that rounds to the nearest cell
## CENTRE. The 1.25 figure exists because this pipeline is not one. If the
## representative sits at a cell EDGE, every budget derived from the
## half-gap is out by a factor of two — `sight::RECT_SHRINK` would have to
## clear 48.9 mm rather than the 24.4 mm it was raised to.
##
## METHOD. Write ONE constant across the whole screen, read one texel of it
## back through `hint_screen_texture`, and take the difference against the
## same constant, inside the shader where the precision still exists. Sweep
## the constant across the channel, because a single value sits at one
## arbitrary place on the quantisation grid and would answer for that place
## alone. Report the worst signed error over the sweep, in nominal codes.
##
## The sweep is deliberately DENSE and deliberately includes the awkward
## bases: 0.25 and 0.5 are the two the platform ladder found a nominal step
## collapsing at.

const READ_SHADER := preload("res://tests/probe/shaders/probe_tap_error_read.gdshader")
const WRITE_SHADER := preload("res://tests/probe/shaders/probe_channel_write.gdshader")

## Nominal codes represented either side of zero. A reading that clips is
## reported as clipped rather than as its clipped value.
const SPAN := 32.0
## Tolerances the SAFE FLOOR is reported against, ascending. The floor for a
## tolerance is the highest written value that still misses it by more —
## everything above is inside. This is the number the packing law needs: the
## channel is only usable above it, so distance must be mapped into
## `[floor, 1]` rather than into `[0, 1]`.
const TOLS: Array[float] = [0.50, 1.00, 1.25, 1.50, 2.00, 3.00]
const SWEEP := 97

var _failures := 0
var _write_mat: ShaderMaterial
var _read_mat: ShaderMaterial


func _ready() -> void:
	await get_tree().process_frame
	_build()
	await _measure()
	get_tree().quit(1 if _failures > 0 else 0)


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
	_read_mat.set_shader_parameter("u_span", SPAN)
	var reader := MeshInstance3D.new()
	var post := QuadMesh.new()
	post.size = Vector2(2, 2)
	reader.mesh = post
	reader.extra_cull_margin = 16384.0
	reader.material_override = _read_mat
	reader.position = Vector3(0, 0, -1)
	camera.add_child(reader)


## The signed error at one written value, in nominal codes. Settles first:
## the pass has to reach the screen before the screen can be read back.
func _error_at(value: float) -> float:
	_write_mat.set_shader_parameter("u_lo", value)
	_write_mat.set_shader_parameter("u_hi", value)
	# the SAME value the write pass is putting on the screen: the reference
	# is what the error is measured against, and leaving it at the shader
	# default measures the sweep instead of the channel
	_read_mat.set_shader_parameter("u_ref", value)
	for _i: int in 6:
		await get_tree().process_frame
		await RenderingServer.frame_post_draw
	var img := get_viewport().get_texture().get_image()
	var level := img.get_pixel(img.get_width() / 2, img.get_height() / 2).r
	return (level - 0.5) * 2.0 * SPAN


func _measure() -> void:
	# The WHOLE channel, not the top two thirds. The earlier sweep started
	# at 0.30 and so could not see the part that matters most: measured on
	# the shipped data pass, a wall 1 m from the eye reads back as 0 m,
	# because everything below about 28 nominal codes comes back as exactly
	# zero. Distance is packed as `vd / 40`, so 1 m IS 0.025.
	var values: PackedFloat64Array = PackedFloat64Array()
	for i: int in SWEEP:
		values.append(0.005 + 0.99 * float(i) / float(SWEEP - 1))
	# the exactly-representable values, which a grid aligned to 1/1023 would
	# return untouched
	for k: int in [8, 24, 32, 64, 256, 400, 512, 700, 900]:
		values.append(float(k) / 1023.0)
	values.sort()

	var worst := 0.0
	var worst_at := 0.0
	# the worst error among values the packing law can actually write —
	# at or above the shipped band floor — which is the number the
	# reconstruction guard composes against
	var band_floor: float = WaveCore.new().dist_safe_floor()
	var in_band_worst := 0.0
	var shape: PackedStringArray = PackedStringArray()
	var i := 0
	var clipped := 0
	var sum := 0.0
	var floor_at: Array[float] = []
	for tol: float in TOLS:
		floor_at.append(-1.0)
	for value: float in values:
		var codes: float = await _error_at(value)
		if absf(codes) >= SPAN - 0.02:
			clipped += 1
		sum += codes
		if absf(codes) > absf(worst):
			worst = codes
			worst_at = value
		if value >= band_floor and absf(codes) > absf(in_band_worst):
			in_band_worst = codes
		for t: int in TOLS.size():
			if absf(codes) > TOLS[t]:
				floor_at[t] = value
		if i % 12 == 0:
			shape.append("%.3f:%+.2f" % [value, codes])
		i += 1
	var floors: PackedStringArray = PackedStringArray()
	for t: int in TOLS.size():
		floors.append(
			"%.2f above %s" % [TOLS[t], "everywhere" if floor_at[t] < 0.0 else "%.4f" % floor_at[t]]
		)
	print("# SAFE FLOOR by tolerance (nominal codes): %s" % " ; ".join(floors))
	print(
		(
			"# tap error over %d values: worst %+.3f codes at base %.4f ; mean %+.3f ; %d clipped"
			% [values.size(), worst, worst_at, sum / float(values.size()), clipped]
		)
	)

	print("# shape (base:codes) %s" % " ".join(shape))
	_check("no reading clipped the +-%.1f code window (%d clipped)" % [SPAN, clipped], clipped == 0)
	# WHAT THIS PROBE CAN HONESTLY ASSERT. The error is bounded, and that
	# bound is the one thing here that is not in question.
	# WHAT THIS CAN HONESTLY GATE ON. Not "the error is small" — it is not,
	# and saying so was only possible while the sweep began at 0.30 and
	# could not see the bottom of the channel. What must hold for the
	# channel to be usable at all is that a band exists where the error is
	# within one measured gap: distance is mapped into that band, so if it
	# vanished the packing law would have nowhere to go.
	var usable: float = floor_at[TOLS.find(1.25)]
	_check(
		(
			"a usable band exists: the channel is inside 1.25 codes above %.4f, leaving %.0f%%"
			% [usable, (1.0 - usable) * 100.0]
		),
		usable >= 0.0 and usable < 0.90
	)
	# THE MEASUREMENT THE GUARD IS BUILT ON, gated as well as printed.
	# Absolute in-band error runs past the half-gap a centred representative
	# would give -- up to about 1.6 nominal codes -- and that is NOT a
	# contradiction of platform_probe, which measures RESOLUTION: a slowly
	# varying bias moves every value together and so preserves every step
	# while breaking absolute accuracy. Every reconstructed wall-test
	# endpoint goes through probe_distance (one whole gap of bias toward
	# the eye) with sight::RECT_SHRINK absorbing the excess. The guard is
	# DERIVED through the recon_guard_codes door (1.955 at the shipped
	# constants, render::channel::guard_codes), never pasted here, so a
	# retune of either constant moves this gate with it — the pre-merge
	# review caught the pasted ladder going stale. A driver whose in-band
	# error beats the guard re-opens the tapped-wall flare occlusion_probe
	# case 8 hard-gates in the same suite run; this check names the number.
	var guard: float = WaveCore.new().recon_guard_codes()
	_check(
		(
			(
				"the in-band error stays inside the composed reconstruction guard"
				+ " (%.3f < %.3f codes). Resolution is unaffected."
			)
			% [absf(in_band_worst), guard]
		),
		absf(in_band_worst) < guard
	)
	print("1..3")
	if _failures > 0:
		print("tap-error-probe: FAIL (%d)" % _failures)
	else:
		print("tap-error-probe: PASS")


func _check(label: String, ok: bool) -> void:
	if ok:
		print("ok - %s" % label)
	else:
		_failures += 1
		print("not ok - %s" % label)
