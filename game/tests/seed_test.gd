extends GdUnitTestSuite
## Seed and demo are SEPARATE switches. UNSEEING_SEED (or ?seed on web)
## makes the flicker stream reproducible WITHOUT arming the 4-second demo
## tap that would contaminate the pool; UNSEEING_DEMO still implies
## seeding, because an unseeded demo run cannot be frame-compared. The
## web ?seed/?demo paths ride the same helper but need a browser — the
## smoke gate covers them; these tests pin the env contract headless.

const WORLD_FIXTURE := preload("res://tests/world_fixture.gd")
const SEED_VALUE := 0x5EED


func after_test() -> void:
	OS.set_environment("UNSEEING_SEED", "")
	OS.set_environment("UNSEEING_DEMO", "")


func _boot() -> UnseeingGame:
	var main: UnseeingGame = auto_free(WORLD_FIXTURE.game())
	add_child(main)
	return main


## Advance the simulated clock past the demo-arming checkpoint (now >= 0.5,
## UnseeingGame::fire_demo_tap) and let one process() run it.
func _run_arming_check(main: UnseeingGame) -> void:
	main.now = 1.0
	await get_tree().process_frame
	await get_tree().process_frame


func test_unseeing_seed_seeds_without_arming_the_demo() -> void:
	OS.set_environment("UNSEEING_SEED", "1")
	var main := _boot()
	assert_int(main.flicker_seed()).is_equal(SEED_VALUE)
	await _run_arming_check(main)
	assert_bool(main.demo_armed()).is_false()


func test_unseeing_demo_still_seeds_and_arms() -> void:
	OS.set_environment("UNSEEING_DEMO", "1")
	var main := _boot()
	assert_int(main.flicker_seed()).is_equal(SEED_VALUE)
	await _run_arming_check(main)
	assert_bool(main.demo_armed()).is_true()


func test_an_unswitched_boot_stays_unseeded_and_unarmed() -> void:
	var main := _boot()
	assert_int(main.flicker_seed()).is_not_equal(SEED_VALUE)
	await _run_arming_check(main)
	assert_bool(main.demo_armed()).is_false()
