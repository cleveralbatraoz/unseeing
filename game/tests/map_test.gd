# gdlint:ignore = max-public-methods
extends GdUnitTestSuite
## Level geometry and perception laws at the Godot/Rust boundary.
##
## Authored scenes are editable content, not golden fixtures: a designer may
## add, delete, move, rename, or re-nest any lawful node without rewriting a
## test. Exact geometry therefore lives in small code-built proof levels. The
## shipped scene is exercised only for content-independent health: it loads,
## derives, and reports no impossible label state for whatever it contains.

const LEVEL_SCENE := preload("res://scenes/level_01.tscn")

## Full-strength crease separation, read off hearing_post.gdshader's
## smoothstep(0.04, 0.08, nrm) upper knee on the G channel.
const MIN_OID_SEP := 0.08

## Boxes that share a face register as touching at exactly zero overlap.
const TOUCH_EPS := 0.01


## The first mesh limb a node built for itself.
func _skin(body: Node) -> MeshInstance3D:
	for child: Node in body.find_children("*", "MeshInstance3D", true, false):
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


## The shipped scene, injected and entered the way main does. Only the two
## content-independent health probes below use it.
func _shipped_level() -> WaveLevel:
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	return level


## An explicit typed spawn for fixtures whose subject is not fallback spawn
## selection.
func _spawn_marker(at: Vector3) -> WaveSpawn:
	var marker := WaveSpawn.new()
	marker.position = at
	return marker


## One authored-in-code wall between one spawn and one source. Its exact
## geometry is test input, so the tap law remains non-vacuous without making
## any wall or source mandatory in a designer-owned scene.
func _demo_level() -> WaveLevel:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3(2, 0, 4)))
	var wall := WaveWall.new()
	wall.length = 6.0
	wall.position = Vector3(6, 0, 4)
	wall.rotation.y = PI * 0.5
	level.add_child(wall)
	var source := SoundFan.new()
	source.position = Vector3(9, 0, 4)
	level.add_child(source)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	return level


## The demo tap lands on the FACE of SOME wall the level derived — no room
## rect, and never pinned to THIS map's own wall by name: level_plan.rs's
## demo_tap() clamps the spawn's own coordinate into the crossed wall's
## span and offsets a half-thickness off its centerline, so the LAW is
## purely geometric — the tap sits on a face plane (centerline ± WALL_T)
## inside a wall's span, and the returned normal points back toward the
## spawn, never into the wall it struck.
func test_demo_tap_sits_on_the_crossed_wall_face() -> void:
	# level_plan.rs::WALL_T, the same half-thickness _occluder() pads by
	const WALL_HALF_T := 0.15
	const FACE_EPS := 0.005
	var level := _demo_level()
	var tap := level.demo_tap()
	var normal := level.demo_tap_normal()
	var spawn := level.spawn_pos()
	var on_a_face := false
	for s: Vector4 in level.wall_segments():
		var z_run := absf(s.x - s.z) < 0.001  # centerline constant in x, spans z
		var span_lo := minf(s.y, s.w) if z_run else minf(s.x, s.z)
		var span_hi := maxf(s.y, s.w) if z_run else maxf(s.x, s.z)
		var along := tap.z if z_run else tap.x
		if along < span_lo - FACE_EPS or along > span_hi + FACE_EPS:
			continue  # outside this wall's span, however close its plane is
		var center := s.x if z_run else s.y
		var across := tap.x if z_run else tap.z
		if absf(absf(across - center) - WALL_HALF_T) < FACE_EPS:
			on_a_face = true
	assert_bool(on_a_face).append_failure_message("tap %s matches no wall face" % tap).is_true()
	# and the struck face's normal points back toward the spawn, never
	# parallel to the wall it was planned from
	(
		assert_float(normal.dot(spawn - tap))
		. append_failure_message("normal %s does not point spawn-ward from tap %s" % [normal, tap])
		. is_greater(0.0)
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


## A WALL COSTS MORE THAN THE LADDER IS WORTH. This fixture puts the quieter
## fan in the eye's own room and the LOUDER radio one wall east: the fan reads
## 0.75 in open air
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


## A cat is optional authored content. When one is present, the level must
## still inject the pulse pool and data material it needs.
func test_a_level_injects_every_cat_it_contains() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3(2, 0, 2)))
	var cat := WaveCat.new()
	cat.position = Vector3(4, 0, 4)
	level.add_child(cat)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	assert_array(level.cats()).contains([cat])
	assert_object(cat.pulses).is_not_null()
	assert_object(cat.data_mat).is_not_null()


