extends GdUnitTestSuite
## Guards against a STALE ENGINE BINARY: `rust/target/release/libunseeing_core.dylib`
## built from an older `rust/` checkout than the source on disk. Godot loads
## a stale binary successfully -- `compatibility_minimum` in
## unseeing.gdextension only gates the ENGINE version, nothing gates the
## library being current with the checked-out Rust -- and it registers
## whatever partial class set the old build produced. Every symptom then
## points at GDScript: "Could not find type ..." at some unrelated .gd line,
## or a scene node silently degrading to a transform-less placeholder on
## load. This test asserts the fault where it actually lives.

## Every #[class(...)] struct registered from rust/src/, found by grepping the
## tree for the attribute and reading the struct name next to it. This list
## is hand-written ON PURPOSE, not derived from rust/src/ at test time: if it
## were regenerated from the same source that produces the registration, a
## stale binary missing half these classes would still pass, because the
## expected list and the (absent) reality would drift together. Keeping it
## by hand means adding a new #[class(...)] in Rust must ALSO mean adding its
## name here -- that duplication is the point of this list, not an oversight.
const REGISTERED_CLASSES := [
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
]


func test_every_engine_class_the_source_declares_is_registered() -> void:
	var missing: Array[String] = []
	for expected_class: String in REGISTERED_CLASSES:
		if not ClassDB.class_exists(expected_class):
			missing.append(expected_class)
	var message := (
		(
			"%d of %d engine classes are missing from ClassDB: %s. "
			+ "This is NOT a GDScript bug, even though the errors above may point at "
			+ "one -- rust/target/release/libunseeing_core.dylib loaded successfully "
			+ "but is STALE: it registers an older, partial class set than the Rust "
			+ "source in rust/src/ now declares. Fix: rebuild the engine core with "
			+ "`cd rust && cargo build --release`, then re-run."
		)
		% [missing.size(), REGISTERED_CLASSES.size(), missing]
	)
	assert_array(missing).override_failure_message(message).is_empty()
