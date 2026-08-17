# gdlint:ignore = max-public-methods
extends GdUnitTestSuite
## WaveObserver — the debug observability boundary.
##
## (The directive above must sit on line 1 — gdlint keys an ignore to the
## line its problem is reported on. A gdUnit4 suite is a list of cases, not
## a class with an API: every case is a public method, so the 20-method
## ceiling counts coverage rather than surface. Suppressed in the two
## suites that outgrew it — this one and level_test.gd — and nowhere else,
## so the rule keeps its teeth over every test/probe GDScript the project
## permits.)
##
## These pin the CONTRACT the live debugging loop depends on: an observer
## missing any system it reads refuses loudly, and an injected one reports
## the state an agent acts on. The maths itself is pinned by cargo tests;
## what is tested here is that the boundary carries it across without
## inventing anything — no zeros standing in for facts it cannot observe.
##
## A refusal is held to being EXACTLY one key, never sampled: a dictionary
## that carried "unavailable" beside an empty slot list would satisfy every
## has()-shaped assertion while telling an agent the pool it cannot see is
## empty, which is the one lie this whole layer exists to prevent.

const LEVEL_SCENE := preload("res://scenes/level_01.tscn")
const MAIN_SCENE := preload("res://scenes/main.tscn")

## SOURCE_THROUGH (0.3, level_plan.rs), the per-wall silhouette muffle, and
## FULL_REACH (12 m, sound_source.rs), the reach a source's wave carries at
## full volume — engine laws quoted by hand. Never read through SoundFan's
## own reach()/slot_pressure() #[func]s below: those compute with the
## identical formula the snapshot's own fields are filled from
## (`nodes/observer.rs::sources`), so calling them here would mirror the
## code under test rather than check it. The fan's own volume, wave_speed
## and cadence are knobs, not law, and are read straight off the live node
## instead of duplicated as constants.
const SOURCE_THROUGH := 0.3
const FULL_REACH := 12.0
## Every world source is born at SOURCE_KIND = 3 (sound_source.rs), whose
## fade tail — pulse_pool::fade_tail(3) — is a fixed 2 s: an engine law,
## kept as the literal below. slot_pressure() itself
## (sound_source.rs::Voice::slot_pressure) is ring time plus that tail,
## divided by cadence: `(reach / speed + 2.0) / cadence`, with reach itself
## FULL_REACH * volume — the formula the assertion below computes over the
## fan's own live knobs rather than a baked constant, so retuning volume,
## wave_speed or cadence in the Inspector tracks instead of reddening
## the gate.
## A flicker value the composition root would have pushed to the world skin
## this frame. Nothing derives from it — it only has to be recognisable.
const FLICK := 0.6

## A point inside the code-built reflection fixture below. Keeping the
## geometry in this suite makes ray-accounting laws independent of whichever
## level a designer currently ships.
const TAP_AT := Vector3(3.0, 0.9, 4.0)
## The fixture's cane tap: 6 m of range at 5.5 m/s, so the reflection
## fan reaches 0.8 x 6 = 4.8 m.
const TAP_MAX_R := 6.0
const TAP_SPEED := 5.5


## One label from the single role table (render::labels::role_label), read
## rather than retyped — a suite carrying its own copy of a label agrees
## with whatever the table says, and the table used to be wrong.
func _role(name: String) -> float:
	var table: Dictionary = WaveCore.new().role_labels()
	assert_bool(table.has(name)).is_true()
	return table.get(name, NAN)


func test_uninjected_observer_refuses_rather_than_reporting_zeros() -> void:
	var obs := _observer()
	var snap: Dictionary = obs.snapshot(0.0)
	assert_int(snap.size()).is_equal(1)
	assert_bool(snap.has("unavailable")).is_true()


func test_uninjected_explainers_refuse_too() -> void:
	var obs := _observer()
	for refusal: Dictionary in [
		obs.explain_ray(Vector3.ZERO, Vector3.ONE), obs.explain_oids(), obs.explain_eviction(0.0)
	]:
		assert_int(refusal.size()).is_equal(1)
		assert_bool(refusal.has("unavailable")).is_true()


## An eye is not optional equipment for a snapshot: how many walls stand
## between the hero and a source, and where the eye itself looks, are measured
## FROM the camera. An observer with no camera would have to invent one at the
## origin, and a plausible wrong number is worse than a refusal — so it
## refuses, while the eye-free explainers keep working.
##
## The REASON is held to being true, not merely present. A refusal that blamed
## the standing image would send a reader looking for a quantity the eye has
## nothing to do with: the standing image is read straight back off the
## source's own limbs, and would be reportable with no camera in the scene at
## all. A debugging layer that misnames its own limits teaches the wrong
## lesson faster than no layer at all.
func test_a_snapshot_without_an_eye_refuses_rather_than_guessing_one() -> void:
	var level := _empty_level(Pulses.new())
	var obs := _observer()
	obs.inject(level, null)
	var snap: Dictionary = obs.snapshot(0.0)
	assert_int(snap.size()).is_equal(1)
	var reason: String = snap["unavailable"]
	assert_str(reason).contains("walls_to_eye")
	assert_str(reason).not_contains("source_volume")
	assert_str(reason).not_contains("source_muffle")
	assert_bool(obs.explain_oids().has("unavailable")).is_false()


## A window onto a scene that has been torn down refuses instead of taking
## the game down with it. A freed node leaves the observer's handle looking
## perfectly valid, so every entry point asks before it reads — the MCP loop
## that drives these outlives scene reloads.
func test_a_freed_level_refuses_rather_than_crashing() -> void:
	var level := WaveLevel.new()
	level.add_child(_spawn_marker())
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var obs := _observer()
	obs.inject(level, _eye())
	level.free()
	for refusal: Dictionary in [
		obs.snapshot(0.0),
		obs.explain_ray(Vector3.ZERO, Vector3.ONE),
		obs.explain_oids(),
		obs.explain_eviction(0.0)
	]:
		assert_int(refusal.size()).is_equal(1)
		assert_str(refusal["unavailable"]).contains("freed")


## The eye can go the same way on its own — the hero is freed, the level
## stands. The snapshot refuses; the explainers, which need no eye, do not.
func test_a_freed_camera_refuses_rather_than_crashing() -> void:
	var level := _empty_level(Pulses.new())
	var eye := Camera3D.new()
	add_child(eye)
	var obs := _observer()
	obs.inject(level, eye)
	eye.free()
	var snap: Dictionary = obs.snapshot(0.0)
	assert_int(snap.size()).is_equal(1)
	assert_str(snap["unavailable"]).contains("freed")
	assert_bool(obs.explain_ray(Vector3.ZERO, Vector3.ONE).has("unavailable")).is_false()


