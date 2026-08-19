extends GdUnitTestSuite
## THE SOUND-SOURCE ABSTRACTION. The world used to have one source and the
## level named it; now it has two of different classes and names neither.
## This suite holds the abstraction itself — the laws that must be true of
## every source there will ever be, asserted against two classes at once so
## that a law quietly re-specialised to the fan cannot pass.
##
## What is being pinned:
##   - the level RECOGNISES a source by what it can do, not what class it
##     is, and finds every one of them wherever it sits in the scene;
##   - injection reaches all of them through one door;
##   - one tick drives all of them, each on its own cadence and volume;
##   - and each one's standing acoustic image is dimmed INDEPENDENTLY, by
##     the walls between the eye and THAT source — the property that a
##     single material-wide muffle could never have.


## One label from the single role table (render::labels::role_label), read
## rather than retyped — a suite carrying its own copy of a label agrees
## with whatever the table says, and the table used to be wrong.
func _role(name: String) -> float:
	var table: Dictionary = WaveCore.new().role_labels()
	assert_bool(table.has(name)).is_true()
	return table.get(name, NAN)


func _spawn_marker(at: Vector3) -> WaveSpawn:
	var marker := WaveSpawn.new()
	marker.position = at
	return marker


## A level holding one fan and one radio, injected the way main does it.
func _two_source_level(fan_at: Vector3, radio_at: Vector3) -> WaveLevel:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var fan := SoundFan.new()
	fan.position = fan_at
	level.add_child(fan)
	var radio := SoundRadio.new()
	radio.position = radio_at
	level.add_child(radio)
	level.add_child(_spawn_marker(Vector3(1, 0, 1)))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	return level


## Every mesh limb beneath a node — a source draws itself with several, and
## the standing image has to reach all of them.
func _limbs(node: Node, out: Array[MeshInstance3D]) -> Array[MeshInstance3D]:
	if node is MeshInstance3D:
		out.append(node as MeshInstance3D)
	for child: Node in node.get_children():
		_limbs(child, out)
	return out


## Assert one actual source limb mesh obeys Godot's clockwise-front-face
## convention. Sources mix indexed boxes with unindexed columns and tori,
## all behind cull_back, so both representations are handled here.
func _assert_limb_winds_clockwise(limb: MeshInstance3D) -> int:
	var mesh := limb.mesh as ArrayMesh
	assert_object(mesh).is_not_null()
	if mesh == null:
		return 0
	var arrays: Array = mesh.surface_get_arrays(0)
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var normals: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
	var indexed := arrays[Mesh.ARRAY_INDEX] is PackedInt32Array
	var indices: PackedInt32Array = (
		arrays[Mesh.ARRAY_INDEX] as PackedInt32Array if indexed else PackedInt32Array()
	)
	var corner_count := indices.size() if indexed else verts.size()
	var witnessed := 0
	for triangle in corner_count / 3:
		var at := triangle * 3
		var i0: int = indices[at] if indexed else at
		var i1: int = indices[at + 1] if indexed else at + 1
		var i2: int = indices[at + 2] if indexed else at + 2
		var cross: Vector3 = (verts[i1] - verts[i0]).cross(verts[i2] - verts[i0])
		if cross.length_squared() > 1e-12:
			var normal := (normals[i0] + normals[i1] + normals[i2]) / 3.0
			assert_float(cross.dot(normal)).is_less(0.0)
			witnessed += 1
	assert_int(witnessed).is_greater(0)
	return witnessed


