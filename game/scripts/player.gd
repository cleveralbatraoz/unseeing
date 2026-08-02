class_name UnseeingPlayer
extends CharacterBody3D
## The blind hero: first-person movement, mouse look, cane taps.
##
## The cane is the ONLY deliberate instrument. A tap picks its mode by what a
## real ~1.7 m arm-plus-cane could actually touch:
##   aimed strike — the 3D gaze ray connects within reach: the wave is born
##                  exactly where the player looked (wall, furniture, floor);
##   rest tap     — no aimed hit: the tap lands wherever the cane tip is
##                  physically resting (tabletop, chair seat, or — when the
##                  player is looking down — the floor);
##   air swish    — the cane rests on nothing raised and the player is not
##                  aiming down: NO wave. Air reflects nothing.
##
## PHYSICS CONTEXT: every raycast in the game runs inside _physics_process.
## Input handlers only queue intent; hero_body and main queue wave requests.
## This keeps all space queries inside Godot's supported physics window.

const EYE := 1.6  # eye height above the floor
const CAM_BASE_Y := EYE - 0.9  # camera rest height in capsule-local space
const SPEED := 2.1  # m/s — a careful walk, not a run
const CANE_REACH := 1.7  # arm + white cane: what can truly be touched
const TAP_COOLDOWN := 0.15
const MOUSE_SENS := 0.0026  # radians per pixel, both axes
const PITCH_LIMIT := 1.35  # radians up/down
const CANE_SCAN_HEIGHT := 0.85  # wall-detection ray height (below tabletops)
const CANE_SCAN_LENGTH := 3.4
const WALL_BACKOFF := 0.06

## Move actions bind PHYSICAL keycodes so WASD works on any keyboard layout
## (ЦФЫВ on Russian, ZQSD keys on AZERTY, etc.).
const MOVE_KEYS: Dictionary[String, Key] = {
	"move_forward": KEY_W,
	"move_left": KEY_A,
	"move_back": KEY_S,
	"move_right": KEY_D,
}

var pulses: Pulses  # injected by main.gd
var camera: Camera3D
var last_tap := -10.0  # drives the cane strike animation
var tap_target := Vector3.ZERO  # where the last tap landed (wall/floor/air)
## Cached cane rest, recomputed every physics tick at the sweep offset the
## viewmodel requested — hero_body reads this instead of raycasting itself.
var cane_rest := CaneRest.new()

var _cane_rest_offset := 0.0  # latest sweep request, honored next physics tick
var _now := 0.0  # the game clock, advanced only through tick()
var _tap_queued := false
var _wave_queue: Array[WaveRequest] = []


## Where the cane tip naturally rests, and whether any surface actually
## holds it up (false over open air at floor level).
class CaneRest:
	var tip := Vector3.ZERO
	var supported := false


## What a tap or footstep asks of the wave pool — carried whole from the
## input/frame context into the physics tick where raycasts may run.
class WaveRequest:
	var type: int
	var at: Vector3
	var max_r: float
	var speed: float
	var gain: float
	var echoes: int
	var normal: Vector3


## The player registers its own senses: idempotent, so a bare instance in a
## test scene polls input without main's help, and main's boot-time call plus
## every player _ready leave exactly one key event per action.
static func ensure_actions() -> void:
	for action: String in MOVE_KEYS:
		if InputMap.has_action(action):
			continue
		InputMap.add_action(action)
		var ev := InputEventKey.new()
		ev.physical_keycode = MOVE_KEYS[action]
		InputMap.action_add_event(action, ev)


## The player owns no spawn: main places it from LevelData. A bare instance
## sits at the origin, which is exactly what the physics tests want.
func _init() -> void:
	var col := CollisionShape3D.new()
	var capsule := CapsuleShape3D.new()
	capsule.radius = 0.35
	capsule.height = 1.7
	col.shape = capsule
	add_child(col)
	camera = Camera3D.new()
	camera.position = Vector3(0, CAM_BASE_Y, 0)
	camera.near = 0.05
	camera.far = 60.0
	camera.fov = 66.0  # ~1.15 rad vertical, the validated design FOV
	add_child(camera)


func _ready() -> void:
	ensure_actions()
	# no silent nulls: without its pulse pool the player cannot voice a single
	# tap or footstep — refuse to run instead of crashing frames later
	if pulses == null:
		push_error("UnseeingPlayer: pulses not injected — physics disabled")
		set_physics_process(false)
		return
	# on web the browser only grants capture on a user gesture; the click
	# handler below recaptures, so skip the doomed attempt and console noise
	if not OS.has_feature("web"):
		Input.mouse_mode = Input.MOUSE_MODE_CAPTURED


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion and Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
		var motion := event as InputEventMouseMotion
		rotate_y(-motion.relative.x * MOUSE_SENS)
		camera.rotation.x = clampf(
			camera.rotation.x - motion.relative.y * MOUSE_SENS, -PITCH_LIMIT, PITCH_LIMIT
		)
	elif event.is_action_pressed("ui_cancel"):
		Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
	elif event is InputEventMouseButton:
		var click := event as InputEventMouseButton
		if not click.pressed:
			return
		if Input.mouse_mode != Input.MOUSE_MODE_CAPTURED:
			Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
		if click.button_index == MOUSE_BUTTON_LEFT:
			_tap_queued = true  # executed next physics tick, in physics context


## The clock is handed, never poked: main advances the simulated time here
## every frame — the player never reads a wall clock of its own.
func tick(now_t: float) -> void:
	_now = now_t


