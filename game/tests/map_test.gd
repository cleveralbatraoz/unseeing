extends GdUnitTestSuite
## THE SHIPPED MAP — scenes/level_01.tscn held to the design it realises.
## Everything here reads the authored scene back through the level root, so
## a node dragged, deleted or mistyped in the editor trips a test rather
## than a play session: the wall table the sight shaders occlude by, the
## spawn and the dev demo tap the level derives, the census of shapes that
## furnish it, and the object-id seam law across every touching pair.
##
## The two LAWS the map exists to make audible — how much of a source
## survives a wall, and what a wall costs against the volume ladder — are
## held on levels built in code instead. They are true of every level, and
## asserting them through the shipped scene's node ORDER made a source added
## in the editor crash the suite rather than fail it.
##
## The map is a 28 x 28 m plan of rooms on wall centerlines 0.6 m inside
## its edges:
##
##   z=0.6  +----------+---------------+------+-----------------+
##          |          |               | cor  |  RADIO ROOM     |
##          |  spawn   |   FAN ROOM    | ri   |  (its only door |
##          |  room    |               | dor  |   is the south) |
##   z=8.0  |          +---------+-----+      +--------+--------+
##          |          |         | east|      |                 |
##          |          |  hall   | room|   (doorway z 10..13)   |
##   z=13   +----+     |         |     |      |                 |
##          |nook|     +---+     |     |      |                 |
##   z=19.4 +----------+---+-----+-----+------+                 |
##          | workshop |  south corridor | store  |    yard      |
##   z=27.4 +----------+-----------------+--------+--------------+
##          x=0.6      x=8      x=14   x=19.4    x=21.4      x=27.4
##
## The radio room's WEST wall is the fan room's EAST wall, which is the
## whole point of the layout: the hero can stand in the fan room, one wall
## from a LOUDER source, and hear which is which.

const LEVEL_SCENE := preload("res://scenes/level_01.tscn")

## Full-strength crease separation, read off hearing_post.gdshader's
## smoothstep(0.04, 0.08, nrm) upper knee on the G channel.
const MIN_OID_SEP := 0.08

## Boxes that share a face register as touching at exactly zero overlap.
const TOUCH_EPS := 0.01


## The first mesh limb a node built for itself.
func _skin(body: Node) -> MeshInstance3D:
	for child: Node in body.get_children():
		if child is MeshInstance3D:
			return child as MeshInstance3D
	return null


## The Rust-side occluder inflation, mirrored: a centerline padded by a
## wall half-thickness (0.15) MINUS the 0.02 contact shrink each way —
## the exact rect sight.rs::wall_rect derives for the sight shaders.
func _occluder(seg: Vector4) -> Vector4:
	const PAD := 0.13
	return Vector4(
		minf(seg.x, seg.z) - PAD,
		minf(seg.y, seg.w) - PAD,
		maxf(seg.x, seg.z) + PAD,
		maxf(seg.y, seg.w) + PAD
	)


## The shipped level, instanced the way main does: injected first, then
## entered — every contract below is read back from the scene itself.
func _shipped_level() -> WaveLevel:
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	return level


## A level's sound source, found by the NAME its scene gives it rather than
## by where it sits in scene order. Scene order is what every derivation
## leans on, so it is exactly what a designer changes by dragging one node
## above another — and a positional lookup answers such an edit with a null
## to crash on instead of a sentence. A map that has lost the named source
## says so here, once, in words.
func _source_named(level: WaveLevel, node_name: String) -> Node3D:
	for source: Node3D in level.sources():
		if str(source.name) == node_name:
			return source
	fail("the level carries no sound source named '%s'" % node_name)
	return null


## The spawn marker every legal level needs — a level with no hero start is
## an error, and these built-in-code levels are not testing that.
func _spawn_marker(at: Vector3) -> Marker3D:
	var marker := Marker3D.new()
	marker.name = "SpawnPoint"
	marker.position = at
	return marker


func test_shipped_walls_axis_aligned_and_bordered() -> void:
	var segs := _shipped_level().wall_segments()
	assert_int(segs.size()).is_greater_equal(4)
	for s: Vector4 in segs:
		var axis_aligned := absf(s.w - s.y) < 0.001 or absf(s.z - s.x) < 0.001
		assert_bool(axis_aligned).append_failure_message("segment %s" % s).is_true()


