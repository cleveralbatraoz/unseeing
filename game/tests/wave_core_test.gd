extends GdUnitTestSuite
## The extension boundary, proven end to end: the Rust wave core is loaded,
## its class is registered, and a call crosses the FFI and answers from the
## pure Rust module.


func test_wave_core_class_is_registered() -> void:
	assert_bool(ClassDB.class_exists("WaveCore")).is_true()


func test_wave_core_answers_across_the_boundary() -> void:
	var core := WaveCore.new()
	assert_int(core.ray_fan_size()).is_equal(26)