## The STANDING acoustic image a source is currently carrying, read back off
## its limbs: what the x-ray skin draws for it while no wave is washing its
## body, which is `u_source_muffle * u_source_volume` — the shader forms
## `muffle * max(wave, volume)`, and with `wave` at zero that is this
## product. The two uniforms are read separately and multiplied HERE rather
## than by the level, because the level pushing a product is precisely the
## bug this split exists to remove.
##
## Fails the caller loudly if the limbs disagree on either half, because a
## source that dimmed unevenly would tear along its own seams.
func _image_of(source: Node) -> float:
	var limbs := _limbs(source, [] as Array[MeshInstance3D])
	assert_bool(limbs.size() > 0).is_true()
	var volume: float = limbs[0].get_instance_shader_parameter("u_source_volume")
	var muffle: float = limbs[0].get_instance_shader_parameter("u_source_muffle")
	for limb: MeshInstance3D in limbs:
		var limb_volume: float = limb.get_instance_shader_parameter("u_source_volume")
		var limb_muffle: float = limb.get_instance_shader_parameter("u_source_muffle")
		assert_float(limb_volume).is_equal_approx(volume, 0.0001)
		assert_float(limb_muffle).is_equal_approx(muffle, 0.0001)
	return volume * muffle


## The level recognises a source by what it CAN DO. Two different classes,
## one of them buried under a grouping node a designer added, and both are
## found — in scene order, which is the order every derivation leans on.
func test_the_level_finds_every_source_whatever_class_it_is() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var fan := SoundFan.new()
	level.add_child(fan)
	var folder := Node3D.new()  # the grouping folders designers really add
	folder.name = "Furniture"
	level.add_child(folder)
	var radio := SoundRadio.new()
	folder.add_child(radio)
	level.add_child(_spawn_marker(Vector3.ZERO))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var sources := level.sources()
	assert_int(sources.size()).is_equal(2)
	assert_object(sources[0]).is_same(fan)
	assert_object(sources[1]).is_same(radio)


## One injection door for all of them: the pool they sound into and the
## acoustic-image skin they render through, dealt by the level, never
## reached for by a source.
func test_injection_reaches_every_source_through_one_door() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var fan := SoundFan.new()
	level.add_child(fan)
	var radio := SoundRadio.new()
	level.add_child(radio)
	level.add_child(_spawn_marker(Vector3.ZERO))
	var world := ShaderMaterial.new()
	var image := ShaderMaterial.new()
	var pulses := Pulses.new()
	level.inject(world, image, pulses)
	add_child(level)
	# read by name, not by cast: the point is that ONE call dressed two
	# different classes. The abstraction is a Rust trait, so GDScript has no
	# common base type to declare here.
	for source: Node3D in level.sources():
		assert_object(source.get("pulses")).is_same(pulses)
		assert_object(source.get("data_mat")).is_same(image)  # the IMAGE skin


## One tick drives them all, and each keeps its own clock: at t = 0.7 the
## fan (cadence 0.4) has sounded and the radio (cadence 0.7) has just
## sounded, and the two waves in the pool carry DIFFERENT gains and ranges
## — the ladder, arriving in the pool rather than only in a knob.
func test_one_tick_drives_every_source_on_its_own_voice() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var fan := SoundFan.new()
	fan.position = Vector3(2, 0, 2)
	level.add_child(fan)
	var radio := SoundRadio.new()
	radio.position = Vector3(6, 0, 2)
	level.add_child(radio)
	level.add_child(_spawn_marker(Vector3.ZERO))
	var pulses := Pulses.new()
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), pulses)
	add_child(level)
	level.tick_sources(0.7, Vector3(0, 1.6, 0))
	assert_int(pulses.live_count(0.7)).is_equal(2)
	var gains: Array[float] = []
	var ranges: Array[float] = []
	for i: int in 2:
		assert_int(int(floorf(pulses.dat[i].w / 10.0))).is_equal(3)  # both are world sources
		gains.append(fmod(pulses.dat[i].w, 10.0) / 9.0)
		ranges.append(pulses.dat[i].y)
	assert_float(gains[0]).is_equal_approx(fan.volume, 0.001)
	assert_float(gains[1]).is_equal_approx(radio.volume, 0.001)
	assert_float(ranges[0]).is_equal_approx(fan.reach(), 0.001)
	assert_float(ranges[1]).is_equal_approx(radio.reach(), 0.001)
	assert_bool(gains[1] > gains[0]).is_true()
	assert_bool(ranges[1] > ranges[0]).is_true()