## The demo tap lands on the FACE of the wall between the spawn and the
## fan — DividerNorth, whose centerline is x = 6.4 — a wall half-thickness
## (0.15) west toward the spawn, inside the wall's z-span, striking toward
## the spawn side (−X). No room rect: the wall is found from the spawn→fan
## line alone.
func test_shipped_demo_tap_sits_on_the_dividing_wall_face() -> void:
	var level := _shipped_level()
	var tap := level.demo_tap()
	assert_float(tap.x).is_equal_approx(6.25, 0.001)
	var on_wall := false
	for s: Vector4 in level.wall_segments():
		if absf(s.x - 6.4) < 0.001 and absf(s.z - 6.4) < 0.001:  # a z-run wall at x = 6.4
			if tap.z >= minf(s.y, s.w) and tap.z <= maxf(s.y, s.w):
				on_wall = true
	assert_bool(on_wall).is_true()
	assert_vector(level.demo_tap_normal()).is_equal(Vector3(-1, 0, 0))


func test_shipped_spawn_inside_bounds() -> void:
	var level := _shipped_level()
	var lo := Vector2(INF, INF)
	var hi := Vector2(-INF, -INF)
	for s: Vector4 in level.wall_segments():
		lo = Vector2(minf(lo.x, minf(s.x, s.z)), minf(lo.y, minf(s.y, s.w)))
		hi = Vector2(maxf(hi.x, maxf(s.x, s.z)), maxf(hi.y, maxf(s.y, s.w)))
	var spawn := level.spawn_pos()
	assert_bool(spawn.x > lo.x and spawn.x < hi.x).is_true()
	assert_bool(spawn.z > lo.y and spawn.z < hi.y).is_true()


## The scene IS the validated design: the numbers the retired LevelData
## carried by hand now derive from the authored nodes, unchanged.
func test_shipped_level_matches_validated_design() -> void:
	var level := _shipped_level()
	assert_int(level.wall_segments().size()).is_equal(19)
	assert_vector(level.extents).is_equal(Vector2(28, 28))
	assert_vector(level.spawn_pos()).is_equal_approx(Vector3(3, 0.9, 4), Vector3.ONE * 0.001)
	assert_float(level.spawn_yaw()).is_equal_approx(-1.9, 0.0001)
	# the demo tap is unchanged by the map's growth: it is planned from the
	# spawn and the FIRST source, and the fan is still first in scene order
	assert_vector(level.demo_tap()).is_equal_approx(Vector3(6.25, 0.8, 4.0), Vector3.ONE * 0.001)
	assert_vector(level.demo_tap_normal()).is_equal(Vector3(-1, 0, 0))
	var sources := level.sources()
	assert_int(sources.size()).is_equal(2)
	assert_vector(sources[0].position).is_equal_approx(Vector3(8.6, 0, 4.4), Vector3.ONE * 0.001)
	assert_vector(sources[1].position).is_equal_approx(
		Vector3(21.4, 0.78, 3.4), Vector3.ONE * 0.001
	)


## The per-object source muffle, on a level built in code: a source's
## standing silhouette is untouched in the open (1.0), survives ONE wall at
## SOURCE_THROUGH (0.3, a faint ghost), and is multiplied down again by
## every further wall. The compounding is the law — a muffle that merely
## flagged "walled" would read 0.3 through two walls as well, and a hero
## could not tell a source in the next room from one three rooms away.
##
## Two z-run walls at x = 4 and x = 12, an eye west of both: the fan shares
## the eye's room, the radio stands one wall east, and the far hub is two
## walls out.
func test_the_source_muffle_dims_once_per_wall_it_crosses() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	for x: float in [4.0, 12.0]:
		var wall := WaveWall.new()  # a z-run wall spanning z 0..8
		wall.length = 8.0
		wall.position = Vector3(x, 0, 4)
		wall.rotation.y = PI * 0.5
		level.add_child(wall)
	var fan := SoundFan.new()
	fan.position = Vector3(2, 0, 4)  # the eye's own room
	level.add_child(fan)
	var radio := SoundRadio.new()
	radio.position = Vector3(8, 0, 4)  # one wall east
	level.add_child(radio)
	level.add_child(_spawn_marker(Vector3(1, 0, 4)))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var eye := Vector3(1, 1.6, 4)
	var fan_hub := fan.global_position + Vector3(0, SoundFan.head_h(), 0)
	var radio_hub := radio.global_position + SoundRadio.hub_offset()
	assert_float(level.source_muffle(eye, fan_hub)).is_equal_approx(1.0, 0.0001)
	assert_float(level.source_muffle(eye, radio_hub)).is_equal_approx(0.3, 0.0001)
	assert_float(level.source_muffle(eye, Vector3(14, 1.15, 4))).is_equal_approx(0.09, 0.0001)


