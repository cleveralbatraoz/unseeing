extends GdUnitTestSuite
## Footsteps keep honest time. The viewmodel's _footsteps is driven with
## fixed 1/60 s steps over a scripted walk and held to its clock: each
## footfall queues exactly one type-2 wave on the player (where the physics
## tick would emit it), the cadence never drifts, the wave carries the
## footstep voice verbatim, and the shoes take turns. The player's physics
## is switched off so the suite alone owns time — nothing drains the queue
## or overwrites the scripted velocity between steps.

const DT := 1.0 / 60.0
const WALK_VEL := Vector3(0, 0, -UnseeingPlayer.SPEED)

var _player: UnseeingPlayer
var _hero: HeroBody
var _now := 0.0


func before_test() -> void:
	_player = auto_free(UnseeingPlayer.new())
	_player.pulses = Pulses.new()
	_player.position = Vector3(0, 0.9, 0)
	_player.rotation.y = 0.0  # face -Z: the shoes sit at world x = ±0.07
	add_child(_player)
	_player.set_physics_process(false)  # the test owns the clock and queue
	_hero = auto_free(HeroBody.new())
	_hero.player = _player
	_hero.camera = _player.camera
	_hero.pulses = _player.pulses
	_hero.cane_mat = ShaderMaterial.new()
	_hero.body_mat = ShaderMaterial.new()
	add_child(_hero)
	_now = 0.0


## One scripted frame: the player reports the given velocity and the
## viewmodel updates once at the fixed step.
func _step(vel: Vector3) -> void:
	_player.velocity = vel
	_now += DT
	_hero.update(_now, DT)


## 1.5 s of walking: the first-ever step falls on the first moving frame
## (_step_t starts spent), then every following step lands exactly when the
## 0.42 s cadence has fully elapsed — 26 fixed frames, never drifting.
func test_walk_cadence_never_drifts() -> void:
	var fires: Array[int] = []
	for frame: int in 90:
		_step(WALK_VEL)
		if _player._wave_queue.size() > fires.size():
			fires.append(frame)
	assert_int(fires.size()).is_equal(4)
	assert_int(fires[0]).is_equal(0)  # a fresh walker steps at once
	for k: int in fires.size() - 1:
		assert_int(fires[k + 1] - fires[k]).is_equal(26)  # first frame past 0.42 s


## Every footfall speaks with the footstep voice, verbatim from the source:
## type 2, radius 1.6, speed 4.0, gain 0.8, two echoes, born flat on the
## ground (y 0.04) under the striking shoe — and the shoes alternate,
## starting with the right one (_step_side begins at +1).
func test_step_waves_carry_the_footstep_voice_and_alternate() -> void:
	for frame: int in 90:
		_step(WALK_VEL)
	var queue := _player._wave_queue
	assert_int(queue.size()).is_equal(4)
	var side := 1.0  # right shoe first: hip + right-vector * 0.07
	for w: UnseeingPlayer.WaveRequest in queue:
		assert_int(w.type).is_equal(2)
		assert_float(w.max_r).is_equal(1.6)
		assert_float(w.speed).is_equal(4.0)
		assert_float(w.gain).is_equal(0.8)
		assert_int(w.echoes).is_equal(2)
		assert_vector(w.normal).is_equal(Vector3.UP)
		assert_float(w.at.y).is_equal_approx(0.04, 0.0001)
		assert_float(w.at.x).is_equal_approx(side * 0.07, 0.001)
		side = -side


## Standing still re-arms the 0.1 s stop grace every idle frame: the pause
## itself is silent, and the resumed walk fires exactly ONE step only after
## the grace has elapsed — never an instant double-fire on the first frame.
func test_stop_grace_prevents_a_double_fire_on_resume() -> void:
	for frame: int in 30:
		_step(WALK_VEL)
	var walked := _player._wave_queue.size()
	assert_int(walked).is_equal(2)  # frames 0 and 26
	for frame: int in 30:
		_step(Vector3.ZERO)
	assert_int(_player._wave_queue.size()).is_equal(walked)  # silence while still
	var resume_fires: Array[int] = []
	for frame: int in 12:
		_step(WALK_VEL)
		if _player._wave_queue.size() > walked + resume_fires.size():
			resume_fires.append(frame)
	assert_int(resume_fires.size()).is_equal(1)
	assert_int(resume_fires[0]).is_between(5, 7)  # ~0.1 s of walking first
