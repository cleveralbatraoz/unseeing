extends GdUnitTestSuite
## The fan, held to what makes it a FAN rather than a sound source in
## general. The shared half — the volume ladder, the cadence gate, the
## acoustic image, the injection door — is source_test's; this suite pins
## the motion curves the head and its collider both ride, and the one
## acoustic property no other source has: the wash is AIMED, and aimed
## wherever the head points at the instant of birth.
##
## The acoustic voice lives in designer knobs (exported properties
## defaulting to the shipped constants), so the envelope pins read a fresh
## node's defaults, and the build constants cross as static methods —
## ClassDB registers only integer constants, so floats cannot cross as
## constants.


## The head's oscillation must actually sweep, and never exceed its range —
## the collider rides the same curve, so this is also a physics bound.
func test_fan_motion_envelope() -> void:
	var lo := 0.0
	var hi := 0.0
	for i: int in 200:
		var a := SoundFan.pivot_angle(float(i) * 0.1)
		lo = minf(lo, a)
		hi = maxf(hi, a)
	assert_bool(hi <= SoundFan.pivot_range() + 0.001).is_true()
	assert_bool(lo >= -SoundFan.pivot_range() - 0.001).is_true()
	assert_bool(hi > SoundFan.pivot_range() * 0.9).is_true()
	assert_bool(lo < -SoundFan.pivot_range() * 0.9).is_true()


func test_fan_blades_spin() -> void:
	assert_bool(SoundFan.spin_angle(1.0) != SoundFan.spin_angle(1.1)).is_true()


## The volume law, read back off a fresh fan: one knob at 0.75 reproduces
## the hum the map was validated against — 9 m of reach — instead of a
## separate range knob that could contradict it.
func test_fan_volume_derives_the_validated_reach() -> void:
	var fan: SoundFan = auto_free(SoundFan.new())
	assert_float(fan.volume).is_equal_approx(0.75, 0.0001)
	assert_float(fan.reach()).is_equal_approx(9.0, 0.0001)
	fan.volume = 0.5  # half as loud is half as far: the law, not a table
	assert_float(fan.reach()).is_equal_approx(6.0, 0.0001)


## A hum slot lives ring + tail; the constant wash must not flood the pool
## the hero's own taps and footsteps share. The envelope holds for the
## DEFAULT knobs a fresh fan ships with.
func test_fan_wash_stays_within_slot_headroom() -> void:
	var fan: SoundFan = auto_free(SoundFan.new())
	assert_bool(fan.slot_pressure() <= 12.0).is_true()


func test_fan_wash_is_directed_cone() -> void:
	var fan: SoundFan = auto_free(SoundFan.new())
	assert_bool(fan.beam_cos > 0.7).is_true()
	assert_bool(fan.beam_cos < 0.95).is_true()


## The whoosh rides the pivot: a cadence beat emits ONE directed hum aimed
## exactly where the mounted, pivoting head points at that very moment — and
## a stalled clock buys a single beat, never a backfilled burst of them.
func test_whoosh_beam_rides_pivot_and_keeps_cadence() -> void:
	const MOUNT_YAW := 0.9  # an arbitrary mounting, like main's level data
	var pulses := Pulses.new()
	var fan: SoundFan = auto_free(SoundFan.new())
	fan.pulses = pulses
	fan.data_mat = ShaderMaterial.new()
	fan.rotation.y = MOUNT_YAW
	add_child(fan)
	fan.update(0.4)  # the first beat: the cadence gate starts at 0.4
	assert_int(pulses.live_count(0.4)).is_equal(1)
	assert_float(pulses.dat[0].x).is_equal_approx(0.4, 0.0001)
	assert_float(pulses.dat[0].y).is_equal_approx(fan.reach(), 0.0001)
	assert_float(pulses.dat[0].z).is_equal(fan.wave_speed)
	assert_int(int(floorf(pulses.dat[0].w / 10.0))).is_equal(3)
	assert_float(fmod(pulses.dat[0].w, 10.0) / 9.0).is_equal_approx(fan.volume, 0.001)
	# the beam: cone width from the knob, direction from the mounting yaw
	# composed with the pivot's oscillation at this very moment
	assert_float(pulses.dir[0].w).is_equal_approx(fan.beam_cos, 0.0001)
	var total_yaw := MOUNT_YAW + SoundFan.pivot_angle(0.4)
	var beam := Vector3(-sin(total_yaw), 0.0, -cos(total_yaw))
	var got := Vector3(pulses.dir[0].x, pulses.dir[0].y, pulses.dir[0].z)
	assert_vector(got).is_equal_approx(beam, Vector3(0.001, 0.001, 0.001))
	# born at the spinner hub, 0.1 m down the beam from the pivot point
	var hub := Vector3(0, SoundFan.head_h(), 0) + beam * 0.1
	assert_vector(pulses.pos[0]).is_equal_approx(hub, Vector3(0.001, 0.001, 0.001))
	fan.update(0.41)  # inside the cadence: not a sound
	assert_int(pulses.live_count(0.41)).is_equal(1)
	assert_float(pulses.dat[0].x).is_equal_approx(0.4, 0.0001)
	fan.update(5.0)  # the first hum expired at 4.4, freeing its slot...
	assert_float(pulses.dat[0].x).is_equal(5.0)
	assert_float(pulses.dat[1].x).is_equal(-1.0)  # ...and NO burst backfilled


## The beam MOVES: two beats a quarter of the sweep apart are aimed in
## measurably different directions. Nothing else in the world does this —
## it is the whole difference between a fan and a radio.
func test_the_wash_sweeps_between_beats() -> void:
	var pulses := Pulses.new()
	var fan: SoundFan = auto_free(SoundFan.new())
	fan.pulses = pulses
	fan.data_mat = ShaderMaterial.new()
	add_child(fan)
	fan.update(0.4)
	var first := Vector3(pulses.dir[0].x, pulses.dir[0].y, pulses.dir[0].z)
	fan.update(3.0)
	var later := Vector3(pulses.dir[1].x, pulses.dir[1].y, pulses.dir[1].z)
	assert_bool(first.dot(later) < 0.97).is_true()


## No silent nulls: a fan without its injected pool and material reports the
## miss, builds nothing, and update() becomes a harmless no-op.
func test_uninjected_fan_reports_and_skips_update() -> void:
	var fan: SoundFan = auto_free(SoundFan.new())
	var enter := func() -> void: add_child(fan)
	await assert_error(enter).is_push_error("SoundFan: pulses/data_mat not injected — fan disabled")
	fan.update(99.0)  # would crash on the missing head if _ready had built
	assert_int(fan.get_child_count()).is_equal(0)