## THE map's reason for growing: the radio sits in a dedicated room whose
## west wall is the fan room's east wall, so from the fan room the hero is
## ONE wall from a louder source — and the two sources are heard from the
## same spot at different strengths. From the spawn, three rooms away, the
## radio is a ghost of a ghost.
func test_the_radio_is_one_wall_from_the_fan_room() -> void:
	var level := _shipped_level()
	var radio := _source_named(level, "Radio")
	var hub := radio.global_position + SoundRadio.hub_offset()
	var in_fan_room := Vector3(18.0, 1.6, 4.0)
	assert_float(level.source_muffle(in_fan_room, hub)).is_equal_approx(0.3, 0.0001)
	# and standing INSIDE the radio room there is nothing between them
	assert_float(level.source_muffle(Vector3(24.0, 1.6, 5.0), hub)).is_equal_approx(1.0, 0.0001)
	# from the spawn it is further than one wall — a much fainter ghost
	assert_bool(level.source_muffle(level.spawn_pos(), hub) < 0.3).is_true()


## A THIRD SOURCE dropped into the map must not re-point the test above.
## Scene order is the map's most editable property — a designer adds a
## source, or drags one above another, and every positional lookup in this
## suite silently changes what it is talking about. Worse than changing:
## `sources()[1] as SoundRadio` on a level whose second source is the fan
## yields a NULL, and the suite dies on the next property read instead of
## naming the map's fault.
func test_a_source_added_first_leaves_the_map_claim_where_it_was() -> void:
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	var intruder := SoundRadio.new()
	intruder.name = "Intruder"
	level.add_child(intruder)
	level.move_child(intruder, 0)  # ahead of the shipped fan and radio
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	assert_int(level.sources().size()).is_equal(3)
	assert_object(level.sources()[0]).is_same(intruder)  # it really is first
	var radio := _source_named(level, "Radio")
	assert_object(radio).is_not_same(intruder)
	var hub := radio.global_position + SoundRadio.hub_offset()
	assert_float(level.source_muffle(Vector3(18.0, 1.6, 4.0), hub)).is_equal_approx(0.3, 0.0001)


## A WALL COSTS MORE THAN THE LADDER IS WORTH, and the map is laid out so
## the hero meets that fact head on. With the quieter fan in the eye's own
## room and the LOUDER radio one wall east, the fan reads 0.75 in open air
## while the radio reads 0.30 — because the ladder is a factor of 1.33 and
## a wall is a factor of 3.33. That is not a bug to tune away: a quiet
## thing beside you genuinely does sound louder than a loud thing in the
## next room, which is what SOURCE_THROUGH exists to say. The standing
## image is a COMPOSITE of loudness and geometry, and this pins both halves
## at one concrete eye point.
##
## The layout is built in code because the law is not the map's: any level
## that puts the two sources this way owes these numbers.
##
## The ladder's own ordering is pinned like-for-like elsewhere — at equal
## wall count, in radio_wave.rs — so the two facts do not have to be true
## of the same number.
func test_a_wall_costs_more_than_the_volume_ladder_is_worth() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var wall := WaveWall.new()  # a z-run wall at x = 4, spanning z 0..8
	wall.length = 8.0
	wall.position = Vector3(4, 0, 4)
	wall.rotation.y = PI * 0.5
	level.add_child(wall)
	var fan := SoundFan.new()
	fan.position = Vector3(2, 0, 4)  # the eye's own room
	level.add_child(fan)
	var radio := SoundRadio.new()
	radio.position = Vector3(8, 0, 4)  # one wall east
	level.add_child(radio)
	level.add_child(_spawn_marker(Vector3(1, 0, 4)))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var eye := Vector3(1.0, 1.6, 4.0)
	var fan_hub := fan.global_position + Vector3(0, SoundFan.head_h(), 0)
	var radio_hub := radio.global_position + SoundRadio.hub_offset()
	var fan_image := fan.volume * level.source_muffle(eye, fan_hub)
	var radio_image := radio.volume * level.source_muffle(eye, radio_hub)
	assert_float(fan_image).is_equal_approx(0.75, 0.0001)  # unobstructed, quiet
	assert_float(radio_image).is_equal_approx(0.3, 0.0001)  # walled, but loud
	# the wall wins over the ladder — 3.33x against 1.33x
	assert_bool(fan_image > radio_image).is_true()
	# ...but the louder source is still FELT through it, which is the whole
	# privilege of being a world source: a hero one room away knows it is there
	assert_bool(radio_image > 0.0).is_true()
	# and the ladder is still doing its work underneath: the same radio behind
	# the same one wall reads stronger than the fan would behind that wall
	assert_bool(radio_image > fan.volume * 0.3).is_true()


