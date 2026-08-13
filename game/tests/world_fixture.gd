extends RefCounted
## Code-built composition fixtures for tests of the Rust game root. Shipped
## levels are designer content: their walls, sources, creatures, positions,
## and names may all change without rewriting an engine test. Each caller asks
## explicitly for only the collaborators its behavior needs.

const DEFAULT_EXTENTS := Vector2(16, 12)
const SPAWN_AT := Vector3(2, 0, 6)
const WALL_AT := Vector3(6, 0, 6)
const SOURCE_AT := Vector3(10, 0, 6)


static func level_scene(
	extents: Vector2 = DEFAULT_EXTENTS,
	with_wall: bool = false,
	with_source: bool = false,
	with_cat: bool = false,
) -> PackedScene:
	var level := WaveLevel.new()
	level.name = "FixtureLevel"
	level.extents = extents

	var spawn := WaveSpawn.new()
	spawn.name = "Spawn"
	spawn.position = SPAWN_AT
	_add_authored(level, spawn)

	if with_wall:
		var wall := WaveWall.new()
		wall.name = "Divider"
		wall.length = 6.0
		wall.position = WALL_AT
		wall.rotation.y = PI * 0.5
		_add_authored(level, wall)

	if with_source:
		var source := SoundFan.new()
		source.name = "FixtureFan"
		source.position = SOURCE_AT
		_add_authored(level, source)

	if with_cat:
		var cat := WaveCat.new()
		cat.name = "FixtureCat"
		cat.position = Vector3(3, 0, 3)
		cat.seed = 7
		cat.roam_size = Vector2(2, 2)
		_add_authored(level, cat)

	var packed := PackedScene.new()
	var result := packed.pack(level)
	level.free()
	if result != OK:
		push_error("world fixture: could not pack its code-built WaveLevel")
		return null
	return packed


static func game(
	extents: Vector2 = DEFAULT_EXTENTS,
	with_wall: bool = false,
	with_source: bool = false,
	with_cat: bool = false,
) -> UnseeingGame:
	var root := UnseeingGame.new()
	root.level_scene = level_scene(extents, with_wall, with_source, with_cat)
	return root


static func _add_authored(level: WaveLevel, child: Node) -> void:
	level.add_child(child)
	child.owner = level
