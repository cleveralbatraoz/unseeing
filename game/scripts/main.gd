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
var cane_mat := ShaderMaterial.new()   # standing reveal: the hero knows their grip
var body_mat := ShaderMaterial.new()   # legs/torso: revealed only by waves
var post_mat := ShaderMaterial.new()
var player: UnseeingPlayer
var hero: HeroBody
var fan: SoundFan

## The game clock: simulated seconds accumulated from frame deltas — NOT wall
## time, so offline rendering (movie maker) and time scaling stay correct.
var now := 0.0

# Nervous light: the reveal intensity wavers, with rare brief dropouts.
# Part of the mood, not noise; envelope carried over from the validated design.
var _flick := 1.0
var _drop_until := -1.0
var _next_drop := 9.0

# Dev-only demo tap (see _demo_tap below).
var _demo_next := 0.6
var _demo_checked := false
var _demo_wanted := false

func _ready() -> void:
	_setup_input()
	# deterministic flicker for offline frame-comparison runs
	if not OS.get_environment("UNSEEING_DEMO").is_empty():
		seed(0x5EED)
	data_mat.shader = DATA_SHADER
	post_mat.shader = POST_SHADER
	for m: ShaderMaterial in [cane_mat, body_mat]:
		m.shader = DATA_SHADER
	cane_mat.set_shader_parameter("u_base", 0.85)
	pulses = Pulses.new()
	MapBuilder.build_world(self, data_mat)
	# a constant sound source in the NEXT room, behind the wall the spawn
	# faces: its hum is felt through the wall before it is ever found
	fan = SoundFan.new()
	fan.pulses = pulses
	fan.data_mat = data_mat
	fan.position = Vector3(8.6, 0, 4.4)
	fan.rotation.y = PI * 0.5   # mounted facing the shared wall
	add_child(fan)
	# the fan's room (east area, wall centerlines): its waves reveal nothing
	# beyond these bounds — walls stop air, even though the shells are felt
	for m: ShaderMaterial in [data_mat, cane_mat, body_mat, post_mat]:
		m.set_shader_parameter("u_hum_room", Vector4(6.4, 0.6, 19.4, 8.0))
	player = UnseeingPlayer.new()
	player.pulses = pulses
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
	player.now = now
	_flick += (1.0 - _flick) * 0.12 + (randf() - 0.5) * 0.09
	_flick = clampf(_flick, 0.72, 1.2)
	_next_drop -= dt
	if _next_drop <= 0.0:
		_drop_until = now + 0.08 + randf() * 0.1
		_next_drop = 8.0 + randf() * 10.0
	if now < _drop_until:
		_flick *= 0.55
	for m: ShaderMaterial in [data_mat, cane_mat, body_mat, post_mat]:
		m.set_shader_parameter("u_time", now)
		m.set_shader_parameter("u_flick", _flick)
	post_mat.set_shader_parameter("u_breath", 1.0 + sin(now * 0.5) * 0.045)
	post_mat.set_shader_parameter("u_grain_t", fmod(now, 1.0) * 61.7)
	fan.update(now)
	pulses.apply(now, [data_mat, cane_mat, body_mat, post_mat])
	hero.update(now, dt)
	_demo_tap()

## Dev-only: fires a wall tap every few seconds so input-less runs can verify
## the renderer visually — movie-maker locally (UNSEEING_DEMO=1 env), or the
## deployed web build (?demo in the URL). Queued through the player so its
## reflection raycasts run in physics context.
func _demo_tap() -> void:
	if not _demo_checked and now >= 0.5:
		_demo_checked = true
		_demo_wanted = not OS.get_environment("UNSEEING_DEMO").is_empty()
		if OS.has_feature("web"):
			var search := str(JavaScriptBridge.eval("window.location.search", true))
			_demo_wanted = _demo_wanted or search.contains("demo")
	if not _demo_wanted or now < _demo_next:
		return
	_demo_next = now + 4.0   # repeat, so any screenshot timing catches a wave
	player.queue_wave(0, Vector3(6.4, 0.8, 4.0), 6.0, 5.5, 1.0, 6, Vector3(-1, 0, 0))

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

## Input actions are defined in code with PHYSICAL keycodes so WASD works on
## any keyboard layout (ЦФЫВ on Russian, ZQSD keys on AZERTY, etc.).
func _setup_input() -> void:
	var keys := {
		"move_forward": KEY_W,
		"move_left": KEY_A,
		"move_back": KEY_S,
		"move_right": KEY_D,
	}
	for action: String in keys:
		if not InputMap.has_action(action):
			InputMap.add_action(action)
			var ev := InputEventKey.new()
			ev.physical_keycode = keys[action]
			InputMap.action_add_event(action, ev)