## THE reason the image floor is a per-INSTANCE uniform. Both sources share
## one acoustic-image material; from an eye with a wall in front of the fan
## and clear line of sight to the radio, the fan must read as a faint ghost
## while the radio stays whole. A material-wide muffle would give them the
## same number and this test could not be written at all.
func test_each_source_dims_by_its_own_walls_not_the_others() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var wall := WaveWall.new()  # a z-run wall at x = 4, spanning z 0.5..8.5
	wall.length = 8.0
	wall.position = Vector3(4, 0, 4.5)
	wall.rotation.y = PI * 0.5
	level.add_child(wall)
	var fan := SoundFan.new()
	fan.position = Vector3(7, 0, 4)  # behind the wall from the eye
	level.add_child(fan)
	var radio := SoundRadio.new()
	radio.position = Vector3(1, 0, 4)  # same side as the eye
	level.add_child(radio)
	level.add_child(_spawn_marker(Vector3.ZERO))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var eye := Vector3(0.5, 1.6, 4)
	level.tick_sources(1.0, eye)
	var sources := level.sources()
	var fan_image := _image_of(sources[0])
	var radio_image := _image_of(sources[1])
	# the radio is unobstructed: its whole volume is felt
	assert_float(radio_image).is_equal_approx(radio.volume, 0.0001)
	# the fan is one wall away: its own volume, dimmed by SOURCE_THROUGH
	assert_float(fan_image).is_equal_approx(fan.volume * 0.3, 0.0001)
	assert_bool(fan_image < radio_image).is_true()


## And the ladder survives the wall: put BOTH sources behind the same one
## wall and the louder source is still the louder ghost. Volume drives the
## standing image, so a wall dims without levelling.
func test_the_volume_ladder_survives_a_wall() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	var wall := WaveWall.new()
	wall.length = 8.0
	wall.position = Vector3(4, 0, 4.5)
	wall.rotation.y = PI * 0.5
	level.add_child(wall)
	var fan := SoundFan.new()
	fan.position = Vector3(7, 0, 3)
	level.add_child(fan)
	var radio := SoundRadio.new()
	radio.position = Vector3(7, 0, 5)
	level.add_child(radio)
	level.add_child(_spawn_marker(Vector3.ZERO))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	level.tick_sources(1.0, Vector3(0.5, 1.6, 4))
	var sources := level.sources()
	var fan_image := _image_of(sources[0])
	var radio_image := _image_of(sources[1])
	assert_bool(fan_image < radio_image).is_true()
	assert_float(fan_image / radio_image).is_equal_approx(fan.volume / radio.volume, 0.001)


## A source's image follows the EYE, frame to frame: walking around a wall
## brings a ghost up to full strength without anything else changing.
func test_the_image_follows_the_eye() -> void:
	var level := _two_source_level(Vector3(7, 0, 4), Vector3(1, 0, 4))
	var walled := level.source_muffle(Vector3(0.5, 1.6, 4), Vector3(7, 1.15, 4))
	assert_float(walled).is_equal_approx(1.0, 0.0001)  # no wall in this level at all
	level.tick_sources(1.0, Vector3(6.5, 1.6, 4))
	assert_float(_image_of(level.sources()[0])).is_equal_approx(1.0 * 0.75, 0.0001)


## A silent level is legal: no sources, no error, and the tick simply finds
## nothing to drive.
func test_a_sourceless_level_ticks_quietly() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3.ZERO))
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	assert_int(level.sources().size()).is_equal(0)
	level.tick_sources(1.0, Vector3.ZERO)
	assert_vector(level.demo_tap()).is_equal(Vector3.ZERO)


