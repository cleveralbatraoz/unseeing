extends GdUnitTestSuite
## The wave pool's laws: packing, lifetimes, eviction, and the no-physics
## degradation path. Ported 1:1 from the retired custom runner.


## The shader decodes type/gain from dat.w as floor(w/10) and mod(w,10)/9 —
## verify emit() packs exactly what that decode expects.
func test_packing_roundtrip() -> void:
	var p := Pulses.new()
	p.emit(0, Vector3.ZERO, 6.0, 5.5, 1.0, 10.0)
	p.emit(2, Vector3.ONE, 1.6, 4.0, 0.8, 10.0)
	var w0 := p.dat[0].w
	var w1 := p.dat[1].w
	assert_int(int(floorf(w0 / 10.0))).is_equal(0)
	assert_float(fmod(w0, 10.0) / 9.0).is_equal_approx(1.0, 0.001)
	assert_int(int(floorf(w1 / 10.0))).is_equal(2)
	assert_float(fmod(w1, 10.0) / 9.0).is_equal_approx(0.8, 0.001)
	assert_float(p.dat[0].x).is_equal(10.0)
	assert_float(p.dat[0].y).is_equal(6.0)
	assert_float(p.dat[0].z).is_equal(5.5)


## Echoes and footsteps must expire sooner than cane taps: the live-slot
## count drives per-pixel shader cost.
func test_per_type_lifetimes() -> void:
	var p := Pulses.new()
	p.emit(0, Vector3.ZERO, 5.5, 5.5, 1.0, 0.0)  # tap: ring 1s + 6s tail
	p.emit(1, Vector3.ZERO, 5.5, 5.5, 1.0, 0.0)  # echo: ring 1s + 3.5s tail
	p.emit(2, Vector3.ZERO, 5.5, 5.5, 1.0, 0.0)  # step: ring 1s + 2.5s tail
	assert_int(p.live_count(3.0)).is_equal(3)
	assert_int(p.live_count(4.0)).is_equal(2)
	assert_int(p.live_count(5.0)).is_equal(1)
	assert_int(p.live_count(8.0)).is_equal(0)


## When the pool is full, the oldest footstep is evicted before anything
## precious (taps) is touched.
func test_eviction_prefers_footsteps() -> void:
	var p := Pulses.new()
	for i: int in Pulses.MAXP:
		var type := 2 if i == 10 else 0
		p.emit(type, Vector3(i, 0, 0), 6.0, 5.5, 1.0, 100.0 + i * 0.001)
	p.emit(0, Vector3(999, 0, 0), 6.0, 5.5, 1.0, 101.0)
	assert_vector(p.pos[10]).is_equal(Vector3(999, 0, 0))
	assert_vector(p.pos[0]).is_equal(Vector3(0, 0, 0))


func test_live_count_is_highest_slot() -> void:
	var p := Pulses.new()
	assert_int(p.live_count(0.0)).is_equal(0)
	p.emit(0, Vector3.ZERO, 6.0, 5.5, 1.0, 0.0)
	p.emit(0, Vector3.ZERO, 6.0, 5.5, 1.0, 0.0)
	assert_int(p.live_count(0.5)).is_equal(2)


## emit_reflecting with no physics space must emit the primary and schedule
## nothing — the web/CI-safe degradation path.
func test_null_space_schedules_no_echoes() -> void:
	var p := Pulses.new()
	p.emit_reflecting(0, Vector3.ZERO, 6.0, 5.5, 1.0, 0.0, null, 6, Vector3.UP)
	assert_int(p.live_count(0.1)).is_equal(1)
	assert_int(p.pending_echo_count()).is_equal(0)


## apply() is the only bridge from the CPU pool to the GPU: it must push the
## live count and all three uniform arrays into every wave material. A real
## ShaderMaterial with the real data-pass shader — parameter storage needs
## no GPU, so this runs headless.
func test_apply_pushes_pool_to_materials() -> void:
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/data_pass.gdshader")
	var p := Pulses.new()
	p.emit(0, Vector3(1, 2, 3), 6.0, 5.5, 1.0, 10.0)
	p.emit(2, Vector3(4, 5, 6), 1.6, 4.0, 0.8, 10.0)
	p.apply(10.5, [mat])
	var count: int = mat.get_shader_parameter("u_count")
	assert_int(count).is_equal(2)
	var ppos: PackedVector3Array = mat.get_shader_parameter("u_ppos")
	assert_vector(ppos[0]).is_equal(Vector3(1, 2, 3))
	assert_vector(ppos[1]).is_equal(Vector3(4, 5, 6))
	var pdat: PackedVector4Array = mat.get_shader_parameter("u_pdat")
	assert_float(pdat[0].x).is_equal(10.0)
	assert_float(pdat[0].y).is_equal(6.0)
	assert_float(pdat[0].z).is_equal(5.5)
	var pdir: PackedVector4Array = mat.get_shader_parameter("u_pdir")
	assert_float(pdir[0].w).is_equal(-2.0)


