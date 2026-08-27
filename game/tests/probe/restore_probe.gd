extends SceneTree
## Advance-and-compare, the omission detector. Mode "capture": boot
## seeded, live T frames, write the blob, live N more, print the state
## hash. Mode "restore": boot fresh, restore the blob, live the SAME N,
## print the hash. tools/restore_probe.sh demands the pair agree — any
## state that influences the future but escaped the blob diverges here.
## Frame counting rides process_frame; refusals exit 2 with no hash line.
##
## Round-trip hashing (restore_test.gd) proves SERIALIZATION: what the blob
## carries survives the journey. It is structurally blind to OMISSION — a
## field absent from both capture and restore agrees with itself forever.
## Only running the world forward can see it, so this probe does.
##
## The anchors are not interchangeable. Run A's N frames run on from the
## LIVE state at frame T; run B's run from the RESTORED state at frame 1.
## Both legs therefore count N process frames and N physics ticks from
## their own anchor, in the same order, which is what makes the two hashes
## comparable at all.

const MAIN_SCENE := preload("res://scenes/main.tscn")
const T_FRAMES := 180
const N_FRAMES := 240

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

var _main: UnseeingGame
var _mode := ""
var _blob_path := ""
var _frames := 0
var _captured := false


func _initialize() -> void:
	_mode = OS.get_environment("UNSEEING_RESTORE_MODE")
	_blob_path = OS.get_environment("UNSEEING_RESTORE_BLOB")
	if OS.get_environment("UNSEEING_SEED").is_empty():
		push_error("restore probe: refusing an unseeded run")
		quit(2)
		return
	if _mode != "capture" and _mode != "restore":
		push_error("restore probe: UNSEEING_RESTORE_MODE must be capture|restore")
		quit(2)
		return
	_main = MAIN_SCENE.instantiate() as UnseeingGame
	root.add_child(_main)
	process_frame.connect(_on_frame)


func _on_frame() -> void:
	_frames += 1
	if _mode == "capture":
		_capture_leg()
	else:
		_restore_leg()


func _capture_leg() -> void:
	if _frames == T_FRAMES:
		# Leave one request in the hero's out-tray at the capture boundary. The
		# live leg emits it on the following physics tick; the restore leg must
		# carry the format-2 gate and emit it on its corresponding tick.
		_main.player.queue_wave(2, Vector3(1.875, 0.375, -2.625), 4.75, 5.25, 0.625, 2, Vector3.UP)
		var blob: Dictionary = _main.observer.capture(_main.now, _main.capture_env())
		if blob.has("unavailable"):
			push_error("restore probe: capture refused: %s" % blob["unavailable"])
			quit(2)
			return
		var format_fault := _format2_activation_fault(blob)
		if not format_fault.is_empty():
			push_error("restore probe: format-2 activation capture fault: %s" % format_fault)
			quit(2)
			return
		var out := FileAccess.open(_blob_path, FileAccess.WRITE)
		if out == null:
			push_error("restore probe: cannot write the blob to %s" % _blob_path)
			quit(2)
			return
		out.store_string(JSON.stringify(blob, "", true, true))
		out.close()
		_captured = true
	if _captured and _frames == T_FRAMES + N_FRAMES:
		_print_hash_and_quit()


## Any refusal here is fatal and prints NO hash. Artifact refusals are
## preflight-atomic; a post-write refusal now denotes an internal prepared
## commit defect, whose world is equally ineligible for a comparison hash.
func _restore_leg() -> void:
	if _frames == 1:
		var text := FileAccess.get_file_as_string(_blob_path)
		var raw: Variant = JSON.parse_string(text)
		# typeof, not `!= null`: a truncated file can parse to a number or a
		# String just as easily as to nothing, and `null as Dictionary` would
		# then be assigned into a Dictionary-typed local at runtime
		if typeof(raw) != TYPE_DICTIONARY:
			push_error("restore probe: blob file unreadable")
			quit(2)
			return
		var blob := raw as Dictionary
		var format_fault := _format2_activation_fault(blob)
		if not format_fault.is_empty():
			push_error("restore probe: format-2 activation blob fault: %s" % format_fault)
			quit(2)
			return
		var verdict: Dictionary = _main.restore_blob(blob)
		if verdict.has("unavailable"):
			push_error("restore probe: restore refused: %s" % verdict["unavailable"])
			quit(2)
			return
	if _frames == 1 + N_FRAMES:
		_print_hash_and_quit()


