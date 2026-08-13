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


## The standing acoustic image a source is currently carrying, read back
## off its limbs — the value the x-ray skin will use as its reveal floor.
## Fails the caller loudly if the limbs disagree, because a source that
## dimmed unevenly would tear along its own seams.
func _image_of(source: Node) -> float:
	var limbs := _limbs(source, [] as Array[MeshInstance3D])
	assert_bool(limbs.size() > 0).is_true()
	var first: float = limbs[0].get_instance_shader_parameter("u_source_floor")
	for limb: MeshInstance3D in limbs:
		var value: float = limb.get_instance_shader_parameter("u_source_floor")
		assert_float(value).is_equal_approx(first, 0.0001)
	return first


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


## Every source's limbs carry fixed role labels in CUSTOM0. Sources never
## enter the world's superface colouring or merge with it; their few coherent
## roles give the shader the stable seams within each source.
func test_sources_paint_fixed_role_labels_into_custom0() -> void:
	var level := _two_source_level(Vector3(3, 0, 3), Vector3(8, 0, 8))
	for source: Node3D in level.sources():
		var ids := {}
		for limb: MeshInstance3D in _limbs(source, [] as Array[MeshInstance3D]):
			var oid := _limb_label(limb)
			assert_bool(oid >= 0.0).is_true()
			ids[oid] = true
		# a source reads as a few coherent parts, never as a heap of limbs
		assert_bool(ids.size() >= 1 and ids.size() <= 3).is_true()


## Every limb a source builds carries its role label baked into its mesh's
## per-vertex CUSTOM0 — what the shader's G channel reads directly. Every
## vertex of a limb's mesh carries the SAME label, since a source's own
## body has no internal seams of its own — one label per limb, never split
## across it. Hand-derived against render::labels::role_label
## (rust/src/render/labels.rs): Shell 0.33, Moving 0.63, Case 0.05.
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
	_assert_limbs_carry_their_labels(fan, [0.33, 0.63] as Array[float])


func test_radio_limbs_carry_case_and_shell_labels() -> void:
	var radio: SoundRadio = auto_free(SoundRadio.new())
	radio.pulses = Pulses.new()
	radio.data_mat = ShaderMaterial.new()
	add_child(radio)
	_assert_limbs_carry_their_labels(radio, [0.05, 0.33] as Array[float])


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