## Every authored world solid carrying per-face CUSTOM0 labels, paired with
## the world box it fills. Sound sources and the cat are deliberately absent:
## neither contributes geometric faces to the world-superface merge census.
func _painted_boxes(node: Node, out: Array[Dictionary], root: Node = null) -> void:
	if root == null:
		root = node
	for child: Node in node.get_children():
		var skin := _skin(child)
		var painted := (
			child is WaveWall or child is WaveProp or child is WaveColumn or child is WaveWedge
		)
		if skin != null and painted:
			(
				out
				. append(
					{
						"name": str(root.get_path_to(child)),
						"box": skin.global_transform * skin.get_aabb(),
						"labels": _labels_of(skin),
					}
				)
			)
		_painted_boxes(child, out, root)


## Which solids the MERGE LAW ITSELF puts in one superface class with
## another solid, as a set of "A|B" keys in both orders — read off
## `explain_oids()['superfaces']`, which reports each class by the distinct
## names of the solids whose faces belong to it.
##
## Read from the law rather than inferred from the labels, deliberately:
## two touching solids that share a label because the COLOURING ran out of
## room would otherwise excuse themselves from the very check that exists
## to catch them.
func _merged_pairs(level: WaveLevel) -> Dictionary:
	var obs: WaveObserver = auto_free(WaveObserver.new())
	obs.inject(level, null)
	var e: Dictionary = obs.explain_oids()
	assert_bool(e.has("superfaces")).append_failure_message("the census refused: %s" % e).is_true()
	var pairs := {}
	for superface: Dictionary in e.get("superfaces", []):
		var members: Array = superface.get("members", [])
		for a: int in members.size():
			for b: int in range(a + 1, members.size()):
				pairs["%s|%s" % [members[a], members[b]]] = true
				pairs["%s|%s" % [members[b], members[a]]] = true
	return pairs


## Every DISTINCT label a solid's mesh actually carries, read off the whole
## CUSTOM0 array. Every vertex, so no face-ordering convention decides what
## this sees — which is the point: the reading this replaced was
## `WaveSolid::oid()`, the label of whichever face happens to be ordinal 0,
## and for a solid that genuinely merged, that one value names only its own
## first face's class and never its partner's.
func _labels_of(skin: MeshInstance3D) -> Array[float]:
	var custom: PackedFloat32Array = skin.mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	var out: Array[float] = []
	for label: float in custom:
		if not out.has(label):
			out.append(label)
	return out


## Two separate boxes meet face-to-face without a same-facing coplanar merge.
## Their exact geometry is a test input rather than a shipped-scene promise.
func _touching_prop_level() -> WaveLevel:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3(2, 0, 2)))
	for at: Vector3 in [Vector3(4, 0.5, 4), Vector3(5, 0.5, 4)]:
		var prop := WaveProp.new()
		prop.size = Vector3.ONE
		prop.position = at
		level.add_child(prop)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	return level


## Where two boxes meet there is no depth step, so the silhouette
## Laplacian on B has nothing to bite on — only the G-channel crease can
## draw their seam, and the shader fades it over smoothstep(0.04, 0.08).
## Two touching solids closer than 0.08 in label therefore draw a weak
## seam, and IDENTICAL labels draw none at all: the pair melts into one
## silhouette. Every touching pair in different superfaces must clear the
## knee; genuinely fused faces are intentionally one piece.
##
## EVERY label of one against EVERY label of the other, which is exactly
## what `render::superface`'s rule (c) promises for two touching solids in
## DIFFERENT clusters: it separates their classes blanket, not pairwise by
## face. A pair the law did fuse is skipped whole here rather than held to
## rule (b)'s finer per-face law; that finer case has its own pin, at real
## code-built geometry, in
## `test_a_junction_style_pair_merges_its_cap_and_separates_its_corner`.
func test_touching_boxes_draw_their_seam() -> void:
	var level := _touching_prop_level()
	var boxes: Array[Dictionary] = []
	_painted_boxes(level, boxes)
	assert_int(boxes.size()).is_equal(2)
	var merged := _merged_pairs(level)
	var melted: Array[String] = []
	for i: int in boxes.size():
		for j: int in range(i + 1, boxes.size()):
			var near: Dictionary = boxes[i]
			var far: Dictionary = boxes[j]
			var near_box: AABB = near["box"]
			var far_box: AABB = far["box"]
			if not near_box.grow(TOUCH_EPS).intersects(far_box):
				continue
			if merged.has("%s|%s" % [near["name"], far["name"]]):
				continue
			var closest := INF
			for near_label: float in near["labels"]:
				for far_label: float in far["labels"]:
					closest = minf(closest, absf(near_label - far_label))
			if closest < MIN_OID_SEP:
				melted.append(
					(
						"%s touches %s, closest labels %.3f apart"
						% [near["name"], far["name"], closest]
					)
				)
	(
		assert_array(melted)
		. append_failure_message("touching solids with no seam between them: %s" % str(melted))
		. is_empty()
	)


