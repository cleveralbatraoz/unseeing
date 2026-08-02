extends Node3D
## Unseeing — composition root.
##
## The hero is BLIND. Nothing on screen is ever "the world": the player sees
## only sound — expanding wave rings in the air, and thin white outlines that
## flare where a wave strikes geometry, then fade within seconds. The design
## laws (validated in the original web prototype, frozen since):
##   - waves exist only when the player causes them (cane taps, footsteps);
##   - geometry appears only as edge outlines near a strike, never whole shapes;
##   - everything fades; nothing is ever in inventory-style "known" state.
##
## This script wires the systems together and owns per-frame globals
## (clock, flicker) that both shader passes consume. Systems never reach
## into each other — they meet here.

const DATA_SHADER := preload("res://shaders/data_pass.gdshader")
const POST_SHADER := preload("res://shaders/hearing_post.gdshader")

var pulses: Pulses
var data_mat := ShaderMaterial.new()
var cane_mat := ShaderMaterial.new()  # standing reveal: the hero knows their grip
var body_mat := ShaderMaterial.new()  # legs/torso: revealed only by waves
var post_mat := ShaderMaterial.new()
## Every material that renders waves — all four consume the same pool and the
## same per-frame globals.
var wave_mats: Array[ShaderMaterial] = [data_mat, cane_mat, body_mat, post_mat]
var player: UnseeingPlayer
var hero: HeroBody
var fan: SoundFan
var level: LevelData

## The game clock: simulated seconds accumulated from frame deltas — NOT wall
## time, so offline rendering (movie maker) and time scaling stay correct.
var now := 0.0

# Nervous light: the reveal intensity wavers, with rare brief dropouts.
# Part of the mood, not noise; owned by Flicker with its own seeded stream.
var _flicker: Flicker

# Dev-only demo tap (see _demo_tap below).
var _demo: DemoTap
var _demo_checked := false


func _ready() -> void:
	UnseeingPlayer.ensure_actions()
	# deterministic flicker for offline frame-comparison runs
	var rng := RandomNumberGenerator.new()
	if not OS.get_environment("UNSEEING_DEMO").is_empty():
		rng.seed = 0x5EED
	_flicker = Flicker.new(rng)
	data_mat.shader = DATA_SHADER
	post_mat.shader = POST_SHADER
	for m: ShaderMaterial in [cane_mat, body_mat]:
		m.shader = DATA_SHADER
	cane_mat.set_shader_parameter("u_base", 0.85)
	pulses = Pulses.new()
	level = MapBuilder.build_world(self, data_mat)
	_demo = DemoTap.new(level.demo_tap, level.demo_tap_normal)
	# a constant sound source in the NEXT room, behind the wall the spawn
	# faces: its hum is felt through the wall before it is ever found
	fan = SoundFan.new()
	fan.pulses = pulses
	fan.data_mat = data_mat
	fan.position = level.fan_spawn
	fan.rotation.y = level.fan_yaw
	add_child(fan)
	# the fan's room (east area, wall centerlines): its waves reveal nothing
	# beyond these bounds — walls stop air, even though the shells are felt
	for m: ShaderMaterial in wave_mats:
		m.set_shader_parameter("u_hum_room", level.hum_room)
	player = UnseeingPlayer.new()
	player.pulses = pulses
	player.position = level.spawn_pos
	player.rotation.y = level.spawn_yaw
	add_child(player)
	hero = HeroBody.new()
	hero.player = player
	hero.camera = player.camera
	hero.pulses = pulses
	hero.cane_mat = cane_mat
	hero.body_mat = body_mat
	add_child(hero)
	_setup_post_quad(player.camera)


func _process(dt: float) -> void:
	now += dt
	player.tick(now)
	var flick := _flicker.next(dt)
	for m: ShaderMaterial in wave_mats:
		m.set_shader_parameter("u_time", now)
		m.set_shader_parameter("u_flick", flick)
	post_mat.set_shader_parameter("u_breath", 1.0 + sin(now * 0.5) * 0.045)
	post_mat.set_shader_parameter("u_grain_t", fmod(now, 1.0) * 61.7)
	fan.update(now)
	pulses.apply(now, wave_mats)
	hero.update(now, dt)
	_demo_tap()


## Dev-only: fires a wall tap every few seconds so input-less runs can verify
## the renderer visually — movie-maker locally (UNSEEING_DEMO=1 env), or the
## deployed web build (?demo in the URL). Queued through the player so its
## reflection raycasts run in physics context.
func _demo_tap() -> void:
	if not _demo_checked and now >= 0.5:
		_demo_checked = true
		_demo.armed = not OS.get_environment("UNSEEING_DEMO").is_empty()
		if OS.has_feature("web"):
			var search := str(JavaScriptBridge.eval("window.location.search", true))
			_demo.armed = _demo.armed or search.contains("demo")
	if _demo.fire_due(now):
		player.queue_wave(0, _demo.point, 6.0, 5.5, 1.0, 6, _demo.normal)


## The "hearing" pass: a fullscreen quad glued to the camera. It edge-detects
## the data the world pass wrote (reveal / normals / distance) and ray-traces
## the wave shells — the only two ways anything becomes visible.
func _setup_post_quad(cam: Camera3D) -> void:
	var quad := MeshInstance3D.new()
	var mesh := QuadMesh.new()
	mesh.size = Vector2(2, 2)
	quad.mesh = mesh
	quad.material_override = post_mat
	# The vertex shader pins the quad to the full screen; a huge cull margin
	# stops Godot from frustum-culling the tiny quad mesh it thinks this is.
	quad.extra_cull_margin = 16384.0
	quad.position = Vector3(0, 0, -1)
	cam.add_child(quad)
