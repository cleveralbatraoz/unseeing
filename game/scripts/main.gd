class_name UnseeingMain
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
# the acoustic-image skin — EVERY world sound source wears it: always-on-top
# (felt through walls), each source's standing floor pushed per instance by
# the level, so a quiet fan and a loud radio dim independently
const XRAY_SHADER := preload("res://shaders/data_xray.gdshader")
const POST_SHADER := preload("res://shaders/hearing_post.gdshader")
const LEVEL_SCENE := preload("res://scenes/level_01.tscn")

## The perceptual ladder. Both data-writing skins stay in the OPAQUE pass
## (only opaque surfaces reach the screen texture the hearing pass reads),
## where the sort key is render_priority first — so once the source skin
## fakes its depth to always-pass, draw order lays it OVER the world: the
## world at real depth, the acoustic image of sources on top of it.
const PRIORITY_WORLD := 0
const PRIORITY_SOURCES := 20

var pulses: Pulses
var data_mat := ShaderMaterial.new()
var source_mat := ShaderMaterial.new()  # the acoustic image of sound sources
var cane_mat := ShaderMaterial.new()  # standing reveal: the hero knows their grip
var body_mat := ShaderMaterial.new()  # legs/torso: revealed only by waves
var post_mat := ShaderMaterial.new()
## Every material that renders waves — all five consume the same pool and the
## same per-frame globals.
var wave_mats: Array[ShaderMaterial] = [data_mat, source_mat, cane_mat, body_mat, post_mat]
var player: UnseeingPlayer
var hero: HeroBody
var cats: Array[WaveCat] = []
var level: WaveLevel
## The settings overlay — Escape freezes the world and frees the mouse.
var settings: SettingsMenu
## The agent's window into the engine — reads every system, drives none.
var observer: WaveObserver

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
	data_mat.render_priority = PRIORITY_WORLD
	post_mat.shader = POST_SHADER
	# the source image is LIVE: a source is always heard. Its standing reveal
	# floor keeps it a coherent whole with no wave nearby, and each wall
	# between the eye and its hub muffles that floor instead of silencing it
	# — pushed per INSTANCE by the level (u_source_floor), because all
	# sources share this one material and each carries its own volume
	source_mat.shader = XRAY_SHADER
	source_mat.render_priority = PRIORITY_SOURCES
	# the hero's cane and body render at real depth like the world — they
	# ride with the hero, never across a wall from their own sounds
	for m: ShaderMaterial in [cane_mat, body_mat]:
		m.shader = DATA_SHADER
	cane_mat.set_shader_parameter("u_base", 0.85)
	pulses = Pulses.new()
	# the world: an editor-authored scene — walls, furniture, the sound
	# sources, the cat, the spawn marker under a WaveLevel root. Injected
	# BEFORE entering the tree (children run _ready first, and a source
	# refuses to build uninjected); the root distributes the materials and
	# pool, then derives the technical contracts the systems below read back.
	level = LEVEL_SCENE.instantiate() as WaveLevel
	level.inject(data_mat, source_mat, pulses)
	add_child(level)
	_demo = DemoTap.new(level.demo_tap(), level.demo_tap_normal())
	# the hearing pass cuts player-sound shells by the walls too: the
	# always-on-top source skin corrupts the packed depth at its pixels, so a
	# ring must not lean on depth alone to stop at a wall behind a source.
	# Hand it the same wall table the data skins occlude by.
	var rects := level.wall_rects()
	post_mat.set_shader_parameter("u_walls", rects)
	post_mat.set_shader_parameter("u_wall_count", rects.size())
	post_mat.set_shader_parameter("u_wall_top", WaveLevel.wall_height())
	# the level's companion creatures — the cat wanders the floor beside the
	# hero, revealing itself with its own soft paw waves; we drive its clock
	cats.assign(level.cats())
	player = UnseeingPlayer.new()
	player.pulses = pulses
	player.position = level.spawn_pos()
	player.rotation.y = level.spawn_yaw()
	add_child(player)
	hero = HeroBody.new()
	hero.player = player
	hero.camera = player.camera
	hero.pulses = pulses
	hero.cane_mat = cane_mat
	hero.body_mat = body_mat
	add_child(hero)
	_setup_post_quad(player.camera)
	# the debug window: the level (which already holds the wave pool) and
	# the hero's own eye, because how many walls stand between the hero and
	# a source is measured from there. It reads and never drives, so a run
	# with nothing asking it questions is a run it does not exist in.
	observer = WaveObserver.new()
	observer.inject(level, player.camera)
	add_child(observer)
	# the settings overlay, added LAST on purpose: unhandled input walks
	# the tree bottom-up, so the overlay sees Escape before the world does
	# and can swallow every key it takes.
	settings = SettingsMenu.new()
	add_child(settings)


func _process(dt: float) -> void:
	now += dt
	player.tick(now)
	var flick := _flicker.next(dt)
	for m: ShaderMaterial in wave_mats:
		m.set_shader_parameter("u_time", now)
		m.set_shader_parameter("u_flick", flick)
	post_mat.set_shader_parameter("u_breath", 1.0 + sin(now * 0.5) * 0.045)
	post_mat.set_shader_parameter("u_grain_t", fmod(now, 1.0) * 61.7)
	# every world sound source, driven by the level itself: it advances each
	# one's clockwork on the simulated clock and dims each one's standing
	# image by the walls between the eye and THAT source's hub. A silent
	# level is legal — the loop simply finds nothing.
	level.tick_sources(now, player.camera.global_position)
	for cat: WaveCat in cats:  # a catless level is legal too
		cat.tick(now)
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
