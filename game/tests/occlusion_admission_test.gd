extends GdUnitTestSuite
## Which solids actually stop sound — the geometric admission rule, held
## against the shipped level rather than against itself.
##
## Occlusion used to be decided by node CLASS: the occluder table was built
## from the wall census alone, so no prop could enter it whatever its shape,
## and three separate comments came to describe that as deliberate. It was
## not — the only argument ever recorded was the cost of admitting all 106
## props at once, which is real and is why `level_plan::spans_the_corridor`
## is narrow rather than why props were exempt.
##
## The rule now asks the geometry: floor to ceiling, and no thinner than a
## wall. What makes one rule sufficient is that the level's author had
## already separated the two populations in BOTH dimensions before anyone
## wrote it down — the pillars run [0.00, 3.00] and are 0.44-0.50 m across,
## the standpipes stop at 2.90 and are 0.14-0.20 m. These cases pin that
## against the real scene, because a geometric rule can only be trusted
## against real content.

const MAIN_SCENE := preload("res://scenes/main.tscn")


func _observer() -> WaveObserver:
	return auto_free(WaveObserver.new()) as WaveObserver


func _eye() -> Camera3D:
	var cam: Camera3D = auto_free(Camera3D.new())
	cam.position = Vector3(3.0, 0.9, 4.0)
	add_child(cam)
	return cam


func _wall_names(obs: WaveObserver) -> Array[String]:
	var names: Array[String] = []
	for wall: Dictionary in obs.explain_ray(Vector3.ZERO, Vector3.ONE)["walls"]:
		names.append(wall["name"])
	return names


func _one_wall_level() -> WaveLevel:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(WaveSpawn.new())
	var wall := WaveWall.new()
	wall.name = "TheWall"
	wall.length = 7.4
	wall.position = Vector3(6.4, 0, 4.3)
	wall.rotation.y = PI * 0.5
	level.add_child(wall)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	return level


## THE BREAK: the occluder table growing a second population while the name
## list only knows the first — after which explain_ray, whose entire job is
## to say WHICH occluder stopped a ray, runs off the end of its own list or
## blames the last authored wall for a pillar's work.
##
## The invariant is positional and unforgiving: wall_names()[i] names
## occluders[i]. Walls occupy the low slots and geometry-admitted solids are
## appended after them, so the two lists must grow together.
func test_geometry_admitted_solids_are_named_in_the_slots_they_occupy() -> void:
	var level := _one_wall_level()
	var pillar := WaveColumn.new()
	pillar.name = "APillarThatReachesTheCeiling"
	pillar.radius = 0.25
	pillar.height = 3.0
	level.add_child(pillar)
	# a column STANDS ON its origin (prop_shape::cylinder_lift), so y = 0
	# spans [0, 3] — floor to WALL_H
	pillar.position = Vector3(3.0, 0.0, 3.0)
	level.rederive()
	var obs := _observer()
	obs.inject(level, _eye())
	var names: Array[String] = _wall_names(obs)
	assert_array(names).contains(["APillarThatReachesTheCeiling"])
	# the authored wall must keep slot 0 — solids are APPENDED, never
	# interleaved, or every fault message points at the wrong node
	assert_str(names[0]).is_equal("TheWall")
	assert_int(names.find("APillarThatReachesTheCeiling")).is_greater(0)


## THE SHIPPED LEVEL, measured. The admission rule is geometric, so the only
## way to know what it actually admits is to ask the real scene.
##
## THE BREAK it catches runs both ways and both are expensive. Widen the rule
## and the seven standpipes come in — full height, 14-20 cm across — each
## casting a square shadow nearly a metre wide that no player can account
## for, on top of eating MAXW slots. Narrow it and the pillars stop
## occluding, which is the behaviour this whole change exists to add.
##
## The two populations were separated by the level's author in BOTH
## dimensions before anyone wrote a rule, which is why one rule can tell them
## apart: pillars run [0.00, 3.00] and are 0.44-0.50 m across; pipes stop at
## 2.90 and are 0.14-0.20 m.
func test_the_shipped_level_admits_its_pillars_and_refuses_its_pipes() -> void:
	var main: UnseeingGame = auto_free(MAIN_SCENE.instantiate() as UnseeingGame)
	add_child(main)
	var names: Array[String] = _wall_names(main.observer)
	var admitted: Array[String] = []
	for n: String in names:
		if n.contains("Pillar") or n.contains("Pipe"):
			admitted.append(n)
	admitted.sort()
	(
		assert_array(admitted)
		. is_equal(
			[
				"CorridorPillar",
				"HallPillarNorth",
				"HallPillarSouth",
				"RadioPillar",
				"SouthPillarA",
				"SouthPillarB",
				"WorkPillar",
			]
		)
	)
	# 19 authored wall segments + 7 pillars, inside MAXW = 32 with headroom
	assert_int(names.size()).is_equal(26)
	assert_int(names.size()).is_less_equal(WaveLevel.wall_slots())


