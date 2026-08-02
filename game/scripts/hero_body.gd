class_name HeroBody
extends Node3D
## The hero's visible body — cane, arm, legs, torso. Everything renders
## through the data pass, so the body is OUTLINE-ONLY like the world:
##   - the cane and the arm holding it carry a standing reveal (u_base):
##     the hero always knows their own grip;
##   - legs and torso are revealed ONLY while a wave sweeps them — each
##     footstep ripple makes your own feet blink into outline.
## The arm is a classical first-person viewmodel: anchored in CAMERA space
## with a figure-eight walk bob, look-sway lag, and a strike kick that
## reaches the cane tip out to the actual tap target and eases back.
## The cane is BODY-anchored in yaw, so it doubles as a pitch indicator.
##
## No raycasts here: the cane rest comes pre-computed from the player's
## physics tick (player.cane_rest), and footsteps are queued to the player
## so their reflection rays also run in physics context.

var player: UnseeingPlayer
var camera: Camera3D
var pulses: Pulses
var cane_mat: ShaderMaterial
var body_mat: ShaderMaterial

var _cane_mesh := ImmediateMesh.new()
var _body_mesh := ImmediateMesh.new()
var _cam_base_y := 0.0

# animation state — constants carried over verbatim from the validated design
var _walk_amp := 0.0
var _leg_phase := 0.0
var _swing_phase := 0.0
var _cane_swing := 0.15
var _sway_x := 0.0
var _sway_y := 0.0
var _last_yaw := 0.0
var _last_pitch := 0.0
var _step_t := 0.0
var _step_side := 1
var _shoe: Array[Vector3] = [Vector3.ZERO, Vector3.ZERO]


func _ready() -> void:
	assert(player != null and camera != null, "hero_body: player/camera not injected")
	assert(
		pulses != null and cane_mat != null and body_mat != null,
		"hero_body: pulses/materials not injected"
	)
	_add_layer(_cane_mesh, cane_mat)
	_add_layer(_body_mesh, body_mat)
	_cam_base_y = camera.position.y
	_last_yaw = player.rotation.y
	_last_pitch = camera.rotation.x


## One render layer of the body: an immediate mesh drawn through the given
## data-pass material, never frustum-culled (the mesh mutates every frame).
func _add_layer(mesh: ImmediateMesh, mat: ShaderMaterial) -> void:
	var mi := MeshInstance3D.new()
	mi.mesh = mesh
	mi.material_override = mat
	mi.extra_cull_margin = 16384.0
	add_child(mi)


## Called by main every frame after movement has settled.
func update(now: float, dt: float) -> void:
	var vel := Vector2(player.velocity.x, player.velocity.z)
	var moving := vel.length() > 0.1
	_walk_amp += ((1.0 if moving else 0.0) - _walk_amp) * minf(dt * 6.0, 1.0)
	if moving:
		_swing_phase += dt * 7.4
		_leg_phase += dt * 7.4
	var swing_target := (0.26 * sin(_swing_phase)) if moving else 0.12
	_cane_swing += (swing_target - _cane_swing) * minf(dt * 10.0, 1.0)

	# strike envelope: quick visible reach-out to the tap target, ease back
	var st_age: float = now - player.last_tap
	var thrust: float = (
		(maxf(st_age, 0.0) / 0.07) if st_age < 0.07 else exp(-(st_age - 0.07) / 0.28)
	)

	# look-sway: the viewmodel lags a touch behind mouse movement
	var inv_dt := 1.0 / maxf(dt, 0.001)
	var yaw := player.rotation.y
	var pitch := camera.rotation.x
	_sway_x += (
		(clampf((yaw - _last_yaw) * inv_dt * 0.02, -0.07, 0.07) - _sway_x) * minf(dt * 9.0, 1.0)
	)
	_sway_y += (
		(clampf((pitch - _last_pitch) * inv_dt * 0.015, -0.06, 0.06) - _sway_y)
		* minf(dt * 9.0, 1.0)
	)
	_last_yaw = yaw
	_last_pitch = pitch

	# classic head-bob while walking
	camera.position.y = _cam_base_y + 0.028 * sin(_leg_phase * 2.0) * _walk_amp

	# ask the player's next physics tick to compute the rest at our sweep angle
	player.cane_rest_offset = _cane_swing * (1.0 - thrust)

	_build_cane(thrust)
	_build_body()
	_footsteps(dt, moving)


func _build_cane(thrust: float) -> void:
	var bx := 0.016 * sin(_leg_phase) * _walk_amp + _sway_x
	var by := 0.012 * sin(_leg_phase * 2.0) * _walk_amp + _sway_y
	var hand := _view_to_world(0.30 + bx, -0.40 + by - 0.03 * thrust, 0.55 + 0.16 * thrust)
	var elbow := _view_to_world(0.48 + bx * 0.5, -0.64 + by * 0.5, 0.26)

	# rest: the tip lies on whatever surface the cane reaches — floor, table,
	# chair seat — pre-computed by the player's physics tick; a small hover
	# animates the sweep so the tip touches down at the extremes
	var rest_tip := player.cane_rest.tip
	var moving := _walk_amp > 0.5
	var lift := maxf(0.0, 1.0 - absf(_cane_swing) / 0.26) if moving else 0.3
	rest_tip.y += 0.12 * lift * (1.0 - thrust)
	var target: Vector3 = player.tap_target
	var tip := rest_tip.lerp(target, clampf(thrust, 0.0, 1.0))

	_cane_mesh.clear_surfaces()
	_cane_mesh.surface_begin(Mesh.PRIMITIVE_TRIANGLES)
	_tube(_cane_mesh, elbow, hand, 0.055, 0.045)
	_sphere(_cane_mesh, hand, 0.055)
	_tube(_cane_mesh, hand, tip, 0.013, 0.010)
	_sphere(_cane_mesh, tip, 0.040)
	_cane_mesh.surface_end()