## The pool, read back through the boundary. Hand-derived from the wave
## contract: a cane tap (kind 0) at 5.5 m/s, half a second old, has a ring
## 2.75 m across and is still alive. A level that has been driven for a
## frame leaves nothing unobserved, so `unknown` is empty and every key is
## present — the other half of the contract the next test pins.
##
## The hero group joined that contract in this task: a snapshot with a live
## player injected has nothing left to name, so the player is built and
## injected here too — otherwise the absent hero, correctly named in
## `unknown`, would falsify a claim this test makes about a different part
## of the snapshot entirely.
func test_snapshot_reports_a_tap_that_was_emitted() -> void:
	var pulses := Pulses.new()
	var level := _empty_level(pulses)
	var obs := _observer()
	var player: UnseeingPlayer = auto_free(UnseeingPlayer.new())
	player.pulses = pulses
	add_child(player)
	obs.inject(level, _eye())
	obs.inject_hero(player)
	pulses.emit(0, Vector3.ZERO, 6.0, 5.5, 1.0, 0.0)
	var snap: Dictionary = obs.snapshot(0.5)
	assert_float(snap["now"]).is_equal_approx(0.5, 0.0001)
	assert_float(snap["flick"]).is_equal_approx(FLICK, 0.0001)
	assert_array(snap["unknown"]).is_empty()
	assert_int(snap["slot_scan_limit"]).is_equal(1)
	assert_int(snap["live_slots"]).is_equal(1)
	assert_int((snap["slots"] as Array).size()).is_equal(64)
	var slot: Dictionary = snap["slots"][0]
	assert_int(slot["kind"]).is_equal(0)
	assert_float(slot["ring_radius"]).is_equal_approx(2.75, 0.001)
	assert_str(slot["state"]).is_equal("Live")
	assert_str(snap["slots"][1]["state"]).is_equal("Never")


## The two pool numbers on the wire answer different questions, and a hole
## in the pool is where they part company. `slot_scan_limit` is the bound
## the shaders break their per-pixel loop at — highest live slot + 1, holes
## spanned — while `live_slots` counts the slots that actually report
## "Live". The fixed pool wraps continuously, so the bound sits at 64 for
## a whole slot lifetime once slot 63 has been claimed: an agent reading it
## as a census would chase eviction pressure that does not exist.
##
## Hand-derived from the wave contract: a kind-2 sound with a 1.6 m ring at
## 4 m/s dies at 0.4 + 2.5 = 2.9 s; the kind-0 tap behind it lives to
## 6/5.5 + 6 = 7.09 s. Observed at 5 s, one is dead under one that is live.
func test_snapshot_separates_the_scan_bound_from_the_live_count() -> void:
	var pulses := Pulses.new()
	var level := _empty_level(pulses)
	var obs := _observer()
	obs.inject(level, _eye())
	pulses.emit(2, Vector3.ZERO, 1.6, 4.0, 0.8, 0.0)
	pulses.emit(0, Vector3.ZERO, 6.0, 5.5, 1.0, 0.0)
	var snap: Dictionary = obs.snapshot(5.0)
	assert_int(snap["slot_scan_limit"]).is_equal(2)
	assert_int(snap["live_slots"]).is_equal(1)
	assert_str(snap["slots"][0]["state"]).is_equal("Expired")
	assert_str(snap["slots"][1]["state"]).is_equal("Live")


## The echo book, on the wire. An echo is an APPOINTMENT — scheduled the
## moment the reflection fan finds a surface, fired when the primary
## wavefront reaches it — so "the echo fired a frame late" and "the echo was
## never scheduled" are different bugs that look identical in a single frame
## of pixels. The snapshot carries every pending appointment with the
## seconds left before it fires.
##
## An empty book is an empty LIST, asserted first: a level with nothing
## scheduled and a level whose book could not be read must never serialise
## the same way.
func test_the_snapshot_carries_the_echo_book() -> void:
	var pulses := Pulses.new()
	var level := _reflection_level(pulses)
	var obs := _observer()
	obs.inject(level, _eye())
	assert_array(obs.snapshot(0.0)["echoes"]).is_empty()
	var points: Array[Vector3] = await _load_the_echo_book(pulses)
	assert_int(points.size()).is_greater(0)
	var echoes: Array = obs.snapshot(0.0)["echoes"]
	assert_int(echoes.size()).is_equal(points.size())
	for i in echoes.size():
		var echo: Dictionary = echoes[i]
		assert_vector(echo["pos"]).is_equal(points[i])
		# the tap was born at t = 0 and every appointment it made is still
		# ahead of it, so observed at t = 0 the wait IS the appointment
		var at_t: float = echo["at_t"]
		assert_float(echo["fires_in"]).is_greater(0.0)
		assert_float(echo["fires_in"]).is_equal_approx(at_t, 0.0001)
		assert_float(echo["gain"]).is_greater(0.0)


## The same book read half a second late: the appointments have not moved,
## and the wait on each has shortened by exactly that half second. An
## appointment whose moment has passed reports a NEGATIVE wait rather than a
## clamped zero — a late echo is the fault worth seeing, and a floor at zero
## would hide how late it is.
func test_the_echo_wait_counts_down_and_goes_negative_when_overdue() -> void:
	var pulses := Pulses.new()
	var level := _reflection_level(pulses)
	var obs := _observer()
	obs.inject(level, _eye())
	var points: Array[Vector3] = await _load_the_echo_book(pulses)
	assert_int(points.size()).is_greater(0)
	var now: Dictionary = obs.snapshot(0.0)["echoes"][0]
	var later: Dictionary = obs.snapshot(0.5)["echoes"][0]
	var at_t: float = now["at_t"]
	var fires_in: float = now["fires_in"]
	assert_float(later["at_t"]).is_equal_approx(at_t, 0.0001)
	assert_float(later["fires_in"]).is_equal_approx(fires_in - 0.5, 0.0001)
	# the cane tap reaches 4.8 m at 5.5 m/s, so no appointment stands more
	# than 0.88 s out: a clock 10 s on is past every one of them
	var overdue: Dictionary = obs.snapshot(10.0)["echoes"][0]
	assert_float(overdue["fires_in"]).is_less(0.0)


## What could not be observed is NAMED, and its key is absent rather than
## zero. On a level no frame has ever run over, the world skin carries no
## flicker and no source has been pushed its standing image — and a flicker
## of zero, or a silhouette that reads as fully muffled, are both states the
## game can genuinely be in.
func test_a_snapshot_names_what_it_could_not_observe() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker())
	var fan := SoundFan.new()
	level.add_child(fan)
	var radio := SoundRadio.new()
	level.add_child(radio)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var obs := _observer()
	obs.inject(level, _eye())
	var snap: Dictionary = obs.snapshot(0.0)
	assert_bool(snap.has("flick")).is_false()
	assert_bool((snap["sources"][0] as Dictionary).has("source_volume")).is_false()
	assert_bool((snap["sources"][0] as Dictionary).has("source_muffle")).is_false()
	(
		assert_array(snap["unknown"])
		. contains(
			[
				"flick",
				"sources[0].source_volume",
				"sources[0].source_muffle",
				"sources[1].source_volume",
				"sources[1].source_muffle",
			]
		)
	)