## The seam law's evil twin. Two same-facing faces sharing one plane AND
## rasterised area sit at the SAME depth, so the GPU picks a winner per
## pixel per frame — a genuine problem only where the two writers disagree
## on colour.
##
## `explain_oids()`'s `faults` census (`observe::oids::coplanar_label_faults`)
## reasons at real FACE granularity, over the exact per-face labels
## `WaveLevel::paint_labels` baked (`WaveLevel::face_census`) — not the
## solid-granularity bridge the old fight census read, which this suite
## used to have to cross-check against the raw mesh bytes by hand to tell
## a genuine defect from a stale artifact of that bridging.
##
## WHAT THIS CASE ACTUALLY BINDS, stated plainly because the name promises
## more: the merge law and this census share ONE predicate and ONE labels
## array, so a merge candidate reads the same array element twice and
## `faults` is `[]` for any level, any geometry, any bug. That is the
## design's "impossible by construction" goal reported back — a
## postcondition over the CLASS GRAPH — and the only assertion here with
## teeth is `has("faults")`: the census REFUSES (returns no key) when the
## labels array is shorter than the faces array, which is a real
## derive-time defect this catches.
##
## The BAKE is pinned separately, by the cases in this suite that read real
## ARRAY_CUSTOM0 bytes and locate faces geometrically —
## `test_a_junction_style_pair_merges_its_cap_and_separates_its_corner` and
## `test_a_wall_clears_the_floor_and_ceiling_labels`.
func test_shipped_level_has_no_label_faults() -> void:
	var level := _shipped_level()
	var obs: WaveObserver = auto_free(WaveObserver.new())
	obs.inject(level, null)
	var e: Dictionary = obs.explain_oids()
	assert_bool(e.has("faults")).append_failure_message("the census refused: %s" % e).is_true()
	(
		assert_array(e.get("faults", []))
		. append_failure_message(
			"same-facing coplanar faces z-fight into speckled bands: %s" % str(e.get("faults"))
		)
		. is_empty()
	)


## A shipped level must derive without exhausting the label graph, whatever
## lawful content it currently contains. Exact graph shapes are proved by the
## code-built cases; this is only the scene-level health boundary that turns a
## real derivation error into a focused failure.
func test_shipped_level_derives_with_no_starved_classes() -> void:
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var enter := func() -> void: add_child(level)
	await assert_error(enter).is_success()


## The wall<->slab seam at real per-face resolution. A wall's OWN six face
## labels — read straight off its mesh CUSTOM0 channel, not the coarser
## first-face bridge — must clear BOTH Floor (0.15)
## and Ceiling (0.90) by at least MIN_OID_SEP. Hand-derived: every wall
## takes its label from the five-entry WORLD_OIDS palette
## ([0.25, 0.34, 0.43, 0.52, 0.61], `rust/src/nodes/level.rs`) — walls and
## source roles are graph-coloured; only slabs are anchored — and every one
## of those five sits comfortably clear of both slab role labels
## (`render::labels::role_label`) already; this is the wiring pin that
## would catch a wall ever inheriting 0.15/0.90 directly, the way it
## could if a wall's own cluster were ever silently merged into a slab's
## (the ledgered, currently-unreachable anchor-conflict case). The one wall
## below is exact test input; no authored scene is required to keep a wall.
func test_a_wall_clears_the_floor_and_ceiling_labels() -> void:
	const FLOOR_LABEL := 0.15
	const CEILING_LABEL := 0.90
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3(2, 0, 2)))
	var wall := WaveWall.new()
	wall.length = 4.0
	wall.position = Vector3(5, 0, 5)
	level.add_child(wall)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var violations: Array[String] = []
	var custom: PackedFloat32Array = _skin(wall).mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	for f in 6:
		var label: float = custom[f * 4]
		if absf(label - FLOOR_LABEL) < MIN_OID_SEP:
			violations.append("face %d = %.3f too close to Floor (%.2f)" % [f, label, FLOOR_LABEL])
		if absf(label - CEILING_LABEL) < MIN_OID_SEP:
			violations.append(
				"face %d = %.3f too close to Ceiling (%.2f)" % [f, label, CEILING_LABEL]
			)
	(
		assert_array(violations)
		. append_failure_message("wall labels too close to a slab role label: %s" % str(violations))
		. is_empty()
	)


