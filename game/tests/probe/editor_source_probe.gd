extends SceneTree
## Editor-mode law for sound sources and the cat: placed in the editor
## they BUILD their blueprint limbs with no injection; placed at run
## time uninjected they build NOTHING (the runtime guard still holds).
## Runs twice from tools/probe_editor_sources.sh: once with -e, once
## without. Each run proves its mode before judging.

const READY_FRAMES := 30
## One hand-derived f32 ULP at the cat's own scale — the flat collider
## datum law (`COLLIDER_CENTER_Y = COL_HEIGHT * 0.5`) must hold this tight,
## the same tolerance `cat_elevation_test.gd` and `player_elevation_test.gd`
## already hold their own capsule datum checks to.
const F32_ULP := 2.384185791015625e-7

var _fan: Node3D = null
var _radio: Node3D = null
var _cat: CharacterBody3D = null
var _cat_born_at := Vector3.ZERO
## The physics clock reading right after the cat is born — the mark the
## settle wait polls against, so "several physics ticks elapsed" is a
## measured fact rather than a guessed frame count.
var _cat_physics_mark := 0
var _frames := 0
var _checks := 0
var _failed := 0


func _initialize() -> void:
	if (
		not ClassDB.class_exists("SoundFan")
		or not ClassDB.class_exists("SoundRadio")
		or not ClassDB.class_exists("WaveCat")
	):
		_check("the Rust extension is loaded (see .godot/extension_list.cfg)", false)
		_report()
		return
	_fan = ClassDB.instantiate("SoundFan") as Node3D
	root.add_child(_fan)
	_radio = ClassDB.instantiate("SoundRadio") as Node3D
	root.add_child(_radio)
	_cat = ClassDB.instantiate("WaveCat") as CharacterBody3D
	root.add_child(_cat)
	_cat_born_at = _cat.position
	_cat_physics_mark = Engine.get_physics_frames()


func _process(_delta: float) -> bool:
	_frames += 1
	var editor := Engine.is_editor_hint()
	var still_building := (
		editor
		and (
			(_fan != null and _fan.get_child_count() == 0)
			or (_radio != null and _radio.get_child_count() == 0)
			or (_cat != null and _cat.get_child_count() == 0)
		)
	)
	# Condition-based, not a guessed duration: keep polling the REAL physics
	# clock until it has actually advanced since the cat was born, so a cat
	# that still ticks in the editor gets the CHANCE to move before its
	# stillness checks are judged — counting idle _process() iterations as a
	# proxy would prove nothing about physics frames actually elapsing.
	# READY_FRAMES bounds the whole wait (build + settle), so a stuck build
	# or a stalled physics loop fails loudly instead of spinning forever.
	var cat_unsettled := (
		editor and _cat != null and Engine.get_physics_frames() - _cat_physics_mark < 2
	)
	if (still_building or cat_unsettled) and _frames < READY_FRAMES:
		return false
	if _fan == null or _radio == null or _cat == null:
		return true
	_judge()
	_report()
	return true


func _judge() -> void:
	var editor := Engine.is_editor_hint()
	print("# sources: mode=%s" % ("editor" if editor else "run"))
	_judge_fan(editor)
	_judge_radio(editor)
	_judge_cat(editor)


func _judge_fan(editor: bool) -> void:
	var pedestal := _fan.get_node_or_null("FanPedestal")
	var pivot := _fan.get_node_or_null("FanPivot")
	if editor:
		_check("editor: the fan builds its pedestal", pedestal != null)
		_check("editor: the fan builds its pivot head", pivot != null)
	else:
		_check("run uninjected: the fan builds nothing", _fan.get_child_count() == 0)


func _judge_radio(editor: bool) -> void:
	var case := _radio.get_node_or_null("RadioCase")
	var grille := _radio.get_node_or_null("RadioGrille")
	var antenna := _radio.get_node_or_null("RadioAntenna")
	if editor:
		_check("editor: the radio builds its case", case != null)
		_check("editor: the radio builds its grille", grille != null)
		_check("editor: the radio builds its antenna", antenna != null)
	else:
		_check("run uninjected: the radio builds nothing", _radio.get_child_count() == 0)


