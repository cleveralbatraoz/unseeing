extends RefCounted
## Checked, code-built physical scenes shared by player and cat elevation
## tests. Every wait has a fixed physics-frame bound; every authored solid is
## verified through the collider the runtime node actually generated.

const DT := 1.0 / 60.0
const RAMP_SIZE := Vector3(1.4, 0.45, 1.0)
const PLATFORM_SIZE := Vector3(1.2, 0.45, 1.0)
const TABLE_SCENE := preload("res://scenes/props/table.tscn")
const TABLE_TOP_Y := 0.75  # Top centre 0.725 + half-height 0.025.
const BED_TOP_Y := 0.48  # BedFrame centre 0.42 + half-height 0.06.

## Settled-contact tolerance for every support-height assertion these
## suites make: `SAFE_MARGIN_M` (0.001, `rust/src/nodes/support.rs`) plus
## one f32 ULP taken in the binade [1, 2) (2^-23, 1.1920928955078125e-07),
## not the ULP at each assertion's own magnitude. It is deliberately a
## SINGLE shared bound covering every support height these suites assert,
## from `BED_TOP_Y` (0.48) up through the player's 0.9, so it is
## conservative rather than exact — larger than the true ULP at each of
## those magnitudes (2^-24 or 2^-25), never smaller — and can neither
## tighten a test into flakiness nor loosen one enough to hide a defect.
const SETTLED_CONTACT_TOLERANCE_M := 0.0010001192092895508


static func add_box(
	parent: Node, centre: Vector3, size: Vector3, body_name: String
) -> StaticBody3D:
	var body := StaticBody3D.new()
	body.name = body_name
	body.position = centre
	var collision := CollisionShape3D.new()
	var shape := BoxShape3D.new()
	shape.size = size
	collision.shape = shape
	body.add_child(collision)
	parent.add_child(body)
	assert(collision.shape == shape, "%s must retain its checked BoxShape3D" % body_name)
	return body


static func add_floor(parent: Node, top_y := 0.0, size := Vector2(20.0, 20.0)) -> StaticBody3D:
	return add_box(parent, Vector3(0.0, top_y - 0.05, 0.0), Vector3(size.x, 0.1, size.y), "Floor")


static func add_ramp(parent: Node, datum: Vector3, size := RAMP_SIZE) -> WaveWedge:
	assert(parent.is_inside_tree(), "Ramp collider readback requires a live tree parent")
	var ramp := WaveWedge.new()
	ramp.name = "Ramp"
	ramp.size = size
	ramp.position = datum
	parent.add_child(ramp)
	var collisions := ramp.find_children("*", "CollisionShape3D", true, false)
	assert(collisions.size() == 1, "Ramp must generate exactly one collision shape")
	var collision := collisions[0] as CollisionShape3D
	var hull := collision.shape as ConvexPolygonShape3D
	assert(hull != null, "Ramp must generate its ConvexPolygonShape3D")
	assert(hull.points.size() == 6, "Ramp hull must retain all six checked wedge points")
	return ramp


static func add_ramp_platform(parent: Node, datum: Vector3) -> WaveProp:
	assert(parent.is_inside_tree(), "Platform collider readback requires a live tree parent")
	var platform := WaveProp.new()
	platform.name = "RampPlatform"
	platform.size = PLATFORM_SIZE
	platform.position = datum + Vector3(1.3, 0.225, 0.0)
	parent.add_child(platform)
	var collisions := platform.find_children("*", "CollisionShape3D", true, false)
	assert(collisions.size() == 1, "Ramp platform must generate exactly one collision shape")
	var collision := collisions[0] as CollisionShape3D
	var box := collision.shape as BoxShape3D
	assert(box != null, "Ramp platform must generate its BoxShape3D")
	assert(box.size == PLATFORM_SIZE, "Ramp platform collider must match its authored size")
	return platform


static func add_player(parent: Node, at := Vector3(0.0, 0.9, 0.0)) -> UnseeingPlayer:
	var player := UnseeingPlayer.new()
	player.pulses = Pulses.new()
	player.position = at
	parent.add_child(player)
	return player


