extends Node
## Task 7's end-to-end elevation fixture. Three actors, spawned already in
## place, sit at three labelled X lanes a single fixed camera frames at
## once: flat, `0.45 m` supported, and unsupported `3 m` — the last of
## which is sampled once mid-fall (`airborne`) and once more after it
## settles on the lower floor beneath it (`landed`). Four `ELEVATION_STATE`
## records total.
##
## Every `WaveLevel` derives its own floor AND a `level_plan::WALL_H` =
## 3 m ceiling, spanning `0..extents` from its origin (`nodes/level.rs`) —
## real `StaticBody3D` colliders the player's own floor snap happily
## accepts. A 3 m unsupported drop cannot fit under that ceiling, so the
## flat and elevated lanes share a small room (`LEVEL_EXTENTS`) while the
## airborne lane's X sits outside it entirely, clear of both the room's
## floor and its ceiling, with its own hand-built lower floor standing
## alone in open space so the whole drop is genuinely unsupported.
##
## Every record is built from evidence Task 7's own dictionary-writing code
## did not author: the actor root and its collider's own bottom (read
## straight off the live `CollisionShape3D`/`CapsuleShape3D`, never off the
## observer), and the baked body mesh's own Y extrema (`HeroBody.body_mesh`,
## the SAME mesh `player_elevation_test.gd`'s transport tests already pin).
## The record's `motion` field is the Task 7 observer dictionary itself
## (`WaveObserver.snapshot`'s `hero.motion`) — carried alongside, not folded
## into, the independent channels above, so a reader can see whether the
## three agree rather than trusting one of them to prove the other two.
##
## Test-only: this scene references only this probe and `character_elevation_
## fixture.gd`, never ships in an authored level, and its `.uid` is
## committed like every other GDScript sidecar.

const ELEVATION_FIXTURE := preload("res://tests/character_elevation_fixture.gd")

const DT := 1.0 / 60.0
const LEVEL_EXTENTS := Vector2(9.0, 6.0)
const LANE_Z := 3.0
const FLAT_X := 1.0
const PLATFORM_X := 5.0
const PLATFORM_TOP_Y := 0.45
const AIRBORNE_X := 15.0  # Outside LEVEL_EXTENTS.x — clear of the room entirely.
const AIRBORNE_DROP_M := 3.0
const AIRBORNE_LOWER_TOP_Y := -2.1
const SETTLE_TICKS := 25
const LAND_TIMEOUT_TICKS := 240
const FLAT_LANE := 0
const ELEVATED_LANE := 1
const AIRBORNE_LANE := 2

var _players: Array[UnseeingPlayer] = []
var _heroes: Array[HeroBody] = []
var _observer: WaveObserver
var _level: WaveLevel
var _now := 0.0
var _failures := 0


func _ready() -> void:
	_build()
	await _run()
	get_tree().quit(1 if _failures > 0 else 0)


func _build() -> void:
	var pulses := Pulses.new()
	_level = WaveLevel.new()
	_level.name = "MovieLevel"
	_level.extents = LEVEL_EXTENTS
	_level.add_child(WaveSpawn.new())
	_level.inject(ShaderMaterial.new(), ShaderMaterial.new(), pulses)
	add_child(_level)

	# The 0.45 m platform stands on the room's own auto-floor. The airborne
	# lane's lower floor is its own solid, standing alone past the room's
	# far wall, well clear of both the room's floor and its 3 m ceiling.
	ELEVATION_FIXTURE.add_box(
		self,
		Vector3(PLATFORM_X, PLATFORM_TOP_Y * 0.5, LANE_Z),
		Vector3(2.0, PLATFORM_TOP_Y, 2.0),
		"Platform045"
	)
	ELEVATION_FIXTURE.add_box(
		self,
		Vector3(AIRBORNE_X, AIRBORNE_LOWER_TOP_Y - 0.05, LANE_Z),
		Vector3(4.0, 0.1, 4.0),
		"LowerFloor"
	)

	_spawn_actor(pulses, Vector3(FLAT_X, 0.9, LANE_Z))
	_spawn_actor(pulses, Vector3(PLATFORM_X, PLATFORM_TOP_Y + 0.9, LANE_Z))
	_spawn_actor(pulses, Vector3(AIRBORNE_X, AIRBORNE_LOWER_TOP_Y + 0.9 + AIRBORNE_DROP_M, LANE_Z))

	var camera := Camera3D.new()
	add_child(camera)
	camera.position = Vector3(8.0, 4.0, 16.0)
	camera.look_at(Vector3(8.0, 0.0, LANE_Z), Vector3.UP)

	_observer = WaveObserver.new()
	add_child(_observer)
	_observer.inject(_level, camera)


