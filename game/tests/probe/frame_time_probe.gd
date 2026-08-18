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
## WHY MONITORS AND NOT A STOPWATCH IN `_process`. A timer around our own
## `process` measures the CPU half and is blind to the GPU, which is where
## the per-fragment sight loop lives — and that loop is the one cost in this
## renderer that grows with the level.
##
## `TIME_PROCESS` is reported for context and is NOT a per-frame reading:
## the engine refreshes that monitor on its own slow cadence, which shows up
## here as p95, p99 and max landing on the same value. The frame DELTA is
## the honest series, and the way to split it is to run this at several
## viewport sizes: a cost that scales with pixels is the fragment passes and
## a cost that does not is everything else.
##
## Measured that way on AMD Radeon / Mesa 25.0, at 240 frames each:
##   640x360    5.00 ms p50
##   1280x720  10.69 ms p50
##   1920x1080 27.85 ms p50
## which is roughly 2-3 ms of fixed cost and 8-12 ms per megapixel — this
## renderer is fragment-bound, and the Rust per-frame work measured by
## rust/examples/hot_paths.rs is microseconds against it. Disabling each
## per-fragment loop in turn at 1080p attributes 10.2 ms to the wall sight
## loop and 9.6 ms to the ring loop, about 70% of the frame between them.
##
## The report is percentiles, never a mean: a renderer that misses one frame
## in twenty is a renderer that stutters, and a mean hides exactly that.

const SETTLE_FRAMES := 60
const SAMPLE_FRAMES := 240

var _cpu: PackedFloat64Array = PackedFloat64Array()
var _gpu: PackedFloat64Array = PackedFloat64Array()
var _total: PackedFloat64Array = PackedFloat64Array()
var _wall := 0.0


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
	for _i: int in SETTLE_FRAMES:
		await get_tree().process_frame
	print("# settled")
	var began := Time.get_ticks_usec()
	for _i: int in SAMPLE_FRAMES:
		await get_tree().process_frame
		_cpu.append(Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0)
		_gpu.append(Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME))
		_total.append(get_process_delta_time() * 1000.0)
	_wall = float(Time.get_ticks_usec() - began) / 1.0e6


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
	_line("script process (ms)", _cpu)
	_line("frame delta (ms)", _total)
	_line("draw calls", _gpu)
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
	print("frame-time-probe: DONE")
