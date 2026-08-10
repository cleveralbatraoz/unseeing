extends GdUnitTestSuite
## WaveObserver — the debug observability boundary.
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

## The fan's shipped voice, from rust/src/fan_wave.rs: volume 0.75, cadence
## 0.4 s, wavefront 4.5 m/s. Everything the snapshot reports about it is
## derived here by hand rather than read from the code under test.
const FAN_VOLUME := 0.75
## FULL_REACH (12 m, sound_source.rs) x volume.
const FAN_REACH := 9.0
## Ring time 9 / 4.5 = 2 s, plus the source kind's 2 s fade tail, a wave
## every 0.4 s: (2 + 2) / 0.4 slots held at steady state.
const FAN_SLOT_PRESSURE := 10.0
## SOURCE_THROUGH (0.3, level_plan.rs) per wall between the eye and the hub:
## the standing image of a 0.75-loud fan one wall away, which is what the
## level pushes to the fan's limbs and what the observer reads back.
const FAN_FLOOR_ONE_WALL := 0.225
## A flicker value the composition root would have pushed to the world skin
## this frame. Nothing derives from it — it only has to be recognisable.
const FLICK := 0.6

## Where the hero wakes, which is where the explained cane tap lands: a
## point with walls, a table and a ceiling inside the fan's reach, and open
## floor to the south beyond it — so the same fan both strikes and misses.
const TAP_AT := Vector3(3.0, 0.9, 4.0)
## The cane tap's shipped wave: 6 m of range at 5.5 m/s, so the reflection
## fan reaches 0.8 x 6 = 4.8 m.
const TAP_MAX_R := 6.0
const TAP_SPEED := 5.5


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
## between the hero and a source, and how muffled its standing image is, are
## measured FROM the camera. An observer with no camera would have to invent
## one at the origin, and a plausible wrong number is worse than a refusal —
## so it refuses, while the eye-free explainers keep working.
func test_a_snapshot_without_an_eye_refuses_rather_than_guessing_one() -> void:
	var level := _shipped_level(Pulses.new())
	var obs := _observer()
	obs.inject(level, null)
	var snap: Dictionary = obs.snapshot(0.0)
	assert_int(snap.size()).is_equal(1)
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
	var level := _shipped_level(Pulses.new())
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
func test_snapshot_reports_a_tap_that_was_emitted() -> void:
	var pulses := Pulses.new()
	var level := _shipped_level(pulses, _eye())
	var obs := _observer()
	obs.inject(level, _eye())
	pulses.emit(0, Vector3.ZERO, 6.0, 5.5, 1.0, 0.0)
	var snap: Dictionary = obs.snapshot(0.5)
	assert_float(snap["now"]).is_equal_approx(0.5, 0.0001)
	assert_float(snap["flick"]).is_equal_approx(FLICK, 0.0001)
	assert_array(snap["unknown"]).is_empty()
	assert_int(snap["live_count"]).is_equal(1)
	assert_int((snap["slots"] as Array).size()).is_equal(64)
	var slot: Dictionary = snap["slots"][0]
	assert_int(slot["kind"]).is_equal(0)
	assert_float(slot["ring_radius"]).is_equal_approx(2.75, 0.001)
	assert_str(slot["state"]).is_equal("Live")
	assert_str(snap["slots"][1]["state"]).is_equal("Never")


## What could not be observed is NAMED, and its key is absent rather than
## zero. On a level no frame has ever run over, the world skin carries no
## flicker and no source has been pushed its standing image — and a flicker
## of zero, or a silhouette that reads as fully muffled, are both states the
## game can genuinely be in.
func test_a_snapshot_names_what_it_could_not_observe() -> void:
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var obs := _observer()
	obs.inject(level, _eye())
	var snap: Dictionary = obs.snapshot(0.0)
	assert_bool(snap.has("flick")).is_false()
	assert_bool((snap["sources"][0] as Dictionary).has("source_floor")).is_false()
	assert_array(snap["unknown"]).contains(
		["flick", "sources[0].source_floor", "sources[1].source_floor"]
	)


## The eviction prediction reaches the same pool the snapshot does: a virgin
## pool gives up slot 0 by the expired rule.
func test_explain_eviction_reads_the_injected_pool() -> void:
	var level := _shipped_level(Pulses.new())
	var obs := _observer()
	obs.inject(level, _eye())
	var plan: Dictionary = obs.explain_eviction(0.0)
	assert_int(plan["slot"]).is_equal(0)
	assert_str(plan["rule"]).is_equal("Expired")


## Every sound source the level holds, described as an agent reads it. The
## fan stands one wall from the spawn, so its standing image is its volume
## dimmed once — and that number is READ BACK off the limb the level pushed
## it to, not recomputed here or in the observer. Whatever the shader is
## actually holding is what the agent is told.
func test_snapshot_describes_the_levels_sound_sources() -> void:
	var level := _shipped_level(Pulses.new(), _eye())
	var obs := _observer()
	obs.inject(level, _eye())
	var sources: Array = obs.snapshot(0.0)["sources"]
	assert_int(sources.size()).is_equal(2)
	var fan: Dictionary = sources[0]
	assert_str(fan["name"]).is_equal("Fan")
	assert_float(fan["volume"]).is_equal_approx(FAN_VOLUME, 0.0001)
	assert_float(fan["reach"]).is_equal_approx(FAN_REACH, 0.0001)
	assert_float(fan["slot_pressure"]).is_equal_approx(FAN_SLOT_PRESSURE, 0.0001)
	assert_int(fan["walls_to_eye"]).is_equal(1)
	assert_float(fan["source_floor"]).is_equal_approx(FAN_FLOOR_ONE_WALL, 0.0001)


