# Editor Authoring SP1 — Place It and See It — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A designer places a sound source, a cat, or a solid in the Godot editor and sees it immediately; a broken arrangement shows a yellow warning on the offending node while dragging; knobs have ranges and units; one bootstrap command makes a fresh clone editable.

**Architecture:** `SoundFan`/`SoundRadio`/`WaveCat` become `#[class(tool)]` and build their limb geometry skinless in the editor (blueprint mode) while wave wiring stays behind the runtime injection gate. `WaveLevel` stops early-returning under `is_editor_hint`, runs `derive()` as a fault-collecting planner, and surfaces faults through `get_configuration_warnings()` — per-node for placement faults, level-wide for spawn/tap/budget faults — re-deriving when the scene it reads actually changes. Everything is Rust except probe/test GDScript; no `EditorPlugin` is needed in this sub-project.

**Tech Stack:** Rust (gdext 0.5.4, pinned `=0.5.4`, features `api-4-7`, `experimental-wasm`, `lazy-function-tables`), Godot 4.7.1.stable.official, gdUnit4 (vendored), headless editor probes (`godot --headless -e -s`), POSIX sh.

## Global Constraints

Copied from CLAUDE.md and the campaign spec (`docs/superpowers/specs/2026-08-11-editor-authoring-campaign-design.md`). Every task's requirements implicitly include this section.