## The viewmodel's sweep asks for the cane rest to be computed at this yaw
## offset. One frame of latency BY DESIGN: requested during the render frame,
## honored on the next physics tick, read back through cane_rest after that —
## raycasts stay in physics context, and the sweep is too slow to notice.
func request_cane_sweep(offset: float) -> void:
	_cane_rest_offset = offset


## The player owns its camera: the viewmodel reports the walk head-bob and
## the player alone moves the eye around its fixed base height. Called by
## hero_body mid-update, BEFORE the arm anchors read the camera transform,
## so the bob shapes the same frame's viewmodel — as it always has.
func set_head_bob(offset: float) -> void:
	camera.position.y = CAM_BASE_Y + offset


## Other systems (hero footsteps, main's demo tap) request waves here; they
## are emitted next physics tick so reflection raycasts run in-context.
func queue_wave(
	type: int,
	at: Vector3,
	max_r: float,
	speed: float,
	gain: float,
	max_echoes: int,
	origin_normal := Vector3.ZERO
) -> void:
	var req := WaveRequest.new()
	req.type = type
	req.at = at
	req.max_r = max_r
	req.speed = speed
	req.gain = gain
	req.echoes = max_echoes
	req.normal = origin_normal
	_wave_queue.append(req)


func _physics_process(_dt: float) -> void:
	var input := Input.get_vector("move_left", "move_right", "move_forward", "move_back")
	var dir3 := transform.basis * Vector3(input.x, 0, input.y)
	velocity.x = dir3.x * SPEED
	velocity.z = dir3.z * SPEED
	velocity.y = 0.0  # flat map: no gravity, no jumping — walking is the verb
	move_and_slide()

	cane_rest = _compute_cane_rest(_cane_rest_offset)
	if _tap_queued:
		_tap_queued = false
		_cane_tap()
	var space := get_world_3d().direct_space_state
	for w: WaveRequest in _wave_queue:
		pulses.emit_reflecting(
			w.type, w.at, w.max_r, w.speed, w.gain, _now, space, w.echoes, w.normal
		)
	_wave_queue.clear()


func _cane_tap() -> void:
	if _now - last_tap < TAP_COOLDOWN:
		return
	last_tap = _now
	var pitch := camera.rotation.x
	var aim := -camera.global_transform.basis.z
	var flat := Vector3(aim.x, 0, aim.z).normalized()
	var from := camera.global_position
	var space := get_world_3d().direct_space_state
	var query := PhysicsRayQueryParameters3D.create(from, from + aim * CANE_REACH)
	var hit := space.intersect_ray(query)
	if hit:
		# aimed strike: the wave is born exactly where you looked
		var hit_pos: Vector3 = hit.position
		var hit_normal: Vector3 = hit.normal
		tap_target = hit_pos
		var floorish := hit_normal.y > 0.7 and hit_pos.y < 0.2
		var r := 5.0 if floorish else 6.0
		var g := 0.85 if floorish else 1.0
		pulses.emit_reflecting(0, tap_target, r, 5.5, g, _now, space, 6, hit_normal)
		return
	var rest := _compute_cane_rest(0.0)
	var raised := rest.supported and rest.tip.y > 0.15
	if raised or (rest.supported and pitch <= -0.12):
		# no aim needed: tap whatever the cane is physically resting on —
		# tabletop, chair seat, or (when looking down) the floor
		tap_target = rest.tip
		var r2 := 6.0 if raised else 5.0
		var g2 := 1.0 if raised else 0.85
		pulses.emit_reflecting(0, tap_target, r2, 5.5, g2, _now, space, 6, Vector3.UP)
	else:
		# air swish: the cane sweeps up through nothing; air reflects nothing
		var hy := clampf(EYE + tan(pitch) * 1.5, 0.3, 1.7)
		var p := from + flat * 1.5
		tap_target = Vector3(p.x, hy, p.z)


## Where the cane tip naturally rests for a given sweep offset: reach forward
## (walls shorten the reach at cane height), then settle onto the first
## supporting surface below — floor, tabletop, chair seat. This is the cane
## "touching" the world; the tap and the visuals both use it.
## Physics-context only: called from _physics_process.
func _compute_cane_rest(yaw_offset: float) -> CaneRest:
	var fw := -global_transform.basis.z
	var dir := Vector3(fw.x, 0, fw.z).normalized().rotated(Vector3.UP, yaw_offset)
	var space := get_world_3d().direct_space_state
	var from := Vector3(global_position.x, CANE_SCAN_HEIGHT, global_position.z)
	var wall := space.intersect_ray(
		PhysicsRayQueryParameters3D.create(from, from + dir * CANE_SCAN_LENGTH)
	)
	var wall_d := CANE_SCAN_LENGTH
	if wall:
		var wall_pos: Vector3 = wall.position
		wall_d = (wall_pos - from).length()
	var reach := minf(CANE_REACH, wall_d - WALL_BACKOFF)
	var px := global_position.x + dir.x * reach
	var pz := global_position.z + dir.z * reach
	var down := space.intersect_ray(
		PhysicsRayQueryParameters3D.create(Vector3(px, 1.05, pz), Vector3(px, -0.1, pz))
	)
	var rest := CaneRest.new()
	if down:
		var down_pos: Vector3 = down.position
		rest.tip = Vector3(px, down_pos.y + 0.02, pz)
		rest.supported = true
	else:
		rest.tip = Vector3(px, 0.02, pz)
	return rest