## A limb's own painted label, read off its mesh's CUSTOM0 — what the
## shader's G channel reads directly, with no per-instance uniform between
## them any more.
func _limb_label(limb: MeshInstance3D) -> float:
	var mesh: ArrayMesh = limb.mesh
	assert_int(mesh.get_surface_count()).is_equal(1)
	var custom: PackedFloat32Array = mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	assert_int(custom.size()).is_greater(0)
	return custom[0]


## Free placement includes two sources of the SAME class. These radio cases
## stand side by side at their exact 0.44 m width, so their front faces are
## coplanar and their side planes meet at one X coordinate. Distance and
## normal therefore cannot invent the boundary: the actual CUSTOM0 labels on
## the two case meshes must clear the hearing shader's 0.08 upper crease knee.
func test_two_touching_radios_keep_their_coplanar_seam() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3(1, 0, 1)))
	var gate := WaveWall.new()
	gate.name = "Gate"
	gate.length = 2.0
	gate.position = Vector3(2, 0, 2)
	level.add_child(gate)
	var radio_a := SoundRadio.new()
	radio_a.name = "RadioA"
	radio_a.position = Vector3(3, 0, 3)
	level.add_child(radio_a)
	var radio_b := SoundRadio.new()
	radio_b.name = "RadioB"
	radio_b.position = Vector3(3.44, 0, 3)
	level.add_child(radio_b)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)

	var case_a: MeshInstance3D = _limbs(radio_a.find_child("RadioCase", true, false), [])[0]
	var case_b: MeshInstance3D = _limbs(radio_b.find_child("RadioCase", true, false), [])[0]
	var box_a: AABB = case_a.global_transform * case_a.get_aabb()
	var box_b: AABB = case_b.global_transform * case_b.get_aabb()
	assert_float(box_a.end.x).is_equal_approx(box_b.position.x, 0.0001)
	assert_float(box_a.position.z).is_equal_approx(box_b.position.z, 0.0001)
	assert_float(box_a.end.z).is_equal_approx(box_b.end.z, 0.0001)
	var label_a := _limb_label(case_a)
	var label_b := _limb_label(case_b)
	for limb: MeshInstance3D in [case_a, case_b]:
		var custom: PackedFloat32Array = limb.mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
		var chosen: float = label_a if limb == case_a else label_b
		assert_int(custom.size()).is_greater(0)
		for vertex_label: float in custom:
			assert_float(vertex_label).is_equal(chosen)
	assert_float(absf(label_a - label_b)).is_greater_equal(0.08)


## Three coincident two-role sources form K6, one class beyond the five-label
## palette. The allocator remains total, but this is authoring-invalid because
## one seam cannot draw: the level shouts once and the affected source owns the
## repairable warning triangle. The other two sources must not inherit it.
func test_a_starved_source_role_warns_its_source_and_the_level() -> void:
	var level: WaveLevel = auto_free(WaveLevel.new())
	level.add_child(_spawn_marker(Vector3(1, 0, 1)))
	var gate := WaveWall.new()
	gate.name = "Gate"
	gate.length = 2.0
	gate.position = Vector3(2, 0, 2)
	level.add_child(gate)
	var radios: Array[SoundRadio] = []
	for index in 3:
		var radio := SoundRadio.new()
		radio.name = "Radio%d" % (index + 1)
		radio.position = Vector3(3, 0, 3)
		radios.append(radio)
		level.add_child(radio)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	var level_warning := (
		"WaveLevel: 1 face/source-role class(es) could not take a label distinct from "
		+ "everything they touch — those seams will not draw. Spread the geometry or widen "
		+ "WORLD_OIDS."
	)
	var enter := func() -> void: add_child(level)
	await assert_error(enter).is_push_error(level_warning)
	assert_array(level.get_configuration_warnings()).contains([level_warning])
	var source_warning := (
		"one or more source roles cannot take a label distinct from everything they touch — "
		+ "those seams will not draw."
	)
	var warning_owners := 0
	var warning_owner: SoundRadio = null
	for radio: SoundRadio in radios:
		var warnings := radio.get_configuration_warnings()
		if warnings.has(source_warning):
			warning_owners += 1
			warning_owner = radio
	assert_int(warning_owners).is_equal(1)
	if warning_owner == null:
		fail("the starved source-role class had no repairable source owner")
		return
	warning_owner.position = Vector3(10, 0, 3)
	var replay := func() -> void: level.rederive()
	await assert_error(replay).is_success()
	assert_array(level.get_configuration_warnings()).not_contains([level_warning])
	for radio: SoundRadio in radios:
		assert_array(radio.get_configuration_warnings()).not_contains([source_warning])


