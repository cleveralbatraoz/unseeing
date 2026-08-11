extends GdUnitTestSuite
## Every designer-facing class gets an icon in the Create Node dialog.
## This test verifies the [icons] section is correctly wired in unseeing.gdextension
## and all referenced SVGs exist.


func test_icons_section_exists() -> void:
	var config := ConfigFile.new()
	var err: int = config.load("res://unseeing.gdextension")
	assert_int(err).is_equal(OK)
	assert_bool(config.has_section("icons")).is_true()


func test_icons_contains_exactly_eight_classes() -> void:
	var config := ConfigFile.new()
	var err: int = config.load("res://unseeing.gdextension")
	assert_int(err).is_equal(OK)
	var expected_classes: Array = [
		"WaveLevel",
		"WaveWall",
		"WaveProp",
		"WaveColumn",
		"WaveWedge",
		"SoundFan",
		"SoundRadio",
		"WaveCat"
	]
	var keys: PackedStringArray = config.get_section_keys("icons")
	assert_array(keys).contains_exactly(expected_classes)


func test_all_icon_files_exist() -> void:
	var config := ConfigFile.new()
	var err: int = config.load("res://unseeing.gdextension")
	assert_int(err).is_equal(OK)
	var wave_level: String = config.get_value("icons", "WaveLevel")
	var wave_wall: String = config.get_value("icons", "WaveWall")
	var wave_prop: String = config.get_value("icons", "WaveProp")
	var wave_column: String = config.get_value("icons", "WaveColumn")
	var wave_wedge: String = config.get_value("icons", "WaveWedge")
	var sound_fan: String = config.get_value("icons", "SoundFan")
	var sound_radio: String = config.get_value("icons", "SoundRadio")
	var wave_cat: String = config.get_value("icons", "WaveCat")
	assert_bool(FileAccess.file_exists(wave_level)).is_true()
	assert_bool(FileAccess.file_exists(wave_wall)).is_true()
	assert_bool(FileAccess.file_exists(wave_prop)).is_true()
	assert_bool(FileAccess.file_exists(wave_column)).is_true()
	assert_bool(FileAccess.file_exists(wave_wedge)).is_true()
	assert_bool(FileAccess.file_exists(sound_fan)).is_true()
	assert_bool(FileAccess.file_exists(sound_radio)).is_true()
	assert_bool(FileAccess.file_exists(wave_cat)).is_true()
