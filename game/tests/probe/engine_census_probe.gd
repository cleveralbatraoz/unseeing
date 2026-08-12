extends SceneTree
## The engine census -- a headless probe `tools/bootstrap.sh` runs as its
## final step, so a fresh clone's install command can trust a real exit
## code rather than eyeballing Godot's output.
##
## `_initialize()` only: this is a pure ClassDB census, over before the
## first frame -- there is nothing to wait on, unlike the editor-mode
## probes next door (editor_slab_probe.gd, editor_source_probe.gd,
## editor_level_probe.gd) that poll for a node's deferred _ready.
##
## Every #[class(...)] struct registered from rust/src/, same 16 names as
## game/tests/engine_binary_test.gd:25-42. Hand-written ON PURPOSE, not
## derived from rust/src/ at probe time: if it were regenerated from the
## same source that produces the registration, a stale or absent binary
## missing every one of these classes would still pass, because the
## expected list and the (absent) reality would drift together. Keeping it
## by hand means adding a new #[class(...)] in Rust must ALSO mean adding
## its name here -- that duplication is the point of this list, not an
## oversight.
const REGISTERED_CLASSES: Array[String] = [
	"WaveCore",
	"WaveLevel",
	"WaveWall",
	"WaveProp",
	"WaveColumn",
	"WaveWedge",
	"SoundFan",
	"SoundRadio",
	"WaveCat",
	"HeroBody",
	"UnseeingPlayer",
	"CaneRest",
	"SettingsMenu",
	"SettingsFrame",
	"WaveObserver",
	"UnseeingGame",
]

const _REMEDY := "tools/bootstrap.sh builds the engine -- run it, then relaunch Godot"

var _checks := 0
var _failed := 0


func _initialize() -> void:
	for expected_class: String in REGISTERED_CLASSES:
		_check(expected_class)
	_report()


func _check(expected_class: String) -> void:
	_checks += 1
	var ok := ClassDB.class_exists(expected_class)
	print(
		(
			("ok %d - %s is registered" if ok else "not ok %d - %s is registered")
			% [_checks, expected_class]
		)
	)
	if not ok:
		_failed += 1
		print("# %s: %s" % [expected_class, _REMEDY])


func _report() -> void:
	print("1..%d" % _checks)
	var verdict := (
		"PASS (%d checks)" % _checks if _failed == 0 else "FAIL (%d of %d)" % [_failed, _checks]
	)
	print("probe: %s" % verdict)
	quit(1 if _failed > 0 else 0)
