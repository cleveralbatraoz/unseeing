extends GdUnitTestSuite
## The companion cat against real physics: builds only when injected,
## wanders its patch of floor on its own, and voices its fore paws into
## the wave pool as soft kind-2 steps — the walking-lantern contract.

const DT := 1.0 / 60.0

var _pulses: Pulses
var _cat: WaveCat


func _add_floor() -> void:
	var body: StaticBody3D = auto_free(StaticBody3D.new())
	body.position = Vector3(0, -0.05, 0)
	var col := CollisionShape3D.new()
	var shape := BoxShape3D.new()
	shape.size = Vector3(20, 0.1, 20)
	col.shape = shape
	body.add_child(col)
	add_child(body)


func _add_cat() -> void:
	_pulses = Pulses.new()
	_cat = auto_free(WaveCat.new())
	_cat.pulses = _pulses
	_cat.data_mat = ShaderMaterial.new()
	_cat.position = Vector3(0, 0, 0)
	add_child(_cat)


## No silent nulls: a cat without its injected pool and material reports
## the miss, builds nothing, and disables its own processing.
func test_uninjected_cat_reports_and_disables() -> void:
	var bare: WaveCat = auto_free(WaveCat.new())
	var enter := func() -> void: add_child(bare)
	await assert_error(enter).is_push_error("WaveCat: pulses/data_mat not injected — cat disabled")
	assert_bool(bare.is_physics_processing()).is_false()
	assert_int(bare.get_child_count()).is_equal(0)


## The cat lives its own life: within a few simulated seconds it leaves its
## spawn (the brain's first pause is 0.8 s, then it wanders), and it sounds
## two kind-2 voices into the pool as it goes — the fore paw's step (born at
## floor height, paw reach and loudness) and the idle presence heartbeat
## (born at chest height, its own reach and loudness). Both must be present
## among the live slots after a stretch of life.
func test_cat_wanders_and_paw_waves_sound() -> void:
	_add_floor()
	_add_cat()
	var start := _cat.position
	var now := 0.0
	for i: int in 300:
		now = float(i) * DT
		_cat.tick(now)
		await get_tree().physics_frame
	var travelled := (_cat.position - start).length()
	assert_float(travelled).is_greater(0.3)
	assert_int(_pulses.live_count(now)).is_greater(0)
	# scan every live slot for each voice — the pool packs whichever fired
	# most recently into slot 0, so we must not assume a fixed slot
	var found_paw := false
	var found_presence := false
	for i: int in _pulses.live_count(now):
		var d := _pulses.dat[i]
		if int(floorf(d.w / 10.0)) != 2:  # both voices are kind 2 (footstep)
			continue
		var gain := fmod(d.w, 10.0) / 9.0
		if is_equal_approx(d.y, WaveCat.paw_range()) and absf(gain - WaveCat.paw_gain()) < 0.01:
			assert_float(_pulses.pos[i].y).is_equal_approx(0.02, 0.001)  # floor
			assert_float(d.z).is_equal(WaveCat.paw_speed())
			found_paw = true
		elif (
			is_equal_approx(d.y, WaveCat.presence_range())
			and absf(gain - WaveCat.presence_gain()) < 0.01
		):
			assert_float(_pulses.pos[i].y).is_equal_approx(0.18, 0.001)  # chest
			found_presence = true
	assert_bool(found_paw).override_failure_message("no paw-voiced pulse in the pool").is_true()
	assert_bool(found_presence).override_failure_message("no presence heartbeat pulse").is_true()


## The silhouette exists: after a rendered frame the baked mesh carries
## one surface — the whole cat, tubes and spheres and whiskers.
func test_cat_mesh_builds() -> void:
	_add_floor()
	_add_cat()
	await get_tree().physics_frame
	await get_tree().process_frame
	assert_int(_cat.cat_mesh().get_surface_count()).is_equal(1)


## Every vertex of the silhouette carries the SAME Cat label (0.70,
## render::labels::role_label, rust/src/render/labels.rs) in its mesh's
## CUSTOM0 — the new per-vertex truth — and the mesh instance's own u_oid
## carries the identical value: the TEMPORARY BRIDGE the shader still reads
## until Task 8 flips it to CUSTOM0 directly must never disagree with the
## mesh underneath it.
func test_cat_mesh_carries_the_cat_label_in_custom0_and_bridges_it() -> void:
	_add_floor()
	_add_cat()
	await get_tree().physics_frame
	await get_tree().process_frame
	var mesh := _cat.cat_mesh()
	assert_int(mesh.get_surface_count()).is_equal(1)
	var custom: PackedFloat32Array = mesh.surface_get_arrays(0)[Mesh.ARRAY_CUSTOM0]
	assert_int(custom.size()).is_greater(0)
	for label: float in custom:
		assert_float(label).is_equal_approx(0.70, 0.0001)
	var skin := _mesh_instance_of(_cat)
	assert_object(skin).is_not_null()
	var oid: float = skin.get_instance_shader_parameter("u_oid")
	assert_float(oid).is_equal_approx(0.70, 0.0001)


## The first MeshInstance3D a node built for itself — the cat adds its own
## directly, with no `Skin` indirection the way a solid or a source has.
func _mesh_instance_of(node: Node) -> MeshInstance3D:
	for child: Node in node.get_children():
		if child is MeshInstance3D:
			return child as MeshInstance3D
	return null


## Four paws on the floor: the observable paw positions stay at ground
## level (planted or barely lifted) while the cat lives its life.
func test_paws_ride_the_floor() -> void:
	_add_floor()
	_add_cat()
	for i: int in 180:
		_cat.tick(float(i) * DT)
		await get_tree().physics_frame
		var paws := _cat.paw_positions()
		assert_int(paws.size()).is_equal(4)
		for paw: Vector3 in paws:
			assert_float(paw.y).is_between(-0.001, 0.05)


## The voice constants cross the boundary intact — the suite's handle on
## the Rust-side design envelope. Softer and shorter than the hero's own
## footsteps (1.6 m, gain 0.8), but a real, visible ripple.
func test_paw_voice_constants() -> void:
	assert_float(WaveCat.paw_range()).is_equal(1.3)
	assert_float(WaveCat.paw_speed()).is_equal(4.0)
	assert_float(WaveCat.paw_gain()).is_equal_approx(0.6, 0.0001)