## Sources do not become world superfaces, but their few semantic limb roles
## join the same separation graph and are baked back into CUSTOM0. Each source
## therefore reads as a few coherent parts rather than a heap of limb IDs.
func test_sources_paint_graph_coloured_roles_into_custom0() -> void:
	var level := _two_source_level(Vector3(3, 0, 3), Vector3(8, 0, 8))
	for source: Node3D in level.sources():
		var ids := {}
		for limb: MeshInstance3D in _limbs(source, [] as Array[MeshInstance3D]):
			var oid := _limb_label(limb)
			assert_bool(oid >= 0.0).is_true()
			ids[oid] = true
		# fan and radio each define exactly two roles at this boundary
		assert_int(ids.size()).is_equal(2)
		var values: Array[float] = []
		for value: float in ids.keys():
			values.append(value)
		assert_float(absf(values[0] - values[1])).is_greater_equal(0.08)


## A source can re-enter the tree with `request_ready()` without any authored
## path/transform change. Its ownerless limbs are rebuilt, but the last derived
## semantic role labels must survive that generation change immediately; a
## scene-signature poll is allowed to see the same scene and do nothing.
func test_a_rebuilt_source_keeps_its_derived_role_labels() -> void:
	var level := _two_source_level(Vector3(3, 0, 3), Vector3(3.44, 0, 3))
	var radio: SoundRadio = level.sources()[1]
	var first: Array[float] = []
	for limb: MeshInstance3D in _limbs(radio, [] as Array[MeshInstance3D]):
		first.append(_limb_label(limb))
	level.remove_child(radio)
	radio.request_ready()
	level.add_child(radio)
	var second: Array[float] = []
	for limb: MeshInstance3D in _limbs(radio, [] as Array[MeshInstance3D]):
		second.append(_limb_label(limb))
	assert_int(second.size()).is_equal(first.size())
	for index in first.size():
		assert_float(second[index]).is_equal_approx(first[index], 0.000001)


## A standalone source blueprint starts with its semantic-role defaults baked
## into per-vertex CUSTOM0 — what the shader's G channel reads directly.
## Every vertex of a limb carries the SAME label: one label per role limb,
## never split across it. The expected values are READ from
## render::labels::role_label rather than retyped, so the suite cannot
## quietly agree with a table that has drifted out of its own separation
## law.
##
## Matched APPROXIMATELY, not by exact membership: CUSTOM0 is packed as
## `ARRAY_CUSTOM_R_FLOAT`, a 32-bit float, so the f64 role label round-trips
## through an f32 lane and lands a hair off the f64 literal typed here
## (0.33 reads back as 0.33000001311302) — a real, expected precision loss
## from the format the shader actually reads, not a bug to chase.
func _assert_limbs_carry_their_labels(source: Node3D, expected: Array[float]) -> void:
	var seen: Array[bool] = []
	for _e: float in expected:
		seen.append(false)
	for limb: MeshInstance3D in _limbs(source, [] as Array[MeshInstance3D]):
		var mesh: ArrayMesh = limb.mesh
		assert_int(mesh.get_surface_count()).is_equal(1)
		var custom: PackedFloat32Array = mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
		assert_int(custom.size()).is_greater(0)
		var label: float = custom[0]
		var matched := -1
		for i: int in expected.size():
			if absf(label - expected[i]) < 0.0001:
				matched = i
				break
		(
			assert_bool(matched >= 0)
			. override_failure_message(
				"limb carries an unexpected label %.3f, not one of %s" % [label, expected]
			)
			. is_true()
		)
		if matched >= 0:
			seen[matched] = true
		for v: float in custom:
			(
				assert_float(v)
				. append_failure_message(
					"limb's first vertex carries %.3f but another vertex carries %.3f" % [label, v]
				)
				. is_equal_approx(label, 0.0001)
			)
	for i: int in expected.size():
		(
			assert_bool(seen[i])
			. override_failure_message("expected label %.3f never seen on any limb" % expected[i])
			. is_true()
		)