## The shipped level carries the companion cat, exposes it for the root to
## tick, and has injected it the same way it injects a source — so it can
## both sound (pulse pool) and be seen (data-pass material).
func test_shipped_level_exposes_and_injects_the_cat() -> void:
	var level := _shipped_level()
	var cats := level.cats()
	assert_int(cats.size()).is_equal(1)
	var cat: WaveCat = cats[0]
	assert_object(cat.pulses).is_not_null()
	assert_object(cat.data_mat).is_not_null()
	# it wakes in the west room a few steps south of the hero, on the floor
	assert_vector(cat.position).is_equal_approx(Vector3(2.8, 0, 7.6), Vector3.ONE * 0.001)


func _count_kind(node: Node, kind: String) -> int:
	var n := 0
	for child: Node in node.get_children():
		if child.get_class() == kind:
			n += 1
		n += _count_kind(child, kind)
	return n


## The furniture census, by shape. A contours-only world has no material and
## no texture, so the SILHOUETTE is the whole of an object — which is why
## the vocabulary is three: boxes draw corners, columns draw curves, wedges
## draw diagonals, and a room furnished from only one of them reads flat.
## A prop added or lost in the editor trips this before any probe has to.
func test_shipped_prop_census() -> void:
	var level := _shipped_level()
	assert_int(_count_kind(level, "WaveProp")).is_equal(72)
	assert_int(_count_kind(level, "WaveColumn")).is_equal(27)
	assert_int(_count_kind(level, "WaveWedge")).is_equal(7)


## Every authored solid in the level that carries a flat object id, paired
## with the world box it fills. The sound sources and the cat are
## deliberately absent: each is a MULTI-limb object whose parts SHARE one id
## on purpose, so that it reads as a single silhouette — a pairwise "must
## differ" law is exactly wrong for them.
func _painted_boxes(node: Node, out: Array[Dictionary]) -> void:
	for child: Node in node.get_children():
		var skin := _skin(child)
		if skin != null:
			var oid := -1.0
			if child is WaveWall:
				oid = (child as WaveWall).oid()
			elif child is WaveProp:
				oid = (child as WaveProp).oid()
			elif child is WaveColumn:
				oid = (child as WaveColumn).oid()
			elif child is WaveWedge:
				oid = (child as WaveWedge).oid()
			if oid >= 0.0:
				(
					out
					. append(
						{
							"name": str(child.name),
							"box": skin.global_transform * skin.get_aabb(),
							"oid": oid,
						}
					)
				)
		_painted_boxes(child, out)