func _spawn_actor(pulses: Pulses, at: Vector3) -> void:
	var player: UnseeingPlayer = ELEVATION_FIXTURE.add_player(self, at)
	player.pulses = pulses
	var hero := HeroBody.new()
	hero.player = player
	hero.camera = player.camera
	hero.pulses = pulses
	hero.cane_mat = ShaderMaterial.new()
	hero.body_mat = ShaderMaterial.new()
	add_child(hero)
	_players.append(player)
	_heroes.append(hero)


func _run() -> void:
	await _tick(SETTLE_TICKS)
	_emit(FLAT_LANE, "flat")
	_emit(ELEVATED_LANE, "elevated")

	if _players[AIRBORNE_LANE].collision_layer != 4:
		_fail("the unsupported actor never departed onto the airborne layer")
	_emit(AIRBORNE_LANE, "airborne")

	var landed := false
	for _i: int in LAND_TIMEOUT_TICKS:
		await _tick(1)
		if _players[AIRBORNE_LANE].collision_layer == 2 and _players[AIRBORNE_LANE].is_on_floor():
			landed = true
			break
	if landed:
		_emit(AIRBORNE_LANE, "landed")
	else:
		_fail("the unsupported actor never landed within %d ticks" % LAND_TIMEOUT_TICKS)


func _tick(ticks: int) -> void:
	for _i: int in ticks:
		_now += DT
		for player: UnseeingPlayer in _players:
			player.tick(_now)
		for hero: HeroBody in _heroes:
			hero.update(_now, DT)
		await get_tree().physics_frame


func _emit(lane: int, state: String) -> void:
	_observer.inject_hero(_players[lane])
	var record: Variant = _record(_players[lane], _heroes[lane])
	if record == null:
		_fail("%s: could not build a complete record" % state)
		return
	print("ELEVATION_STATE %s %s" % [state, JSON.stringify(record)])


## Channel 1 (actor root/collider) and channel 2 (mesh Y extrema) are read
## straight off the live engine nodes; channel 3 (`motion`) is the Task 7
## observer dictionary. `null` on any missing mark — never a partial or
## invented record.
func _record(player: UnseeingPlayer, hero: HeroBody) -> Variant:
	var collisions := player.find_children("*", "CollisionShape3D", false, false)
	var collision: CollisionShape3D = collisions[0] if not collisions.is_empty() else null
	var capsule: CapsuleShape3D = collision.shape if collision != null else null
	var arrays := hero.body_mesh().surface_get_arrays(0)
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var snap: Dictionary = _observer.snapshot(_now)
	var hero_dict: Dictionary = snap.get("hero", {})
	if (
		capsule == null
		or verts.is_empty()
		or snap.has("unavailable")
		or not hero_dict.has("motion")
	):
		return null
	var collider_bottom_y: float = (
		player.global_position.y + collision.position.y - capsule.height * 0.5
	)
	var mesh_min_y := verts[0].y
	var mesh_max_y := verts[0].y
	for v: Vector3 in verts:
		mesh_min_y = minf(mesh_min_y, v.y)
		mesh_max_y = maxf(mesh_max_y, v.y)
	return {
		"actor_root_y": player.global_position.y,
		"collider_bottom_y": collider_bottom_y,
		"mesh_min_y": mesh_min_y,
		"mesh_max_y": mesh_max_y,
		"motion": hero_dict["motion"],
	}


func _fail(message: String) -> void:
	_failures += 1
	print("not ok - %s" % message)
