extends SceneTree
## Headless determinism probe: boot the full game seeded, run a fixed
## number of frames, print ONE hash of the whole snapshot, quit. The gate
## (tools/determinism_probe.sh) runs this twice under --fixed-fps and
## demands the pair agree — the warm-boot-pair law applied to state.
##
## Frame counting rides the process_frame SIGNAL, never a SceneTree
## _process override, which would shadow the engine loop. 240 frames at a
## fixed 1/60 delta is now = 4.0 s exactly: several source beats, cat paw
## taps, and flicker jitter all land inside the hashed window.
##
## Refusals are loud: an unseeded run or a refused snapshot exits 2 with
## no hash line — the gate treats a missing hash as failure, never a pass.

const MAIN_SCENE := preload("res://scenes/main.tscn")
const FRAMES := 240

## Packed*Array kinds whose elements are themselves recursed rather than
## handed to JSON.stringify as-is — named explicitly rather than inferred
## from Variant.Type ordering, so a future Godot re-ordering the enum can't
## silently change which kinds this probe walks.
const _PACKED_ARRAY_TYPES := [
	TYPE_PACKED_BYTE_ARRAY,
	TYPE_PACKED_INT32_ARRAY,
	TYPE_PACKED_INT64_ARRAY,
	TYPE_PACKED_FLOAT32_ARRAY,
	TYPE_PACKED_FLOAT64_ARRAY,
	TYPE_PACKED_STRING_ARRAY,
	TYPE_PACKED_VECTOR2_ARRAY,
	TYPE_PACKED_VECTOR3_ARRAY,
	TYPE_PACKED_VECTOR4_ARRAY,
	TYPE_PACKED_COLOR_ARRAY,
]

var _main: UnseeingMain
var _frames_left := FRAMES


func _initialize() -> void:
	var seeded := not OS.get_environment("UNSEEING_SEED").is_empty()
	var demoed := not OS.get_environment("UNSEEING_DEMO").is_empty()
	if not seeded and not demoed:
		push_error("determinism probe: refusing an unseeded run — set UNSEEING_SEED=1")
		quit(2)
		return
	_main = MAIN_SCENE.instantiate() as UnseeingMain
	root.add_child(_main)
	process_frame.connect(_on_frame)


func _on_frame() -> void:
	_frames_left -= 1
	if _frames_left > 0:
		return
	var snap: Dictionary = _main.observer.snapshot(_main.now)
	if snap.has("unavailable"):
		push_error("determinism probe: snapshot refused: %s" % snap["unavailable"])
		quit(2)
		return
	# sorted keys + FULL float precision on every BARE float — but
	# JSON.stringify's full_precision flag never reaches a float sitting
	# inside a Vector2/Vector3/Vector4: those serialize through Godot's
	# String(Vector3) pretty-printer first ("(1.0, 2.0, 3.0)"), which
	# truncates well short of f32 precision, so a one-ULP drift in a
	# vector component would vanish before the hash ever saw it.
	# `canonicalize` decomposes every vector lane into bare floats BEFORE
	# stringify, which is what makes "full float precision" actually true
	# for the whole snapshot rather than only its scalar keys.
	var hash_text := JSON.stringify(canonicalize(snap), "", true, true).md5_text()
	print("DETERMINISM_HASH=%s" % hash_text)
	quit(0)


## A JSON-safe deep copy of `value`: every vector-valued Variant becomes a
## bare-float Array, so nothing downstream can round-trip it through a
## pretty-printer. Total over any Variant the snapshot can hand back —
## floats, ints, bools and Strings pass through unchanged, which is exactly
## what the old direct-stringify path already did right for those.
static func canonicalize(value: Variant) -> Variant:
	var kind := typeof(value)
	match kind:
		TYPE_VECTOR2:
			var v2: Vector2 = value
			return [v2.x, v2.y]
		TYPE_VECTOR3:
			var v3: Vector3 = value
			return [v3.x, v3.y, v3.z]
		TYPE_VECTOR4:
			var v4: Vector4 = value
			return [v4.x, v4.y, v4.z, v4.w]
		TYPE_BASIS:
			var b: Basis = value
			return [canonicalize(b.x), canonicalize(b.y), canonicalize(b.z)]
		TYPE_DICTIONARY:
			var src: Dictionary = value
			var out := {}
			for key: String in src:
				out[key] = canonicalize(src[key])
			return out
		TYPE_ARRAY:
			return _canonicalize_sequence(value)
		_:
			if kind in _PACKED_ARRAY_TYPES:
				return _canonicalize_sequence(value)
			return value


## Shared walk for Array and every Packed*Array kind: both are iterable in
## GDScript, neither shares a common typed parameter with the other, so the
## parameter stays Variant rather than pretending to a type that doesn't
## exist.
static func _canonicalize_sequence(sequence: Variant) -> Array:
	var out: Array = []
	for item: Variant in sequence:
		out.append(canonicalize(item))
	return out