## THE SUPERFACE LAW, live, off two code-built `WaveWall` meshes. A crossbar
## centred on z = 4 and a perpendicular stem whose centerline starts on z = 4
## meet at world z = 4 − 0.15 (WALL_T) = 3.85 — hand-derived, not read back
## off the built mesh — exactly the geometry
## `render::superface`'s own `a_junction_cap_merges_into_the_partners_flank`
## fixture proves merges, now checked through the real node → mesh
## pipeline instead of bare `Shape` literals.
func test_a_junction_style_pair_merges_its_cap_and_separates_its_corner() -> void:
	const MERGE_Z := 3.85
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3(2, 0, 2)))
	var a := WaveWall.new()
	a.length = 6.0
	a.position = Vector3(5, 0, 4)
	level.add_child(a)
	var b := WaveWall.new()
	b.length = 3.0
	b.position = Vector3(5, 0, 5.5)
	b.rotation.y = PI * 0.5
	level.add_child(b)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)

	var a_skin := _skin(a)
	var b_skin := _skin(b)
	var a_face := _face_nearest_world_z(a_skin, MERGE_Z)
	var b_face := _face_nearest_world_z(b_skin, MERGE_Z)
	# both faces genuinely SIT on the merge plane — not merely the closest
	# of the six candidates each box happens to offer
	assert_float(_face_centroid(a_skin, a_face).z).is_equal_approx(MERGE_Z, 0.0001)
	assert_float(_face_centroid(b_skin, b_face).z).is_equal_approx(MERGE_Z, 0.0001)
	assert_bool(_face_labels_are_uniform(a_skin, a_face)).is_true()
	assert_bool(_face_labels_are_uniform(b_skin, b_face)).is_true()

	# THE MERGE: the same plane draws ONE label on both meshes, bit-equal
	# as f32 (a PackedFloat32Array element widens losslessly to GDScript's
	# float, so a plain `is_equal` IS the bit check).
	var merged_label := _face_label(a_skin, a_face)
	assert_float(_face_label(b_skin, b_face)).is_equal(merged_label)

	# THE CORNER: the stem's own east/west thickness face is
	# PERPENDICULAR to the merged plane and must differ by at least
	# MIN_SEP — the crease the corner itself draws.
	var perp_face := _face_with_centroid_x_above(b_skin, 5.1)
	var perp_label := _face_label(b_skin, perp_face)
	assert_float(absf(perp_label - merged_label)).is_greater_equal(0.08)


## The four vertices belonging to face `f` (0..6, `render::paint::FACE_ORDER`
## order: −X,+X,−Y,+Y,−Z,+Z) as an UNSHARED block — a face is identified by
## its OWN block, never by a coordinate two neighbouring faces would also
## match at a shared corner (`mesh_label_test.gd` holds this same trap).
func _face_centroid(skin: MeshInstance3D, f: int) -> Vector3:
	var verts: PackedVector3Array = skin.mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
	var sum := Vector3.ZERO
	for i in range(f * 4, f * 4 + 4):
		sum += skin.global_transform * verts[i]
	return sum / 4.0


func _face_label(skin: MeshInstance3D, f: int) -> float:
	var custom: PackedFloat32Array = skin.mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	return custom[f * 4]


func _face_labels_are_uniform(skin: MeshInstance3D, f: int) -> bool:
	var custom: PackedFloat32Array = skin.mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	var first: float = custom[f * 4]
	for i in range(f * 4 + 1, f * 4 + 4):
		if custom[i] != first:
			return false
	return true


## The face (0..6) whose CENTROID sits nearest `plane_z` in world space —
## identified by centroid, immune to the corner-sharing trap a raw
## per-vertex coordinate filter would fall into.
func _face_nearest_world_z(skin: MeshInstance3D, plane_z: float) -> int:
	var best := -1
	var best_d := INF
	for f in 6:
		var d := absf(_face_centroid(skin, f).z - plane_z)
		if d < best_d:
			best_d = d
			best = f
	return best