## A classic viewmodel anchor: camera-space offsets (x right, y up, z depth
## into the view) to a world point.
func _view_to_world(x: float, y: float, z: float) -> Vector3:
	var cb := camera.global_transform.basis
	return camera.global_position + cb.x * x + cb.y * y - cb.z * z


func _build_body() -> void:
	var p := player.global_position
	var fw := -player.global_transform.basis.z
	fw = Vector3(fw.x, 0, fw.z).normalized()
	var rv := player.global_transform.basis.x
	rv = Vector3(rv.x, 0, rv.z).normalized()

	_body_mesh.clear_surfaces()
	_body_mesh.surface_begin(Mesh.PRIMITIVE_TRIANGLES)
	# small slim torso ending in a pelvis the legs grow out of
	var tc := Vector3(p.x, 0, p.z) - fw * 0.20
	_tube(_body_mesh, Vector3(tc.x, 0.90, tc.z), Vector3(tc.x, 1.28, tc.z), 0.11, 0.10)
	_sphere(_body_mesh, Vector3(tc.x, 1.28, tc.z), 0.10)
	_sphere(_body_mesh, Vector3(tc.x, 0.90, tc.z), 0.13)
	# full legs: thigh, knee, shin, round shoe — phase-mirrored walk cycle
	for s: int in [-1, 1]:
		var ph := _leg_phase + (PI if s < 0 else 0.0)
		var thigh_a := 0.5 * sin(ph) * _walk_amp
		var knee_a := maxf(0.0, 0.95 * sin(ph - 0.9)) * _walk_amp
		var shin_a := thigh_a - knee_a
		var hip := Vector3(p.x, 0.90, p.z) + rv * 0.07 * s - fw * 0.20
		var knee := hip + fw * sin(thigh_a) * 0.45
		knee.y = hip.y - cos(thigh_a) * 0.45
		var ankle := knee + fw * sin(shin_a) * 0.45
		ankle.y = maxf(0.07, knee.y - cos(shin_a) * 0.45)
		var shoe := ankle + fw * 0.08
		shoe.y = maxf(0.065, ankle.y - 0.02)
		_shoe[0 if s < 0 else 1] = shoe
		_tube(_body_mesh, hip, knee, 0.06, 0.05)
		_sphere(_body_mesh, knee, 0.055)
		_tube(_body_mesh, knee, ankle, 0.05, 0.04)
		_sphere(_body_mesh, shoe, 0.08)
	_body_mesh.surface_end()


## Each footfall: a small wave rippling out around that very shoe, queued to
## the player so its reflection rays run in the physics tick.
func _footsteps(dt: float, moving: bool) -> void:
	if not moving:
		_step_t = 0.1
		return
	_step_t -= dt
	if _step_t > 0.0:
		return
	var shoe: Vector3 = _shoe[0 if _step_side < 0 else 1]
	player.queue_wave(2, Vector3(shoe.x, 0.04, shoe.z), 1.6, 4.0, 0.8, 2, Vector3.UP)
	_step_side = -_step_side
	_step_t = 0.42


# --- smooth cartoon geometry: per-vertex normals mean the edge detector
# --- draws only one clean silhouette per shape, never facet lines
func _sphere(mesh: ImmediateMesh, c: Vector3, r: float) -> void:
	const LA := 6
	const LO := 12
	for i: int in LA:
		var t0 := (float(i) / LA) * PI
		var t1 := (float(i + 1) / LA) * PI
		for j: int in LO:
			var p0 := (float(j) / LO) * TAU
			var p1 := (float(j + 1) / LO) * TAU
			var n00 := Vector3(sin(t0) * cos(p0), cos(t0), sin(t0) * sin(p0))
			var n01 := Vector3(sin(t0) * cos(p1), cos(t0), sin(t0) * sin(p1))
			var n10 := Vector3(sin(t1) * cos(p0), cos(t1), sin(t1) * sin(p0))
			var n11 := Vector3(sin(t1) * cos(p1), cos(t1), sin(t1) * sin(p1))
			for tri: Array in [[n00, n10, n11], [n00, n11, n01]]:
				for n: Vector3 in tri:
					mesh.surface_set_normal(n)
					mesh.surface_add_vertex(c + n * r)


func _tube(mesh: ImmediateMesh, a: Vector3, b: Vector3, r1: float, r2: float) -> void:
	const SEG := 10
	var axis := b - a
	var al := axis.length()
	axis = axis / al if al > 0.0001 else Vector3.UP
	var ref := Vector3(1, 0, 0) if absf(axis.y) > 0.9 else Vector3(0, 1, 0)
	var u := axis.cross(ref).normalized()
	var v := axis.cross(u)
	for k: int in SEG:
		var a0 := (float(k) / SEG) * TAU
		var a1 := (float(k + 1) / SEG) * TAU
		var d0 := u * cos(a0) + v * sin(a0)
		var d1 := u * cos(a1) + v * sin(a1)
		var p00 := a + d0 * r1
		var p01 := a + d1 * r1
		var p10 := b + d0 * r2
		var p11 := b + d1 * r2
		for pv: Array in [[p00, d0], [p10, d0], [p11, d1], [p00, d0], [p11, d1], [p01, d1]]:
			mesh.surface_set_normal(pv[1])
			mesh.surface_add_vertex(pv[0])
