class_name SoundFan
extends Node3D
## A constant sound source: an oscillating pedestal fan. A blind person
## FEELS a steady source even from another room, so the fan's hum is pulse
## type 3 ("source hum"): its wave shells pass through walls muffled instead
## of being cut like every player-made sound. The head pivots left and right
## while the blades spin; each hum is born at the moving hub, and the head
## carries a real collider that pivots with it, so the cane and echo rays
## strike the fan like anything else in the world.

const WHOOSH_EVERY := 1.15   # seconds between hums — the room breathes
const HUM_RANGE := 9.0       # meters a hum travels
const HUM_SPEED := 4.5
const HUM_GAIN := 0.75
const PIVOT_RANGE := 0.85    # rad each way from the mounting yaw
const PIVOT_SPEED := 0.55
const SPIN_SPEED := 9.0      # rad/s — reads as motion across reveals
const HEAD_H := 1.15         # hub height: within the cane's reach

var pulses: Pulses
var data_mat: Material

var _pivot: Node3D
var _spinner: Node3D
var _next_whoosh := 0.4

## Pure motion curves, split out for the headless tests.
static func pivot_angle(t: float) -> float:
	return sin(t * PIVOT_SPEED) * PIVOT_RANGE

static func spin_angle(t: float) -> float:
	return fmod(t * SPIN_SPEED, TAU)

func _ready() -> void:
	# pedestal: base disc + pole, as static as the walls
	var pedestal := StaticBody3D.new()
	add_child(pedestal)
	_mesh(pedestal, _cyl(0.22, 0.06), Vector3(0, 0.03, 0))
	_mesh(pedestal, _cyl(0.03, HEAD_H), Vector3(0, HEAD_H * 0.5, 0))
	var base_col := CollisionShape3D.new()
	var pole := CylinderShape3D.new()
	pole.radius = 0.22
	pole.height = HEAD_H
	base_col.shape = pole
	base_col.position = Vector3(0, HEAD_H * 0.5, 0)
	pedestal.add_child(base_col)

	# the pivoting head: motor, guard ring and a collider that swings along
	_pivot = Node3D.new()
	_pivot.position = Vector3(0, HEAD_H, 0)
	add_child(_pivot)
	var head := AnimatableBody3D.new()
	_pivot.add_child(head)
	_mesh(head, _box(0.16, 0.16, 0.24), Vector3(0, 0, 0.10))
	var torus := TorusMesh.new()
	torus.inner_radius = 0.40
	torus.outer_radius = 0.44
	_mesh(head, torus, Vector3(0, 0, -0.10), PI * 0.5)
	var head_col := CollisionShape3D.new()
	var disc := CylinderShape3D.new()
	disc.radius = 0.45
	disc.height = 0.30
	head_col.shape = disc
	head_col.rotation.x = PI * 0.5   # cylinder axis Y -> face along Z
	head_col.position = Vector3(0, 0, -0.06)
	head.add_child(head_col)

	# the blades: three flat paddles around a hub, spinning about the facing axis
	_spinner = Node3D.new()
	_spinner.position = Vector3(0, 0, -0.10)
	_pivot.add_child(_spinner)
	_mesh(_spinner, _cyl(0.045, 0.08), Vector3.ZERO, PI * 0.5)
	for k: int in 3:
		var arm := Node3D.new()
		arm.rotation.z = TAU * float(k) / 3.0
		_spinner.add_child(arm)
		_mesh(arm, _box(0.32, 0.11, 0.016), Vector3(0.24, 0, 0))

## Driven by main with the simulated clock, like every animated thing —
## movie-maker runs and time scaling stay correct.
func update(t: float) -> void:
	_pivot.rotation.y = pivot_angle(t)
	_spinner.rotation.z = spin_angle(t)
	if t < _next_whoosh:
		return
	_next_whoosh = t + WHOOSH_EVERY
	pulses.emit(3, _spinner.global_position, HUM_RANGE, HUM_SPEED, HUM_GAIN, t)

func _mesh(parent: Node3D, m: Mesh, at: Vector3, rx := 0.0) -> void:
	var mi := MeshInstance3D.new()
	mi.mesh = m
	mi.material_override = data_mat
	mi.position = at
	mi.rotation.x = rx
	parent.add_child(mi)

func _cyl(radius: float, height: float) -> CylinderMesh:
	var c := CylinderMesh.new()
	c.top_radius = radius
	c.bottom_radius = radius
	c.height = height
	return c

func _box(x: float, y: float, z: float) -> BoxMesh:
	var b := BoxMesh.new()
	b.size = Vector3(x, y, z)
	return b