## The one face whose centroid's world X exceeds `threshold` — a wall's own
## thickness face, off to one side, never its end caps or its top/bottom
## (both centred on the wall's own run).
func _face_with_centroid_x_above(skin: MeshInstance3D, threshold: float) -> int:
	for f in 6:
		if _face_centroid(skin, f).x > threshold:
			return f
	return -1


## THE FLANK'S OWN LAW, live, re-derived for Wave S. Base and Post never
## genuinely MERGE: Post's base rim faces DOWN, Base's top face faces UP —
## an ordinary ABUTMENT, not a same-direction coplanar overlap — so each
## stays alone in its own singleton cluster
## (`render::superface::superfaces`'s own collapse). Post's rims and
## flank now read as ONE uniform label — no internal seam, today's look,
## `render::paint::add_flank_classes`'s singleton branch aliasing the
## flank onto the rims' own collapsed class rather than winning a fresh
## one — while the OUTER seam against Base still draws, carried entirely
## by rule (c)'s ordinary blanket law between two different, touching
## clusters (the same law `an_abutment_through_the_coordinate_origin_still_does_not_merge`
## and its siblings hold in `rust/src/render/superface.rs`), inherited by
## the flank automatically since it now shares Post's own class number.
##
## Base spans y 0..1 (size 1x1x1); Post radius 0.1, height 1, at y = 1, so
## its base rim sits exactly on Base's own top — and, as a singleton
## itself, ALL SIX of Base's own faces now read the identical label too;
## checked directly rather than assumed, so this test's own premise is
## self-verifying, not inherited from the fixture description alone.
func test_a_lone_columns_flank_joins_its_rims_and_still_differs_from_its_neighbour() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var base := WaveProp.new()
	base.name = "Base"
	base.size = Vector3(1, 1, 1)
	base.position = Vector3(3, 0.5, 3)
	level.add_child(base)
	var post := WaveColumn.new()
	post.name = "Post"
	post.radius = 0.1
	post.height = 1.0
	post.position = Vector3(3, 1, 3)
	level.add_child(post)
	level.add_child(_spawn_marker(Vector3(3, 0, 3)))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)

	var post_skin := _skin(post)
	var bottom_rim := _column_ordinal_label(post_skin, 0)
	var top_rim := _column_ordinal_label(post_skin, 1)
	var flank := _column_ordinal_label(post_skin, 2)

	# a real palette value, never the freshly-allocated 0.0 default the
	# flank slot starts at — catches the sibling mutation (skipping the
	# flank-label write in `WaveLevel::paint_labels`), which the equality
	# checks alone would pass vacuously (0.0 would equal a 0.0 default
	# on all three, not for the reason under test).
	assert_float(flank).is_between(0.15, 0.96)

	# NO internal seam: rim and flank now read the SAME label — the
	# mutation this catches is the singleton aliasing being removed
	# (reverted to the old behaviour of a fresh, separated flank class),
	# which is exactly what the OLD assertions here used to require.
	assert_float(flank).is_equal(bottom_rim)
	assert_float(flank).is_equal(top_rim)

	# Base is ALSO a singleton: every one of its six real faces reads the
	# identical label — the fixture's own premise, self-checked.
	var base_custom: PackedFloat32Array = _skin(base).mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	var base_label: float = base_custom[0]
	for ord in 6:
		var base_face: float = base_custom[ord * 4]
		var msg := (
			"Base ordinal %d = %.3f, expected the uniform %.3f" % [ord, base_face, base_label]
		)
		assert_float(base_face).append_failure_message(msg).is_equal(base_label)

	# the OUTER seam still draws: Post's whole class (rims+flank) differs
	# from Base's own by at least MIN_SEP — carried by rule (c)'s blanket
	# law between the two different, touching clusters, inherited by the
	# flank purely by sharing Post's class number.
	assert_float(absf(flank - base_label)).is_greater_equal(0.08)