## The eviction prediction reaches the same pool the snapshot does: a virgin
## pool gives up slot 0 by the expired rule.
func test_explain_eviction_reads_the_injected_pool() -> void:
	var level := _empty_level(Pulses.new())
	var obs := _observer()
	obs.inject(level, _eye())
	var plan: Dictionary = obs.explain_eviction(0.0)
	assert_int(plan["slot"]).is_equal(0)
	assert_str(plan["rule"]).is_equal("Expired")


## A code-built fan stands one wall from the eye, so its standing image is its
## volume dimmed once. That number is READ BACK off the limb the level pushed
## it to, not recomputed here or in the observer.
func test_snapshot_describes_the_levels_sound_sources() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker())
	var wall := WaveWall.new()
	wall.length = 6.0
	wall.position = Vector3(6, 0, 4)
	wall.rotation.y = PI * 0.5
	level.add_child(wall)
	var fan := SoundFan.new()
	fan.name = "FixtureFan"
	fan.position = Vector3(9, 0, 4)
	level.add_child(fan)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var eye := _eye()
	level.tick_sources(0.0, eye.global_position)
	var obs := _observer()
	obs.inject(level, eye)
	var sources: Array = obs.snapshot(0.0)["sources"]
	assert_int(sources.size()).is_equal(1)
	var entry := _source_entry(sources, str(fan.name))
	# knobs read straight off the live node, not duplicated as census: the
	# fixture fan's own volume and cadence
	assert_float(entry["volume"]).is_equal_approx(fan.volume, 0.0001)
	assert_float(entry["cadence"]).is_equal_approx(fan.cadence, 0.0001)
	# reach and slot pressure are FULL_REACH/the wave contract scaled by the
	# fan's own live knobs — hand-derived formulas, never fan.reach()/
	# fan.slot_pressure() themselves (see the constants block above), so a
	# knob turned in the Inspector moves both sides of the assertion together
	assert_float(entry["reach"]).is_equal_approx(FULL_REACH * fan.volume, 0.0001)
	assert_float(entry["slot_pressure"]).is_equal_approx(
		(FULL_REACH * fan.volume / fan.wave_speed + 2.0) / fan.cadence, 0.0001
	)
	# The fixture hand-places exactly one wall. Do not derive the expectation
	# through level.source_muffle(eye, hub): that
	# would call the identical function tick_sources used to WRITE the
	# muffle in the first place, with the identical eye and hub, and
	# so would mirror the code under test rather than check it
	assert_int(entry["walls_to_eye"]).is_equal(1)
	# the two halves are reported APART, because the renderer consumes them
	# apart: the volume stands on its own and the muffle multiplies the
	# whole image. Their product is no longer a number the shader forms.
	assert_float(entry["source_volume"]).is_equal_approx(fan.volume, 0.0001)
	assert_float(entry["source_muffle"]).is_equal_approx(SOURCE_THROUGH, 0.0001)
	# the clockwork, not only the voice: "the fan has gone quiet" is a whole
	# question class, and a snapshot that carried neither the interval nor the
	# standing appointment could only answer it by waiting to see
	assert_float(entry["next_emit"]).is_equal_approx(fan.cadence, 0.0001)


## Where the hero woke. Every position in a snapshot is a world coordinate,
## and the one landmark that turns those into a story — "the tap is two
## metres behind the spawn", "the fan is through the wall from where I
## started" — is the spawn itself. It is derived from the marker, not
## authored anywhere an agent can read, so a snapshot without it leaves the
## reader unable to place anything else it reports.
##
## Checked against the level's own accessors, never a literal spawn-node
## coordinate: this is a read-back law (did the boundary carry the value
## through, not invent or drop it), and level_test.gd already hand-derives
## spawn_pos()/spawn_yaw() themselves on levels built in code. Both halves
## checked is what catches the boundary inventing one of the two rather
## than carrying it.
func test_the_snapshot_says_where_the_hero_woke() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var marker := _spawn_marker()
	marker.position = Vector3(7, 0, 5)
	marker.rotation.y = 0.4
	level.add_child(marker)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var obs := _observer()
	obs.inject(level, _eye())
	var spawn: Dictionary = obs.snapshot(0.0)["spawn"]
	assert_vector(spawn["position"]).is_equal_approx(level.spawn_pos(), Vector3.ONE * 0.0001)
	assert_float(spawn["yaw"]).is_equal_approx(level.spawn_yaw(), 0.0001)


## A source that cannot fire has no next emit, and says so. The cadence gate
## refuses a non-positive interval outright (rust/src/sound_source.rs), so the
## appointment it is holding will never be kept — reporting the stale number
## would tell an agent a wave is due in a fifth of a second when none is ever
## coming, which is exactly the plausible-wrong-answer failure this layer is
## built against. The key is absent and named, like every other absence.
func test_a_silenced_source_reports_no_next_emit() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker())
	var fan := SoundFan.new()
	fan.name = "Fan"
	fan.cadence = 0.0
	level.add_child(fan)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var obs := _observer()
	obs.inject(level, _eye())
	var snap: Dictionary = obs.snapshot(0.0)
	var source: Dictionary = snap["sources"][0]
	assert_float(source["cadence"]).is_equal_approx(0.0, 0.0001)
	assert_bool(source.has("next_emit")).is_false()
	assert_array(snap["unknown"]).contains(["sources[0].next_emit"])


## A code-built creature level keeps the census tests non-vacuous without
## requiring any designer-owned level to contain a cat.
func _cat_level() -> Dictionary:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker())
	var cat := WaveCat.new()
	cat.name = "FixtureCreature"
	cat.position = Vector3(4, 0, 4)
	level.add_child(cat)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	return {"level": level, "cat": cat}


## Every censused box reports the id it carries, touching something or not.
## The pairs only carry the ids of boxes that MEET, so a solid standing alone
## in a room had a name in the report and no id anywhere — and "which id did
## this thing actually get?" is the first question after "which seams are
## broken". The expected labels are READ from the one role table
## (render::labels::role_label, rust/src/render/labels.rs) rather than
## retyped: a suite carrying its own copy agrees with whatever the table
## says, and the table used to be wrong.
func test_the_oid_census_reports_the_id_of_every_box() -> void:
	var fixture := _cat_level()
	var level: WaveLevel = fixture["level"]
	var cat: WaveCat = fixture["cat"]
	var obs := _observer()
	obs.inject(level, _eye())
	var e: Dictionary = obs.explain_oids()
	var names: Array = e["names"]
	var oids: PackedFloat64Array = e["oids"]
	assert_int(oids.size()).is_equal(names.size())
	assert_array(names).contains(["Floor", str(cat.name)])
	assert_float(oids[names.find("Floor")]).is_equal_approx(_role("Floor"), 0.0001)
	var cat_idx := names.find(str(cat.name))
	assert_float(oids[cat_idx]).is_equal_approx(_role("Cat"), 0.0001)