## THE BREAK: a crate answering the cane with an echo while taking nothing
## at all from the source standing behind it.
##
## That asymmetry shipped for months and is indefensible under any reading:
## props ARE physics colliders, so the reflection fan strikes them and they
## spawn echoes, while the same tap's reveal passed straight through and lit
## the wall behind at full strength. Waves still pass a crate — that law is
## untouched — but a solid in the line now costs a source some of its
## clarity.
##
## Hand-derived from SOURCE_THROUGH = 0.3: one prop leaves sqrt(0.3) =
## 0.5477, and two props must leave exactly what one wall leaves, 0.3.
func test_a_prop_between_the_eye_and_a_source_costs_it_clarity() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(WaveSpawn.new())
	var crate := WaveProp.new()
	crate.name = "ACrateInTheWay"
	crate.size = Vector3(1.0, 0.8, 1.0)
	level.add_child(crate)
	crate.position = Vector3(3.0, 0.4, 0.0)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)

	# the crate tops out at 0.8 m in a 3 m room, so it is NOT a wave occluder
	var clear: float = level.source_muffle(Vector3(0.0, 0.4, 0.0), Vector3(1.0, 0.4, 0.0))
	assert_float(clear).is_equal_approx(1.0, 1e-6)

	# ...but a sight line straight through it does lose clarity
	var through: float = level.source_muffle(Vector3(0.0, 0.4, 0.0), Vector3(6.0, 0.4, 0.0))
	assert_float(through).is_equal_approx(0.5477225575, 1e-6)
	# strictly brighter than a wall would leave: a crate is not a barrier
	assert_float(through).is_greater(0.3)


## THE BREAK: the observer being handed only the wall table, after which it
## reports a source's muffle by a different law from the one the level
## composed and pushed. `explain_ray`'s whole reason to exist is that "a
## disagreement between the two would surface as a failing test here rather
## than as a plausible-looking wrong answer in the field", and the shipped
## level is full of crates, so this was wrong on most sight lines in the
## game. The cargo case pins the ARITHMETIC; only this end can pin that the
## boundary actually hands the prop table over.
func test_the_observers_muffle_is_the_one_the_level_composed() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(WaveSpawn.new())
	var crate := WaveProp.new()
	crate.name = "ACrateInTheWay"
	crate.size = Vector3(1.0, 0.8, 1.0)
	level.add_child(crate)
	crate.position = Vector3(3.0, 0.4, 0.0)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var obs := _observer()
	obs.inject(level, _eye())

	var eye := Vector3(0.0, 0.4, 0.0)
	var beyond := Vector3(6.0, 0.4, 0.0)
	var ray: Dictionary = obs.explain_ray(eye, beyond)

	# the crate is no wave occluder, so no WALL stands in the line...
	assert_int(ray["camera_crossings"]).is_equal(0)
	# ...but a prop does, and the observer must say so
	assert_int(ray["prop_crossings"]).is_equal(1)
	# and the number it reports must be the level's own, not walls-only
	assert_float(ray["source_transmission"]).is_equal_approx(level.source_muffle(eye, beyond), 1e-9)
	# walls alone would have answered 1.0 here, which is the defect
	assert_float(ray["source_transmission"]).is_less(0.9)


## THE BREAK: the prop clarity walk reading the WALL table, or vice versa —
## after which a crate would start stopping waves, or a pillar would stop
## dimming what stands behind it.
##
## The two tables are deliberately separate: the wall table is the one the
## shaders also read, so a wave and a silhouette agree on what a barrier is;
## the prop table exists only on the CPU and never enters a uniform.
func test_a_pillar_dims_a_source_as_a_wall_does_not_as_a_prop() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(WaveSpawn.new())
	var pillar := WaveColumn.new()
	pillar.name = "AStructuralPillar"
	pillar.radius = 0.25
	pillar.height = 3.0
	level.add_child(pillar)
	pillar.position = Vector3(3.0, 0.0, 0.0)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)

	# admitted as a wave occluder, so it dims by the FULL wall factor
	var through: float = level.source_muffle(Vector3(0.0, 1.6, 0.0), Vector3(6.0, 1.6, 0.0))
	assert_float(through).is_equal_approx(0.3, 1e-6)


## THE BREAK: the Inspector's answer drifting from the occluder table, or
## becoming authorable, or going stale after a knob moves.
##
## The geometric admission rule made `radius` and `height` silently decide
## barrier-versus-decoration: a standpipe nudged from 2.90 to 2.95 becomes an
## invisible sound-proof wall, and a pillar lowered six centimetres deletes a
## barrier the level depended on. Neither produces a warning or a visible
## change, and a designer had to ask a programmer to read
## `level_plan::spans_the_corridor`. This pins that the read-out exists, is
## read-only, and recomputes.
func test_a_solid_declares_in_the_inspector_whether_it_stops_sound() -> void:
	var col: WaveColumn = auto_free(WaveColumn.new())
	add_child(col)

	# a pillar: floor to ceiling, half a metre across
	col.radius = 0.25
	col.height = 3.0
	assert_bool(col.stops_sound).is_true()
	assert_str(col.sound_verdict).contains("Stops sound")

	# a standpipe: reaches, but far too thin — and the sentence must name
	# the criterion a designer can act on, not the other one
	col.radius = 0.07
	assert_bool(col.stops_sound).is_false()
	assert_str(col.sound_verdict).contains("too thin")
	assert_str(col.sound_verdict).contains("14 cm")

	# a crate-height column: wide enough, but sound goes over it
	col.radius = 0.25
	col.height = 0.9
	assert_bool(col.stops_sound).is_false()
	assert_str(col.sound_verdict).contains("go over it")
	assert_str(col.sound_verdict).contains("90 cm")

	# and it is a READ-OUT, not a knob
	var found := false
	for prop: Dictionary in ClassDB.class_get_property_list("WaveColumn", true):
		if prop["name"] == "stops_sound":
			found = true
			var usage: int = prop["usage"]
			assert_int(usage & PROPERTY_USAGE_READ_ONLY).is_greater(0)
	assert_bool(found).append_failure_message("no stops_sound property").is_true()
