extends GdUnitTestSuite
## The test bench must not rewrite itself. gdUnit4 ships an in-editor updater
## that polls GitHub on every project open and, on one click, deletes
## res://addons/gdUnit4/ and unpacks an unreviewed release over it — bypassing
## review entirely, since the pre-commit hook deliberately skips game/addons/.
## Upstream gates that whole path behind one project setting, so the setting is
## the seam we hold: pinned off, and pinned *in the project file*, because a
## value that only exists at runtime would be recreated as true on next open.

## Upstream's own key, from GdUnitSettings: MAIN_CATEGORY + "/settings" +
## "/common" + "/update_notification_enabled".
const UPDATE_NOTIFICATION := "gdunit4/settings/common/update_notification_enabled"


func test_update_notification_is_disabled() -> void:
	assert_bool(ProjectSettings.has_setting(UPDATE_NOTIFICATION)).is_true()
	assert_bool(ProjectSettings.get_setting(UPDATE_NOTIFICATION)).is_false()


## Persisted, not merely defaulted: GdUnitSettings.create_property_if_need only
## respects a value that already exists, so it has to survive in project.godot.
func test_disabled_state_is_persisted_in_the_project_file() -> void:
	var file := FileAccess.open("res://project.godot", FileAccess.READ)
	assert_object(file).is_not_null()
	var text := file.get_as_text()
	file.close()
	assert_str(text).contains("settings/common/update_notification_enabled=false")


## The key we pin has to be the key upstream reads — a rename upstream would
## silently re-arm the updater, so bind the two together here.
func test_pinned_key_matches_the_framework_constant() -> void:
	assert_str(GdUnitSettings.UPDATE_NOTIFICATION_ENABLED).is_equal(UPDATE_NOTIFICATION)