## The label a code-built column's mesh carries at ordinal `ord` (0 bottom
## rim, 1 top rim, 2 flank) — read by POSITION, not by value:
## `resize_triangle_surface` never groups by ordinal into a fixed block
## the way a box's FACE_ORDER does, but `column_triangles`'s own emission
## order is fixed — 12 vertices per `COLUMN_SEGMENTS` segment, laid out
## [bottom x3, top x3, flank x6] — so ordinal membership is a POSITION
## fact (index % 12) that survives relabelling, unlike the value once
## painted over its own placeholder ordinal.
func _column_ordinal_label(skin: MeshInstance3D, ord: int) -> float:
	var custom: PackedFloat32Array = skin.mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	var first := NAN
	for i: int in custom.size():
		var local: int = i % 12
		var this_ord := 2 if local >= 6 else (1 if local >= 3 else 0)
		if this_ord == ord:
			if is_nan(first):
				first = custom[i]
			else:
				assert_float(custom[i]).is_equal(first)
	return first


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


## The reveal-occlusion wall table reaches BOTH occluding skins — the world
## and the source image — with one occluder rect per wall. Two deliberately
## different code-built walls make the table non-vacuous without freezing a
## designer-owned scene's count, order, position, or names.
func test_wall_table_reaches_the_occluding_skins() -> void:
	var data_mat := ShaderMaterial.new()
	var source_mat := ShaderMaterial.new()
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3(2, 0, 2)))
	var x_run := WaveWall.new()
	x_run.length = 5.0
	x_run.position = Vector3(5, 0, 3)
	level.add_child(x_run)
	var z_run := WaveWall.new()
	z_run.length = 4.0
	z_run.position = Vector3(9, 0, 7)
	z_run.rotation.y = PI * 0.5
	level.add_child(z_run)
	level.inject(data_mat, source_mat, Pulses.new())
	add_child(level)
	var segs := level.wall_segments()
	assert_int(segs.size()).is_equal(2)
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


## A level of one crate, sized by its extents knob — the smallest thing
## that can be inside a room or outside it.
func _one_crate_level(extents: Vector2, at: Vector3) -> WaveLevel:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.extents = extents
	var crate := WaveProp.new()  # the node default: a 0.5 m box
	crate.name = "Crate"
	crate.position = at
	level.add_child(crate)
	level.add_child(_spawn_marker(Vector3(1, 0, 1)))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	return level


## The room's edge is the EXTENTS knob's, not one authored map's. One crate,
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


## The seam law's tightest cross-domain pair: derived semantic source roles
## against the world face classes they touch. Check every real CUSTOM0 label,
## not the retired first-face bridge or a semantic-role preview default. The
## swept neighbour is exact code-built input, so the check cannot pass merely
## because an authored level currently has no source or no touching solid.
func test_world_faces_clear_the_source_roles_they_touch() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3(2, 0, 4)))
	var wall := WaveWall.new()
	wall.length = 4.0
	wall.position = Vector3(3, 0, 4)
	wall.rotation.y = PI * 0.5
	level.add_child(wall)
	var source := SoundFan.new()
	source.position = Vector3(5, 0, 4)
	level.add_child(source)
	var neighbour := WaveProp.new()
	neighbour.size = Vector3(0.2, 0.2, 0.2)
	neighbour.position = Vector3(5.6, 1.15, 4)
	level.add_child(neighbour)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var boxes: Array[Dictionary] = []
	_painted_boxes(level, boxes)
	var melted: Array[String] = []
	var compared := 0
	var reach: AABB = _limb_box(source)
	for solid: Dictionary in boxes:
		var box: AABB = solid["box"]
		if not box.grow(TOUCH_EPS).intersects(reach):
			continue
		compared += 1
		for world_label: float in solid["labels"]:
			for role_label: float in _source_labels(source):
				if absf(world_label - role_label) < MIN_OID_SEP:
					melted.append(
						(
							"%s(%.2f) touches source role(%.2f)"
							% [solid["name"], world_label, role_label]
						)
					)
	assert_int(compared).is_equal(1)
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


## Every distinct semantic role label actually baked into a source's limbs,
## read from CUSTOM0 so the test cannot drift from this instance's graph
## assignment. CUSTOM0 is the shader's G-channel source.
func _source_labels(source: Node) -> Array[float]:
	var labels: Array[float] = []
	for limb: MeshInstance3D in _limbs(source, [] as Array[MeshInstance3D]):
		var mesh: ArrayMesh = limb.mesh
		if mesh == null or mesh.get_surface_count() == 0:
			continue
		var custom: PackedFloat32Array = mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
		if custom.is_empty():
			continue
		var label: float = custom[0]
		if label >= 0.0 and not labels.has(label):
			labels.append(label)
	return labels
