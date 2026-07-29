class_name Materials
extends RefCounted
## The material concept — a self-contained module, deliberately separated
## from the map, the hero, and the render wiring.
##
## A material is what a surface IS to sound: when an echo sweeps it, the
## reveal carries a minimal cartoon signature of the surface character —
## roughness, grain, weave — as a procedural monochrome pattern (defined
## shader-side in materials.gdshaderinc; this file is the only place that
## knows which numeric id each kind maps to).
##
## Consumers ask for materials by KIND and never touch shader internals:
##   Materials.shared(Materials.Kind.ROCK)  — cached instance, one per kind,
##                                            for world geometry
##   Materials.unique(Materials.Kind.WOOD)  — private instance, for anything
##                                            needing its own uniforms (the
##                                            hero's cane carries u_base)
## Every created material self-registers; main pushes the per-frame wave
## uniforms to Materials.registry() without knowing what exists.
##
## Extension point: per-kind acoustic properties (echo gain, tap sound)
## belong here when the audio stage lands.

enum Kind { ROCK, WOOD, WOOL, CLOTH, GLASS }

const _DATA_SHADER := preload("res://shaders/data_pass.gdshader")

static var _shared: Dictionary = {}
static var _registry: Array[ShaderMaterial] = []

## Call once at game start: static state survives scene reloads otherwise.
static func reset() -> void:
	_shared.clear()
	_registry.clear()

static func shared(kind: Kind) -> ShaderMaterial:
	if not _shared.has(kind):
		_shared[kind] = _create(kind)
	return _shared[kind]

static func unique(kind: Kind) -> ShaderMaterial:
	return _create(kind)

## All live materials, for per-frame uniform application (wave pool, clock).
static func registry() -> Array[ShaderMaterial]:
	return _registry

static func _create(kind: Kind) -> ShaderMaterial:
	var mat := ShaderMaterial.new()
	mat.shader = _DATA_SHADER
	mat.set_shader_parameter("u_material", int(kind))
	_registry.append(mat)
	return mat
