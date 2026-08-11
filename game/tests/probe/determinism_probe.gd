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
	# sorted keys + FULL float precision: a hash over rounded floats would
	# wave through exactly the drift this probe exists to catch
	print("DETERMINISM_HASH=%s" % JSON.stringify(snap, "", true, true).md5_text())
	quit(0)
