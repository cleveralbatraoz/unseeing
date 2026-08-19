extends GdUnitTestSuite
## The extension boundary, proven end to end: the Rust wave core is loaded,
## its class is registered, and a call crosses the FFI and answers from the
## pure Rust module.


func test_wave_core_class_is_registered() -> void:
	assert_bool(ClassDB.class_exists("WaveCore")).is_true()


func test_wave_core_answers_across_the_boundary() -> void:
	var core := WaveCore.new()
	assert_int(core.ray_fan_size()).is_equal(26)


## THE BREAK: the desktop channel probe measuring one step while the
## renderer derives its tolerance from another. `channel_probe.gd` writes a
## pair of values one worst-case step apart and demands they survive the
## screen texture; if that step were retyped in GDScript it could drift from
## `render::channel` and the probe would keep passing while guarding a gap
## the shader no longer assumes.
##
## Hand-derived: 1.25 worst measured codes over 1023 nominal steps is
## 0.00122190. A nominal code is 1/1023 = 0.00097752, so the answer must be
## strictly the larger of the two — a probe that measured the nominal step
## would pass on hardware the guard does not survive.
func test_the_channel_reports_the_gap_it_was_measured_to_leave() -> void:
	var core := WaveCore.new()
	assert_float(core.channel_worst_step()).is_equal_approx(0.001_221_9, 1e-7)
	assert_float(core.channel_worst_step()).is_greater(1.0 / 1023.0)
