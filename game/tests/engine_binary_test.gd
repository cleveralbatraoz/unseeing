extends GdUnitTestSuite
## Guards against a STALE OR MISSING ENGINE BINARY: the native GDExtension
## library under `rust/target/` -- a different artifact per platform
## (libunseeing_core.dylib on macOS, .so on Linux, an arch-specific
## unseeing_core.dll on Windows, resolved by game/unseeing.gdextension) --
## built from an older `rust/` checkout than the source on disk, or not
## built at all. `compatibility_minimum` in unseeing.gdextension only gates
## the ENGINE version; nothing gates the library being current with the
## checked-out Rust. A stale binary loads successfully and registers
## whatever partial class set the old build produced; an absent binary
## never loads and registers nothing. Either way, every symptom then points
## at GDScript: "Could not find type ..." at some unrelated .gd line, or a
## scene node silently degrading to a transform-less placeholder on load.
## This test asserts the fault where it actually lives, and distinguishes
## the two cases rather than guessing.

## Every #[class(...)] struct registered from rust/src/, found by grepping the
## tree for the attribute and reading the struct name next to it. This list
## is hand-written ON PURPOSE, not derived from rust/src/ at test time: if it
## were regenerated from the same source that produces the registration, a
## stale binary missing half these classes would still pass, because the
## expected list and the (absent) reality would drift together. Keeping it
## by hand means adding a new #[class(...)] in Rust must ALSO mean adding its
## name here -- that duplication is the point of this list, not an oversight.
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
	"WaveRestorer",
	"WaveSpawn",
	"UnseeingGame",
]

## The remedy is identical in both failure modes -- a fresh build -- and is
## deliberately platform-neutral: naming any one platform's artifact path
## would misdirect developers on the other two.
const _REMEDY := (
	"Fix: rebuild the native GDExtension library under `rust/target/` with "
	+ "`cd rust && cargo build --release`, then re-run."
)


func test_every_engine_class_the_source_declares_is_registered() -> void:
	var missing: Array[String] = []
	for expected_class: String in REGISTERED_CLASSES:
		if not ClassDB.class_exists(expected_class):
			missing.append(expected_class)
	assert_array(missing).override_failure_message(_binary_fault_message(missing)).is_empty()


## Builds the failure message for a set of classes ClassDB does not know.
## Distinguishes the two ways the engine binary goes wrong -- conflating them
## would be exactly the confidently-wrong diagnosis this test exists to
## replace:
## - EVERY registered class missing means the GDExtension never loaded at
##   all, most likely because it does not exist yet (a fresh worktree before
##   its first `cargo build`) rather than because it is stale -- a binary
##   that loaded would register at least something.
## - A proper SUBSET missing means the library loaded successfully (the
##   classes that ARE present prove it) but was built from an older `rust/`
##   checkout than the one now on disk, so it registers a partial set.
func _binary_fault_message(missing: Array[String]) -> String:
	if missing.is_empty():
		return ""
	if missing.size() == REGISTERED_CLASSES.size():
		return (
			(
				"ALL %d registered engine classes are missing from ClassDB: the "
				+ "native GDExtension library under `rust/target/` did not load at "
				+ "all, most likely because it has not been built yet rather than "
				+ "because it is stale. This is NOT a GDScript bug. %s"
			)
			% [REGISTERED_CLASSES.size(), _REMEDY]
		)
	return (
		(
			"%d of %d engine classes are missing from ClassDB: %s. This is NOT a "
			+ "GDScript bug, even though the errors above may point at one -- the "
			+ "native GDExtension library under `rust/target/` loaded successfully "
			+ "(the other %d classes prove it) but is STALE: it registers an older, "
			+ "partial class set than the Rust source in rust/src/ now declares. %s"
		)
		% [
			missing.size(),
			REGISTERED_CLASSES.size(),
			missing,
			REGISTERED_CLASSES.size() - missing.size(),
			_REMEDY,
		]
	)


## The message must not assume WHICH platform's artifact is missing: this is
## a three-platform project and unseeing.gdextension resolves a different
## file per platform (libunseeing_core.dylib on macOS, .so on Linux, a
## Windows-arch-specific unseeing_core.dll). Naming any one of those paths
## would misdirect the other two platforms' developers.
func test_message_reads_stale_and_platform_neutral_when_some_classes_are_present() -> void:
	var message := _binary_fault_message(["SoundRadio"])
	assert_str(message).contains("STALE")
	assert_str(message).not_contains("did not load")
	assert_str(message).contains("rust/target/")
	assert_str(message).not_contains(".dylib")
	assert_str(message).not_contains(".so")
	assert_str(message).not_contains(".dll")


## A binary that never loaded at all -- the default state of a fresh
## worktree before its first `cargo build` -- registers NO classes, not a
## partial set. Calling that "stale" would be confidently wrong: the
## library never loaded "successfully" in the first place.
func test_message_reads_missing_and_platform_neutral_when_every_class_is_absent() -> void:
	var message := _binary_fault_message(REGISTERED_CLASSES.duplicate())
	assert_str(message).contains("did not load")
	assert_str(message).not_contains("STALE")
	assert_str(message).contains("rust/target/")
	assert_str(message).not_contains(".dylib")
	assert_str(message).not_contains(".so")
	assert_str(message).not_contains(".dll")
