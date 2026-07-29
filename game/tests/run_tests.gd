extends SceneTree
## Dependency-free headless test runner for pure game logic:
##   godot --headless --path game -s res://tests/run_tests.gd
## Exits non-zero on any failure, so shell pipelines can gate on it.
## Physics-dependent behavior (raycasts, movement) is covered by the
## browser smoke test and the movie-maker demo instead.

var _passes := 0
var _fails := 0

func _init() -> void:
	_test_packing_roundtrip()
	_test_per_type_lifetimes()
	_test_eviction_prefers_footsteps()
	_test_live_count_is_highest_slot()
	_test_null_space_schedules_no_echoes()
	_test_map_segments_axis_aligned()
	_test_hum_pulses()
	_test_fan_motion_envelope()
	print("tests: %d passed, %d failed" % [_passes, _fails])
	quit(1 if _fails > 0 else 0)

func check(cond: bool, name: String) -> void:
	if cond:
		_passes += 1
		print("PASS  ", name)
	else:
		_fails += 1
		print("FAIL  ", name)

## The shader decodes type/gain from dat.w as floor(w/10) and mod(w,10)/9 —
## verify emit() packs exactly what that decode expects.
func _test_packing_roundtrip() -> void:
	var p := Pulses.new()
	p.emit(0, Vector3.ZERO, 6.0, 5.5, 1.0, 10.0)
	p.emit(2, Vector3.ONE, 1.6, 4.0, 0.8, 10.0)
	var w0 := p.dat[0].w
	var w1 := p.dat[1].w
	check(int(floor(w0 / 10.0)) == 0, "packing: type 0 decodes as 0")
	check(absf(fmod(w0, 10.0) / 9.0 - 1.0) < 0.001, "packing: gain 1.0 roundtrips")
	check(int(floor(w1 / 10.0)) == 2, "packing: type 2 decodes as 2")
	check(absf(fmod(w1, 10.0) / 9.0 - 0.8) < 0.001, "packing: gain 0.8 roundtrips")
	check(p.dat[0].x == 10.0 and p.dat[0].y == 6.0 and p.dat[0].z == 5.5,
			"packing: birth/maxR/speed stored verbatim")

## Echoes and footsteps must expire sooner than cane taps: the live-slot
## count drives per-pixel shader cost.
func _test_per_type_lifetimes() -> void:
	var p := Pulses.new()
	p.emit(0, Vector3.ZERO, 5.5, 5.5, 1.0, 0.0)   # tap: ring 1s + 6s tail
	p.emit(1, Vector3.ZERO, 5.5, 5.5, 1.0, 0.0)   # echo: ring 1s + 3.5s tail
	p.emit(2, Vector3.ZERO, 5.5, 5.5, 1.0, 0.0)   # step: ring 1s + 2.5s tail
	check(p.live_count(3.0) == 3, "lifetimes: all three alive at 3s")
	check(p.live_count(4.0) == 2, "lifetimes: footstep expired by 4s")
	check(p.live_count(5.0) == 1, "lifetimes: echo expired by 5s")
	check(p.live_count(8.0) == 0, "lifetimes: tap expired by 8s")

## When the pool is full, the oldest footstep is evicted before anything
## precious (taps) is touched.
func _test_eviction_prefers_footsteps() -> void:
	var p := Pulses.new()
	for i: int in Pulses.MAXP:
		var type := 2 if i == 10 else 0
		p.emit(type, Vector3(i, 0, 0), 6.0, 5.5, 1.0, 100.0 + i * 0.001)
	p.emit(0, Vector3(999, 0, 0), 6.0, 5.5, 1.0, 101.0)
	check(p.pos[10] == Vector3(999, 0, 0), "eviction: footstep slot reused first")
	check(p.pos[0] == Vector3(0, 0, 0), "eviction: oldest tap untouched")

func _test_live_count_is_highest_slot() -> void:
	var p := Pulses.new()
	check(p.live_count(0.0) == 0, "live_count: empty pool is 0")
	p.emit(0, Vector3.ZERO, 6.0, 5.5, 1.0, 0.0)
	p.emit(0, Vector3.ZERO, 6.0, 5.5, 1.0, 0.0)
	check(p.live_count(0.5) == 2, "live_count: two live slots -> 2")

## emit_reflecting with no physics space must emit the primary and schedule
## nothing — the web/CI-safe degradation path.
func _test_null_space_schedules_no_echoes() -> void:
	var p := Pulses.new()
	p.emit_reflecting(0, Vector3.ZERO, 6.0, 5.5, 1.0, 0.0, null, 6, Vector3.UP)
	check(p.live_count(0.1) == 1, "null space: primary emitted")
	check(p._echoes.size() == 0, "null space: no echoes scheduled")

## The fan's hum is type 3: omnidirectional, short-tailed (it recurs every
## second, so slots must free fast), and packed like every other pulse.
func _test_hum_pulses() -> void:
	var p := Pulses.new()
	p.emit(3, Vector3(8.6, 1.15, 4.4), 9.0, 4.5, 0.75, 0.0)
	check(int(floor(p.dat[0].w / 10.0)) == 3, "hum: type 3 packs")
	check(p.dir[0].w < -1.5, "hum: omnidirectional")
	# ring time 9/4.5 = 2s, tail 2s -> gone just after 4s
	check(p.live_count(3.9) == 1 and p.live_count(4.1) == 0,
			"hum: expires at ring + 2s tail")

## The head's oscillation must actually sweep, and never exceed its range —
## the collider rides the same curve, so this is also a physics bound.
func _test_fan_motion_envelope() -> void:
	var lo := 0.0
	var hi := 0.0
	for i: int in 200:
		var a := SoundFan.pivot_angle(float(i) * 0.1)
		lo = minf(lo, a)
		hi = maxf(hi, a)
	check(hi <= SoundFan.PIVOT_RANGE + 0.001 and lo >= -SoundFan.PIVOT_RANGE - 0.001,
			"fan: pivot never exceeds its sweep")
	check(hi > SoundFan.PIVOT_RANGE * 0.9 and lo < -SoundFan.PIVOT_RANGE * 0.9,
			"fan: pivot sweeps fully both ways")
	check(SoundFan.spin_angle(1.0) != SoundFan.spin_angle(1.1), "fan: blades spin")

## The map builder's box math trusts axis alignment.
func _test_map_segments_axis_aligned() -> void:
	var ok := true
	for s: Array in MapBuilder.SEGS:
		if absf(s[3] - s[1]) >= 0.001 and absf(s[2] - s[0]) >= 0.001:
			ok = false
	check(ok, "map: all wall segments axis-aligned")
	check(MapBuilder.SEGS.size() >= 4, "map: border walls present")