## Where two boxes interpenetrate there is no depth step, so the silhouette
## Laplacian on B has nothing to bite on — only the G-channel crease can
## draw their seam, and the shader fades it over smoothstep(0.04, 0.08).
## Two touching boxes closer than 0.08 in id therefore draw a weak seam, and
## IDENTICAL ids draw none at all: the pair melts into one silhouette. The
## shipped level must clear the knee on EVERY touching pair across all four
## solid shapes — nineteen walls and a hundred and six props, coloured from
## a five-entry palette by the touch graph, never by scene index.
func test_shipped_touching_boxes_draw_their_seam() -> void:
	var boxes: Array[Dictionary] = []
	_painted_boxes(_shipped_level(), boxes)
	assert_int(boxes.size()).is_equal(125)
	var melted: Array[String] = []
	for i: int in boxes.size():
		for j: int in range(i + 1, boxes.size()):
			var near: Dictionary = boxes[i]
			var far: Dictionary = boxes[j]
			var near_box: AABB = near["box"]
			var far_box: AABB = far["box"]
			if not near_box.grow(TOUCH_EPS).intersects(far_box):
				continue
			var near_oid: float = near["oid"]
			var far_oid: float = far["oid"]
			if absf(near_oid - far_oid) < MIN_OID_SEP:
				melted.append(
					"%s(%.2f) touches %s(%.2f)" % [near["name"], near_oid, far["name"], far_oid]
				)
	(
		assert_array(melted)
		. append_failure_message("touching boxes with no seam between them: %s" % str(melted))
		. is_empty()
	)


## Ids are reused wherever no pixel shows two boxes meeting — that reuse is
## what lets a five-entry palette dress a level of any size. A run that gave
## every box its own id would pass the seam law above while proving nothing.
func test_shipped_level_reuses_ids_between_distant_boxes() -> void:
	var boxes: Array[Dictionary] = []
	_painted_boxes(_shipped_level(), boxes)
	var distinct := {}
	for box: Dictionary in boxes:
		distinct[box["oid"]] = true
	assert_int(distinct.size()).is_less(boxes.size())


## The reveal-occlusion wall table reaches BOTH occluding skins — the
## world (reveal occlusion) and the source image (its silhouette's
## per-object muffle): one occluder rect per wall, the count and the wall
## top riding along, and exposed through wall_rects() for the hearing pass
## too. Nineteen walls now, against the sight shaders' 32 slots.
func test_wall_table_reaches_the_occluding_skins() -> void:
	var data_mat := ShaderMaterial.new()
	var source_mat := ShaderMaterial.new()
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(data_mat, source_mat, Pulses.new())
	add_child(level)
	var segs := level.wall_segments()
	assert_int(segs.size()).is_equal(19)
	for m: ShaderMaterial in [data_mat, source_mat]:
		var rects: PackedVector4Array = m.get_shader_parameter("u_walls")
		assert_int(rects.size()).is_equal(segs.size())
		for i: int in segs.size():
			assert_vector(rects[i]).is_equal_approx(_occluder(segs[i]), Vector4.ONE * 0.001)
		assert_int(m.get_shader_parameter("u_wall_count")).is_equal(segs.size())
		assert_float(m.get_shader_parameter("u_wall_top")).is_equal(3.0)
	assert_int(level.wall_rects().size()).is_equal(segs.size())


## Every solid the level draws, as the world box it actually fills — walls
## included, since the question below is whether the two collide.
func _solid_boxes(node: Node, out: Array[Dictionary]) -> void:
	for child: Node in node.get_children():
		var skin := _skin(child)
		var kind := child.get_class()
		if skin != null and kind in ["WaveWall", "WaveProp", "WaveColumn", "WaveWedge"]:
			(
				out
				. append(
					{
						"name": str(child.name),
						"kind": kind,
						"box": skin.global_transform * skin.get_aabb(),
					}
				)
			)
		_solid_boxes(child, out)


## How deeply two boxes interpenetrate on their least-overlapping axis.
## Zero or less means they merely touch, which is legal and often deliberate
## — a plank leaning on a wall, a board screwed to one.
func _depth(a: AABB, b: AABB) -> float:
	var lo := Vector3(
		maxf(a.position.x, b.position.x),
		maxf(a.position.y, b.position.y),
		maxf(a.position.z, b.position.z)
	)
	var hi := Vector3(
		minf(a.position.x + a.size.x, b.position.x + b.size.x),
		minf(a.position.y + a.size.y, b.position.y + b.size.y),
		minf(a.position.z + a.size.z, b.position.z + b.size.z)
	)
	return minf(hi.x - lo.x, minf(hi.y - lo.y, hi.z - lo.z))


