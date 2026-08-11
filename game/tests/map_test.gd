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


## A level's sound source, found by the NAME its scene gives it AND the
## class it is, rather than by where it sits in scene order. Scene order is
## what every derivation leans on, so it is exactly what a designer changes
## by dragging one node above another — and a positional lookup answers
## such an edit with a null to crash on instead of a sentence.
##
## Both halves of the identity are checked because each closes the other's
## hole: a class alone is re-pointed by a second source of that class, a
## name alone by anything renamed into its place, and neither miss is
## noisy. A map that has lost the source says so here, once, in words.
func _source_named(level: WaveLevel, node_name: String, kind: String) -> Node3D:
	for source: Node3D in level.sources():
		if str(source.name) == node_name and source.is_class(kind):
			return source
	fail("the level carries no %s named '%s'" % [kind, node_name])
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
	var radio := _source_named(level, "Radio", "SoundRadio")
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
	var radio := _source_named(level, "Radio", "SoundRadio")
	assert_object(radio).is_not_same(intruder)
	var hub := radio.global_position + SoundRadio.hub_offset()
	assert_float(level.source_muffle(Vector3(18.0, 1.6, 4.0), hub)).is_equal_approx(0.3, 0.0001)


## A NODE WEARING THE RADIO'S NAME is not the radio. A name is exactly as
## editable as scene order — a designer copies a source and renames it, or
## drops a fan into a grouping folder and calls it Radio — so a lookup that
## trusts only the name is re-pointed as silently as the positional one it
## replaced. The two halves of the identity close each other's hole: a
## class alone is re-pointed by a second radio, a name alone by a rename,
## and together they name one node.
func test_a_node_wearing_the_radios_name_is_not_the_radio() -> void:
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	var folder := Node3D.new()  # a grouping folder, so the name is free to reuse
	folder.name = "Furniture"
	level.add_child(folder)
	var impostor := SoundFan.new()
	impostor.name = "Radio"
	impostor.position = Vector3(18.0, 0, 4.0)  # in the fan room, beside the eye
	folder.add_child(impostor)
	level.move_child(folder, 0)  # and first in scene order
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	assert_object(level.sources()[0]).is_same(impostor)  # it really does come first
	var radio := _source_named(level, "Radio", "SoundRadio")
	assert_object(radio).is_not_same(impostor)
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


## The seam law's evil twin. Two same-facing faces sharing one plane AND
## rasterised area sit at the SAME depth, so the GPU picks a winner per
## pixel per frame — and where their object ids differ, the crease shader
## honours whichever id won, speckling the shared patch into a jagged band
## (issue #14). CAP_INSET cures exactly one family of these: a cap
## arriving in a junction partner's FLANK plane sinks 5 mm inside it. Two
## COLLINEAR WaveWall segments butted end-to-end are a different family —
## they share both flank planes along the whole joint and genuinely fight,
## and no inset can cure them; their cure is one longer wall, or a
## perpendicular joint. Furniture is the third family, authored clear by
## tucking flush panels. The shipped scene is held to the observer's
## coplanar-fight census coming back EMPTY.
func test_shipped_level_has_no_coplanar_face_fights() -> void:
	var level := _shipped_level()
	var obs: WaveObserver = auto_free(WaveObserver.new())
	obs.inject(level, null)
	var e: Dictionary = obs.explain_oids()
	assert_bool(e.has("fights")).append_failure_message("the census refused: %s" % e).is_true()
	var fights: Array[String] = []
	for fight: Dictionary in e.get("fights", []):
		fights.append(
			(
				"%s vs %s share the %s = %.3f plane"
				% [fight["name_a"], fight["name_b"], fight["axis"], fight["plane"]]
			)
		)
	(
		assert_array(fights)
		. append_failure_message(
			(
				(
					"same-facing coplanar faces z-fight into speckled bands: %s. Two "
					+ "cures, by family: a flush same-facing PANEL is tucked a few "
					+ "millimetres behind its neighbour's plane (on a 40 mm back panel "
					+ "a 5 mm tuck breaks the plane and still leaves the pair deeply "
					+ "touching); two COLLINEAR WaveWall segments butted end-to-end "
					+ "fight on both flank planes and no inset helps — merge them into "
					+ "one wall, or meet at a perpendicular joint."
				)
				% str(fights)
			)
		)
		. is_empty()
	)


## Every disagreement between a level's derived wall centerlines and the
## occluder table its skin was actually handed, as sentences.
##
## The walk stops at the SHORTER of the two, because a length mismatch is
## the very fault this check exists to catch: the table is truncated at the
## sight shaders' slots (MAXW, rust/src/sight.rs:32) and every wall past
## them stops occluding. Indexing the table by the SEGMENTS' index instead
## reads off its end, and Godot unwinds the whole case at that first read —
## measured on a 33-wall level: one "Out of bounds get index '32'", and the
## second skin never reached at all. The count mismatch does still report,
## because it is asserted before the walk; everything after it is lost.
## Stopping at the shorter length is what keeps the case alive to its end,
## and the fault sentence carries what the truncation COSTS — the rest stop
## occluding — rather than a bare pair of numbers to interpret.
func _table_faults(level: WaveLevel, mat: ShaderMaterial) -> Array[String]:
	var segs := level.wall_segments()
	var rects: PackedVector4Array = mat.get_shader_parameter("u_walls")
	var faults: Array[String] = []
	if rects.size() != segs.size():
		faults.append(
			(
				"%d walls but %d occluder rects: truncated, and the rest stop occluding"
				% [segs.size(), rects.size()]
			)
		)
	for i: int in mini(segs.size(), rects.size()):
		var want := _occluder(segs[i])
		if (rects[i] - want).length() > 0.001:
			faults.append("wall %d occludes as %s, not %s" % [i, rects[i], want])
	return faults


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
		var faults := _table_faults(level, m)
		(
			assert_array(faults)
			. append_failure_message(
				"the occluding skin was handed a wrong table: %s" % str(faults)
			)
			. is_empty()
		)
		assert_int(m.get_shader_parameter("u_wall_count")).is_equal(segs.size())
		assert_float(m.get_shader_parameter("u_wall_top")).is_equal(3.0)
	assert_int(level.wall_rects().size()).is_equal(segs.size())


