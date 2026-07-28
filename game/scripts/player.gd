extends CharacterBody3D
## The blind hero: first-person movement, mouse look, cane taps, footsteps.
##
## The cane is the ONLY deliberate instrument. A tap picks one of three modes
## by what a real ~1.7 m arm-plus-cane could actually touch:
##   wall strike — raycast hit within reach: the wave is born ON the wall at
##                 the height the player is aiming (pitch-projected);
##   floor tap   — no wall and genuinely aiming down: wave born on the floor
##                 where the gaze lands, clamped to cane reach;
##   air swish   — nothing in reach: NO wave at all. Air reflects nothing,
##                 so nothing may appear. (Sound-only once audio lands.)
## Footsteps ripple as small waves from alternating feet while walking.

const EYE := 1.6           # eye height above the floor
const SPEED := 2.1         # m/s — a careful walk, not a run
const CANE_REACH := 1.7    # arm + white cane: what can truly be touched
const TAP_COOLDOWN := 0.15

var pulses            # injected by main.gd
var camera: Camera3D
var last_tap := -10.0          # drives the cane strike animation
var tap_target := Vector3.ZERO # where the last tap landed (wall/floor/air)

func _init() -> void:
	position = Vector3(3, 0.9, 4)
	rotation.y = -1.9   # same spawn facing as the web reference
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
	camera.fov = 66.0   # matches the web reference's 1.15 rad vertical FOV
	add_child(camera)

func _ready() -> void:
	Input.mouse_mode = Input.MOUSE_MODE_CAPTURED

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion and Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
		rotate_y(-event.relative.x * 0.0026)
		camera.rotation.x = clampf(camera.rotation.x - event.relative.y * 0.0026, -1.35, 1.35)
	elif event is InputEventKey and event.pressed and event.physical_keycode == KEY_ESCAPE:
		Input.mouse_mode = Input.MOUSE_MODE_VISIBLE
	elif event is InputEventMouseButton and event.pressed:
		if Input.mouse_mode != Input.MOUSE_MODE_CAPTURED:
			Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
		if event.button_index == MOUSE_BUTTON_LEFT:
			_cane_tap()

func _physics_process(dt: float) -> void:
	var input := Input.get_vector("move_left", "move_right", "move_forward", "move_back")
	var dir3 := (transform.basis * Vector3(input.x, 0, input.y))
	velocity.x = dir3.x * SPEED
	velocity.z = dir3.z * SPEED
	velocity.y = 0.0   # flat map: no gravity, no jumping — walking is the verb
	move_and_slide()
	# footsteps live in hero_body.gd: they ripple from the animated shoes

func _cane_tap() -> void:
	var now: float = get_parent().now   # the one simulated game clock lives in main
	if now - last_tap < TAP_COOLDOWN:
		return
	last_tap = now
	var pitch := camera.rotation.x
	var aim := -camera.global_transform.basis.z
	var flat := Vector3(aim.x, 0, aim.z).normalized()
	var from := camera.global_position
	# a true 3D gaze ray: strikes whatever the cane can actually reach —
	# walls, furniture, or nearby floor — at the exact aimed point
	var query := PhysicsRayQueryParameters3D.create(from, from + aim * CANE_REACH)
	var hit := get_world_3d().direct_space_state.intersect_ray(query)
	if hit:
		tap_target = hit.position
		var floorish: bool = hit.normal.y > 0.7 and hit.position.y < 0.2
		if floorish:
			pulses.emit(0, tap_target, 5.0, 5.5, 0.85, now)
		else:
			pulses.emit(0, tap_target, 6.0, 5.5, 1.0, now)
	elif pitch <= -0.12:
		var df := minf(1.6, EYE / tan(-pitch))
		var p := from + flat * df
		tap_target = Vector3(p.x, 0.02, p.z)
		pulses.emit(0, tap_target, 5.0, 5.5, 0.85, now)
	else:
		# air swish: the cane still reaches out, but air reflects nothing
		var hy := clampf(EYE + tan(pitch) * 1.5, 0.3, 1.7)
		var p := from + flat * 1.5
		tap_target = Vector3(p.x, hy, p.z)
