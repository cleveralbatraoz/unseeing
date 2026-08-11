extends GdUnitTestSuite
## The restore TRANSACTION against the live scene: a captured blob applied
## back to a running game, and the proof that the fit is exact.
##
## The read side's own suite is `restore_test.gd`; this one is its write-side
## twin, split off because a suite has a public-method ceiling and the read
## side had reached it.

const MAIN_SCENE := preload("res://scenes/main.tscn")


## A world that has actually run: sources hold appointments, the hero body
## has built its viewmodel, and every clock has a reading. Capture refuses an
## unticked world by design, so every capture test starts here.
##
## Duplicated from `restore_test.gd::_boot_ticked`, which is where this
## idiom lives — the two suites are one story told from two sides.
func _boot_ticked() -> UnseeingMain:
	var main: UnseeingMain = auto_free(MAIN_SCENE.instantiate() as UnseeingMain)
	add_child(main)
	# one real process frame so sources book appointments and the viewmodel
	# exists — capture refuses an unticked world by design
	await _one_frame()
	return main


## A world with a life behind it, and appointments that are NOT their own
## interval.
##
## A fresh gate books its first beat one interval out, so at boot every
## source's appointment EQUALS its cadence knob and a restore that mixed the
## two up would be invisible. Jumping the clock past both first beats (fan
## 0.4 s, radio 0.7 s) and letting one frame run spends them: the jumped-clock
## law buys exactly one wave per source and rebooks from NOW, so every
## appointment in the blob is a date nothing else in the scene can be mistaken
## for. The tap adds real reflections — echo appointments the book must carry.
##
## The hero looks DOWN before tapping, and that is not decoration: a cane
## swung at eye level in open air strikes nothing and makes no wave at all
## (the air-swish voice), so a tap taken standing level would leave the echo
## book empty and every field of it untested.
##
## The tap comes AFTER the jumped clock has landed, for the same kind of
## reason: the cane is voiced on the physics tick off the clock the hero was
## last handed, so a tap queued before the jump is dated before it, and its
## reflections are all overdue and drained by the time the blob is taken.
func _lively(main: UnseeingMain) -> void:
	main.now += 1.0
	await _one_frame()
	main.player.look(Vector2(0.0, 100.0))
	main.player.tap()
	for _i in 2:
		await _one_frame()


## One process frame and one physics frame — the pair every clock in the
## game needs to see a change. The composition root advances `now` in
## `_process` and hands it to the hero and the cats there; the cane, the
## footsteps and every reflection cast run on the PHYSICS tick, off the copy
## that frame left them.
func _one_frame() -> void:
	await get_tree().process_frame
	await get_tree().physics_frame


## The headline property: capture, restore, capture again, and the two blobs
## are the same instant — bit for bit, over every field the hash covers.
##
## The world is deliberately allowed to LIVE ON between the capture and the
## restore, because a restore into the world it was taken from proves almost
## nothing: every door could be a no-op and the hashes would still agree. Here
## the cat has wandered, the sources have beaten, echoes have fired and the
## pool has turned over — so a door that does not write lands on a field that
## has moved, and the proof names it.
func test_round_trip_capture_restore_capture_is_exact() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(blob.has("unavailable")).is_false()
	# a blob of an empty world would round-trip perfectly and prove nothing:
	# these pin that there is something in it to get wrong — waves in the
	# air, reflections still owed, and a cat with a life behind it
	assert_int(main.observer.snapshot(main.now)["live_slots"]).is_greater(0)
	assert_int((blob["echoes"] as Array).size()).is_greater(0)
	for _i in 30:
		await _one_frame()
	var verdict: Dictionary = main.restore_blob(blob)
	assert_str(str(verdict.get("unavailable", ""))).is_empty()
	assert_str(verdict["hash"]).is_equal(blob["hash"])


## The spurious-beat trap. A source's gate is re-pinned AFTER the clock lands,
## so the restored appointment stands; a gate left holding a stale date beats
## the moment the level is ticked and the world sounds a wave the original
## never made.
##
## The tick is driven by hand at the SAME instant the blob is dated at: the
## clock does not move, so nothing can expire and no appointment can
## legitimately fall due — a slot that appears was a spurious beat and nothing
## else.
func test_restore_repins_appointments_no_spurious_beat() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(blob.has("unavailable")).is_false()
	var live_before: int = main.observer.snapshot(main.now)["live_slots"]
	var verdict: Dictionary = main.restore_blob(blob)
	assert_str(str(verdict.get("unavailable", ""))).is_empty()
	main.level.tick_sources(main.now, main.player.camera.global_position)
	var live_after: int = main.observer.snapshot(main.now)["live_slots"]
	assert_int(live_after).is_equal(live_before)


## A blob from another format is refused whole, before a single field is
## written — the version is the first thing checked and the world is left
## exactly as it stood. The env group is moved too, because the env half is
## applied by the composition root BEFORE the engine half is asked: a
## transaction that refused after that half had landed would leave the clock
## a thousand seconds from the world it dates.
func test_a_wrong_version_refuses_before_touching_anything() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	blob["format_version"] = 999
	(blob["env"] as Dictionary)["now"] = "999.0"
	var before: Dictionary = main.observer.snapshot(main.now)
	var clock: float = main.now
	var verdict: Dictionary = main.restore_blob(blob)
	assert_bool(verdict.has("unavailable")).is_true()
	assert_str(verdict["unavailable"]).contains("999")
	assert_float(main.now).is_equal(clock)
	var after: Dictionary = main.observer.snapshot(main.now)
	assert_int(after["live_slots"]).is_equal(before["live_slots"])


## A restore that cannot prove itself refuses, and names the field it failed
## at rather than shrugging at a hash.
##
## The eye's pitch is the field a hand-edited blob can genuinely be caught
## at: the restore door clamps it to the same limit the look law does, so a
## blob asking for 99 radians restores as 1.35 and the re-capture disagrees
## with the file. (A pool slot could not stand in here — the pool is verbatim
## storage in both directions, so a tampered slot restores exactly and the
## world honestly agrees with the blob. Whether the FILE agrees with its own
## hash is the read side's question, and `blob_round_trip_ok` answers it.)
func test_a_tampered_blob_is_named_at_its_divergent_field() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	(blob["hero"] as Dictionary)["pitch"] = "99.0"
	var verdict: Dictionary = main.restore_blob(blob)
	assert_bool(verdict.has("unavailable")).is_true()
	assert_str(verdict["unavailable"]).contains("hero.pitch")


## The cane's out-tray is state like any other, and it travels BOTH ways. A
## tap queued after the capture must be gone once that capture is restored —
## `tap()` can only ever set the flag, so a restore that used it as its one
## door could never clear one, and would refuse itself at `hero.tap_queued`
## over a difference it was perfectly able to fix.
func test_a_queued_tap_the_blob_never_held_is_cleared() -> void:
	var main := await _boot_ticked()
	await _lively(main)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(blob["hero"]["tap_queued"]).is_false()
	main.player.tap()
	var verdict: Dictionary = main.restore_blob(blob)
	assert_str(str(verdict.get("unavailable", ""))).is_empty()
	assert_str(verdict["hash"]).is_equal(blob["hash"])
