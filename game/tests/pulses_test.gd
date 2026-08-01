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
	assert_int(int(floor(w0 / 10.0))).is_equal(0)
	assert_float(fmod(w0, 10.0) / 9.0).is_equal_approx(1.0, 0.001)
	assert_int(int(floor(w1 / 10.0))).is_equal(2)
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


## The fan's hum is type 3: omnidirectional, short-tailed (it recurs every
## second, so slots must free fast), and packed like every other pulse.
func test_hum_pulses() -> void:
	var p := Pulses.new()
	p.emit(3, Vector3(8.6, 1.15, 4.4), 9.0, 4.5, 0.75, 0.0)
	assert_int(int(floor(p.dat[0].w / 10.0))).is_equal(3)
	assert_bool(p.dir[0].w < -1.5).is_true()
	# ring time 9/4.5 = 2s, tail 2s -> gone just after 4s
	assert_int(p.live_count(3.9)).is_equal(1)
	assert_int(p.live_count(4.1)).is_equal(0)
