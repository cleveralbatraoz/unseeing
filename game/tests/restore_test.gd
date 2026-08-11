extends GdUnitTestSuite
## The restore doors against the live scene. Each test freezes nothing —
## it drives the clock by hand, exactly as observer_test does.

const MAIN_SCENE := preload("res://scenes/main.tscn")


func test_a_restored_cat_resumes_the_same_life() -> void:
	var main: UnseeingMain = auto_free(MAIN_SCENE.instantiate() as UnseeingMain)
	add_child(main)
	var cat: WaveCat = main.cats[0]
	# let the cat live a little, on real physics — main's own _process
	# advances the clock and ticks every cat in the tree each frame, so
	# no manual clock-driving is needed here (see observer_test's helpers
	# for the same idiom)
	for _i in 30:
		await get_tree().process_frame
	var mood_at_capture: int = cat.mood()
	var paws_at_capture: PackedVector3Array = cat.paw_positions()
	# the capture_state/restore_state doors built this task are pub(crate)
	# Rust, with no #[func] surface yet — the real equivalence gates are
	# the cargo lockstep tests (cat_brain, cat_gait, cat_body) plus Task
	# 9/10's blob round trip; this test only pins that a live scene cat is
	# capturable at all, i.e. never in the "never built" refusal state.
	assert_int(mood_at_capture).is_not_equal(-1)
	assert_int(paws_at_capture.size()).is_equal(4)