## The creatures are IN the report. WaveCat and the hero's body occupy the
## 0.7+ id band on purpose — a cat walking in front of a wall must not melt
## into it — and a census that stopped at the walls, props and sources would
## give a seam bug involving the one moving thing in the room a clean bill of
## health, under a doc comment promising "every painted box in the level".
##
## The hero's body is deliberately NOT here and cannot be: HeroBody is a child
## of the composition root, not of the level, so the level's own walk never
## sees it. That is named in the census doc comment rather than left to be
## discovered.
func test_the_oid_census_includes_the_levels_creatures() -> void:
	var fixture := _cat_level()
	var level: WaveLevel = fixture["level"]
	var cat: WaveCat = fixture["cat"]
	var obs := _observer()
	obs.inject(level, _eye())
	var e: Dictionary = obs.explain_oids()
	assert_array(e["names"]).contains([str(cat.name)])
	assert_array(e["pairs"]).is_not_empty()
	assert_array(e["violations"]).is_empty()


## The compatibility census measures the swept source box, not one pose.
## WaveLevel::paint_labels grows each source-role envelope by the source's
## sweep_margin before assigning the shared graph; a check
## built from the ungrown box is weaker than the law it explains, and would
## hand back "no such pair, no violations" for a prop the fan's guard ring
## reaches on half of every cycle.
##
## Hand-derived from the fan's own build dimensions (rust/src/nodes/fan.rs),
## and measured from the FAN, so the pair may stand anywhere on the floor:
## the guard ring's outer radius is 0.44 m, so an unswept fan reaches 0.44 m
## along x; the head swings PIVOT_RANGE = 0.85 rad each way
## (rust/src/fan_wave.rs), so the sweep margin is 0.44 x sin(0.85) = 0.331 and
## the swept fan reaches 0.771. A 0.2 m cube centred 0.60 m out spans 0.50 to
## 0.70 from the fan: clear of the pose by 0.06 (six times Box3::TOUCH_EPS),
## and well inside the sweep. The two are placed at (1, 0, 1) rather than at
## the level's own corner, so the prop stands ON the floor slab the level
## builds and the pair is not reported as hanging over its edge.
func test_the_oid_census_measures_a_source_by_what_it_sweeps() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker())
	var fan := SoundFan.new()
	fan.name = "Fan"
	fan.position = Vector3(1, 0, 1)
	level.add_child(fan)
	var neighbour := WaveProp.new()
	neighbour.name = "SweptNeighbour"
	neighbour.size = Vector3(0.2, 0.2, 0.2)
	neighbour.position = Vector3(1.6, 1.15, 1.0)
	level.add_child(neighbour)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var obs := _observer()
	obs.inject(level, _eye())
	var e: Dictionary = obs.explain_oids()
	var seams: Array[String] = []
	for pair: Dictionary in e["pairs"]:
		seams.append("%s|%s" % [pair["name_a"], pair["name_b"]])
	# Pairs come back in ascending census order, with painted solids before
	# source-role entries, so the prop is the a side. Both semantic roles must
	# be present, whatever numeric labels this particular graph derived.
	var fan_pairs: Array[String] = []
	for seam: String in seams:
		if seam.begins_with("SweptNeighbour|Fan @"):
			fan_pairs.append(seam)
	assert_int(fan_pairs.size()).is_equal(2)
	assert_array(e["violations"]).is_empty()


## The SAME fixture at real per-face rather than bridged resolution:
## nothing forces the neighbour's BRIDGED face onto either derived fan role,
## so deleting the source/world role edges can stay green there while an
## un-bridged face collides. This checks every REAL mesh label directly.
func test_a_swept_neighbours_own_faces_all_clear_the_sources_oids() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker())
	var fan := SoundFan.new()
	fan.name = "Fan"
	fan.position = Vector3(1, 0, 1)
	level.add_child(fan)
	var neighbour := WaveProp.new()
	neighbour.name = "SweptNeighbour"
	neighbour.size = Vector3(0.2, 0.2, 0.2)
	neighbour.position = Vector3(1.6, 1.15, 1.0)
	level.add_child(neighbour)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var obs := _observer()
	obs.inject(level, _eye())
	var e: Dictionary = obs.explain_oids()
	var custom: PackedFloat32Array = _skin_of(neighbour).mesh.surface_get_arrays(0)[
		Mesh.ARRAY_CUSTOM0
	]
	var labels: Array[float] = []
	for label: float in custom:
		if not labels.has(label):
			labels.append(label)
	assert_array(labels).is_not_empty()
	var source_labels: Array[float] = []
	for name: String in e["names"]:
		if not name.begins_with("Fan @"):
			continue
		var label := name.trim_prefix("Fan @").to_float()
		if not source_labels.has(label):
			source_labels.append(label)
	assert_int(source_labels.size()).is_equal(2)
	for label: float in labels:
		for source_oid: float in source_labels:
			var msg := (
				"SweptNeighbour carries %.3f, within MIN_SEP of the fan's %.3f"
				% [label, source_oid]
			)
			assert_float(absf(label - source_oid)).append_failure_message(msg).is_greater_equal(
				0.08
			)