func _judge_cat(editor: bool) -> void:
	var collider := _cat.get_node_or_null("CatCollider") as CollisionShape3D
	var skin := _cat.get_node_or_null("CatSkin") as MeshInstance3D
	if editor:
		_check("editor: the cat builds its collider", collider != null)
		_check("editor: the cat builds its skin", skin != null)
		if skin != null:
			_check("editor: the cat skin has a mesh surface", skin.mesh.get_surface_count() >= 1)
			_check("editor: the cat skin rides the node, not the world", not skin.top_level)
		_check("editor: the cat does not tick", not _cat.is_physics_processing())
		# Defense in depth, not the primary guard: physics_process()
		# (cat.rs) bails immediately whenever brain/gait/tail are None,
		# which is exactly what build_editor_pose() leaves them as — so
		# today this check would pass even with processing left on. It
		# catches the day a future change BOTH previews something (e.g.
		# mood) by populating those fields in the editor build AND leaves
		# processing enabled; "does not tick" above is what actually holds
		# the line today.
		_check("editor: the cat has not moved", _cat.position.is_equal_approx(_cat_born_at))

		var capsule := collider.shape as CapsuleShape3D if collider != null else null
		if capsule != null:
			var datum := _cat.position.y + collider.position.y - capsule.height * 0.5
			_check(
				"editor: the cat collider bottom meets the flat datum within one f32 ULP",
				absf(datum) <= F32_ULP
			)
		else:
			_check("editor: the cat collider bottom meets the flat datum within one f32 ULP", false)

		# Typed as WaveCat (not the `_cat: CharacterBody3D` field) so the
		# direct calls below resolve statically: gdext registers
		# `get_configuration_warnings`/`motion_config_snapshot` on WaveCat
		# itself, not on the base engine class GDScript's static checker
		# sees through the field.
		var cat := _cat as WaveCat
		# `ready()` returns before ever validating/installing the active
		# motion config in editor mode (a blueprint cat never physics-ticks,
		# so it never needs one) — the field's own `#[init(val=...)]` is the
		# ONLY thing that can hold this line in the editor. Checked before
		# any `.set(...)` below, which stages a fresh active config as a
		# side effect and would otherwise mask a wrong `#[init]` default.
		var snapshot: PackedFloat64Array = cat.call("motion_config_snapshot")
		_check(
			(
				"editor: the cat's motion config snapshot is CAT_DEFAULT even though ready() never"
				+ " validates one in editor mode"
			),
			snapshot == PackedFloat64Array([9.8, 20.0, 1.5, 4.0, 0.60, 2.5])
		)

		# An out-of-order threshold pair must reach BOTH the virtual warning
		# read the Inspector's triangle uses and the registered callable
		# forwarder tests reach through `.call(...)` — the same text, same
		# channel contract every warning-bearing node in this codebase keeps.
		cat.set("landing_silent_speed", 8.0)
		cat.set("landing_full_speed", 7.0)
		var expected_warning := "landing full speed 7 m/s must be greater than silent speed 8 m/s"
		var virtual_warnings := cat.get_configuration_warnings()
		_check(
			"editor: an invalid threshold pair reaches the virtual warning channel",
			virtual_warnings.size() == 1 and virtual_warnings[0] == expected_warning
		)
		var callable_warnings: PackedStringArray = cat.call("get_configuration_warnings")
		_check(
			"editor: the same warning reaches the registered callable forwarder",
			callable_warnings.size() == 1 and callable_warnings[0] == expected_warning
		)
		cat.set("landing_full_speed", 9.0)
		var cleared_callable: PackedStringArray = cat.call("get_configuration_warnings")
		_check(
			"editor: a complementary valid edit clears both warning channels",
			cat.get_configuration_warnings().is_empty() and cleared_callable.is_empty()
		)
	else:
		_check("run uninjected: the cat builds nothing", _cat.get_child_count() == 0)


func _check(what: String, ok: bool) -> void:
	_checks += 1
	if not ok:
		_failed += 1
	print(("ok %d - %s" if ok else "not ok %d - %s") % [_checks, what])


func _report() -> void:
	print("1..%d" % _checks)
	if _failed > 0:
		print("probe: FAIL (%d of %d)" % [_failed, _checks])
	else:
		print("probe: PASS (%d checks)" % _checks)
	quit(1 if _failed > 0 else 0)