func test_fan_limbs_carry_shell_and_moving_labels() -> void:
	var fan: SoundFan = auto_free(SoundFan.new())
	fan.pulses = Pulses.new()
	fan.data_mat = ShaderMaterial.new()
	add_child(fan)
	var roles: Dictionary = WaveCore.new().role_labels()
	_assert_limbs_carry_their_labels(fan, [roles["Shell"], roles["Moving"]] as Array[float])


func test_radio_limbs_carry_case_and_shell_labels() -> void:
	var radio: SoundRadio = auto_free(SoundRadio.new())
	radio.pulses = Pulses.new()
	radio.data_mat = ShaderMaterial.new()
	add_child(radio)
	_assert_limbs_carry_their_labels(radio, [_role("Case"), _role("Shell")] as Array[float])


## Fan and radio together exercise every source mesh family: indexed boxes,
## conventional columns and conventional tori. Check every limb from the
## real registered nodes so changing any one call back to the direct
## animated-limb adapter is mutation-live.
func test_source_meshes_wind_clockwise_for_godot() -> void:
	var fan: SoundFan = auto_free(SoundFan.new())
	fan.pulses = Pulses.new()
	fan.data_mat = ShaderMaterial.new()
	add_child(fan)
	var radio: SoundRadio = auto_free(SoundRadio.new())
	radio.pulses = Pulses.new()
	radio.data_mat = ShaderMaterial.new()
	add_child(radio)
	var limb_count := 0
	var triangle_count := 0
	for source: Node3D in [fan, radio]:
		for limb: MeshInstance3D in _limbs(source, [] as Array[MeshInstance3D]):
			limb_count += 1
			triangle_count += _assert_limb_winds_clockwise(limb)
	assert_int(limb_count).is_equal(14)
	assert_int(triangle_count).is_greater(0)


## Nothing about a running source is frozen at build time. Volume, speed and
## cone width are re-read on every beat, and so is the CADENCE — a knob that
## stopped taking effect once the level was built would be exactly the hidden
## state the abstraction exists to remove, and would make slot_pressure()
## describe a source that is not the one running.
func test_the_cadence_knob_stays_live_after_the_level_is_built() -> void:
	var pulses := Pulses.new()
	var radio: SoundRadio = auto_free(SoundRadio.new())
	radio.pulses = pulses
	radio.data_mat = ShaderMaterial.new()
	add_child(radio)
	radio.update(0.7)  # the shipped cadence fires, booking 1.4
	assert_int(pulses.live_count(0.7)).is_equal(1)
	radio.cadence = 3.0  # a designer quiets it down mid-flight
	assert_float(radio.slot_pressure()).is_equal_approx((12.0 / 5.0 + 2.0) / 3.0, 0.001)
	radio.update(1.4)  # the appointment already booked STANDS — no jump
	assert_int(pulses.live_count(1.4)).is_equal(2)
	# ...and then the new rhythm governs: the old 0.7 gate would have fired
	# at 2.1, 2.8, 3.5 and 4.2, and none of them do
	for t: float in [2.1, 2.8, 3.5, 4.2, 4.3]:
		radio.update(t)
		assert_int(pulses.live_count(t)).is_equal(2)
	radio.update(4.4)  # 1.4 + the new three seconds
	assert_int(pulses.live_count(4.4)).is_equal(3)