- **Perception laws:** black & white, thin outlines only; no textures, fills, materials beyond the wave shaders; the world is revealed by waves. One outline per object, every seam between two objects draws: any new object that can touch another needs an object id at least **0.08** clear of it; ids come from the graph colouring in `rust/src/oid_palette.rs` — **never assign ids by cycling a list**.
- **Platforms:** one Godot project (`game/`) exported to Windows, macOS, web. Architecture-independent: x86_64 + arm64 + wasm32; no arch-specific code.
- **Two layers, tightened to the designer-facing razor (campaign spec):** Godot holds only what a designer needs (scenes, knobs, thin probes/tests); everything else is Rust in the single crate. No new GDScript except tests/probes.
- **No unsafe Rust** (`#![deny(unsafe_code)]`; the one `ffi.rs` exception stands; never add another).
- **Strict TDD:** write the failing test, watch it fail for the right reason, minimal code, watch it pass, refactor. Production code written before its test gets deleted. No mirror assertions (never compute the expectation with the code under test); no change detectors (test behaviour, not constants). Run the mutation check before finishing each task: flip a constant/branch/early-return you added — at least one test must fail.
- **Formatters/analyzers before every commit:** `cargo fmt` + `cargo clippy` (warnings are errors) + `cargo test` in `rust/`; `gdformat` + `gdlint` on changed `.gd` files (CI lints every `.gd` under `game/` except `game/addons/` and `game/.godot/`).
- **Commits:** small, self-contained, green (test + code land together). Narrative, evocative subject line matching the existing history, body carrying the precise technical what/why. **All work is authored on behalf of the user, never the assistant. No Co-Authored-By, no "Generated with", no mention of Claude, AI, or any assistant anywhere in the repository.** Repo-local identity `Dmitrii Galchenko <dggrus@gmail.com>` is already configured. A commit step below names *what the commit is about*; write the subject yourself in the house style.
- **Gate honesty:** always run `"$GODOT" --headless --path game --import >/dev/null 2>&1 || true` before any gdUnit run in a fresh worktree (no `game/.godot` = zero tests run with exit 0). Trust suite/case counts, not exit codes. Baseline at branch start: **275 cargo tests, 221 gdUnit cases / 26 suites** — counts only go up.
- **Boot-gate contract:** any new `godot_error!`/`godot_warn!` whose message opens class-style (`"SomeClass: "` or `"SomeClass '"`) must be a **literal** string opening in `rust/src` (synthesised `format!("{cls}: …")` prefixes are banned) and its `ERROR: SomeClass` alternative must exist in `ci/boot_error_pattern.sh:38`, or `test/ci_boot_error_gate.sh` (pipeline stage 2) fails. This plan adds **no new class-style openings**; if you find yourself adding one, update the pattern in the same commit.
- **Never call `set_owner` on engine-built limbs** — ownerless limbs are the entire never-serialize mechanism. Ghost limbs are `.free()`d immediately, never `queue_free()`.
- **Scope guard (campaign spec):** touching sound sources melting into one silhouette (#16) is sub-project 2's palette surgery — do not half-fix it here. No designer-maintained intermediate artifacts of any kind.
- **Godot binary:** the pinned version check is a prefix match against `.godot-version` (`4.7.1.stable.official`); pass `GODOT` down to probe scripts explicitly (`GODOT="$GODOT" tools/<probe>.sh`).
- **Full pipeline green before claiming a task done:** `SKIP_EXPORT=1 ci/pipeline.sh` exits 0 **and** prints the expected suite/case counts and probe verdicts.

## File Structure

- `rust/src/nodes/fan.rs`, `radio.rs`, `cat.rs` — become tool classes; named limbs; editor build path (Tasks 1–3).
- `rust/src/nodes/source.rs` — `SourceRig::clear()` (Task 1).
- `rust/src/level_plan.rs` — structured placement faults `PlacementFault { path, text }` (Task 4).
- `rust/src/oid_palette.rs` — starved slot indices exposed (Task 4).
- `rust/src/nodes/level.rs` — fault store, editor derive, `rederive()`, warnings, change-watch (Tasks 5–7).
- `rust/src/nodes/wall.rs`, `props.rs` — per-solid `get_configuration_warnings` (Task 6), knob ranges (Task 8).
- `game/tests/probe/editor_source_probe.gd` + `tools/probe_editor_sources.sh` — editor-mode law for sources/cat (Tasks 1–3).
- `game/tests/probe/editor_level_probe.gd` + `tools/probe_editor_level.sh` — editor-mode law for derive/warnings (Tasks 5–7).
- `game/tests/knob_hint_test.gd` — property-hint law (Task 8).
- `game/icons/*.svg` + `game/unseeing.gdextension` `[icons]` — class identity (Task 9).
- `rust/Cargo.toml` + `CLAUDE.md` — `editor-docs` feature, #44 correction (Task 10).
- `tools/bootstrap.sh` + `game/tests/probe/engine_census_probe.gd` + `game/README.md` — install path (Task 11).
- `ci/pipeline.sh` — new probes wired in (Tasks 1, 5, 11), `cargo check --features editor-docs` (Task 10).
- `docs/superpowers/plans/2026-08-11-editor-authoring-wiki-debt.md` — deferred wiki write-back ledger (Task 12).

---

### Task 1: The editor-source probe, and SoundFan builds in the editor

**Files:**
- Create: `game/tests/probe/editor_source_probe.gd`
- Create: `tools/probe_editor_sources.sh`
- Modify: `rust/src/nodes/fan.rs` (class attr :58, ready :97-112, builders :170-277)
- Modify: `rust/src/nodes/source.rs` (add `SourceRig::clear`)
- Modify: `ci/pipeline.sh` (wire probe after :146)

**Interfaces:**
- Consumes: `clear_limbs(self, &LIMBS)` from `rust/src/nodes/solid.rs:78-95` (`pub(crate)`, generic over `WithBaseField`); `Engine::singleton().is_editor_hint()` (repo pattern `rust/src/nodes/level.rs:171`); `SourceRig::limb` already takes `skin: Option<&Gd<Material>>` and skips the override when `None` (`source.rs:163-165`).
- Produces: `const LIMBS: [&str; 2] = ["FanPedestal", "FanPivot"];` in `fan.rs`; `SourceRig::clear(&mut self)` in `source.rs`; probe files and pipeline wiring that Tasks 2–3 extend. Tasks 2/3 rely on the probe's `_judge_fan/_judge_radio/_judge_cat` per-class function shape and the runner's mode-proof line `# sources: mode=<editor|run>`.

**Background you must know (measured, do not rediscover):**
- Non-`tool` gdext classes run **no** virtual lifecycle in the editor; `#[class(tool)]` runs them all (default `EditorRunBehavior::ToolClassesOnly`, not overridden in `rust/src/ffi.rs:25-26`).
- The fan's `ready()` currently refuses uninjected with an exact error the tests pin (`fan_test.gd:114-119`: exact text, plus `get_child_count() == 0`). That runtime behaviour must not change.
- The fan's limb hierarchy is pinned by `fan_test.gd:86-87` (pulse origin = `(0, head_h(), 0) + beam*0.1`): do **not** restructure parents, only name the two top-level built children.
- Ghost limbs: Ctrl+D duplicates live children regardless of owner; names are the only handle (`solid.rs:44-49`).

- [ ] **Step 1: Write the probe (the failing test).** Create `game/tests/probe/editor_source_probe.gd`, copying the structure of `game/tests/probe/editor_slab_probe.gd` (SceneTree script, `_initialize` builds, `_process(_delta) -> bool` polls up to `READY_FRAMES := 30`, TAP output, `probe: PASS/FAIL` verdict, `quit(1 if _failed > 0 else 0)`):

```gdscript
## Editor-mode law for sound sources and the cat: placed in the editor
## they BUILD their blueprint limbs with no injection; placed at run
## time uninjected they build NOTHING (the runtime guard still holds).
## Runs twice from tools/probe_editor_sources.sh: once with -e, once
## without. Each run proves its mode before judging.
extends SceneTree

const READY_FRAMES := 30

var _fan: Node3D = null
var _frames := 0
var _checks := 0
var _failed := 0


func _initialize() -> void:
	if not ClassDB.class_exists("SoundFan"):
		_check("the Rust extension is loaded (see .godot/extension_list.cfg)", false)
		_report()
		return
	_fan = ClassDB.instantiate("SoundFan") as Node3D
	root.add_child(_fan)


func _process(_delta: float) -> bool:
	_frames += 1
	if _frames < READY_FRAMES and _fan != null and _fan.get_child_count() == 0:
		if Engine.is_editor_hint():
			return false
	if _fan == null:
		return true
	_judge()
	_report()
	return true


func _judge() -> void:
	var editor := Engine.is_editor_hint()
	print("# sources: mode=%s" % ("editor" if editor else "run"))
	_judge_fan(editor)


func _judge_fan(editor: bool) -> void:
	var pedestal := _fan.get_node_or_null("FanPedestal")
	var pivot := _fan.get_node_or_null("FanPivot")
	if editor:
		_check("editor: the fan builds its pedestal", pedestal != null)
		_check("editor: the fan builds its pivot head", pivot != null)
	else:
		_check("run uninjected: the fan builds nothing", _fan.get_child_count() == 0)


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
```

- [ ] **Step 2: Write the runner.** Create `tools/probe_editor_sources.sh` as a copy of `tools/probe_editor_slabs.sh` with `PROBE="res://tests/probe/editor_source_probe.gd"`, the mode-proof grep changed to `^# sources: mode=$want$`, and the final success line `echo "probe: sources OK — blueprint limbs in the editor, silence uninjected at run time"`. Keep everything else identical: godot discovery loop, the `--import || true` pre-step, `run_mode editor -e` then `run_mode run`, TAP filtering, exit codes (2 no binary / 1 failure / 0 pass). `chmod +x` it.

- [ ] **Step 3: Watch it fail for the right reason.** Run `tools/probe_editor_sources.sh`. Expected: the **editor** pass fails on "the fan builds its pedestal" (SoundFan is not `tool`, so nothing runs in the editor and `get_child_count() == 0` forever); the run pass would succeed. Confirm the failure lines mention the editor mode, not a missing extension.

- [ ] **Step 4: Add `SourceRig::clear`.** In `rust/src/nodes/source.rs`, next to `is_built` (:184-186):

```rust
/// Forget every limb handle. A rebuilding `ready()` frees the old
/// limbs by name first; the rig must not keep pointers into them.
pub(crate) fn clear(&mut self) {
    self.limbs.clear();
}
```

- [ ] **Step 5: Make SoundFan a tool class with an editor build.** In `rust/src/nodes/fan.rs`: change :58 to `#[class(tool, init, base=Node3D)]`. Add near the other constants:

```rust
/// The two built subtrees, named so a rebuilding ready() can free the
/// ghosts a Ctrl+D duplicate carries in (names are the only handle —
/// a duplicate reaches _ready as a fresh Rust object).
const LIMBS: [&str; 2] = ["FanPedestal", "FanPivot"];
```

Restructure `ready()` (:97-112) to:

```rust
fn ready(&mut self) {
    clear_limbs(self, &LIMBS);
    self.rig.clear();
    if Engine::singleton().is_editor_hint() {
        // blueprint mode: the same geometry the game outlines, skinless
        // (SourceRig::limb skips the override while data_mat is None).
        // Nothing ticks, emits, or registers here — advance() is only
        // ever called by the level at run time.
        self.build_pedestal();
        self.build_head();
        self.build_blades();
        return;
    }
    if self.pulses.is_none() || self.data_mat.is_none() {
        godot_error!("SoundFan: pulses/data_mat not injected — fan disabled");
        return;
    }
    let voice = SoundSource::voice(self);
    self.rig.tune(&voice);
    self.build_pedestal();
    self.build_head();
    self.build_blades();
}
```

(Keep the exact existing error string — `fan_test.gd:115` pins it. Keep the existing build order and the `voice`/`tune` lines exactly as they are today; only the clear/editor-branch lines are new.) Add imports: `use crate::nodes::solid::clear_limbs;` and `use godot::classes::Engine;` if absent. In `build_pedestal` (:170-198) name the body before adding: `body.set_name("FanPedestal");`. In `build_head` (:202-241) name the pivot: `pivot.set_name("FanPivot");`.

- [ ] **Step 6: Watch it pass.** `cd rust && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test` (275 passing), then `tools/probe_editor_sources.sh`. Expected: `probe: PASS` in both modes, final `probe: sources OK …` line.

- [ ] **Step 7: Run the runtime suites that pin the fan.** `"$GODOT" --headless --path game --import >/dev/null 2>&1 || true` then run gdUnit on `res://tests/fan_test.gd`, `res://tests/source_test.gd`, `res://tests/map_test.gd` (single-suite form: `-a res://tests/fan_test.gd`, etc.). Expected: all green — the uninjected error text, `child_count == 0`, hub geometry, and limb params are unchanged at run time.

- [ ] **Step 8: Wire the probe into CI.** In `ci/pipeline.sh`, immediately after the editor-mode slab probe invocation (:145-146) and before the `SKIP_EXPORT` early-exit (:148), add, following the exact same pattern:

```sh
echo "ci: editor-source probe (blueprint limbs vs the runtime guard)"
GODOT="$GODOT" "$DIR/tools/probe_editor_sources.sh" || { echo "ci: editor-source probe FAILED"; exit 1; }
```

- [ ] **Step 9: Mutation check.** Revert the class attr to non-tool (mentally or via `git stash`-free edit): the probe's editor pass must fail. Delete the `clear_limbs` call: duplicate a fan in a headless editor probe run… cheaper: trust the named-limb reasoning but flip the editor branch to skip `build_pedestal()` — the probe must fail on the pedestal check. Restore. Run `gdformat`/`gdlint` on the new probe file.

- [ ] **Step 10: Full pipeline, then commit.** `SKIP_EXPORT=1 ci/pipeline.sh` → exit 0, gdUnit still 221/26, both probes PASS. Commit everything from this task together; the subject should say what the fan now does in the editor (blueprint limbs without injection) in the house narrative style.

---

### Task 2: SoundRadio builds in the editor

**Files:**
- Modify: `rust/src/nodes/radio.rs` (class attr :63, ready :96-110, build_case :144-164, build_fascia :169-219)
- Modify: `game/tests/probe/editor_source_probe.gd` (add radio)

**Interfaces:**
- Consumes: `clear_limbs`, `SourceRig::clear`, probe scaffold from Task 1.
- Produces: `const LIMBS: [&str; 6] = ["RadioCase", "RadioGrille", "RadioTuner", "RadioDialA", "RadioDialB", "RadioAntenna"];` in `radio.rs`.

**Background:** `radio_test.gd:101-115` counts **exactly 1 `StaticBody3D`** among the radio's direct children (the case) with exactly 1 `CollisionShape3D` under it; the five fascia limbs are direct children of the radio node itself (`radio.rs:171`). Do not change that shape — only name the six built children. `radio_test.gd:120-127` pins the uninjected error text and `child_count == 0`.

- [ ] **Step 1: Extend the probe (failing first).** In `editor_source_probe.gd`: instantiate `SoundRadio` alongside the fan in `_initialize`, poll it too, and add:

```gdscript
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
```

Call it from `_judge`; extend the polling condition so the probe waits for both nodes to grow children in editor mode. Run `tools/probe_editor_sources.sh` — expected: editor pass fails on "the radio builds its case".

- [ ] **Step 2: Make SoundRadio a tool class.** Mirror Task 1 exactly: `#[class(tool, init, base=Node3D)]` at :63; the `LIMBS` const above; `ready()` restructured with `clear_limbs(self, &LIMBS); self.rig.clear();`, an editor branch calling `self.build_case(); self.build_fascia();`, and the untouched runtime guard + exact error string. Name the built children: in `build_case` the body → `"RadioCase"`; in `build_fascia` the grille torus → `"RadioGrille"`, the tuning-scale box → `"RadioTuner"`, the two dials → `"RadioDialA"`/`"RadioDialB"`, the antenna → `"RadioAntenna"`.

- [ ] **Step 3: Verify.** `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`; `tools/probe_editor_sources.sh` → PASS both modes; gdUnit single suites `radio_test.gd`, `source_test.gd`, `map_test.gd` green after `--import`.

- [ ] **Step 4: Mutation check, then commit.** Flip the editor branch to skip `build_case()` — probe must fail; restore. `gdformat`/`gdlint` the probe. `SKIP_EXPORT=1 ci/pipeline.sh` green. Commit (subject: the radio's blueprint presence in the editor).

---

### Task 3: WaveCat stands still in the editor

**Files:**
- Modify: `rust/src/nodes/cat.rs` (class attr :52, ready :98-150, mesh build :346-382)
- Modify: `game/tests/probe/editor_source_probe.gd` (add cat)

**Interfaces:**
- Consumes: `clear_limbs`, probe scaffold.
- Produces: `const LIMBS: [&str; 2] = ["CatCollider", "CatSkin"];` in `cat.rs`.

**Background (all measured):** a `tool` node really runs `process`/`physics_process` in the editor; `cat.rs:174` runs `move_and_slide()` and `:169-171` writes the node's world yaw — an editor-ticking cat wanders the viewport and Ctrl+S saves the drift. The runtime mesh is built in **world space** with `set_as_top_level(true)` (`cat.rs:125`) — if the editor copied that, dragging the cat node would leave its silhouette behind. `cat_test.gd:34-39` pins the uninjected runtime error, `is_physics_processing() == false`, and 0 children.

- [ ] **Step 1: Extend the probe (failing first).** Instantiate `WaveCat` in `_initialize` (as `CharacterBody3D`), and add:

```gdscript
func _judge_cat(editor: bool) -> void:
	var collider := _cat.get_node_or_null("CatCollider")
	var skin := _cat.get_node_or_null("CatSkin") as MeshInstance3D
	if editor:
		_check("editor: the cat builds its collider", collider != null)
		_check("editor: the cat builds its skin", skin != null)
		if skin != null:
			_check("editor: the cat skin has a mesh surface", skin.mesh.get_surface_count() >= 1)
			_check("editor: the cat skin rides the node, not the world", not skin.top_level)
		_check("editor: the cat does not tick", not _cat.is_physics_processing())
		_check("editor: the cat has not moved", _cat.position.is_equal_approx(_cat_born_at))
	else:
		_check("run uninjected: the cat builds nothing", _cat.get_child_count() == 0)
```

Record `_cat_born_at := _cat.position` right after adding it, and make the editor pass wait extra frames (raise the poll budget for the cat branch to let several physics frames elapse) so "has not moved" means something. Run the probe — expected: editor pass fails on "the cat builds its collider".

- [ ] **Step 2: Make WaveCat a tool class with a frozen editor pose.** `#[class(tool, init, base=CharacterBody3D)]` at :52; add the `LIMBS` const. Restructure `ready()`:

```rust
fn ready(&mut self) {
    clear_limbs(self, &LIMBS);
    if Engine::singleton().is_editor_hint() {
        // blueprint mode: one standing pose, frozen. The mesh is built
        // in LOCAL space (pose seeded at the origin) so the silhouette
        // rides the node when the designer drags it; the runtime mesh
        // stays world-space + top_level as before. No brain, no clock:
        // an editor-ticking cat would walk the viewport and Ctrl+S
        // would save its drift into the scene.
        self.base_mut().set_physics_process(false);
        self.base_mut().set_process(false);
        self.build_editor_pose();
        return;
    }
    // ... existing uninjected guard and runtime build, unchanged ...
}
```

`build_editor_pose()` reuses the existing pieces: build the capsule collider exactly as :108-114 but `collider.set_name("CatCollider")`; build the `MeshInstance3D` as :116-125 but named `"CatSkin"`, **without** `set_material_override` (data_mat is None), **without** `set_as_top_level(true)` and without the cull margin; then seed a throwaway `CatBrain::new(self.seed as u64, RoamRect::around(Vector3::ZERO, self.roam_size.x, self.roam_size.y), 0.0)` + gait/tail/pose exactly as :134-147 but around `Vector3::ZERO` with yaw `0.0`, and invoke the same mesh-writing routine `process()` uses (:346-382) once so the `ImmediateMesh` holds one standing pose. Name the runtime-built collider and skin with the same two names (`"CatCollider"`, `"CatSkin"`) so `clear_limbs` covers both paths.

- [ ] **Step 3: Verify.** `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`; probe PASS both modes; gdUnit `cat_test.gd`, `map_test.gd`, `observer_test.gd` green after `--import` (the runtime path — names aside — is untouched, and limbs are never serialized, so the shipped scene is unaffected).

- [ ] **Step 4: Mutation check, then commit.** Remove the `set_physics_process(false)` line — the probe's "does not tick"/"has not moved" checks must fail; restore. Full pipeline green. Commit (subject: the cat holds a pose in the editor instead of living there).

---

### Task 4: Placement faults become data (pure module)

**Files:**
- Modify: `rust/src/level_plan.rs` (`unfloored` :545-576, `sunken` :599-628, their tests)
- Modify: `rust/src/oid_palette.rs` (the `assign` result)
- Modify: `rust/src/nodes/level.rs` (call sites :522-538, :623-630 — mechanical adaptation only)

**Interfaces:**
- Consumes: existing `PlacedSolid { path, area }` (`level_plan.rs:480-486`), existing complaint sentences (do not reword them — the boot gate and existing tests pin their `WaveLevel: ` openings and phrasing).
- Produces: `pub struct PlacementFault { pub path: String, pub text: String }` in `level_plan.rs`; `unfloored`/`sunken` return `Vec<PlacementFault>` (text = the exact sentence they emit today, path = the solid's path already embedded in it); `oid_palette::assign`'s result gains `pub starved_slots: Vec<usize>` (indices into the input `areas`), with the existing `starved` count kept equal to `starved_slots.len()`. Task 5 consumes both.

- [ ] **Step 1: Failing cargo tests first.** In `level_plan.rs` tests, take an existing `unfloored` fixture (e.g. the half-sunk-crate case) and assert the new shape: the returned fault's `path` equals the planted solid's path and `text` still contains the current sentence. It fails to compile (the type doesn't exist) — that is the right failure for a signature change. Same for `sunken`. In `oid_palette.rs` tests: build the existing starvation fixture and assert `starved_slots` names exactly the starving slot indices and `starved == starved_slots.len()`.

- [ ] **Step 2: Implement.** Introduce `PlacementFault`, convert the two functions (each already knows the path when composing its sentence — return it alongside instead of discarding it). In `oid_palette.rs`, record which area indices could not be coloured (wherever `starved` is incremented today, push the index too).

- [ ] **Step 3: Adapt call sites mechanically.** `level.rs:522-538`: iterate faults, print `fault.text` exactly where the sentences printed before; counts stay `strays.len()`/`sunk.len()`. `level.rs:623-630`: keep the count-based message unchanged (it is pinned by the boot gate); the slot indices are stored for Task 5, unused yet is fine — but if clippy flags dead code, expose them now as a `pub(super)` accessor on the level instead of suppressing the lint.

- [ ] **Step 4: Verify + mutation + commit.** `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test` — all green, count grows (new assertions). Mutation: make `unfloored` return an empty path — the new test must fail. gdUnit `level_test.gd` + `map_test.gd` green (sentences unchanged). Commit (subject: placement faults carry their node's address).

---

### Task 5: WaveLevel derives in the editor and wears the level-wide faults

**Files:**
- Modify: `rust/src/nodes/level.rs` (ready :169-182, derive :486-498, fault relays :444-446, :477-479, :535-537, :625-630, :760-768)
- Create: `game/tests/probe/editor_level_probe.gd`
- Create: `tools/probe_editor_level.sh`
- Modify: `ci/pipeline.sh` (wire after the sources probe)

**Interfaces:**
- Consumes: `PlacementFault`, `starved_slots` (Task 4); tool sources (Tasks 1–3) so the editor census sees them (`try_dynify` works on tool-class instances, and their limbs exist for `mesh_world_box` anchoring).
- Produces, in `level.rs`:
  - fields `level_faults: Vec<String>` and `node_faults: Vec<level_plan::PlacementFault>`, rewritten on **every** derive (the totals-rewrite law of :522-534 extends to them);
  - `#[func] fn rederive(&mut self)` — public manual refresh, calls `derive()`;
  - `pub(super) fn faults_for(&self, node: &Gd<Node>) -> PackedStringArray` — the texts of `node_faults` whose `path == root.get_path_to(node)` (Task 6 consumes);
  - `get_configuration_warnings()` override in `INode3D for WaveLevel` returning `level_faults` as a `PackedStringArray`.

**Design rules (from the measured facts):**
- The editor early-return at :171-173 is **deleted**; `ready()` becomes: `build_slabs()`; then the injection-gate error (:177-179) only when **not** editor (an editor level is legitimately uninjected — printing that error on every scene open is noise); then `derive()`.
- `derive()` gets one local `let editor = Engine::singleton().is_editor_hint();` and every fault site does both jobs: **always** push into the fault vecs (cleared at derive start), and print via `godot_error!`/`godot_warn!` **only when `!editor`** — the boot gate reads runtime prints and must keep seeing every one of them, with unchanged text.
- Fault routing: spawn complaints (:444-446), tap complaint (:477-479), wall budget + pack-range budgets (via `say`, :760-768 — pass `editor` in or store-then-print at the call sites) → `level_faults`. Placement faults (:535-537) → `node_faults` (their `text` also mentions the path, which is fine). Oid starvation: keep the level-wide count message in `level_faults` **and** push one `PlacementFault { path, text: "cannot take an object id distinct from everything it touches — its seams will not draw" }` per starved slot, using `starved_slots` mapped through the same scene-order solid list that built `areas` (:614-621).
- Painting instance shader params during an editor derive is safe (limbs and slab skins are ownerless and never serialize) — leave `assign_oids`'s painting and `push_wall_table`'s no-op material pushes exactly as they are.
- After every derive: `self.base_mut().update_configuration_warnings();`.
- `WaveWall::segment` and every `mesh_world_box` read global transforms — they require the tree; `derive()` from `ready()` is in-tree already, and `rederive()` documents in-tree as its contract.

- [ ] **Step 1: Write the probe (failing first).** `game/tests/probe/editor_level_probe.gd` + `tools/probe_editor_level.sh` (copy the Task-1 shapes; mode line `# level: mode=`; runner success line about the level deriving at edit time). In `_initialize`, build **without injection**: a `WaveLevel` (`extents` `Vector2(20, 20)`), one `WaveWall`, deliberately **no** SpawnPoint. Editor-mode checks:

```gdscript
func _judge(editor: bool) -> void:
	print("# level: mode=%s" % ("editor" if editor else "run"))
	var warnings := _level.get_configuration_warnings()
	if editor:
		_check("editor: the level derives and complains about the missing spawn", _has(warnings, "SpawnPoint"))
		_check("editor: wall segments were derived at edit time", (_level.call("wall_segments") as PackedVector4Array).size() == 1)
		var fixed := Marker3D.new()
		fixed.name = "SpawnPoint"
		_level.add_child(fixed)
		_level.call("rederive")
		_check("editor: giving it a spawn clears the warning", not _has(_level.get_configuration_warnings(), "SpawnPoint"))
	else:
		_check(
			"run: an uninjected level still derives honest geometry",
			(_level.call("wall_segments") as PackedVector4Array).size() == 1
		)
```

(`_has` = helper looping the `PackedStringArray` for a substring.) Run `tools/probe_editor_level.sh` — expected editor failure: warnings empty and `wall_segments` empty, because today `ready()` returns before `derive()` under the editor hint.

- [ ] **Step 2: Failing cargo tests for the pure edges.** None needed beyond Task 4's — the fault-vec plumbing is node-bound. The probe plus existing suites are the tests here.

- [ ] **Step 3: Implement** per the design rules above. Keep every printed sentence byte-identical at run time. Add `use godot::prelude::PackedStringArray;` if absent.

- [ ] **Step 4: Verify.** `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`; `tools/probe_editor_level.sh` PASS both modes; `tools/probe_editor_slabs.sh` still PASS (slab law untouched); after `--import`, run the full gdUnit suite (`-a tests`) — 221+/26 green, in particular `level_test.gd` (runtime complaints still print — its planted-fault tests pin them) and `boot`-gate self-test via full pipeline.

- [ ] **Step 5: Mutation check, then commit.** Mutations: (a) skip clearing the fault vecs at derive start — the "clears the warning" check must fail; (b) drop the `update_configuration_warnings` call — the warning checks read stale/never-set state and fail. Restore. `SKIP_EXPORT=1 ci/pipeline.sh` green end to end. Commit (subject: the level derives while the designer watches, and says what is wrong on its own node).

---

### Task 6: The fault lands on the node that caused it

**Files:**
- Modify: `rust/src/nodes/wall.rs`, `rust/src/nodes/props.rs` (all four solid classes)
- Modify: `rust/src/nodes/level.rs` (push refresh to censused nodes after derive)
- Modify: `game/tests/probe/editor_level_probe.gd`

**Interfaces:**
- Consumes: `faults_for(&self, node: &Gd<Node>) -> PackedStringArray` (Task 5).
- Produces: `get_configuration_warnings()` on `WaveWall`, `WaveProp`, `WaveColumn`, `WaveWedge`; a private `warnings_from_level(node: &Gd<Node>) -> PackedStringArray` helper in `solid.rs` both files call.

- [ ] **Step 1: Extend the probe (failing first).** In the editor branch of `editor_level_probe.gd`, after the spawn fix: add a `WaveProp` crate at `Vector3(3, 0, 3)` (a half-sunk drop — the floor top is y=0 and props are centred, the classic designer act), `rederive`, and assert:

```gdscript
var crate_warnings := _crate.get_configuration_warnings()
_check("editor: the half-sunk crate wears its own warning", _has_any(crate_warnings, "sunk"))
_crate.position.y = 0.35
_level.call("rederive")
_check("editor: lifting the crate clears it", _crate.get_configuration_warnings().is_empty())
```

Run — expected failure: solids have no `get_configuration_warnings` override, so the array is empty.

- [ ] **Step 2: Implement.** In `solid.rs`:

```rust
/// The warnings a solid wears are whatever its owning level derived
/// for it. A solid outside any WaveLevel (a prefab edited on its own)
/// wears none — that is a legal authoring context, not a fault.
pub(crate) fn warnings_from_level(node: &Gd<Node>) -> PackedStringArray {
    let mut cursor = node.get_parent();
    while let Some(parent) = cursor {
        if let Ok(level) = parent.clone().try_cast::<WaveLevel>() {
            return level.bind().faults_for(node);
        }
        cursor = parent.get_parent();
    }
    PackedStringArray::new()
}
```

(Import `WaveLevel`; if a module cycle bites — `solid.rs` is a sibling of `level.rs` under `nodes/` so it should not — put the helper in `level.rs` as a free `pub(super) fn` instead; keep ONE copy.) Then in each of the four solid interface impls:

```rust
fn get_configuration_warnings(&self) -> PackedStringArray {
    warnings_from_level(&self.base().clone().upcast::<Node>())
}
```

In `level.rs`, after `update_configuration_warnings()` on itself, refresh every censused solid so cleared faults disappear and new ones show:

```rust
for solid in &census.solids {
    solid.clone().into_gd().update_configuration_warnings();
}
```

(`into_gd()` on the `DynGd` yields the `Gd<Node>`; `update_configuration_warnings` needs `&mut` — clone the handle.)

- [ ] **Step 3: Verify + mutation + commit.** Cargo gates green; both level probes PASS; full gdUnit green. Mutation: make `faults_for` match on node **name** instead of path — build two same-named crates under different parents in the probe... (skip building that fixture; instead flip `faults_for` to return everything unconditionally — the "lifting the crate clears it" check must fail). Full pipeline; commit (subject: the yellow triangle lands on the node that earned it).

---

### Task 7: The level notices the scene changing under it

**Files:**
- Modify: `rust/src/nodes/level.rs`
- Modify: `game/tests/probe/editor_level_probe.gd`

**Interfaces:**
- Consumes: `rederive()` (Task 5) stays for tools/tests; everything else is internal.
- Produces: automatic re-derive in the editor — no API anyone else consumes.

**Design (condition-watching, not event-plumbing):** rather than wiring dirty-flags through six classes' setters and notifications, the level **watches the condition it derives from**: each editor process frame it folds a cheap signature over the censused scene — for every censused node, its path, its global transform, and (for solids) the local AABB of its skin mesh (which captures every knob) — and re-derives when the signature changes. One `u64` FNV-style fold, ~130 nodes, microseconds; the derive itself only runs on change. Runtime pays nothing: `set_process(true)` only under the editor hint, `set_process(false)` otherwise.

- [ ] **Step 1: Extend the probe (failing first).** Editor branch: after the crate checks, move the wall (`_wall.position.x += 2.0`), then advance frames by returning `false` from `_process` for a few more polls **without** calling `rederive`, and assert `wall_segments()` reflects the move and the signature-driven derive also refreshed warnings. Concretely: plant the crate half-sunk again by `_crate.position.y = 0.0`, poll until `_crate.get_configuration_warnings()` is non-empty or a 30-frame budget lapses, and check it. Run — expected failure: nothing re-derives without the manual call.

- [ ] **Step 2: Implement.** In `level.rs`: `ready()` editor branch ends with `self.base_mut().set_process(true);` (runtime branch: `set_process(false);`). Add:

```rust
fn process(&mut self, _delta: f64) {
    if !Engine::singleton().is_editor_hint() {
        return;
    }
    let sig = self.scene_signature();
    if sig != self.last_signature {
        self.last_signature = sig;
        self.derive();
    }
}
```

`scene_signature()` walks the same census the derive would (reuse `census()`; it is one subtree walk) and folds path bytes, the 12 floats of each global transform (to bits), and each solid's skin-mesh local AABB floats into a `u64` FNV-1a. Field `last_signature: u64` initialised from the first derive in `ready()`. Note `process` lives in the `INode3D` interface impl.

- [ ] **Step 3: Verify + mutation + commit.** All gates as before; the probe now proves drag-and-see without manual refresh. Mutation: return a constant from `scene_signature()` — the new probe checks fail. Watch cost: the probe is also the regression harness proving derive doesn't run every frame? (Assert indirectly: the signature short-circuit is the only path — flip `!=` to `==` and the "clears/reappears" checks fail.) Full pipeline; commit (subject: the level watches the scene it derives from).

---

### Task 8: Knobs get ranges and metres

**Files:**
- Modify: `rust/src/nodes/wall.rs` (:42-45), `props.rs` (:64, :153-158, :323), `cat.rs` (:64-71), `level.rs` (extents :135-138)
- Create: `game/tests/knob_hint_test.gd`

**Interfaces:**
- Consumes: gdext 0.5.4 range syntax — positional `min, max`, optional literal `step`, then flags/`suffix = "…"`, all inside `#[export(range = (…))]` (parser: `godot-macros-0.5.4` `field_export.rs` `new_range_list`; `suffix` is the only KV). The repo precedent for range-only is `fan.rs:73-89`.
- Produces: hint metadata on every designer shape knob; no API.

- [ ] **Step 1: Failing gdUnit test.** `game/tests/knob_hint_test.gd` (gdUnit suite, runtime — property hints are class metadata, no editor needed):

```gdscript
## Every knob a designer drags carries a range and a unit, so the
## Inspector shows a bounded slider in metres instead of a bare float.
## The break this catches: a knob whose hint is dropped regresses to a
## free-typing field silently.
class_name KnobHintTest
extends GdUnitTestSuite


func _hint_of(clazz: String, prop: String) -> Dictionary:
	for p: Dictionary in ClassDB.class_get_property_list(clazz):
		if p["name"] == prop:
			return p
	return {}


func test_wall_length_is_a_bounded_slider_in_metres() -> void:
	var p := _hint_of("WaveWall", "length")
	assert_int(p["hint"]).is_equal(PROPERTY_HINT_RANGE)
	assert_str(p["hint_string"]).contains("suffix")


func test_column_knobs_are_bounded() -> void:
	assert_int(_hint_of("WaveColumn", "radius")["hint"]).is_equal(PROPERTY_HINT_RANGE)
	assert_int(_hint_of("WaveColumn", "height")["hint"]).is_equal(PROPERTY_HINT_RANGE)
```

Add analogous cases for `WaveProp.size`, `WaveWedge.size`, `WaveLevel.extents`, `WaveCat.roam_size` **after** step 2 confirms vector knobs accept range hints — start with the scalar ones so the suite compiles-and-fails cleanly (hint today = 0). Run it (single-suite, after `--import`): red.

- [ ] **Step 2: Implement.** Scalars first — the combination to verify is `#[export(range = …)]` stacked with `#[var(get = …, set = …)]` (the solids' setter machinery must keep running — a range hint must never bypass `SignFold`):

```rust
#[export(range = (0.3, 30.0, 0.1, or_greater, suffix = " m"))]
#[var(get = get_length, set = set_length)]
#[init(val = 4.0)]
length: f64,
```

`cargo check` immediately. If the macro rejects the stack, the fallback is: keep `#[var(get, set)]` and register the range by overriding nothing else — report the incompatibility in the task summary and apply the range only where no custom setter exists (`cat.rs` seed/roam) — but expect it to compile; the two attributes configure disjoint halves of the property. Then vectors: try `#[export(range = (0.05, 20.0, 0.05, or_greater, suffix = " m"))]` on `WaveProp.size` (Godot applies range hints per component); if `cargo check` or the hint test shows vectors don't carry it in 0.5.4, leave vectors unhinted and record that in the wiki-debt note (Task 12) — the scalar knobs are the bulk of the value. Chosen ranges: wall length `(0.3, 30.0, 0.1, or_greater, " m")`; column radius `(0.05, 5.0, 0.05, or_greater, " m")`, height `(0.1, 10.0, 0.1, or_greater, " m")`; prop/wedge size `(0.05, 20.0, 0.05, or_greater, " m")`; extents `(4.0, 60.0, 1.0, or_greater, " m")`; cat roam_size `(1.0, 30.0, 0.5, " m")`; cat seed `(0, 999999)` (no suffix — it is not a length).

- [ ] **Step 3: Verify + mutation + commit.** Cargo gates; the knob test green; the **solids' setter tests** still green (`level_test.gd` sign-fold/scale cases — proof the hint didn't bypass the setters); full gdUnit. Mutation: remove one range attr — the hint test fails. Full pipeline; commit (subject: the knobs learn their bounds and their unit).

---

### Task 9: Every class gets a face

**Files:**
- Create: `game/icons/wave_level.svg`, `wave_wall.svg`, `wave_prop.svg`, `wave_column.svg`, `wave_wedge.svg`, `sound_fan.svg`, `sound_radio.svg`, `wave_cat.svg`
- Modify: `game/unseeing.gdextension` (add `[icons]`)
- Create: `game/tests/icon_manifest_test.gd`

**Interfaces:** none consumed/produced beyond files; the `[icons]` section is engine config (gdext does not parse it).

- [ ] **Step 1: Failing gdUnit test.** `game/tests/icon_manifest_test.gd`: parse `res://unseeing.gdextension` with `ConfigFile.load`, assert the `icons` section exists, lists **exactly** the eight designer-facing classes above, and every referenced `res://icons/*.svg` exists (`FileAccess.file_exists`). Run: red (no section).

- [ ] **Step 2: Draw the icons.** Eight 16×16 SVGs in the game's own language — thin white outlines on transparency, `stroke="#e0e0e0"`, `stroke-width="1.2"`, `fill="none"`. Templates (one path each; keep them this simple):

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16"><rect x="2" y="2" width="12" height="12" fill="none" stroke="#e0e0e0" stroke-width="1.2"/></svg>
```

wave_level = the rect above (the room); wave_wall = `<rect x="6.5" y="2" width="3" height="12" …/>`; wave_prop = `<rect x="4" y="4" width="8" height="8" …/>`; wave_column = `<circle cx="8" cy="8" r="5" …/>`; wave_wedge = `<path d="M2 13 L13 13 L13 3 Z" …/>`; sound_fan = `<circle cx="8" cy="8" r="6" …/><path d="M8 8 L8 2 M8 8 L13 11 M8 8 L3 11" …/>`; sound_radio = `<rect x="2" y="6" width="12" height="8" …/><path d="M11 6 L14 2" …/>`; wave_cat = `<path d="M3 13 L3 6 L5 3 L7 6 L9 6 L11 3 L13 6 L13 13 Z" …/>` (all sharing the same stroke attributes).

- [ ] **Step 3: Wire and mint.** Append to `game/unseeing.gdextension`:

```ini
[icons]

WaveLevel = "res://icons/wave_level.svg"
WaveWall = "res://icons/wave_wall.svg"
WaveProp = "res://icons/wave_prop.svg"
WaveColumn = "res://icons/wave_column.svg"
WaveWedge = "res://icons/wave_wedge.svg"
SoundFan = "res://icons/sound_fan.svg"
SoundRadio = "res://icons/sound_radio.svg"
WaveCat = "res://icons/wave_cat.svg"
```

Run `"$GODOT" --headless --path game --import` to mint the `.import` sidecars; commit them with the SVGs (repo policy; precedent `game/icon.svg` + `.import`). Keep everything out of `game/addons/` (the vendor fingerprint would break).

- [ ] **Step 4: Verify + commit.** The manifest test green; probes/suites unaffected but run the full pipeline anyway (repo hygiene checks tracked blob sizes — 8 tiny SVGs pass trivially). Note in the task summary: the Create Node dialog rendering is GUI-only and gets a human eyeball at the end of the sub-project. Commit (subject: the designer's palette gets faces).

---

### Task 10: In-editor docs, honestly gated

**Files:**
- Modify: `rust/Cargo.toml` (:13-16)
- Modify: `CLAUDE.md` (:403-405)
- Modify: `ci/pipeline.sh` (rust stage)
- Modify: `rust/src/nodes/*.rs` (missing `///` on designer knobs only)

**Interfaces:**
- Produces: cargo feature `editor-docs = ["godot/register-docs"]`, non-default. Task 11's bootstrap builds with it.

- [ ] **Step 1: The feature.** In `rust/Cargo.toml` add to the existing `[features]` table (which already has `nothreads`):

```toml
editor-docs = ["godot/register-docs"]
```

`cargo check --features editor-docs` must pass (register-docs is compile-gated `since_api = "4.3"`; the repo pins api-4-7 — fine). The default feature set is unchanged, so shipped wasm/desktop artifacts carry zero doc bytes (the docs are embedded plaintext in any binary built with the feature — that is exactly why it is non-default).

- [ ] **Step 2: Keep it alive in CI.** In `ci/pipeline.sh`'s rust stage (after the existing `cargo clippy`/`cargo test` lines, :50-96), add `cargo check --features editor-docs` with an explanatory echo — a feature nobody compiles rots.

- [ ] **Step 3: The docs themselves.** Sweep the eight designer-facing classes: every `#[export]` knob carries a `///` line a designer can act on (most already do — `props.rs:16-21`, `fan.rs:70-72`; add the missing ones, at minimum `cat.rs` seed/roam_size and `level.rs` extents). No test can reach the Inspector tooltip headlessly (recorded as unverified in the research); the compile gate is the test.

- [ ] **Step 4: Correct CLAUDE.md (#44).** At CLAUDE.md:405 the two-layers bullet claims `in-editor docs (register-docs)` as if enabled. Reword that clause to state the truth: in-editor docs exist behind the non-default `editor-docs` cargo feature (the designer bootstrap builds with it; shipped exports never do). Preserve the 4-space list indentation.

- [ ] **Step 5: Verify + commit.** `cargo fmt`, clippy, test, plus `cargo check --features editor-docs`; full pipeline green. Commit (subject: the knobs' documentation ships to the editor, never to the player).

---

### Task 11: One command from clone to editor

**Files:**
- Create: `tools/bootstrap.sh`
- Create: `game/tests/probe/engine_census_probe.gd`
- Modify: `game/README.md` (:50-65, step 1 of the authoring recipe)
- Modify: `ci/pipeline.sh` (wire the census probe)

**Interfaces:**
- Consumes: the discovery/validation patterns of `ci/pipeline.sh` (godot discovery :11-17, version prefix check :19-29, cargo env sourcing :51-55), the hand-written 15-class roster rationale of `game/tests/engine_binary_test.gd:17-41`, the probe shape from Task 1. The gdextension macOS/Linux keys load `rust/target/release/` from a plain `cargo build --release`; Windows needs per-triple targets and is out of the script's scope (documented, not scripted).
- Produces: `tools/bootstrap.sh` (the command `game/README.md` names) and `game/tests/probe/engine_census_probe.gd`.

- [ ] **Step 1: The census probe (failing first is impossible here — it must pass on a built tree; instead prove both verdicts).** `engine_census_probe.gd`, `extends SceneTree`, `_initialize` only: hard-code the same 15 class names as `engine_binary_test.gd:25-41` **with the same comment explaining why the roster is hand-written** (a roster regenerated from `rust/src` would drift together with the bug it exists to catch); `_check` each `ClassDB.class_exists(name)`; on any miss print the remedy (`tools/bootstrap.sh builds the engine — run it, then relaunch Godot`); TAP lines + `probe: PASS/FAIL` + `quit(...)`. Prove the FAIL verdict once by running it against a scratch `--path` with no extension (e.g. `godot --headless -s` from an empty dir is enough to see it refuse) — then run it in `game/`: PASS.

- [ ] **Step 2: The script.** `tools/bootstrap.sh`, `#!/bin/sh` + `set -eu`, macOS/Linux (guard: `uname` is `Darwin` or `Linux`, else exit 2 with the Windows note: build with `cargo build --release --target x86_64-pc-windows-msvc` — the gdextension's Windows keys are per-triple). Steps, each with a clear echo:
  1. rustup: `[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"`; if `command -v cargo` fails, install rustup non-interactively (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y`) and source the env again — `rust/rust-toolchain.toml` pins 1.97.1 and its targets automatically.
  2. C linker: `command -v cc || command -v gcc` or exit with the Xcode-CLT / build-essential hint (pattern: `ci/pipeline.sh:59-60`).
  3. Build: `cd "$DIR/rust" && cargo build --release --features editor-docs` — the host-arch artifact lands exactly where `game/unseeing.gdextension`'s macos/linux keys point; `editor-docs` gives the designer the Inspector documentation and ships nowhere (note in a comment: a later `tools/export_macos.sh` rebuilds universal, and any plain cargo build clobbers universal back to thin — irrelevant for authoring).
  4. Godot: discovery loop + prefix version check, verbatim pattern from `ci/pipeline.sh:11-29`; on miss, exit 2 with `brew install godot` / download hint naming `4.7.1.stable.official`.
  5. Import: `"$GODOT" --headless --path "$DIR/game" --import >/dev/null 2>&1 || true` — **after** the build, because the engine records the extension in `.godot/extension_list.cfg` at import and a pre-build import records a failure that a running editor never retries.
  6. Verify: `"$GODOT" --headless --path "$DIR/game" -s res://tests/probe/engine_census_probe.gd` — a probe exits with a real code and cannot fall into gdUnit's zero-tests-exit-0 hole.
  7. `echo "bootstrap: OK — open game/project.godot in Godot 4.7.1 and double-click scenes/level_01.tscn"`.

- [ ] **Step 3: Run it on this machine.** Everything is already installed, so the run exercises the check-paths, the build, the import and the census: expect `bootstrap: OK`. That is the verification (a truly cold machine is recorded as unverified in the research — the script's failure modes each name their remedy).

- [ ] **Step 4: Wire the census probe into CI** (after the level probe, same pattern — it keeps the probe honest against class-roster drift) and **rewrite README step 1** (`game/README.md:50-65`): one command, `tools/bootstrap.sh`; keep the MissingNode paragraph (still the symptom of skipping it) and the relaunch-after-build rule; add one line: sound sources and the cat now show their blueprint shapes in the editor, and a yellow triangle on a node means the level found a fault there — hover it.

> **Superseded 2026-08-13:** this executed task's Windows-manual boundary is
> historical. The current native Windows entry point is
> `tools\bootstrap.cmd`, backed by `tools/bootstrap.ps1`; it selects the Godot
> executable's x86_64/ARM64 target and runs the same import and census contract.
> See `docs/superpowers/specs/2026-08-13-cross-platform-bootstrap-design.md`.

- [ ] **Step 5: Verify + commit.** `gdformat`/`gdlint` the probe; full pipeline green (now includes the census probe). Commit (subject: one command stands between a fresh clone and a working editor).

---

### Task 12: The wiki debt is written down

**Files:**
- Create: `docs/superpowers/plans/2026-08-11-editor-authoring-wiki-debt.md`

The wiki describes **shipped** behaviour, and this branch is unmerged — so the write-back is owed at campaign merge, and this task makes the debt un-losable. Write the file with three sections: (1) **Mechanics — Sound Sources** and **Mechanics — Level and Objects**: paragraphs describing blueprint mode (tool sources/cat, skinless limbs, frozen cat pose), editor derive + per-node warnings + the signature watch, knob ranges/units, icons, `editor-docs`; (2) **Engineering — Build, Test, Deploy**: the two new probes, the census probe, `bootstrap.sh`, the `cargo check --features editor-docs` gate; (3) **Research — Editor Authoring**: the stale-claims list from the 2026-08-11 re-verification (the campaign's six-auditor pass already itemised nine), plus whatever Tasks 1–11 resolved (#30 sources visible, #31 warnings, #32 icons, #33 docs, #34 ranges, #38-as-scoped bootstrap, #44 CLAUDE.md), each with its new file:line. State at the top: **push at campaign merge, not before.**

- [ ] Write it, `gdlint`-irrelevant (markdown), commit (subject: the wiki's debt from sub-project 1, itemised).

---

## Self-Review (run before handing the plan over)

1. **Spec coverage:** SP1 spec section ↔ tasks: tool sources/cat (1–3), editor derive + warnings (4–7), knob polish + icons + docs (8–10), bootstrap (11), write-back (12). The spec's "re-derives on child add/remove and transform changes, debounced" is met by the signature watch (7) — a deliberate refinement recorded there.
2. **Placeholders:** none — every step carries code, an exact command, or a file:line.
3. **Type consistency:** `LIMBS` consts per class; `PlacementFault { path, text }` (4) consumed in 5–6; `faults_for(&self, node: &Gd<Node>) -> PackedStringArray` (5) consumed in 6; `rederive()` (5) used by probes (6–7); probe names and mode-lines consistent (`# sources:` / `# level:`).

## Post-rebase supersession (2026-08-13)

This plan is a frozen record of SP1's fail-first execution; do not rewrite its
task bodies or reuse their old counts as current gates.

- `AGENTS.md`, not `CLAUDE.md`, now owns policy, the two-layer law, and the
  new-object checklist. `CLAUDE.md` is only an adapter.
- The plan's `starved_slots`/flat object-id warning model describes the branch
  before the `dfbb69a` superface rebase. Current world solids are painted per
  face; sources/creatures use fixed role labels. Preserve SP1's durable
  outcomes—live warnings, truthful paths, tool blueprints, knob hints/docs,
  icons, and bootstrap—but translate wiki prose to superface labels.
- Named-limb clearing remains correct for each class's known blueprint limbs.
  It must not be generalized to WaveRun: later generated segments are selected
  by `WaveWall` type, private metadata, and typed `WaveRun` parent.
- Task 11's Windows-manual boundary is superseded by the cross-platform
  bootstrap spec already linked above. Current verification is 405 Cargo tests,
  320 gdUnit cases in 31 suites, 19 registered classes, and ten icons.
