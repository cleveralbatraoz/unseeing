extends Node
## What a frame of the shipped game actually costs, measured rather than felt.
##
## Every other probe here answers a question about correctness. This one
## answers the only question that has no right answer, only a budget: how
## long a frame takes, and which half of the engine spends it. It boots the
## SHIPPED level through the same composition root `main.tscn` uses, lets it
## settle, and then samples Godot's own monitors for a fixed number of
## frames.
##
## WHY THE RENDER-TIME MONITORS AND NOT A STOPWATCH IN `_process`. A timer
## around our own `process` measures the CPU half and is blind to the GPU,
## which is where the per-fragment sight loop lives — and that loop is the
## one cost in this renderer that grows with the level. Godot exposes the
## split directly: `viewport_set_measure_render_time` turns on a per-viewport
## timer whose GPU half is a real query on the graphics queue, and that is
## the number this probe exists to print.
##
## WHAT THIS PROBE USED TO REPORT, recorded so the numbers below are not
## read as a continuation of the old ones. `_gpu` held
## `RENDER_TOTAL_DRAW_CALLS_IN_FRAME` — a COUNT, printed under a label that
## promised milliseconds — so the one attribution the probe existed to make
## was the one it never made. `TIME_PROCESS` was run through percentiles
## despite refreshing about once a second, so its p50/p95/p99 were
## percentiles of a handful of distinct values.
##
## Measured on AMD Radeon (radeonsi, raphael_mendocino) / Mesa 25.0.7 /
## GL Compatibility, 240 frames after 90 settling:
##   viewport       GPU p50    render-CPU p50   frame delta p50
##   320x180         0.909 ms      0.142 ms         4.998 ms
##   1280x720       10.194 ms      0.145 ms        10.799 ms
##   1920x1080      30.270 ms      0.158 ms        30.922 ms
##
## Three things follow, and only the first was visible before. The renderer
## is fragment-bound: fitting GPU time alone through 320x180 and 720p gives
## 10.75 ms per megapixel with a +0.29 ms intercept, so it has essentially
## no fixed cost. At the shipped fullscreen size the GPU holds 30.27 of the
## 30.92 ms frame — 97.9% of the critical path — and every CPU cost in the
## game together measures about 1.4 ms. And the ~4 ms floor visible at
## 320x180 is engine frame pacing, not renderer cost: fitting a cost model
## against frame DELTA rather than GPU time charges the renderer 2-3 ms that
## no resolution setting can buy back.
##
## NOT REPORTED HERE: a per-loop attribution. An earlier version of this
## comment split the 1080p frame into a wall loop and a ring loop by
## switching each off in turn. That method does not hold — the live pulse
## count climbs on its own with no player input, so the baseline drifts by
## more than the difference being measured, and zeroing `u_wall_count` does
## not merely delete the wall loop, it deletes wave occlusion, so MORE roots
## survive to reach the expensive noise and tail. Any future attribution
## needs interleaved blocks, a pinned pulse count, and a shader boolean that
## skips a loop body rather than a count that changes what is drawn.
##
const SETTLE_FRAMES := 60
const SAMPLE_FRAMES := 120

var _cpu: PackedFloat64Array = PackedFloat64Array()
var _gpu: PackedFloat64Array = PackedFloat64Array()
var _total: PackedFloat64Array = PackedFloat64Array()
var _wall := 0.0
var _peak_pulses := 0


func _ready() -> void:
	await get_tree().process_frame
	var scene: PackedScene = load("res://scenes/main.tscn")
	var main: Node = scene.instantiate()
	add_child(main)
	print("# main instantiated")
	await _sample()
	_report()
	get_tree().quit(0)


## Settle first: shader compilation, the first pool fill and the level's own
## derive all land in the opening frames, and folding them into the sample
## would report a cost no steady frame ever pays.
func _sample() -> void:
	var vp := get_viewport().get_viewport_rid()
	# The GPU half is a real timer query on the graphics queue. Reading it
	# can force a sync, and a sync changes the thing being measured — so it
	# is switchable, and the probe reports WALL CLOCK per frame beside the
	# engine's own delta so a stall cannot hide between them. If the two
	# disagree, the GPU column is not trustworthy and the run says so.
	var measure := OS.get_environment("UNSEEING_NO_GPU_TIME") == ""
	if measure:
		RenderingServer.viewport_set_measure_render_time(vp, true)
	for _i: int in SETTLE_FRAMES:
		await get_tree().process_frame
	print("# settled")
	var began := Time.get_ticks_usec()
	for _i: int in SAMPLE_FRAMES:
		await get_tree().process_frame
		if measure:
			_gpu.append(RenderingServer.viewport_get_measured_render_time_gpu(vp))
			_cpu.append(RenderingServer.viewport_get_measured_render_time_cpu(vp))
		_total.append(get_process_delta_time() * 1000.0)
		# the scene is NOT stationary: the live pulse count climbs with no
		# player input, and it is the second axis the fragment cost scales
		# on, so a reading without it cannot be compared to another run
		_peak_pulses = maxi(
			_peak_pulses, int(Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME))
		)
	_wall = float(Time.get_ticks_usec() - began) / 1.0e6
	if measure:
		RenderingServer.viewport_set_measure_render_time(vp, false)


func _sorted(samples: PackedFloat64Array) -> PackedFloat64Array:
	var copy := samples.duplicate()
	copy.sort()
	return copy


func _percentile(sorted_ms: PackedFloat64Array, q: float) -> float:
	if sorted_ms.is_empty():
		return 0.0
	var idx := int(clampf(q, 0.0, 1.0) * float(sorted_ms.size() - 1))
	return sorted_ms[idx]


func _line(label: String, samples: PackedFloat64Array) -> void:
	var sorted_ms := samples.duplicate()
	sorted_ms.sort()
	print(
		(
			"# %-22s p50 %7.3f  p95 %7.3f  p99 %7.3f  max %7.3f"
			% [
				label,
				_percentile(sorted_ms, 0.50),
				_percentile(sorted_ms, 0.95),
				_percentile(sorted_ms, 0.99),
				_percentile(sorted_ms, 1.0),
			]
		)
	)


func _report() -> void:
	print(
		(
			"# frames: %d sampled after %d settling, over %.2f s of wall clock"
			% [SAMPLE_FRAMES, SETTLE_FRAMES, _wall]
		)
	)
	_line("GPU (ms)", _gpu)
	_line("render CPU (ms)", _cpu)
	_line("frame delta (ms)", _total)
	# The arbiter. The engine's delta is what IT believes a frame cost; this
	# is what the clock says. They must agree, and when they do not the run
	# is measuring its own instrument.
	print(
		(
			"# wall clock per frame: %.3f ms  (engine delta p50 %.3f ms)"
			% [
				_wall * 1000.0 / float(SAMPLE_FRAMES),
				_percentile(_sorted(_total), 0.50),
			]
		)
	)
	print(
		(
			"# objects %d ; primitives %d ; video mem %.1f MiB"
			% [
				int(Performance.get_monitor(Performance.RENDER_TOTAL_OBJECTS_IN_FRAME)),
				int(Performance.get_monitor(Performance.RENDER_TOTAL_PRIMITIVES_IN_FRAME)),
				Performance.get_monitor(Performance.RENDER_VIDEO_MEM_USED) / 1048576.0,
			]
		)
	)
	print("# peak draw calls in a sampled frame: %d" % _peak_pulses)
	print("frame-time-probe: DONE")
