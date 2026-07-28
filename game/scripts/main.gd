extends Node3D
## Unseeing — composition root.
##
## The hero is BLIND. Nothing on screen is ever "the world": the player sees
## only sound — expanding wave rings in the air, and thin white outlines that
## flare where a wave strikes geometry, then fade within seconds. The design
## laws (validated in web-reference/, the playable spec):
##   - waves exist only when the player causes them (cane taps, footsteps);
##   - geometry appears only as edge outlines near a strike, never whole shapes;
##   - everything fades; nothing is ever in inventory-style "known" state.
##
## This script wires the systems together and owns per-frame globals
## (clock, flicker) that both shader passes consume. Systems never reach
## into each other — they meet here.

const DATA_SHADER := preload("res://shaders/data_pass.gdshader")
const POST_SHADER := preload("res://shaders/hearing_post.gdshader")
const MapBuilder := preload("res://scripts/map_builder.gd")
const Pulses := preload("res://scripts/pulses.gd")
const PlayerBody := preload("res://scripts/player.gd")

var pulses: Pulses
var data_mat := ShaderMaterial.new()
var post_mat := ShaderMaterial.new()
var player: CharacterBody3D

## The game clock: simulated seconds accumulated from frame deltas — NOT wall
## time, so offline rendering (movie maker) and time scaling stay correct.
var now := 0.0

# Nervous light: the reveal intensity wavers, with rare brief dropouts.
# Ported verbatim from the web reference — it is part of the mood, not noise.
var _flick := 1.0
var _drop_until := -1.0
var _next_drop := 9.0

func _ready() -> void:
	_setup_input()
	data_mat.shader = DATA_SHADER
	post_mat.shader = POST_SHADER
	pulses = Pulses.new()
	MapBuilder.build_world(self, data_mat)
	player = PlayerBody.new()
	player.pulses = pulses
	add_child(player)
	_setup_post_quad(player.camera)

func _process(dt: float) -> void:
	now += dt
	_flick += (1.0 - _flick) * 0.12 + (randf() - 0.5) * 0.09
	_flick = clampf(_flick, 0.72, 1.2)
	_next_drop -= dt
	if _next_drop <= 0.0:
		_drop_until = now + 0.08 + randf() * 0.1
		_next_drop = 8.0 + randf() * 10.0
	if now < _drop_until:
		_flick *= 0.55
	for m: ShaderMaterial in [data_mat, post_mat]:
		m.set_shader_parameter("u_time", now)
		m.set_shader_parameter("u_flick", _flick)
	post_mat.set_shader_parameter("u_breath", 1.0 + sin(now * 0.5) * 0.045)
	post_mat.set_shader_parameter("u_grain_t", fmod(now, 1.0) * 61.7)
	pulses.apply(now, [data_mat, post_mat])
	_demo_tap(now)

# Dev-only: fires one wall tap shortly after boot so input-less runs can
# verify the renderer visually — movie-maker locally (UNSEEING_DEMO=1 env),
# or the deployed web build (?demo in the URL).
var _demo_next := 0.6
var _demo_checked := false
var _demo_wanted := false
func _demo_tap(now: float) -> void:
	if not _demo_checked and now >= 0.5:
		_demo_checked = true
		_demo_wanted = not OS.get_environment("UNSEEING_DEMO").is_empty()
		if OS.has_feature("web"):
			var search := str(JavaScriptBridge.eval("window.location.search", true))
			_demo_wanted = _demo_wanted or search.contains("demo")
	if not _demo_wanted or now < _demo_next:
		return
	_demo_next = now + 4.0   # repeat, so any screenshot timing catches a wave
	pulses.emit(0, Vector3(6.4, 0.8, 4.0), 6.0, 5.5, 1.0, now)

## The "hearing" pass: a fullscreen quad glued to the camera. It edge-detects
## the data the world pass wrote (reveal / normals / depth) and ray-traces the
## wave shells — the only two ways anything becomes visible.
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
	if not InputMap.has_action("tap"):
		InputMap.add_action("tap")
		var mb := InputEventMouseButton.new()
		mb.button_index = MOUSE_BUTTON_LEFT
		InputMap.action_add_event("tap", mb)