## A level that outgrows the sight shaders' slots is REPORTED, not indexed
## into. `wall_rects()` truncates at MAXW (rust/src/sight.rs:32) and the
## walls past it silently stop occluding — a level-breaking fault, and the
## one this check exists to name. Walking the table by the SEGMENTS' index
## instead reads off its end and Godot unwinds the case there: the count
## mismatch does still report, since it is asserted first, but every
## assertion after that one overflow index is lost — the second skin, the
## wall count, the wall top, none of them reached.
func test_a_level_past_the_shader_slots_reports_the_truncation() -> void:
	var data_mat := ShaderMaterial.new()
	var level: WaveLevel = auto_free(WaveLevel.new())
	for i: int in 33:
		var wall := WaveWall.new()  # a z-run stub, four meters clear of the next
		wall.length = 2.0
		wall.position = Vector3(float(i) * 4.0, 0, 0)
		wall.rotation.y = PI * 0.5
		level.add_child(wall)
	level.add_child(_spawn_marker(Vector3.ZERO))
	level.inject(data_mat, ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var segs := level.wall_segments()
	assert_int(segs.size()).is_equal(33)
	(
		assert_bool(level.wall_rects().size() < segs.size())
		. append_failure_message("33 walls no longer overflow MAXW — build this level past it")
		. is_true()
	)
	var faults := _table_faults(level, data_mat)
	assert_int(faults.size()).is_equal(1)  # the truncation, and nothing invented past it
	assert_str(faults[0]).contains("33 walls but")
	assert_str(faults[0]).contains("truncated")


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


## Every solid that has left the room: sunk through the floor, poked out
## of the ceiling, or been dragged past the map's edge.
##
## The edge is read off the level's own EXTENTS, never written down. A
## border wall stands on a centerline MAP_BORDER inside that edge and its
## box reaches a wall half-thickness further out again, so the furthest
## anything legally stands is MAP_BORDER − WALL_HALF_T in from the edge —
## 0.45 m on any map, whatever its size — and EDGE_SLACK is the float dust
## that lets a border wall's own box land ON the bound instead of a hair
## outside it. Written down as a literal (0.4 and 27.6, the 28 x 28 map's
## own numbers) the same test passes vacuously on a bigger map and
## condemns the border walls themselves on a smaller one.
func _strays(level: WaveLevel) -> Array[String]:
	const MAP_BORDER := 0.6
	const WALL_HALF_T := 0.15  # level_plan.rs::WALL_T
	const EDGE_SLACK := 0.05
	var solids: Array[Dictionary] = []
	_solid_boxes(level, solids)
	var margin := MAP_BORDER - WALL_HALF_T - EDGE_SLACK
	var far := level.extents - Vector2(margin, margin)
	var strays: Array[String] = []
	for solid: Dictionary in solids:
		var box: AABB = solid["box"]
		var hi := box.position + box.size
		if box.position.y < -0.001 or hi.y > WaveLevel.wall_height() + 0.001:
			strays.append("%s spans y %.2f..%.2f" % [solid["name"], box.position.y, hi.y])
		if box.position.x < margin or hi.x > far.x or box.position.z < margin or hi.z > far.y:
			strays.append(
				(
					"%s spans x %.2f..%.2f z %.2f..%.2f, outside a %s map"
					% [solid["name"], box.position.x, hi.x, box.position.z, hi.z, level.extents]
				)
			)
	return strays


## Nothing sinks through the floor or pokes through the ceiling, and
## nothing has been dragged outside the map — the room is a closed box and
## the waves only light what is inside it.
func test_every_solid_stands_inside_the_room() -> void:
	var strays := _strays(_shipped_level())
	(
		assert_array(strays)
		. append_failure_message("solids out of the room: %s" % str(strays))
		. is_empty()
	)


## A level of one crate, sized by its extents knob — the smallest thing
## that can be inside a room or outside it.
func _one_crate_level(extents: Vector2, at: Vector3) -> WaveLevel:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.extents = extents
	var crate := WaveProp.new()  # the shipped default: a 0.5 m box
	crate.name = "Crate"
	crate.position = at
	level.add_child(crate)
	level.add_child(_spawn_marker(Vector3(1, 0, 1)))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	return level


## The room's edge is the EXTENTS knob's, not the shipped map's. One crate,
## spanning x 11.65..12.15, hangs 0.15 m past a 12 x 12 map's own edge —
## over the void, since the floor slab spans the extents and no further —
## and sits comfortably inside a 28 x 28 one. The law that says so has to
## move with the knob: against a written-down 27.6 the small map's stray
## reads as perfectly placed, which is the whole defect: a resized level
## passes this test by having nothing left to fail it.
func test_the_room_bound_follows_the_extents_knob() -> void:
	var crate_at := Vector3(11.9, 0.25, 6.0)
	var strays := _strays(_one_crate_level(Vector2(12, 12), crate_at))
	assert_int(strays.size()).is_equal(1)
	assert_str(strays[0]).contains("Crate")
	assert_array(_strays(_one_crate_level(Vector2(28, 28), crate_at))).is_empty()


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