## The id budget over the SHIPPED map, checked by the same Rust the renderer
## colours with. Pairs must not be empty: a check that found no touching
## boxes at all would pass vacuously on a map where everything melts.
func test_the_shipped_level_has_no_object_id_violations() -> void:
	var level := _shipped_level(Pulses.new())
	var obs := _observer()
	obs.inject(level, _eye())
	var e: Dictionary = obs.explain_oids()
	assert_bool(e.has("unavailable")).is_false()
	assert_array(e["pairs"]).is_not_empty()
	assert_array(e["violations"]).is_empty()
	assert_float(e["min_sep"]).is_equal_approx(0.08, 0.0001)
	# the whole picture, named: the solids the colouring paints, the slabs
	# everything stands on, and one entry per id a source paints its own
	# limbs with — each of which a wall or a crate may melt into
	assert_array(e["names"]).contains(["Floor", "Ceiling", "DividerNorth", "Fan @0.33"])


## Occlusion, answerable. Spawn to fan head crosses exactly one wall on the
## shipped map — DividerNorth, at x = 6.4 — so the fan's WAVE arrives at
## HUM_THROUGH (0.55) and its silhouette at SOURCE_THROUGH (0.3).
func test_explain_ray_names_the_wall_between_spawn_and_fan() -> void:
	var level := _shipped_level(Pulses.new())
	var obs := _observer()
	obs.inject(level, _eye())
	var e: Dictionary = obs.explain_ray(Vector3(3.0, 0.9, 4.0), Vector3(8.6, 1.15, 4.4))
	assert_int(e["camera_crossings"]).is_equal(1)
	assert_int(e["source_crossings"]).is_equal(1)
	assert_float(e["hum_transmission"]).is_equal_approx(0.55, 0.0001)
	assert_float(e["source_transmission"]).is_equal_approx(0.3, 0.0001)
	var crossed: Array[String] = []
	for wall: Dictionary in e["walls"]:
		if wall["crossed"]:
			crossed.append(wall["name"])
	assert_array(crossed).is_equal(["DividerNorth"])


## The composition root opens the window: main hands the observer the level
## it built and the hero's OWN eye, so a snapshot taken off the live scene
## answers rather than refusing. Read back from the real main scene — a
## window wired to nothing looks exactly like a working one until asked.
func test_the_composition_root_injects_the_observer() -> void:
	var main: UnseeingMain = auto_free(MAIN_SCENE.instantiate() as UnseeingMain)
	add_child(main)
	var snap: Dictionary = main.observer.snapshot(0.0)
	assert_bool(snap.has("unavailable")).is_false()
	assert_int((snap["slots"] as Array).size()).is_equal(64)
	assert_vector(snap["camera"]["position"]).is_equal(main.player.camera.global_position)
	assert_int((snap["sources"] as Array).size()).is_equal(2)


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
	var level := _shipped_level(pulses)
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
	var level := _shipped_level(Pulses.new())
	var obs := _tree_observer(level)
	var id: int = obs.request_explain_reflection(TAP_AT, Vector3.UP, TAP_MAX_R, TAP_SPEED, 6, 0.0)
	var pending: Dictionary = obs.take_explanation(id)
	assert_int(pending.size()).is_equal(1)
	assert_bool(pending["pending"]).is_true()


## An id that was never issued, and one whose answer has already been
## collected, are the same refusal — and it carries exactly one key.
func test_an_unknown_or_already_collected_request_is_refused() -> void:
	var level := _shipped_level(Pulses.new())
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
	var level := _shipped_level(Pulses.new())
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
## them. The identity holds over the shipped room's real geometry.
func test_the_explanation_names_why_each_hit_did_not_answer() -> void:
	var level := _shipped_level(Pulses.new())
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


func _observer() -> WaveObserver:
	return auto_free(WaveObserver.new()) as WaveObserver


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


## The shipped level, instanced the way main does: injected first, then
## entered. The pool goes in from here so a test can put a sound into the
## very pool the observer reads back. The world skin carries a flicker the
## way the composition root leaves one there every frame; passing an eye
## drives one frame of sound sources, which is what puts each source's
## standing image on its limbs.
func _shipped_level(pulses: Pulses, eye: Camera3D = null) -> WaveLevel:
	var data_mat := ShaderMaterial.new()
	data_mat.set_shader_parameter("u_flick", FLICK)
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(data_mat, ShaderMaterial.new(), pulses)
	add_child(level)
	if eye != null:
		level.tick_sources(0.0, eye.global_position)
	return level


## An eye standing where the hero wakes.
func _eye() -> Camera3D:
	var cam: Camera3D = auto_free(Camera3D.new())
	cam.position = Vector3(3.0, 0.9, 4.0)
	add_child(cam)
	return cam


## The marker a hand-built level needs to have somewhere to wake the hero.
func _spawn_marker() -> Marker3D:
	var marker := Marker3D.new()
	marker.name = "SpawnPoint"
	return marker
