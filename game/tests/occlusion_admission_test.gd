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
