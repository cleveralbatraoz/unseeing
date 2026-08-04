extends GdUnitTestSuite
## The radio, held to what makes it a RADIO rather than a sound source in
## general — which is exactly two things, and both must be observable in the
## waves it emits, not merely in a knob:
##
##   1. it is the LOUDEST thing in the world, and by the volume law that
##      means it also reaches furthest and is felt most strongly through a
##      wall;
##   2. it does NOT aim. However the set is turned in the scene, its waves
##      carry the pool's omnidirectional sentinel — where the fan's carry
##      the direction its head happens to point at that instant.
##
## Everything else it shares with the fan through the source abstraction,
## and source_test owns those laws.


## Full volume, and the reach that follows from it — twelve meters, the
## longest carry in the world. The knob and its consequence are asserted
## together on purpose: a volume that stopped driving reach would pass a
## knob-only test and quietly halve the game.
func test_radio_is_the_loudest_source_and_reaches_furthest() -> void:
	var radio: SoundRadio = auto_free(SoundRadio.new())
	var fan: SoundFan = auto_free(SoundFan.new())
	assert_float(radio.volume).is_equal_approx(1.0, 0.0001)
	assert_float(radio.reach()).is_equal_approx(12.0, 0.0001)
	assert_bool(radio.volume > fan.volume).is_true()
	assert_bool(radio.reach() > fan.reach()).is_true()


## An even sphere still has to fit the 64-slot pool the hero's own taps,
## echoes and footsteps live in — the loudest, longest-reaching source in
## the world is also the most expensive, and its lazier cadence is where
## that is paid for.
func test_radio_stays_within_slot_headroom() -> void:
	var radio: SoundRadio = auto_free(SoundRadio.new())
	var fan: SoundFan = auto_free(SoundFan.new())
	assert_bool(radio.slot_pressure() <= 12.0).is_true()
	assert_bool(radio.cadence > fan.cadence).is_true()


## The defining property: turning the set changes nothing. Whatever yaw the
## designer gave it, every wave carries the pool's omni sentinel (dir.w =
## -2) and a zero beam vector, so no cone gate can ever cut it.
func test_radio_waves_are_even_however_it_is_turned() -> void:
	for yaw: float in [0.0, 1.3, -2.6]:
		var pulses := Pulses.new()
		var radio: SoundRadio = auto_free(SoundRadio.new())
		radio.pulses = pulses
		radio.data_mat = ShaderMaterial.new()
		radio.rotation.y = yaw
		add_child(radio)
		radio.update(radio.cadence)
		assert_int(pulses.live_count(radio.cadence)).is_equal(1)
		assert_float(pulses.dir[0].w).is_equal(-2.0)
		var beam := Vector3(pulses.dir[0].x, pulses.dir[0].y, pulses.dir[0].z)
		assert_vector(beam).is_equal(Vector3.ZERO)


## One wave per cadence, packed as a world SOURCE (kind 3) with this
## radio's own reach, speed and gain — and a stalled clock buys a single
## wave, never a backfilled burst.
func test_radio_emits_one_source_wave_per_cadence() -> void:
	var pulses := Pulses.new()
	var radio: SoundRadio = auto_free(SoundRadio.new())
	radio.pulses = pulses
	radio.data_mat = ShaderMaterial.new()
	radio.position = Vector3(4, 0, 5)
	add_child(radio)
	radio.update(0.3)  # inside the first cadence: silence
	assert_int(pulses.live_count(0.3)).is_equal(0)
	radio.update(0.7)
	assert_int(pulses.live_count(0.7)).is_equal(1)
	assert_float(pulses.dat[0].x).is_equal_approx(0.7, 0.0001)
	assert_float(pulses.dat[0].y).is_equal_approx(radio.reach(), 0.0001)
	assert_float(pulses.dat[0].z).is_equal(radio.wave_speed)
	assert_int(int(floorf(pulses.dat[0].w / 10.0))).is_equal(3)
	assert_float(fmod(pulses.dat[0].w, 10.0) / 9.0).is_equal_approx(radio.volume, 0.001)
	# born at the speaker cone, carried by the node's transform
	var hub: Vector3 = radio.position + SoundRadio.hub_offset()
	assert_vector(pulses.pos[0]).is_equal_approx(hub, Vector3(0.001, 0.001, 0.001))
	radio.update(9.0)  # a long jump...
	assert_float(pulses.dat[0].x).is_equal(9.0)
	assert_float(pulses.dat[1].x).is_equal(-1.0)  # ...buys exactly one wave


## A radio turned all the way down is silent, not a zero-radius wave the
## pool would refuse once per cadence forever.
func test_a_silent_radio_asks_the_pool_for_nothing() -> void:
	var pulses := Pulses.new()
	var radio: SoundRadio = auto_free(SoundRadio.new())
	radio.pulses = pulses
	radio.data_mat = ShaderMaterial.new()
	radio.volume = 0.0
	add_child(radio)
	radio.update(5.0)
	assert_int(pulses.live_count(5.0)).is_equal(0)


## The set has a body the cane and the echo rays can strike — it is a thing
## in the room, not a floating point of sound.
func test_radio_builds_a_body_with_a_collider() -> void:
	var radio: SoundRadio = auto_free(SoundRadio.new())
	radio.pulses = Pulses.new()
	radio.data_mat = ShaderMaterial.new()
	add_child(radio)
	var bodies := 0
	var colliders := 0
	for child: Node in radio.get_children():
		if child is StaticBody3D:
			bodies += 1
			for grand: Node in child.get_children():
				if grand is CollisionShape3D:
					colliders += 1
	assert_int(bodies).is_equal(1)
	assert_int(colliders).is_equal(1)


## No silent nulls: a radio without its injected pool and material reports
## the miss, builds nothing, and update() becomes a harmless no-op.
func test_uninjected_radio_reports_and_skips_update() -> void:
	var radio: SoundRadio = auto_free(SoundRadio.new())
	var enter := func() -> void: add_child(radio)
	await (assert_error(enter).is_push_error(
		"SoundRadio: pulses/data_mat not injected — radio disabled"
	))
	radio.update(99.0)
	assert_int(radio.get_child_count()).is_equal(0)