## Total at the door: gain outside [0, 1] is clamped before packing. A raw
## gain of -1 would bleed into the type digits (floor(w/10) reads one type
## lower); the clamp keeps the type field undamaged in both directions.
func test_gain_clamped_into_pack() -> void:
	var p := Pulses.new()
	p.emit(2, Vector3.ZERO, 6.0, 5.5, 1.5, 0.0)
	p.emit(2, Vector3.ZERO, 6.0, 5.5, -1.0, 0.0)
	assert_int(int(floorf(p.dat[0].w / 10.0))).is_equal(2)
	assert_float(fmod(p.dat[0].w, 10.0) / 9.0).is_equal_approx(1.0, 0.001)
	assert_int(int(floorf(p.dat[1].w / 10.0))).is_equal(2)
	assert_float(fmod(p.dat[1].w, 10.0) / 9.0).is_equal_approx(0.0, 0.001)


## A zero-speed wave would divide by zero into an immortal slot (end = now +
## max_r / 0); a zero-radius wave is no sound at all. emit refuses both
## loudly and takes no slot.
func test_non_positive_speed_or_radius_refused() -> void:
	var p := Pulses.new()
	var zero_speed := func() -> void: p.emit(0, Vector3.ZERO, 6.0, 0.0, 1.0, 0.0)
	await assert_error(zero_speed).is_push_error(
		"Pulses.emit: speed and max_r must be positive — wave refused"
	)
	var zero_radius := func() -> void: p.emit(0, Vector3.ZERO, 0.0, 5.5, 1.0, 0.0)
	await assert_error(zero_radius).is_push_error(
		"Pulses.emit: speed and max_r must be positive — wave refused"
	)
	assert_int(p.live_count(0.1)).is_equal(0)
	assert_int(p.live_count(1.0e9)).is_equal(0)  # nothing immortal left behind


## Slot reuse prefers the dead: with an expired footstep in slot 0 and a
## still-live tap in slot 1, a new emit lands in slot 0.
func test_expired_slot_reused_first() -> void:
	var p := Pulses.new()
	p.emit(2, Vector3(1, 0, 0), 1.6, 4.0, 0.8, 0.0)  # dead by t = 2.9
	p.emit(0, Vector3(2, 0, 0), 6.0, 5.5, 1.0, 0.0)  # lives past t = 7
	p.emit(0, Vector3(3, 0, 0), 6.0, 5.5, 1.0, 5.0)
	assert_vector(p.pos[0]).is_equal(Vector3(3, 0, 0))
	assert_vector(p.pos[1]).is_equal(Vector3(2, 0, 0))


## All 64 slots hold live taps — nothing cheap to sacrifice: the oldest tap
## goes.
func test_full_tap_pool_evicts_oldest_tap() -> void:
	var p := Pulses.new()
	for i: int in Pulses.MAXP:
		p.emit(0, Vector3(i, 0, 0), 6.0, 5.5, 1.0, 100.0 + i * 0.001)
	p.emit(0, Vector3(999, 0, 0), 6.0, 5.5, 1.0, 100.1)
	assert_vector(p.pos[0]).is_equal(Vector3(999, 0, 0))
	assert_vector(p.pos[1]).is_equal(Vector3(1, 0, 0))


## A hum (type 3) recurs every second, so it is less precious than any tap:
## with the pool full it is sacrificed even when it is not the oldest slot.
func test_old_hum_sacrificed_before_taps() -> void:
	var p := Pulses.new()
	for i: int in Pulses.MAXP:
		var type := 3 if i == 7 else 0
		p.emit(type, Vector3(i, 0, 0), 6.0, 5.5, 1.0, 100.0 + i * 0.001)
	p.emit(0, Vector3(999, 0, 0), 6.0, 5.5, 1.0, 100.1)
	assert_vector(p.pos[7]).is_equal(Vector3(999, 0, 0))
	assert_vector(p.pos[0]).is_equal(Vector3(0, 0, 0))  # oldest tap untouched


## live_count is the shader's loop bound, not a census: a dead low slot under
## a live high slot still yields high + 1 — holes are spanned, never skipped.
func test_live_count_spans_holes() -> void:
	var p := Pulses.new()
	p.emit(2, Vector3.ZERO, 1.6, 4.0, 0.8, 0.0)  # slot 0: dead by t = 2.9
	p.emit(0, Vector3.ZERO, 6.0, 5.5, 1.0, 0.0)  # slot 1: lives past t = 7
	assert_int(p.live_count(5.0)).is_equal(2)


## The fan's hum is type 3: omnidirectional, short-tailed (it recurs every
## second, so slots must free fast), and packed like every other pulse.
func test_hum_pulses() -> void:
	var p := Pulses.new()
	p.emit(3, Vector3(8.6, 1.15, 4.4), 9.0, 4.5, 0.75, 0.0)
	assert_int(int(floorf(p.dat[0].w / 10.0))).is_equal(3)
	assert_bool(p.dir[0].w < -1.5).is_true()
	# ring time 9/4.5 = 2s, tail 2s -> gone just after 4s
	assert_int(p.live_count(3.9)).is_equal(1)
	assert_int(p.live_count(4.1)).is_equal(0)
