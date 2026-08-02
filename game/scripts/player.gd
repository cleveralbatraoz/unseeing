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
var now := 0.0  # pushed by main every frame (the one clock)
var last_tap := -10.0  # drives the cane strike animation
var tap_target := Vector3.ZERO  # where the last tap landed (wall/floor/air)
## Cached cane rest, recomputed every physics tick at the sweep offset the
## viewmodel requested — hero_body reads this instead of raycasting itself.
var cane_rest: Dictionary = {tip = Vector3.ZERO, supported = false}
var cane_rest_offset := 0.0  # written by hero_body each frame

var _tap_queued := false
var _wave_queue: Array[Dictionary] = []


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
	camera.position = Vector3(0, EYE - 0.9, 0)
	camera.near = 0.05
	camera.far = 60.0
	camera.fov = 66.0  # ~1.15 rad vertical, the validated design FOV
	add_child(camera)


func _ready() -> void:
	ensure_actions()
	# on web the browser only grants capture on a user gesture; the click
	# handler below recaptures, so skip the doomed attempt and console noise
	if not OS.has_feature("web"):
		Input.mouse_mode = Input.MOUSE_MODE_CAPTURED


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion and Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
		rotate_y(-event.relative.x * MOUSE_SENS)
		camera.rotation.x = clampf(
			camera.rotation.x - event.relative.y * MOUSE_SENS, -PITCH_LIMIT, PITCH_LIMIT
		)
	elif event.is_action_pressed("ui_cancel"):
		Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
	elif event is InputEventMouseButton and event.pressed:
		if Input.mouse_mode != Input.MOUSE_MODE_CAPTURED:
			Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
		if event.button_index == MOUSE_BUTTON_LEFT:
			_tap_queued = true  # executed next physics tick, in physics context


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
	(
		_wave_queue
		. append(
			{
				type = type,
				at = at,
				max_r = max_r,
				speed = speed,
				gain = gain,
				echoes = max_echoes,
				normal = origin_normal,
			}
		)
	)


func _physics_process(_dt: float) -> void:
	var input := Input.get_vector("move_left", "move_right", "move_forward", "move_back")
	var dir3 := transform.basis * Vector3(input.x, 0, input.y)
	velocity.x = dir3.x * SPEED
	velocity.z = dir3.z * SPEED
	velocity.y = 0.0  # flat map: no gravity, no jumping — walking is the verb
	move_and_slide()

	cane_rest = _compute_cane_rest(cane_rest_offset)
	if _tap_queued:
		_tap_queued = false
		_cane_tap()
	var space := get_world_3d().direct_space_state
	for w: Dictionary in _wave_queue:
		pulses.emit_reflecting(
			w.type, w.at, w.max_r, w.speed, w.gain, now, space, w.echoes, w.normal
		)
	_wave_queue.clear()


func _cane_tap() -> void:
	if now - last_tap < TAP_COOLDOWN:
		return
	last_tap = now
	var pitch := camera.rotation.x
	var aim := -camera.global_transform.basis.z
	var flat := Vector3(aim.x, 0, aim.z).normalized()
	var from := camera.global_position
	var space := get_world_3d().direct_space_state
	var query := PhysicsRayQueryParameters3D.create(from, from + aim * CANE_REACH)
	var hit := space.intersect_ray(query)
	if hit:
		# aimed strike: the wave is born exactly where you looked
		tap_target = hit.position
		var floorish: bool = hit.normal.y > 0.7 and hit.position.y < 0.2
		var r := 5.0 if floorish else 6.0
		var g := 0.85 if floorish else 1.0
		pulses.emit_reflecting(0, tap_target, r, 5.5, g, now, space, 6, hit.normal)
		return
	var rest := _compute_cane_rest(0.0)
	var raised: bool = rest.supported and rest.tip.y > 0.15
	if raised or (rest.supported and pitch <= -0.12):
		# no aim needed: tap whatever the cane is physically resting on —
		# tabletop, chair seat, or (when looking down) the floor
		tap_target = rest.tip
		var r2 := 6.0 if raised else 5.0
		var g2 := 1.0 if raised else 0.85
		pulses.emit_reflecting(0, tap_target, r2, 5.5, g2, now, space, 6, Vector3.UP)
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
func _compute_cane_rest(yaw_offset: float) -> Dictionary:
	var fw := -global_transform.basis.z
	var dir := Vector3(fw.x, 0, fw.z).normalized().rotated(Vector3.UP, yaw_offset)
	var space := get_world_3d().direct_space_state
	var from := Vector3(global_position.x, CANE_SCAN_HEIGHT, global_position.z)
	var wall := space.intersect_ray(
		PhysicsRayQueryParameters3D.create(from, from + dir * CANE_SCAN_LENGTH)
	)
	var wall_d := CANE_SCAN_LENGTH
	if wall:
		wall_d = (wall.position - from).length()
	var reach := minf(CANE_REACH, wall_d - WALL_BACKOFF)
	var px := global_position.x + dir.x * reach
	var pz := global_position.z + dir.z * reach
	var down := space.intersect_ray(
		PhysicsRayQueryParameters3D.create(Vector3(px, 1.05, pz), Vector3(px, -0.1, pz))
	)
	if down:
		return {tip = Vector3(px, down.position.y + 0.02, pz), supported = true}
	return {tip = Vector3(px, 0.02, pz), supported = false}