## THE issue-14 z-fight's own proof object, now read through the real
## per-face law. The shelf spans x 2..4, y 0..1, z 2.5..3.5; the crate
## embedded in its front half spans x 2.9..3.9, y 0..1, z 2.7..3.5 — so
## TWO patches rasterise twice at one depth: their front faces share the
## plane z = 3.5 (each solid's own +Z face, `render::paint::FACE_ORDER`
## ordinal 5), and their flush TOPS share y = 1 (ordinal 3). This is
## exactly the geometry `render::superface`'s merge law exists to catch —
## same-direction, coplanar, genuinely overlapping — so the two faces
## MERGE into one label, bit-equal, and `explain_oids`'s `faults` census
## (`observe::oids::coplanar_label_faults`, the identical predicate the
## merge law itself used) finds nothing to report. Checked both ways — the
## `faults` key stays present and empty, and the REAL per-face labels at
## both known planes are proven bit-equal AND distinct from their own
## placeholder ordinal (see the ground truth block below for why the
## placeholder check is load-bearing on its own) — so a regression that
## stopped merging this geometry, or stopped painting it at all, still
## fails here.
func test_two_flush_props_report_no_fault() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker())
	var shelf := WaveProp.new()
	shelf.name = "Shelf"
	shelf.size = Vector3(2, 1, 1)
	shelf.position = Vector3(3, 0.5, 3)
	level.add_child(shelf)
	var crate := WaveProp.new()
	crate.name = "Crate"
	crate.size = Vector3(1, 1, 0.8)
	crate.position = Vector3(3.4, 0.5, 3.1)
	level.add_child(crate)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var obs := _observer()
	obs.inject(level, _eye())
	var e: Dictionary = obs.explain_oids()
	assert_bool(e.has("faults")).is_true()
	var faults: Array = e.get("faults", [])
	(
		assert_array(faults)
		. append_failure_message(
			(
				"the real per-face census still finds a fault — the merge did not resolve it: %s"
				% [faults]
			)
		)
		. is_empty()
	)

	# THE GROUND TRUTH, ordinal 5 (+Z, the z = 3.5 merge) and ordinal 3
	# (+Y, the y = 1 merge), `render::paint::FACE_ORDER` order. Comparing
	# shelf[ordinal] to crate[ordinal] ALONE is a mirror assertion in
	# disguise: `labelled_box` writes the PLACEHOLDER ordinal itself
	# (3.0, 5.0) at the SAME block index in both props, so a skipped
	# relabel pass reads them back equal too. Each value is also checked
	# against its own placeholder — a real label is never bit-equal to
	# the ordinal that named its slot — so THIS fails under that mutation
	# where the equality check alone would not (confirmed, not assumed).
	var shelf_custom: PackedFloat32Array = _skin_of(shelf).mesh.surface_get_arrays(0)[
		Mesh.ARRAY_CUSTOM0
	]
	var crate_custom: PackedFloat32Array = _skin_of(crate).mesh.surface_get_arrays(0)[
		Mesh.ARRAY_CUSTOM0
	]
	for ordinal: int in [3, 5]:
		var shelf_label: float = shelf_custom[ordinal * 4]
		var crate_label: float = crate_custom[ordinal * 4]
		var placeholder_msg := (
			"ordinal %d still carries its placeholder — relabel never ran" % ordinal
		)
		assert_float(shelf_label).append_failure_message(placeholder_msg).is_not_equal(
			float(ordinal)
		)
		(
			assert_float(shelf_label)
			. append_failure_message("ordinal %d (shelf vs crate)" % ordinal)
			. is_equal(crate_label)
		)


## The first mesh limb a node built for itself.
func _skin_of(body: Node) -> MeshInstance3D:
	for child: Node in body.find_children("*", "MeshInstance3D", true, false):
		return child as MeshInstance3D
	return null


## The shipped map answers the census too. The KEY is the contract here,
## never the count: the shipped level's zero-faults invariant is pinned in
## map_test.gd, beside the geometry it constrains, and this case holds
## only the boundary's grammar — `faults` present, and an Array. An empty
## array must always mean "no faults" — a census that could not run is a
## one-key refusal, which the uninjected and freed-level tests above
## already hold explain_oids to.
func test_the_shipped_level_reports_its_faults_key() -> void:
	var level := _shipped_level(Pulses.new())
	var obs := _observer()
	obs.inject(level, _eye())
	var e: Dictionary = obs.explain_oids()
	assert_bool(e.has("faults")).is_true()
	assert_bool(e.get("faults") is Array).is_true()


## `superfaces` reports the class graph the last derive coloured, by name.
## The same code-built T-junction used by map_test's mesh-level proof must put
## both wall references in one class. No authored path or node name is part of
## the contract. Every class also carries at least one member, with no duplicate
## member inside a class.
func test_superfaces_groups_a_genuine_wall_junction_under_one_class() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker())
	var crossbar := WaveWall.new()
	crossbar.name = "FixtureCrossbar"
	crossbar.length = 6.0
	crossbar.position = Vector3(5, 0, 4)
	level.add_child(crossbar)
	var stem := WaveWall.new()
	stem.name = "FixtureStem"
	stem.length = 3.0
	stem.position = Vector3(5, 0, 5.5)
	stem.rotation.y = PI * 0.5
	level.add_child(stem)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var obs := _observer()
	obs.inject(level, _eye())
	var e: Dictionary = obs.explain_oids()
	var superfaces: Array = e.get("superfaces", [])
	assert_array(superfaces).is_not_empty()
	var crossbar_name := str(level.get_path_to(crossbar))
	var stem_name := str(level.get_path_to(stem))
	var junction_class: Variant = null
	for entry: Dictionary in superfaces:
		var members: Array = entry["members"]
		assert_array(members).is_not_empty()
		var seen := {}
		for member: String in members:
			(
				assert_bool(seen.has(member))
				. append_failure_message("class %d lists '%s' twice" % [entry["class"], member])
				. is_false()
			)
			seen[member] = true
		if members.has(crossbar_name) and members.has(stem_name):
			junction_class = entry["class"]
	(
		assert_bool(junction_class != null)
		. append_failure_message("no superface class lists both code-built walls")
		. is_true()
	)


## One wall, built in code rather than pinned to the shipped map's own
## layout, which is free to change census underneath this law. The
## centerline (6.4, 0.6)-(6.4, 8.0) is lifted unchanged from sight.rs's own
## `endpoint_grazes_are_not_crossings` cargo fixture, so the geometry is
## already hand-checked on the Rust side; this only pins that the boundary
## carries the same verdict.
func _one_wall_level() -> WaveLevel:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker())
	var wall := WaveWall.new()
	wall.name = "TheWall"
	wall.length = 7.4
	wall.position = Vector3(6.4, 0, 4.3)
	wall.rotation.y = PI * 0.5  # a z-run wall at x = 6.4, spanning z 0.6..8.0
	level.add_child(wall)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	return level


## Occlusion, answerable. The line crosses the one wall this fixture holds
## exactly once, born well clear of it, so the fan's WAVE is extinguished
## (0.0) while its silhouette survives at SOURCE_THROUGH — two different
## laws, which is the break this assertion catches.
func test_explain_ray_names_the_wall_it_crosses() -> void:
	var level := _one_wall_level()
	var obs := _observer()
	obs.inject(level, _eye())
	var e: Dictionary = obs.explain_ray(Vector3(3.0, 0.9, 4.0), Vector3(10.0, 0.9, 4.0))
	assert_int(e["camera_crossings"]).is_equal(1)
	assert_int(e["source_crossings"]).is_equal(1)
	assert_float(e["wave_transmission"]).is_equal_approx(0.0, 0.0001)
	assert_float(e["source_transmission"]).is_equal_approx(SOURCE_THROUGH, 0.0001)
	var crossed: Array[String] = []
	for wall: Dictionary in e["walls"]:
		if wall["crossed"]:
			crossed.append(wall["name"])
	assert_array(crossed).is_equal(["TheWall"])
	# ...and the SAME fixture with a line that crosses nothing, because a
	# 0.0 on its own is the value a dead field also reports: with only the
	# assertion above, `entry.set("wave_transmission", 0.0)` in
	# rust/src/nodes/observer.rs passes every suite in the repository while
	# the debugging surface AGENTS.md points agents at reports a wall that
	# is not there. This pair is what makes the field answerable.
	var clear: Dictionary = obs.explain_ray(Vector3(8.0, 0.9, 4.0), Vector3(10.0, 0.9, 4.0))
	assert_int(clear["source_crossings"]).is_equal(0)
	assert_float(clear["wave_transmission"]).is_equal_approx(1.0, 0.0001)