## A prop buried in a wall is invisible from one side and unreachable from
## the other, and nothing in the engine can notice: a wall does not push a
## prop out, and the outline pass would happily draw the pair as one shape.
## So the SCENE is checked, not the plan that produced it — this holds
## whoever edits it and however.
func test_no_prop_is_buried_in_a_wall() -> void:
	var solids: Array[Dictionary] = []
	_solid_boxes(_shipped_level(), solids)
	assert_int(solids.size()).is_equal(125)
	var buried: Array[String] = []
	for prop: Dictionary in solids:
		if prop["kind"] == "WaveWall":
			continue
		for wall: Dictionary in solids:
			if wall["kind"] != "WaveWall":
				continue
			var prop_box: AABB = prop["box"]
			var wall_box: AABB = wall["box"]
			var deep := _depth(prop_box, wall_box)
			if deep > 0.001:
				buried.append("%s is %.3f m inside %s" % [prop["name"], deep, wall["name"]])
	(
		assert_array(buried)
		. append_failure_message("props buried in walls: %s" % str(buried))
		. is_empty()
	)


## Nothing sinks through the floor or pokes through the ceiling, and
## nothing has been dragged outside the map — the room is a closed box and
## the waves only light what is inside it.
func test_every_solid_stands_inside_the_room() -> void:
	var solids: Array[Dictionary] = []
	_solid_boxes(_shipped_level(), solids)
	var strays: Array[String] = []
	for solid: Dictionary in solids:
		var box: AABB = solid["box"]
		var hi := box.position + box.size
		if box.position.y < -0.001 or hi.y > WaveLevel.wall_height() + 0.001:
			strays.append("%s spans y %.2f..%.2f" % [solid["name"], box.position.y, hi.y])
		if box.position.x < 0.4 or hi.x > 27.6 or box.position.z < 0.4 or hi.z > 27.6:
			strays.append("%s is outside the map" % solid["name"])
	(
		assert_array(strays)
		. append_failure_message("solids out of the room: %s" % str(strays))
		. is_empty()
	)


## The seam law's tightest pair, and the one the box census cannot see: a
## SOURCE against the world it stands on. The id budget deliberately leaves
## a source's shell (0.33) only 0.01 from a world palette entry (0.34), and
## the only thing keeping them apart is the Fixed anchor the level feeds the
## colouring. Where a source and a solid touch there is no depth step, so an
## id difference under the shader's knee is the difference between a seam
## and two objects melted into one.
func test_no_solid_melts_into_a_sound_source_it_touches() -> void:
	var level := _shipped_level()
	var boxes: Array[Dictionary] = []
	_painted_boxes(level, boxes)
	var melted: Array[String] = []
	for source: Node3D in level.sources():
		var reach: AABB = _limb_box(source)
		for solid: Dictionary in boxes:
			var box: AABB = solid["box"]
			if not box.grow(TOUCH_EPS).intersects(reach):
				continue
			var oid: float = solid["oid"]
			for source_oid: float in _source_oids(source):
				if absf(oid - source_oid) < MIN_OID_SEP:
					melted.append(
						"%s(%.2f) touches %s(%.2f)" % [solid["name"], oid, source.name, source_oid]
					)
	(
		assert_array(melted)
		. append_failure_message("solids melted into a source: %s" % str(melted))
		. is_empty()
	)


## The world box a source's limbs fill, grown by whatever its moving parts
## sweep past this one pose — the same envelope the level bans neighbours
## from, so the test asks the question the colouring answered.
func _limb_box(source: Node) -> AABB:
	var box := AABB()
	var first := true
	for limb: MeshInstance3D in _limbs(source, [] as Array[MeshInstance3D]):
		var world: AABB = limb.global_transform * limb.get_aabb()
		box = world if first else box.merge(world)
		first = false
	var margin := 0.45 if source is SoundFan else 0.0
	return box.grow(margin)


func _limbs(node: Node, out: Array[MeshInstance3D]) -> Array[MeshInstance3D]:
	if node is MeshInstance3D:
		out.append(node as MeshInstance3D)
	for child: Node in node.get_children():
		_limbs(child, out)
	return out


## Every distinct flat id a source paints its limbs with, read back off the
## limbs themselves so the test cannot drift from what the data pass writes.
func _source_oids(source: Node) -> Array[float]:
	var ids: Array[float] = []
	for limb: MeshInstance3D in _limbs(source, [] as Array[MeshInstance3D]):
		var oid: float = limb.get_instance_shader_parameter("u_oid")
		if oid >= 0.0 and not ids.has(oid):
			ids.append(oid)
	return ids