## Probe the live adapter capability at the same boundary as the future
## comparison: format 2 is final and ACTIVE. In this fixed probe scene the
## hero stands on the world floor and every cat walks on it, so their motion
## must carry a genuine support contact, not the dormant defaults format 2
## shipped with before Task 3 activated it. Both actors stay
## suppression-clear and every queued gate is open.
func _format2_activation_fault(blob: Dictionary) -> String:
	var fault := ""
	if blob.get("format_version") != 2:
		fault = "format_version"
	var hero: Dictionary = blob.get("hero", {})
	if fault.is_empty() and not _is_live_motion(hero.get("motion", {})):
		fault = "hero.motion"
	if fault.is_empty() and hero.get("footstep_suppression_pending", true):
		fault = "hero.footstep_suppression_pending"
	var waves: Array = hero.get("queued_waves", [])
	if fault.is_empty() and waves.is_empty():
		fault = "hero.queued_waves"
	for wave: Dictionary in waves:
		if fault.is_empty() and wave.get("gate") != "always":
			fault = "hero.queued_waves.gate"
	for cat: Dictionary in blob.get("cats", []):
		if fault.is_empty() and not _is_live_motion(cat.get("motion", {})):
			fault = "cats.motion"
		var gait: Dictionary = cat.get("gait", {})
		var support_y: String = str(gait.get("support_y", "<missing>"))
		for group_name: String in ["planted", "aim"]:
			for point: Array in gait.get(group_name, []):
				if fault.is_empty() and (point.size() != 3 or str(point[1]) != support_y):
					fault = "cats.gait.%s" % group_name
	return fault


## Well formed and LIVE: a known phase kind, and a support contact that is
## present and structurally complete — the shape a standing actor's motion
## must take, never the initial/airborne-without-ground defaults. A landing
## is not required (nothing in the probe scene falls), but if one is present
## it must be structurally complete too, so a capture path that starts
## writing a landing cannot silently write a hollow one.
func _is_live_motion(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY:
		return false
	var motion := value as Dictionary
	var phase: Dictionary = motion.get("phase", {})
	var kind: Variant = phase.get("kind")
	if kind != "controlled" and kind != "airborne":
		return false
	if not _is_complete_support(motion.get("support")):
		return false
	var landing: Variant = motion.get("last_landing")
	if landing == null:
		return true
	if typeof(landing) != TYPE_DICTIONARY:
		return false
	return _is_complete_support((landing as Dictionary).get("support"))


## A support dict is structurally complete when it carries a point and a
## normal, each three components — the shape `support_dict` in the Rust
## writer always produces for a real contact, and the one a dormant default
## or a broken writer cannot fake by accident.
func _is_complete_support(value: Variant) -> bool:
	if typeof(value) != TYPE_DICTIONARY:
		return false
	var support := value as Dictionary
	return _is_three_component(support.get("point")) and _is_three_component(support.get("normal"))


func _is_three_component(value: Variant) -> bool:
	return typeof(value) == TYPE_ARRAY and (value as Array).size() == 3


func _print_hash_and_quit() -> void:
	var snap: Dictionary = _main.observer.snapshot(_main.now)
	if snap.has("unavailable"):
		push_error("restore probe: snapshot refused: %s" % snap["unavailable"])
		quit(2)
		return
	print("RESTORE_HASH=%s" % JSON.stringify(canonicalize(snap), "", true, true).md5_text())
	quit(0)


## A JSON-safe deep copy of `value`: every vector-valued Variant becomes a
## bare-float Array, so nothing downstream can round-trip it through a
## pretty-printer. Total over any Variant the snapshot can hand back —
## floats, ints, bools and Strings pass through unchanged, which is exactly
## what the old direct-stringify path already did right for those.
##
## Copied verbatim from `determinism_probe.gd`, which owns this law: the two
## probes hash the same snapshot the same way, so a hash from either means
## the same thing. Change one and change the other.
# canonicalize-mirror: BEGIN — kept byte-identical with determinism_probe.gd's
# copy of the same two functions; test/repo_hygiene.sh diffs the two marked
# blocks and fails loudly if they drift. Change one, change the other.
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
# canonicalize-mirror: END