## The wall names are pinned to the table they name, not to whatever the
## scene tree holds at the moment the question is asked.
##
## The occluder table is derived ONCE, when the level enters the tree, and
## `walls[i].name` claims to name `wall_rects[i]`. A name list re-walked from
## the live tree on every call breaks that the instant a wall is added in
## front of the others: index 0 would name the newcomer while the table's
## slot 0 still holds the wall it was built from, and explain_ray would blame
## an innocent wall for an occlusion — the exact confident-wrong answer this
## layer exists to prevent. (Re-walking is also O(scene nodes) per sight
## line, with a dynamic-cast probe per node, which a fan of rays pays per
## ray.)
func test_wall_names_stay_pinned_to_the_table_they_name() -> void:
	var level := _one_wall_level()
	var obs := _observer()
	obs.inject(level, _eye())
	var before: Array[String] = _wall_names(obs)
	assert_array(before).is_equal(["TheWall"])
	var newcomer := WaveWall.new()
	newcomer.name = "AddedAfterTheTableWasDerived"
	level.add_child(newcomer)
	level.move_child(newcomer, 0)
	assert_array(_wall_names(obs)).is_equal(before)


## The composition root opens the window: main hands the observer the level
## it built and the hero's OWN eye, so a snapshot taken off the live scene
## answers rather than refusing. Read back from the real main scene — a
## window wired to nothing looks exactly like a working one until asked.
func test_the_composition_root_injects_the_observer() -> void:
	var main: UnseeingGame = auto_free(MAIN_SCENE.instantiate() as UnseeingGame)
	add_child(main)
	var snap: Dictionary = main.observer.snapshot(0.0)
	assert_bool(snap.has("unavailable")).is_false()
	assert_int((snap["slots"] as Array).size()).is_equal(64)
	assert_vector(snap["camera"]["position"]).is_equal(main.player.camera.global_position)
	# the eye's own projection, read off the live camera: without it a
	# reader cannot turn a world position into a screen position at all
	assert_float(snap["camera"]["fov"]).is_equal_approx(main.player.camera.fov, 0.0001)
	assert_bool(snap.has("sources")).is_true()
	assert_bool(snap["sources"] is Array).is_true()


## Asking why a wall stayed silent must not make it speak. The explanation
## re-runs the fan into a scratch buffer; if a single hit reached the real
## echo book, the question would have answered itself by changing the thing
## it asked about. This test was watched failing against a deliberately
## mutating implementation before it was trusted.
##
## The book is LOADED first, with a real reflecting sound, so the assertion
## is not 0 == 0: an explanation that scheduled echoes fails it, and so does
## one that drained or reordered the appointments already standing.
func test_explaining_a_reflection_schedules_no_echoes() -> void:
	var pulses := Pulses.new()
	var level := _reflection_level(pulses)
	var obs := _tree_observer(level)
	var before: Array[Vector3] = await _load_the_echo_book(pulses)
	assert_int(before.size()).is_greater(0)
	var id: int = obs.request_explain_reflection(TAP_AT, Vector3.UP, TAP_MAX_R, TAP_SPEED, 6, 0.0)
	await _physics_answer()
	var e: Dictionary = obs.take_explanation(id)
	assert_bool(e.has("pending")).is_false()
	assert_bool(e.has("unavailable")).is_false()
	assert_int(e["clusters_kept"]).is_greater(0)
	assert_int(pulses.pending_echo_count()).is_equal(before.size())
	assert_array(_echo_points(pulses)).is_equal(before)


## A physics space may only be touched inside the physics tick, so the
## answer cannot be synchronous: the request books an id and the frame does
## the casting. Pending is a state, never a zero-hit fan.
func test_an_explanation_is_pending_before_the_physics_frame_runs() -> void:
	var level := _reflection_level(Pulses.new())
	var obs := _tree_observer(level)
	var id: int = obs.request_explain_reflection(TAP_AT, Vector3.UP, TAP_MAX_R, TAP_SPEED, 6, 0.0)
	var pending: Dictionary = obs.take_explanation(id)
	assert_int(pending.size()).is_equal(1)
	assert_bool(pending["pending"]).is_true()


## An id that was never issued, and one whose answer has already been
## collected, are the same refusal — and it carries exactly one key.
func test_an_unknown_or_already_collected_request_is_refused() -> void:
	var level := _reflection_level(Pulses.new())
	var obs := _tree_observer(level)
	var refusal: Dictionary = obs.take_explanation(9999)
	assert_int(refusal.size()).is_equal(1)
	assert_bool(refusal.has("unavailable")).is_true()
	var id: int = obs.request_explain_reflection(TAP_AT, Vector3.UP, TAP_MAX_R, TAP_SPEED, 6, 0.0)
	await _physics_answer()
	assert_bool(obs.take_explanation(id).has("unavailable")).is_false()
	assert_bool(obs.take_explanation(id).has("unavailable")).is_true()


## The whole fan, not only the hits. The nominal fan is 26 rays; the
## hemisphere cull in front of the birth normal drops the ones that would
## point into the surface, so FEWER are cast — and both numbers are
## reported, or an agent could never see the cull. Rays that reached their
## full length and found nothing are the headline: in this world absence of
## echo is information, and a report of only the hits would hide it.
func test_the_explanation_reports_every_ray_not_only_the_hits() -> void:
	var level := _reflection_level(Pulses.new())
	var obs := _tree_observer(level)
	var id: int = obs.request_explain_reflection(TAP_AT, Vector3.UP, TAP_MAX_R, TAP_SPEED, 6, 0.0)
	await _physics_answer()
	var e: Dictionary = obs.take_explanation(id)
	var fan_size: int = e["fan_size"]
	var cast: int = e["rays_cast"]
	var struck: int = e["rays_struck"]
	var missed: int = e["rays_missed"]
	assert_int(fan_size).is_equal(obs.ray_fan_size())
	assert_int(cast).is_greater(0)
	assert_int(cast).is_less(fan_size)
	assert_int(missed).is_greater(0)
	assert_int(cast).is_equal(missed + struck)
	assert_int(e["clusters_kept"]).is_less_equal(6)
	assert_float(e["reach"]).is_equal_approx(TAP_MAX_R * 0.8, 0.0001)
	assert_vector(e["origin"]).is_equal_approx(TAP_AT + Vector3.UP * 0.08, Vector3.ONE * 0.0001)