static func add_cat(parent: Node, at := Vector3.ZERO, seed := 7) -> WaveCat:
	var cat := WaveCat.new()
	cat.pulses = Pulses.new()
	cat.data_mat = ShaderMaterial.new()
	cat.position = at
	cat.seed = seed
	parent.add_child(cat)
	return cat


static func add_table(parent: Node, at: Vector3) -> Node3D:
	var table := TABLE_SCENE.instantiate() as Node3D
	assert(table != null, "table.tscn must retain its Node3D root")
	table.position = at
	parent.add_child(table)
	return table


static func add_bed(parent: Node, at: Vector3) -> WaveProp:
	var bed := WaveProp.new()
	bed.name = "BedFrame"
	bed.size = Vector3(1.9, 0.12, 0.9)
	bed.position = at + Vector3(0.0, 0.42, 0.0)
	parent.add_child(bed)
	return bed


static func poll_physics(tree: SceneTree, predicate: Callable, max_ticks: int) -> bool:
	assert(max_ticks >= 0, "physics poll bound must not be negative")
	if predicate.call():
		return true
	for _tick: int in max_ticks:
		await tree.physics_frame
		if predicate.call():
			return true
	return false


## Task 7 cross-check: reads the SAME hero the caller just drove through
## `WaveObserver.snapshot`, a channel none of these tests' own assertions
## produced. Builds and tears down its own throwaway level so callers that
## never wrapped their player in one (most of this suite) can still prove
## the observer's motion dictionary agrees with the state they already
## pinned directly. `null` on any refusal (poisoned position/velocity/
## identity, or an absent hero).
static func hero_motion(
	parent: Node, player: UnseeingPlayer, pulses: Pulses, now: float
) -> Variant:
	var level := WaveLevel.new()
	level.add_child(WaveSpawn.new())
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), pulses)
	parent.add_child(level)
	var observer := WaveObserver.new()
	parent.add_child(observer)
	observer.inject(level, player.camera)
	observer.inject_hero(player)
	var snap: Dictionary = observer.snapshot(now)
	observer.queue_free()
	level.queue_free()
	if snap.has("unavailable"):
		return null
	var hero: Dictionary = snap.get("hero", {})
	return hero.get("motion")


## Task 7 cross-check, the cat's side of `hero_motion`: a cat census only
## finds cats parented under the SAME `WaveLevel` an observer was injected
## with, but this suite's own fixtures parent cats directly under the test
## root (matching the level-free floors/tables/beds they stand on). Rather
## than disturb that shared layout, this reparents the cat under a
## throwaway level just long enough to read it back, then restores its
## original parent — `global_position` is preserved by `reparent`'s default
## `keep_global_transform`.
##
## `WaveLevel` census-walks its subtree exactly once, at its own `_ready`
## (`derive()`, `rust/src/nodes/level.rs`) — never again at runtime, by
## design, so a running level pays no per-frame O(scene) walk. Reparenting
## the cat in AFTER that first derive leaves the level's cached
## `cat_children` still empty; `rederive()` (the same `#[func]` the editor's
## own drag-a-node watch calls) re-runs that walk on demand without
## touching the cat's own already-established motion state — `derive()`
## only reads and stores handles, the pulses/data_mat injection that DOES
## write to a cat's properties lives in the separate `inject()` this helper
## already called before the cat ever joined. `null` on any refusal or
## missing entry.
static func cat_motion(parent: Node, cat: WaveCat, pulses: Pulses, now: float) -> Variant:
	var level := WaveLevel.new()
	level.add_child(WaveSpawn.new())
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), pulses)
	parent.add_child(level)
	var original_parent := cat.get_parent()
	cat.reparent(level)
	level.rederive()
	var camera := Camera3D.new()
	parent.add_child(camera)
	var observer := WaveObserver.new()
	parent.add_child(observer)
	observer.inject(level, camera)
	var snap: Dictionary = observer.snapshot(now)
	cat.reparent(original_parent)
	observer.queue_free()
	camera.queue_free()
	level.queue_free()
	if snap.has("unavailable"):
		return null
	var cats: Array = snap.get("cats_motion", [])
	if cats.is_empty():
		return null
	var entry: Dictionary = cats[0]
	return entry.get("motion")
