extends GdUnitTestSuite
## Every knob a designer drags carries a range and a unit, so the Inspector
## shows a bounded slider in metres instead of a bare float. The break this
## catches: a knob whose hint is dropped regresses to a free-typing field
## silently.


func _hint_of(clazz: String, prop: String) -> Dictionary:
	for p: Dictionary in ClassDB.class_get_property_list(clazz):
		if p["name"] == prop:
			return p
	return {}


func test_wall_length_is_a_bounded_slider_in_metres() -> void:
	var p := _hint_of("WaveWall", "length")
	assert_int(p["hint"]).is_equal(PROPERTY_HINT_RANGE)
	assert_str(p["hint_string"]).contains("suffix")


func test_column_knobs_are_bounded() -> void:
	var radius := _hint_of("WaveColumn", "radius")
	var height := _hint_of("WaveColumn", "height")
	assert_int(radius["hint"]).is_equal(PROPERTY_HINT_RANGE)
	assert_int(height["hint"]).is_equal(PROPERTY_HINT_RANGE)
	assert_str(radius["hint_string"]).contains("suffix")
	assert_str(height["hint_string"]).contains("suffix")


func test_cat_seed_is_bounded_with_no_unit_suffix() -> void:
	var p := _hint_of("WaveCat", "seed")
	assert_int(p["hint"]).is_equal(PROPERTY_HINT_RANGE)
	assert_str(p["hint_string"]).not_contains("suffix")


func test_prop_size_is_a_bounded_vector_in_metres() -> void:
	var p := _hint_of("WaveProp", "size")
	assert_int(p["hint"]).is_equal(PROPERTY_HINT_RANGE)
	assert_str(p["hint_string"]).contains("suffix")


func test_wedge_size_is_a_bounded_vector_in_metres() -> void:
	var p := _hint_of("WaveWedge", "size")
	assert_int(p["hint"]).is_equal(PROPERTY_HINT_RANGE)
	assert_str(p["hint_string"]).contains("suffix")


func test_level_extents_is_a_bounded_vector_in_metres() -> void:
	var p := _hint_of("WaveLevel", "extents")
	assert_int(p["hint"]).is_equal(PROPERTY_HINT_RANGE)
	assert_str(p["hint_string"]).contains("suffix")


func test_run_endpoints_and_openings_are_typed_editor_knobs() -> void:
	for knob: String in ["from", "to"]:
		var p := _hint_of("WaveRun", knob)
		assert_int(p["type"]).is_equal(TYPE_VECTOR2)
		assert_int(p["hint"]).is_equal(PROPERTY_HINT_RANGE)
		assert_str(p["hint_string"]).contains("suffix")
	var openings := _hint_of("WaveRun", "openings")
	assert_int(openings["type"]).is_equal(TYPE_PACKED_VECTOR2_ARRAY)


func test_cat_roam_size_is_a_bounded_vector_in_metres() -> void:
	var p := _hint_of("WaveCat", "roam_size")
	assert_int(p["hint"]).is_equal(PROPERTY_HINT_RANGE)
	assert_str(p["hint_string"]).contains("suffix")