## Every hit is accounted for by a NAMED reason. "The wall was found and
## then dropped past the budget", "the wall was the surface the sound was
## born on", and "the wall was never struck at all" are three different
## answers to why a wall stayed silent, and the report must not collapse
## them. The identity holds over the code-built room's known geometry.
func test_the_explanation_names_why_each_hit_did_not_answer() -> void:
	var level := _reflection_level(Pulses.new())
	var obs := _tree_observer(level)
	var id: int = obs.request_explain_reflection(TAP_AT, Vector3.UP, TAP_MAX_R, TAP_SPEED, 2, 0.0)
	await _physics_answer()
	var e: Dictionary = obs.take_explanation(id)
	var struck: int = e["rays_struck"]
	var self_surface: int = e["self_surface_drops"]
	var merged: int = e["merged_into_cells"]
	var cells: int = e["cells_found"]
	var past_budget: int = e["dropped_past_budget"]
	var kept: int = e["clusters_kept"]
	assert_int(e["budget"]).is_equal(2)
	assert_int(kept).is_equal(2)
	assert_int(struck).is_equal(self_surface + merged + cells)
	assert_int(cells).is_equal(past_budget + kept)
	assert_int(past_budget).is_greater(0)
	var point: Dictionary = e["points"][0]
	var dist: float = point["dist"]
	assert_float(point["at_t"]).is_equal_approx(dist / TAP_SPEED, 0.0001)
	assert_float(point["gain_fraction"]).is_greater(0.0)


## The documented debugging loop freezes the world FIRST, then asks. An
## observer that stopped ticking under a pause would answer pending forever
## to every question asked inside the loop it exists to serve — so it opts
## out of the pause exactly as the settings overlay does, and for the same
## reason. Nothing it does can advance the frozen world.
func test_a_frozen_world_still_answers() -> void:
	var level := _reflection_level(Pulses.new())
	var obs := _tree_observer(level)
	get_tree().paused = true
	var id: int = obs.request_explain_reflection(TAP_AT, Vector3.UP, TAP_MAX_R, TAP_SPEED, 6, 0.0)
	await _physics_answer()
	var e: Dictionary = obs.take_explanation(id)
	get_tree().paused = false
	assert_bool(e.has("pending")).is_false()
	assert_bool(e.has("unavailable")).is_false()
	assert_int(e["rays_cast"]).is_greater(0)


## A question whose numbers could only ever produce infinities is refused AT
## ONCE, without a frame. `at_t = now + d / 0` is +INF, and JSON.stringify
## renders that as null — an agent would read a missing field where there
## was an error. No polling, no pending, one key.
func test_a_sound_that_cannot_travel_is_refused_at_once() -> void:
	var level := _empty_level(Pulses.new())
	var obs := _tree_observer(level)
	for bad: PackedFloat64Array in [
		PackedFloat64Array([0.0, TAP_MAX_R]),
		PackedFloat64Array([-5.5, TAP_MAX_R]),
		PackedFloat64Array([TAP_SPEED, 0.0])
	]:
		var id: int = obs.request_explain_reflection(TAP_AT, Vector3.UP, bad[1], bad[0], 6, 0.0)
		var refusal: Dictionary = obs.take_explanation(id)
		assert_int(refusal.size()).is_equal(1)
		assert_str(refusal["unavailable"]).contains("refused")


## An observer standing in no tree has no world to cast in, and says so
## instead of promising an answer no frame will ever deliver. Every other
## verb on this class works out of the tree, so silence here would look
## exactly like a slow frame.
func test_an_observer_outside_the_tree_refuses_rather_than_promising() -> void:
	var obs := _observer()
	var id: int = obs.request_explain_reflection(TAP_AT, Vector3.UP, TAP_MAX_R, TAP_SPEED, 6, 0.0)
	var refusal: Dictionary = obs.take_explanation(id)
	assert_int(refusal.size()).is_equal(1)
	assert_str(refusal["unavailable"]).contains("physics world")


## Every wall the occluder table names, in table order — read through the
## one surface that publishes them.
func _wall_names(obs: WaveObserver) -> Array[String]:
	var names: Array[String] = []
	for wall: Dictionary in obs.explain_ray(Vector3.ZERO, Vector3.ONE)["walls"]:
		names.append(wall["name"])
	return names


func _observer() -> WaveObserver:
	return auto_free(WaveObserver.new()) as WaveObserver


## The snapshot's own entry for a code-built source reference, matched by the
## name the boundary carries.
func _source_entry(sources: Array, node_name: String) -> Dictionary:
	for entry: Dictionary in sources:
		if entry["name"] == node_name:
			return entry
	fail("the snapshot carries no source named '%s'" % node_name)
	return {}


## An observer standing in the same world as the level it reads. Reflection
## rays are cast against the space the observer's own viewport holds, so it
## has to be IN the tree — an observer outside one can only ever answer
## pending.
func _tree_observer(level: WaveLevel) -> WaveObserver:
	var obs := _observer()
	add_child(obs)
	obs.inject(level, _eye())
	return obs


## One physics tick, plus the idle frame that follows it — the request is
## booked from script and drained by the next `_physics_process`.
func _physics_answer() -> void:
	await get_tree().physics_frame
	await get_tree().physics_frame


## Put REAL appointments in the echo book, by emitting a reflecting sound
## the way the game does. Returns the answering points, so a test can pin
## that an explanation neither added to the book nor disturbed it — a
## before of zero would be satisfied by an observer that drained it.
func _load_the_echo_book(pulses: Pulses) -> Array[Vector3]:
	await get_tree().physics_frame  # space queries are legal only in physics
	var space := get_viewport().world_3d.direct_space_state
	pulses.emit_reflecting(0, TAP_AT, TAP_MAX_R, TAP_SPEED, 1.0, 0.0, space, 6, Vector3.UP)
	return _echo_points(pulses)


## The scheduled reflections, as points — the book's contents in its own
## discovery order.
func _echo_points(pulses: Pulses) -> Array[Vector3]:
	var points: Array[Vector3] = []
	for echo: Pulses.Echo in pulses.pending_echoes():
		points.append(echo.pos)
	return points


## The shipped scene's only observer-specific health probe: it can derive and
## expose the documented census grammar regardless of its current content.
func _shipped_level(pulses: Pulses) -> WaveLevel:
	var data_mat := ShaderMaterial.new()
	data_mat.set_shader_parameter("u_flick", FLICK)
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(data_mat, ShaderMaterial.new(), pulses)
	add_child(level)
	return level


## A content-free level for observer boundary tests that do not concern
## geometry. The marker makes its spawn payload explicit without requiring
## authored content.
func _empty_level(pulses: Pulses) -> WaveLevel:
	var data_mat := ShaderMaterial.new()
	data_mat.set_shader_parameter("u_flick", FLICK)
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker())
	level.inject(data_mat, ShaderMaterial.new(), pulses)
	add_child(level)
	return level


## A three-sided code-built room gives the reflection fan real hits, real
## misses through the open south edge, and more clusters than a small answer
## budget. Nothing here is a promise about the designer-owned scenes.
func _reflection_level(pulses: Pulses) -> WaveLevel:
	var data_mat := ShaderMaterial.new()
	data_mat.set_shader_parameter("u_flick", FLICK)
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker())
	for spec: Dictionary in [
		{"name": "North", "at": Vector3(4, 0, 1), "yaw": 0.0},
		{"name": "West", "at": Vector3(1, 0, 4), "yaw": PI * 0.5},
		{"name": "East", "at": Vector3(7, 0, 4), "yaw": PI * 0.5},
	]:
		var wall := WaveWall.new()
		wall.name = spec["name"]
		wall.length = 6.0
		wall.position = spec["at"]
		wall.rotation.y = spec["yaw"]
		level.add_child(wall)
	var table := WaveProp.new()
	table.name = "Reflector"
	table.size = Vector3(1.5, 0.2, 1.0)
	table.position = Vector3(4.5, 1.2, 4.0)
	level.add_child(table)
	level.inject(data_mat, ShaderMaterial.new(), pulses)
	add_child(level)
	return level


## An eye standing where the hero wakes.
func _eye() -> Camera3D:
	var cam: Camera3D = auto_free(Camera3D.new())
	cam.position = Vector3(3.0, 0.9, 4.0)
	add_child(cam)
	return cam


## An explicit typed spawn for fixtures whose subject is not fallback spawn
## selection.
func _spawn_marker() -> WaveSpawn:
	var marker := WaveSpawn.new()
	return marker


## The hero group binds the body, the eye, and the cane's out-tray into
## the SAME snapshot as the pool they feed — before this, the "one
## instant" guarantee stopped at the camera and the hero was eight
## separate reads across frames.
func test_the_snapshot_binds_the_hero_at_one_instant() -> void:
	var pulses := Pulses.new()
	var level := _empty_level(pulses)
	var player: UnseeingPlayer = auto_free(UnseeingPlayer.new())
	player.pulses = pulses
	player.position = Vector3(5.0, 0.9, 3.0)
	player.rotation.y = 0.7
	add_child(player)
	var obs: WaveObserver = auto_free(WaveObserver.new())
	obs.inject(level, player.camera)
	obs.inject_hero(player)
	add_child(obs)
	player.camera.rotation.x = -0.3
	player.queue_wave(2, Vector3.ZERO, 4.0, 4.0, 0.5, 0, Vector3.UP)
	player.tap()
	# read BEFORE any physics tick drains the queue or runs the tap: the
	# flag and the out-tray must appear beside the pool they will feed
	var snap: Dictionary = obs.snapshot(0.0)
	assert_bool(snap.has("unavailable")).is_false()
	var hero: Dictionary = snap["hero"]
	assert_vector(hero["position"]).is_equal_approx(Vector3(5.0, 0.9, 3.0), Vector3.ONE * 0.001)
	assert_float(hero["yaw"]).is_equal_approx(0.7, 0.0001)
	assert_float(hero["pitch"]).is_equal_approx(-0.3, 0.0001)
	assert_float(hero["last_tap"]).is_equal(-10.0)
	assert_bool(hero["tap_queued"]).is_true()
	var queued: Array = hero["queued_waves"]
	assert_int(queued.size()).is_equal(1)
	assert_int(queued[0]["type"]).is_equal(2)
	assert_vector(queued[0]["normal"]).is_equal(Vector3.UP)


## No hero is a NAMED absence, not a hero at the origin: a suite building
## a bare level has no player, and the snapshot says so in `unknown`.
func test_a_heroless_snapshot_names_the_absence() -> void:
	var pulses := Pulses.new()
	var level := _empty_level(pulses)
	var camera: Camera3D = auto_free(Camera3D.new())
	add_child(camera)
	var obs: WaveObserver = auto_free(WaveObserver.new())
	obs.inject(level, camera)
	add_child(obs)
	var snap: Dictionary = obs.snapshot(0.0)
	assert_bool(snap.has("hero")).is_false()
	assert_bool((snap["unknown"] as Array).has("hero")).is_true()


## A hero with no eye reports the SAME named absence a hero with no body
## would — never a pitch invented at zero, which is exactly the
## plausible-wrong-answer failure this whole layer exists to prevent.
## `_ready` always builds the camera as its very first act, so the only
## way `camera` is ever null past that point is to clear it by hand, the
## same way the fixture clears `pulses` above.
func test_a_cameraless_player_names_the_absence_rather_than_guessing_a_pitch() -> void:
	var pulses := Pulses.new()
	var level := _empty_level(pulses)
	var camera: Camera3D = auto_free(Camera3D.new())
	add_child(camera)
	var player: UnseeingPlayer = auto_free(UnseeingPlayer.new())
	player.pulses = pulses
	add_child(player)
	player.camera = null
	var obs: WaveObserver = auto_free(WaveObserver.new())
	obs.inject(level, camera)
	obs.inject_hero(player)
	add_child(obs)
	var snap: Dictionary = obs.snapshot(0.0)
	assert_bool(snap.has("hero")).is_false()
	assert_bool((snap["unknown"] as Array).has("hero")).is_true()


## A freed hero must degrade to the SAME named absence — never a crash,
## and never data read through a dangling handle.
func test_a_freed_hero_reports_unknown_rather_than_crashing() -> void:
	var pulses := Pulses.new()
	var level := _empty_level(pulses)
	var camera: Camera3D = auto_free(Camera3D.new())
	add_child(camera)
	var player := UnseeingPlayer.new()
	player.pulses = pulses
	add_child(player)
	var obs: WaveObserver = auto_free(WaveObserver.new())
	obs.inject(level, camera)
	obs.inject_hero(player)
	add_child(obs)
	remove_child(player)
	player.free()
	var snap: Dictionary = obs.snapshot(0.0)
	assert_bool(snap.has("hero")).is_false()
	assert_bool((snap["unknown"] as Array).has("hero")).is_true()


## The composition root hands the observer the hero it built, exactly as
## it hands it the level and the eye.
func test_the_composition_root_injects_the_hero() -> void:
	var main: UnseeingGame = auto_free(MAIN_SCENE.instantiate() as UnseeingGame)
	add_child(main)
	var snap: Dictionary = main.observer.snapshot(0.0)
	var hero: Dictionary = snap["hero"]
	assert_vector(hero["position"]).is_equal(main.player.global_position)
	assert_bool((snap["unknown"] as Array).has("hero")).is_false()
