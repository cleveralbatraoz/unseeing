# Reproduction Substrate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a seeded, fixed-timestep, input-scriptable run of the full game reproducible and *proven* reproducible — the substrate the capture/restore and tape plans stand on.

**Architecture:** Five small, independently shippable changes: decouple the RNG seed from the demo tap (`main.gd`); give the cane and the eye scripted entry points that run the *real* input paths (`player.rs`); bind the hero's state into `snapshot()` at the same instant as the pool (`observe/mod.rs` + `observer.rs`); and pin the whole thing with a two-boot agreeing-hash gate under `--fixed-fps` in the headless pipeline.

**Tech Stack:** Rust (gdext 0.5.4, stable channel per `rust/rust-toolchain.toml`), Godot 4.7, typed GDScript, gdUnit4 (vendored), bash.

**Source spec:** `docs/superpowers/specs/2026-08-11-reproduction-loop-design.md` (§ "Determinism substrate"). This is Plan 1 of 3. Plan 2 (capture + restore) and Plan 3 (tape + primitives + harness + diff) are out of scope here and get written after this lands.

## Global Constraints

These apply to every task. Copied from `CLAUDE.md` and the spec.

- **No new crate dependencies.** Serialization is `VarDictionary` + GDScript `JSON.stringify`. No serde. The wasm export must not grow.
- **No `unsafe` Rust.** The crate is `#![deny(unsafe_code)]`; the only permitted exception is the existing `unsafe impl ExtensionLibrary` in `ffi.rs`. Never add another.
- **One Rust GDExtension per wasm export.** Everything joins the single `unseeing-core` crate. Never create a second crate.
- **The two layers.** All law lives in pure modules (`rust/src/*.rs`) that compile and test without a Godot runtime. Engine types (`Gd<T>`, `Node`, `VarDictionary`) appear ONLY in `rust/src/ffi.rs` and `rust/src/nodes/*.rs`. A boundary module carries values and adds no law.
- **Architecture independence.** No arch-specific code paths, intrinsics, or assumptions. Must build for x86_64, aarch64, and wasm32.
- **Observation never mutates.** Every observe entry point takes `&self` or plain values. Nothing may emit a pulse, schedule an echo, or move a node. (The new `tap()`/`look()` are *player* actions, not observations — they mutate on purpose, on the player.)
- **A vacuous pass is worse than a failure.** Every observation that cannot be computed returns `{"unavailable": "<reason>"}` with no data fields, or omits the key and names it in `unknown`. An empty pool and an unobservable pool must never serialise to the same JSON.
- **Object id law:** two touching objects need ids at least `oid_palette::MIN_SEP` (0.08) apart. Never assign ids by cycling a list. (No task here paints anything; if one seems to need to, stop and ask.)
- **Perception laws are untouched by this work.** No rendering, no geometry, no light, no fill.
- **Commits:** small, self-contained, each one green. Narrative subject line, technical body. **No `Co-Authored-By`, no "Generated with", no mention of Claude, AI, or any assistant anywhere in the repository.** Repo identity is `Dmitrii Galchenko <dggrus@gmail.com>`.
- **Tooling before every commit:** `cargo fmt`, `cargo clippy` (warnings are errors), `cargo test` for Rust; `gdformat` + `gdlint` for GDScript.
- **TDD is mandatory:** write the test, watch it fail *for the right reason*, write minimal code, watch it pass. Production code written before its test gets deleted, not retrofitted.

---

## File Structure

**Created:**

| File | Responsibility |
|---|---|
| `game/tests/seed_test.gd` | gdUnit suite: seed and demo are separate switches |
| `game/tests/tap_test.gd` | gdUnit suite: the scripted cane runs the real decision tree |
| `game/tests/probe/determinism_probe.gd` | Headless `SceneTree` script: boot, run 240 frames, print one state hash |
| `tools/determinism_probe.sh` | The agreeing-pair gate: two seeded fixed-fps boots must hash identically |

**Modified:**

| File | Change |
|---|---|
| `game/scripts/main.gd` | `_seed_armed()` replaces the env-only seed check; `observer.inject_hero(player)` |
| `rust/src/nodes/player.rs` | `#[func] tap()`, `#[func] look()` + extracted `apply_look()`, observer accessors (`tap_queued()`, `wave_queue()`, `eye_pitch()`) |
| `rust/src/observe/mod.rs` | `HeroObservation`, `QueuedWave`, `hero: Option<…>` on `SceneObservation`/`FrameObservation` |
| `rust/src/nodes/observer.rs` | `inject_hero()`, `hero_observation()`, `hero_dict()`/`queued_wave_dict()`, hero in `frame_dict` |
| `game/tests/movement_test.gd` | `look()` law tests |
| `game/tests/observer_test.gd` | hero-block tests (present / absent / freed / composition root) |
| `ci/pipeline.sh` | New stage: run `tools/determinism_probe.sh` after the unit tests |

**Task order:** 1 → 2 → 3 → 4 → 5. Task 5 consumes Task 1's seed switch and Task 4's hero block (the hash covers them); Tasks 2–4 are otherwise independent.

---

### Task 1: The seed leaves the demo's shadow

Today `game/scripts/main.gd:67-69` seeds the flicker RNG (`0x5EED`) only when `UNSEEING_DEMO` is set — and that same variable arms a demo tap that fires a wave every 4 s (`main.gd:159-167`, `game/scripts/demo_tap.gd`). You cannot seed the game's only RNG without contaminating the pool. On web, `?demo` arms the tap but never seeds — the determinism defect the pixel-oracle spec depends on. Fix both: a new `UNSEEING_SEED` env switch (and `?seed` on web) seeds *without* arming; `UNSEEING_DEMO`/`?demo` now seeds *too*.

**Files:**
- Modify: `game/scripts/main.gd:64-70` (`_ready`), and add `_seed_armed()` below `_demo_tap()`
- Create: `game/tests/seed_test.gd`

**Interfaces:**
- Consumes: `OS.get_environment`, `OS.set_environment` (test), `UnseeingMain.now` / `_flicker` / `_demo` (public-by-convention GDScript fields).
- Produces: the env contract `UNSEEING_SEED=1` → seeded + demo NOT armed; `UNSEEING_DEMO=1` → seeded + armed. Task 5's probe and the future pixel-oracle gate both rely on it.

- [ ] **Step 1: Write the failing tests**

Create `game/tests/seed_test.gd`:

```gdscript
extends GdUnitTestSuite
## Seed and demo are SEPARATE switches. UNSEEING_SEED (or ?seed on web)
## makes the flicker stream reproducible WITHOUT arming the 4-second demo
## tap that would contaminate the pool; UNSEEING_DEMO still implies
## seeding, because an unseeded demo run cannot be frame-compared. The
## web ?seed/?demo paths ride the same helper but need a browser — the
## smoke gate covers them; these tests pin the env contract headless.

const MAIN_SCENE := preload("res://scenes/main.tscn")
const SEED_VALUE := 0x5EED


func after_test() -> void:
	OS.set_environment("UNSEEING_SEED", "")
	OS.set_environment("UNSEEING_DEMO", "")


func _boot() -> UnseeingMain:
	var main: UnseeingMain = auto_free(MAIN_SCENE.instantiate() as UnseeingMain)
	add_child(main)
	return main


## Advance the simulated clock past the demo-arming checkpoint (now >= 0.5,
## main.gd::_demo_tap) and let one _process run it.
func _run_arming_check(main: UnseeingMain) -> void:
	main.now = 1.0
	await get_tree().process_frame
	await get_tree().process_frame


func test_unseeing_seed_seeds_without_arming_the_demo() -> void:
	OS.set_environment("UNSEEING_SEED", "1")
	var main := _boot()
	assert_int(main._flicker._rng.seed).is_equal(SEED_VALUE)
	await _run_arming_check(main)
	assert_bool(main._demo.armed).is_false()


func test_unseeing_demo_still_seeds_and_arms() -> void:
	OS.set_environment("UNSEEING_DEMO", "1")
	var main := _boot()
	assert_int(main._flicker._rng.seed).is_equal(SEED_VALUE)
	await _run_arming_check(main)
	assert_bool(main._demo.armed).is_true()


func test_an_unswitched_boot_stays_unseeded_and_unarmed() -> void:
	var main := _boot()
	assert_int(main._flicker._rng.seed).is_not_equal(SEED_VALUE)
	await _run_arming_check(main)
	assert_bool(main._demo.armed).is_false()
```

- [ ] **Step 2: Run the suite, watch it fail for the right reason**

Run: `"$GODOT" --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd -a res://tests/seed_test.gd`

Expected: `test_unseeing_seed_seeds_without_arming_the_demo` FAILS on the seed assertion (a fresh RNG's seed is not `0x5EED` because `_ready` only checks `UNSEEING_DEMO`). The other two PASS already — they pin current behaviour so the change cannot break it. Per the gdunit4-gate-holes memory: trust only the exit code and the case count, never the green `PASSED` word.

- [ ] **Step 3: Implement `_seed_armed()`**

In `game/scripts/main.gd`, replace the `_ready` seed block:

```gdscript
	# deterministic flicker for offline frame-comparison runs — armed by
	# ANY deterministic-run switch, not only the demo: seeding the one RNG
	# must not cost a pool contaminated by a tap every four seconds
	var rng := RandomNumberGenerator.new()
	if _seed_armed():
		rng.seed = 0x5EED
	_flicker = Flicker.new(rng)
```

and add, below `_demo_tap()`:

```gdscript
## Deterministic runs arm the seed three ways: UNSEEING_SEED (seed alone,
## no demo tap), UNSEEING_DEMO (a demo run must also be reproducible), or
## ?seed / ?demo in a web URL. The demo TAP still arms only from
## UNSEEING_DEMO / ?demo, in _demo_tap above — seed and demo are separate
## switches, and this helper owns only the seed.
func _seed_armed() -> bool:
	if not OS.get_environment("UNSEEING_SEED").is_empty():
		return true
	if not OS.get_environment("UNSEEING_DEMO").is_empty():
		return true
	if OS.has_feature("web"):
		var search := str(JavaScriptBridge.eval("window.location.search", true))
		return search.contains("seed") or search.contains("demo")
	return false
```

Do not touch `_demo_tap()` — its arming logic is already correct and the tests pin it.

- [ ] **Step 4: Run the suite, watch all three pass**

Run: same command as Step 2. Expected: 3 cases, exit 0.

- [ ] **Step 5: Run the full gdUnit gate + linters**

Run: `gdformat game/scripts/main.gd game/tests/seed_test.gd && gdlint game/scripts/main.gd game/tests/seed_test.gd`, then the full suite run from `ci/pipeline.sh` (or at minimum `-a res://tests`). Expected: no new failures — `demo_tap_test.gd` and `observer_test.gd` stay green.

- [ ] **Step 6: Commit**

Commit `game/scripts/main.gd` + `game/tests/seed_test.gd`. Subject (narrative, per repo style): *"The seed steps out of the demo's shadow"*. Body: what the coupling cost (seeding required arming a 4 s wave emitter; web `?demo` ran unseeded), the three switches, and that this delivers the pixel-oracle spec's prerequisite early.

---

### Task 2: The cane answers to a call, not only a click

The tap decision tree (aimed strike / rest tap / air swish, `rust/src/nodes/player.rs::cane_tap`) is reachable only through a mouse-button event. `queue_wave` fakes a wave and bypasses the tree — wrong for reproduction. Add `#[func] tap()`: the scripted twin of the left-click, setting the same `tap_queued` intent flag the click sets, so the tap executes next physics tick, in physics context, cooldown and all.

**Files:**
- Modify: `rust/src/nodes/player.rs` (one new `#[func]` in the `#[godot_api] impl UnseeingPlayer` block, next to `tick`)
- Create: `game/tests/tap_test.gd`

**Interfaces:**
- Consumes: existing private `tap_queued: bool` (`player.rs:134`), `cane_tap()` (`player.rs:389`), `TAP_COOLDOWN` (0.15 s), `last_tap` `#[var]` (init −10.0).
- Produces: `UnseeingPlayer.tap() -> void` — Plan 3's tape replay and any scripted scenario call this. Task 4 reports the flag it sets.

- [ ] **Step 1: Write the failing tests**

Create `game/tests/tap_test.gd`:

```gdscript
extends GdUnitTestSuite
## The scripted cane: tap() must ride the SAME queued-intent path as the
## left click — executed next physics tick, through the full aimed/rest/
## swish decision tree, swallowed by the cooldown. queue_wave() fakes a
## wave; tap() taps the cane. These tests break if tap() ever bypasses
## the tree (e.g. emits directly) or executes outside the physics tick.

var _player: UnseeingPlayer
var _pulses: Pulses


func before_test() -> void:
	_pulses = Pulses.new()
	_player = auto_free(UnseeingPlayer.new())
	_player.pulses = _pulses
	_player.position = Vector3(0, 0.9, 0)
	add_child(_player)
	_add_floor()


func _add_floor() -> void:
	var body: StaticBody3D = auto_free(StaticBody3D.new())
	body.position = Vector3(0, -0.5, 0)
	var col := CollisionShape3D.new()
	var shape := BoxShape3D.new()
	shape.size = Vector3(20, 1, 20)
	col.shape = shape
	body.add_child(col)
	add_child(body)


func test_tap_waits_for_the_physics_tick_then_runs_the_tree() -> void:
	await get_tree().physics_frame
	_player.tick(5.0)
	_player.tap()
	# queued intent only: the clock of the last ACCEPTED tap is untouched
	# until the physics tick runs the decision tree
	assert_float(_player.last_tap).is_equal(-10.0)
	await get_tree().physics_frame
	await get_tree().physics_frame
	assert_float(_player.last_tap).is_equal_approx(5.0, 1e-9)


func test_an_aimed_down_tap_births_a_real_wave() -> void:
	await get_tree().physics_frame
	# pitch below -0.12 with the cane resting on the floor: the rest-tap
	# voice — a kind-0 wave born at the tip. Slot 0's birth lane leaves
	# the virgin sentinel (-1) only when a wave truly entered the pool.
	_player.camera.rotation.x = -0.5
	_player.tick(5.0)
	_player.tap()
	await get_tree().physics_frame
	await get_tree().physics_frame
	assert_float(_player.tap_target.y).is_less(0.2)
	assert_float(_pulses.dat[0].x).is_greater_equal(0.0)


func test_a_second_tap_inside_the_cooldown_is_swallowed() -> void:
	await get_tree().physics_frame
	_player.tick(5.0)
	_player.tap()
	await get_tree().physics_frame
	await get_tree().physics_frame
	_player.tick(5.05)
	_player.tap()
	await get_tree().physics_frame
	await get_tree().physics_frame
	# 0.05 s < TAP_COOLDOWN 0.15 s: the second tap must not restamp
	assert_float(_player.last_tap).is_equal_approx(5.0, 1e-9)
```

- [ ] **Step 2: Run the suite, watch it fail for the right reason**

Run: `"$GODOT" --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd -a res://tests/tap_test.gd`

Expected: all three FAIL with a runtime error naming a nonexistent function `tap` on `UnseeingPlayer`.

- [ ] **Step 3: Implement `tap()`**

In `rust/src/nodes/player.rs`, inside the `#[godot_api] impl UnseeingPlayer` block (place it directly after `tick`):

```rust
    /// The cane speaks on command: the scripted twin of the left click,
    /// riding the SAME queued-intent path — executed next physics tick,
    /// in physics context, through the full aimed/rest/swish decision
    /// tree and the [`TAP_COOLDOWN`]. `queue_wave` fakes a wave; this
    /// taps the cane.
    #[func]
    pub fn tap(&mut self) {
        self.tap_queued = true;
    }
```

- [ ] **Step 4: Run the suite, watch it pass**

Run: same as Step 2, after `cd rust && cargo build` (the gdextension must rebuild before Godot sees the new method; the gate builds it via its normal path). Expected: 3 cases, exit 0.

- [ ] **Step 5: Rust tooling + full gate**

Run: `cd rust && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`, then `gdformat game/tests/tap_test.gd && gdlint game/tests/tap_test.gd`, then the full test gate. Expected: 233+ cargo tests green, all gdUnit suites green.

- [ ] **Step 6: Commit**

Commit `rust/src/nodes/player.rs` + `game/tests/tap_test.gd`. Subject: *"The cane answers to a call, not only a click"*. Body: why `queue_wave` was the wrong door for reproduction (bypasses the aimed raycast, the rest/swish arbitration, the cooldown, `last_tap`/`tap_target`), and that `tap()` is one flag because the click path already queues intent for the physics tick.

---

### Task 3: The eye turns on command, by the same law the mouse writes

Mouse-look lives inline in `unhandled_input` (`player.rs:174-186`), gated on `MOUSE_MODE_CAPTURED` — which headless silently refuses, so no scripted run can turn the hero through the real look law (`MOUSE_SENS` scaling, `PITCH_LIMIT` clamp). Extract the law into `apply_look()`, call it from the event handler, and expose it as `#[func] look(relative)`. Writing `rotation.y` directly stays possible but bypasses the law; the tape (Plan 3) will use `look()`.

**Files:**
- Modify: `rust/src/nodes/player.rs` (extract + one new `#[func]`)
- Modify: `game/tests/movement_test.gd` (two new tests; the existing uncaptured-motion test must stay green)

**Interfaces:**
- Consumes: `MOUSE_SENS` (0.0026 rad/px), `PITCH_LIMIT` (1.35 rad), the `camera` field.
- Produces: `UnseeingPlayer.look(relative: Vector2) -> void` — Plan 3's tape replay consumes it. The event path is unchanged: captured-mouse motion behaves identically, uncaptured motion still does nothing.

- [ ] **Step 1: Write the failing tests**

Append to `game/tests/movement_test.gd`:

```gdscript
## The scripted eye: look() applies the exact captured-mouse law — yaw by
## -x, pitch by -y, both scaled by MOUSE_SENS — without needing a mouse.
## 100 px right = -(100 x 0.0026) = -0.26 rad, hand-derived from the
## constant, not read back from the code under test.
func test_look_turns_the_body_by_the_mouse_law() -> void:
	_player.look(Vector2(100, 0))
	assert_float(_player.rotation.y).is_equal_approx(-0.26, 1e-4)


## The pitch clamp holds for scripted look exactly as for the mouse: a
## huge downward swipe pins the eye at -PITCH_LIMIT, never past it.
func test_look_pitch_stops_at_the_limit() -> void:
	_player.look(Vector2(0, 10000))
	assert_float(_player.camera.rotation.x).is_equal_approx(-1.35, 1e-4)
```

- [ ] **Step 2: Run the suite, watch it fail for the right reason**

Run: `"$GODOT" --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd -a res://tests/movement_test.gd`

Expected: the two new tests FAIL on a nonexistent function `look`; every pre-existing case in the suite PASSES (especially the uncaptured-mouse-motion test — it pins the event gate this task must not touch).

- [ ] **Step 3: Extract the law, add the entry point**

In `rust/src/nodes/player.rs`, replace the motion branch of `unhandled_input`:

```rust
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            if Input::singleton().get_mouse_mode() == input::MouseMode::CAPTURED {
                self.apply_look(motion.get_relative());
            }
            return;
        }
```

Add to the `#[godot_api] impl UnseeingPlayer` block (after `tap()`):

```rust
    /// One mouse-motion's worth of look, as data: yaw by -x, pitch by -y,
    /// both scaled by [`MOUSE_SENS`], pitch clamped to [`PITCH_LIMIT`] —
    /// the exact law the captured-mouse handler applies, callable without
    /// a mouse so a scripted run turns the hero through the player's real
    /// look path instead of teleporting the rotation around it.
    #[func]
    pub fn look(&mut self, relative: Vector2) {
        self.apply_look(relative);
    }
```

And the extracted private law (next to `cane_tap`):

```rust
    /// The look law, shared by the captured mouse and the scripted
    /// `look`: the capture GATE stays at the event handler — it is about
    /// who owns the cursor, not about how rotation works.
    fn apply_look(&mut self, relative: Vector2) {
        self.base_mut()
            .rotate_y((f64::from(-relative.x) * MOUSE_SENS) as f32);
        if let Some(camera) = self.camera.as_mut() {
            let mut rot = camera.get_rotation();
            rot.x = (f64::from(rot.x) - f64::from(relative.y) * MOUSE_SENS)
                .clamp(-PITCH_LIMIT, PITCH_LIMIT) as f32;
            camera.set_rotation(rot);
        }
    }
```

- [ ] **Step 4: Run the suite, watch it pass**

Run: rebuild (`cd rust && cargo build`), then same as Step 2. Expected: whole suite green, including the pre-existing uncaptured-motion gate test.

- [ ] **Step 5: Rust tooling + full gate**

Run: `cd rust && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`, `gdformat`/`gdlint` on `movement_test.gd`, full gate. Expected: green.

- [ ] **Step 6: Commit**

Commit `rust/src/nodes/player.rs` + `game/tests/movement_test.gd`. Subject: *"The eye turns on command, by the same law the mouse writes"*. Body: the capture gate stayed at the event handler (ownership of the cursor), the law moved to one place, and a scripted turn now exercises `MOUSE_SENS`/`PITCH_LIMIT` instead of teleporting rotation.

---

### Task 4: The snapshot learns where the hero stands, in the same instant as everything else

The snapshot binds pool + echoes + sources + walls + camera at one instant — but the hero is missing: position, velocity, yaw, pitch, tap clocks are 8+ separate property reads across frames, and `tap_queued` has no accessor at all. Add a `hero` group: a pure `HeroObservation` carried through `observe::frame`, fetched by the observer from a separately injected player. Absence is *named*, never invented: no player, a freed player, or a camera-less player all omit the key and push `"hero"` into `unknown` — the group is all-or-nothing like the capture blob it will feed in Plan 2.

**Files:**
- Modify: `rust/src/observe/mod.rs` (two new structs, two new fields, one pure test, `test_scene` builder gains `hero: None`)
- Modify: `rust/src/nodes/player.rs` (three `pub(crate)` accessors — no new `#[func]`)
- Modify: `rust/src/nodes/observer.rs` (`player` field, `inject_hero`, `hero_observation`, `hero_dict`, `queued_wave_dict`, `frame_dict` hero handling)
- Modify: `game/scripts/main.gd` (one line: `observer.inject_hero(player)`)
- Modify: `game/tests/observer_test.gd` (four new tests)

**Interfaces:**
- Consumes: `UnseeingPlayer`'s `pub(crate) last_tap` / `tap_target` fields, `tap()` from Task 2, `CharacterBody3D::get_velocity`/`get_global_position`/`get_rotation`.
- Produces:
  - `observe::HeroObservation { position: Vector3, velocity: Vector3, yaw: f64, pitch: f64, last_tap: f64, tap_target: Vector3, tap_queued: bool, queued_waves: Vec<QueuedWave> }`
  - `observe::QueuedWave { kind: i64, at: Vector3, max_r: f64, speed: f64, gain: f64, echoes: i64, normal: Vector3 }`
  - `SceneObservation.hero: Option<HeroObservation>`, `FrameObservation.hero: Option<HeroObservation>`
  - `WaveObserver.inject_hero(player)` `#[func]`; snapshot key `"hero"` with sub-keys `position`, `velocity`, `yaw`, `pitch`, `last_tap`, `tap_target`, `tap_queued`, `queued_waves` (array of dicts keyed `type`/`at`/`max_r`/`speed`/`gain`/`echoes`/`normal`, matching the existing `queued_waves()` `#[func]` naming)
  - `UnseeingPlayer` `pub(crate)` accessors: `tap_queued() -> bool`, `wave_queue() -> Vec<QueuedWave>`, `eye_pitch() -> Option<f64>`
  - Plan 2's `capture()` reuses `HeroObservation` as its hero group.

- [ ] **Step 1: Write the failing pure test**

In `rust/src/observe/mod.rs` tests, first extend the builder — `test_scene` gains `hero: None`:

```rust
    fn test_scene(wall_rects: Vec<Vector4>) -> SceneObservation {
        SceneObservation {
            sources: Vec::new(),
            wall_rects,
            eye: test_eye(),
            spawn: test_spawn(),
            hero: None,
        }
    }
```

Then the test:

```rust
    /// The composer carries the hero through untouched — and an absent
    /// hero stays absent rather than becoming a hero at the origin, which
    /// would be the vacuous pass this layer exists to prevent.
    #[test]
    fn a_frame_carries_the_hero_when_the_scene_has_one() {
        let pool = PulsePool::new();
        let hero = HeroObservation {
            position: Vector3::new(1.0, 0.9, -2.0),
            velocity: Vector3::new(0.0, 0.0, -2.1),
            yaw: 0.7,
            pitch: -0.3,
            last_tap: 4.5,
            tap_target: Vector3::new(1.0, 0.0, -3.5),
            tap_queued: true,
            queued_waves: vec![QueuedWave {
                kind: 2,
                at: Vector3::ZERO,
                max_r: 4.0,
                speed: 4.0,
                gain: 0.5,
                echoes: 0,
                normal: Vector3::UP,
            }],
        };
        let mut scene = test_scene(Vec::new());
        scene.hero = Some(hero.clone());
        let f = frame(&pool, &EchoQueue::new(), 0.0, 1.0, scene);
        assert_eq!(f.hero, Some(hero));
        assert_eq!(empty_frame(&pool, 0.0).hero, None);
    }
```

- [ ] **Step 2: Run it, watch it fail for the right reason**

Run: `cd rust && cargo test observe::` — Expected: compile error, `HeroObservation` not found (and `hero` not a field). A compile failure IS the right first failure for a new type; the assertion failure comes when the struct exists but `frame` drops the field — check for that specifically by implementing the structs *before* threading the field, running once, and seeing `f.hero == None` fail the `Some` assertion.

- [ ] **Step 3: Implement the pure side**

In `rust/src/observe/mod.rs`, above `SceneObservation`:

```rust
/// One wave request still waiting for the physics tick, as an agent reads
/// it — the hero's out-tray, bound into the snapshot at the same instant
/// as the pool it will feed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueuedWave {
    pub kind: i64,
    pub at: Vector3,
    pub max_r: f64,
    pub speed: f64,
    pub gain: f64,
    pub echoes: i64,
    pub normal: Vector3,
}

/// The hero as an agent reads them: where the body stands and moves,
/// where the eye points, and the cane's clocks. Before this group existed
/// an agent stitched the same facts from eight separate property reads
/// across frames, so the "one instant" guarantee never covered the hero.
#[derive(Debug, Clone, PartialEq)]
pub struct HeroObservation {
    pub position: Vector3,
    pub velocity: Vector3,
    /// Body yaw, radians — the way the hero faces.
    pub yaw: f64,
    /// Eye pitch, radians, as the look law last clamped it.
    pub pitch: f64,
    /// The tap clock reading of the last ACCEPTED tap (−10.0 when none).
    pub last_tap: f64,
    /// Where that tap landed.
    pub tap_target: Vector3,
    /// A tap accepted this frame that the physics tick has not yet run.
    pub tap_queued: bool,
    /// Every wave request waiting for the next physics tick.
    pub queued_waves: Vec<QueuedWave>,
}
```

Add `pub hero: Option<HeroObservation>,` to BOTH `SceneObservation` and `FrameObservation` (last field of each), and in `frame()` carry it: `hero: scene.hero,` (place it beside `spawn: scene.spawn`).

- [ ] **Step 4: Run the pure tests, watch them pass**

Run: `cd rust && cargo test observe::` — Expected: all green, including the new test and every pre-existing composer test (they build via `test_scene`, which now supplies `hero: None`).

- [ ] **Step 5: Write the failing gdUnit tests**

Append to `game/tests/observer_test.gd` (it already defines `MAIN_SCENE`, `LEVEL_SCENE`, and uses `auto_free` + `add_child` exactly like this):

```gdscript
## The hero group binds the body, the eye, and the cane's out-tray into
## the SAME snapshot as the pool they feed — before this, the "one
## instant" guarantee stopped at the camera and the hero was eight
## separate reads across frames.
func test_the_snapshot_binds_the_hero_at_one_instant() -> void:
	var pulses := Pulses.new()
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), pulses)
	add_child(level)
	var player: UnseeingPlayer = auto_free(UnseeingPlayer.new())
	player.pulses = pulses
	player.position = Vector3(5.0, 0.9, 3.0)
	player.rotation.y = 0.7
	add_child(player)
	var obs: WaveObserver = auto_free(WaveObserver.new())
	obs.inject(level, player.camera)
	obs.inject_hero(player)
	add_child(obs)
	player.camera.rotation.x = -0.3
	player.queue_wave(2, Vector3.ZERO, 4.0, 4.0, 0.5, 0, Vector3.UP)
	player.tap()
	# read BEFORE any physics tick drains the queue or runs the tap: the
	# flag and the out-tray must appear beside the pool they will feed
	var snap: Dictionary = obs.snapshot(0.0)
	assert_bool(snap.has("unavailable")).is_false()
	var hero: Dictionary = snap["hero"]
	assert_vector(hero["position"]).is_equal_approx(Vector3(5.0, 0.9, 3.0), Vector3.ONE * 0.001)
	assert_float(hero["yaw"]).is_equal_approx(0.7, 0.0001)
	assert_float(hero["pitch"]).is_equal_approx(-0.3, 0.0001)
	assert_float(hero["last_tap"]).is_equal(-10.0)
	assert_bool(hero["tap_queued"]).is_true()
	var queued: Array = hero["queued_waves"]
	assert_int(queued.size()).is_equal(1)
	assert_int(queued[0]["type"]).is_equal(2)
	assert_vector(queued[0]["normal"]).is_equal(Vector3.UP)


## No hero is a NAMED absence, not a hero at the origin: a suite building
## a bare level has no player, and the snapshot says so in `unknown`.
func test_a_heroless_snapshot_names_the_absence() -> void:
	var pulses := Pulses.new()
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), pulses)
	add_child(level)
	var camera: Camera3D = auto_free(Camera3D.new())
	add_child(camera)
	var obs: WaveObserver = auto_free(WaveObserver.new())
	obs.inject(level, camera)
	add_child(obs)
	var snap: Dictionary = obs.snapshot(0.0)
	assert_bool(snap.has("hero")).is_false()
	assert_bool((snap["unknown"] as Array).has("hero")).is_true()


## A freed hero must degrade to the SAME named absence — never a crash,
## and never data read through a dangling handle.
func test_a_freed_hero_reports_unknown_rather_than_crashing() -> void:
	var pulses := Pulses.new()
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), pulses)
	add_child(level)
	var camera: Camera3D = auto_free(Camera3D.new())
	add_child(camera)
	var player := UnseeingPlayer.new()
	player.pulses = pulses
	add_child(player)
	var obs: WaveObserver = auto_free(WaveObserver.new())
	obs.inject(level, camera)
	obs.inject_hero(player)
	add_child(obs)
	remove_child(player)
	player.free()
	var snap: Dictionary = obs.snapshot(0.0)
	assert_bool(snap.has("hero")).is_false()
	assert_bool((snap["unknown"] as Array).has("hero")).is_true()


## The composition root hands the observer the hero it built, exactly as
## it hands it the level and the eye.
func test_the_composition_root_injects_the_hero() -> void:
	var main: UnseeingMain = auto_free(MAIN_SCENE.instantiate() as UnseeingMain)
	add_child(main)
	var snap: Dictionary = main.observer.snapshot(0.0)
	var hero: Dictionary = snap["hero"]
	assert_vector(hero["position"]).is_equal(main.player.global_position)
	assert_bool((snap["unknown"] as Array).has("hero")).is_false()
```

- [ ] **Step 6: Run the suite, watch the four fail for the right reason**

Run: `"$GODOT" --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd -a res://tests/observer_test.gd`

Expected: the four new tests FAIL — the first three on a nonexistent function `inject_hero` / a missing `"hero"` key, the fourth on the missing key. Every pre-existing case PASSES.

- [ ] **Step 7: Implement the boundary**

`rust/src/nodes/player.rs` — three accessors in a **plain** `impl UnseeingPlayer` block (they are crate-internal, not Godot surface; place the block after the `#[godot_api]` one), plus the import `use crate::observe::QueuedWave;`:

```rust
impl UnseeingPlayer {
    /// The cane's queued-intent flag, for the observer: a tap accepted
    /// this frame that the physics tick has not yet executed.
    pub(crate) fn tap_queued(&self) -> bool {
        self.tap_queued
    }

    /// The eye's pitch, radians — `None` before `_ready` has built the
    /// camera, which is a different fact from a level gaze and must not
    /// be reported as one.
    pub(crate) fn eye_pitch(&self) -> Option<f64> {
        self.camera
            .as_ref()
            .map(|camera| f64::from(camera.get_rotation().x))
    }

    /// The wave queue as pure observations — the same content the
    /// `queued_waves` #[func] serialises for the suites.
    pub(crate) fn wave_queue(&self) -> Vec<QueuedWave> {
        self.wave_queue
            .iter()
            .map(|w| QueuedWave {
                kind: w.kind,
                at: w.at,
                max_r: w.max_r,
                speed: w.speed,
                gain: w.gain,
                echoes: w.echoes,
                normal: w.normal,
            })
            .collect()
    }
}
```

`rust/src/nodes/observer.rs` — the field, the injector, the fetcher, the dicts:

```rust
use super::player::UnseeingPlayer;
use crate::observe::{HeroObservation, QueuedWave};   // extend the existing observe import list
```

Add `player: Option<Gd<UnseeingPlayer>>,` to the `WaveObserver` struct fields.

```rust
    /// Hand the observer the hero to read, separately from the world: a
    /// suite building a bare level has no hero, and that absence must be
    /// REPORTED (in `unknown`) rather than refusing the world around it.
    #[func]
    fn inject_hero(&mut self, player: Option<Gd<UnseeingPlayer>>) {
        self.player = player;
    }
```

The fetcher, next to `live_camera` (a plain method on the same impl):

```rust
    /// The hero group, if a live, fully-built hero was injected. `None` —
    /// which the snapshot names in `unknown` — covers never-injected,
    /// freed, and a player whose camera has not been built yet: a pitch
    /// invented for an eyeless hero would be a guess, and the group is
    /// all-or-nothing like the capture blob it will one day feed.
    fn hero_observation(&self) -> Option<HeroObservation> {
        let player = self.player.clone()?;
        if !player.is_instance_valid() {
            return None;
        }
        let position = player.get_global_position();
        let velocity = player.get_velocity();
        let yaw = f64::from(player.get_rotation().y);
        let bound = player.bind();
        let pitch = bound.eye_pitch()?;
        Some(HeroObservation {
            position,
            velocity,
            yaw,
            pitch,
            last_tap: bound.last_tap,
            tap_target: bound.tap_target,
            tap_queued: bound.tap_queued(),
            queued_waves: bound.wave_queue(),
        })
    }
```

In `snapshot()`, the `SceneObservation` literal gains `hero: self.hero_observation(),`.

In `frame_dict`, between the `spawn` line and the `unknown` line:

```rust
    match &observation.hero {
        Some(hero) => {
            state.set("hero", &hero_dict(hero));
        }
        None => unknown.push("hero"),
    }
```

And the two serialisers, beside `spawn_dict`:

```rust
fn hero_dict(hero: &HeroObservation) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set("position", hero.position);
    entry.set("velocity", hero.velocity);
    entry.set("yaw", hero.yaw);
    entry.set("pitch", hero.pitch);
    entry.set("last_tap", hero.last_tap);
    entry.set("tap_target", hero.tap_target);
    entry.set("tap_queued", hero.tap_queued);
    let queued: Array<VarDictionary> = hero.queued_waves.iter().map(queued_wave_dict).collect();
    entry.set("queued_waves", &queued);
    entry
}

/// Keyed exactly as the player's own `queued_waves` #[func] keys them
/// ("type", not "kind"), so a reader sees one vocabulary for one queue.
fn queued_wave_dict(wave: &QueuedWave) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set("type", wave.kind);
    entry.set("at", wave.at);
    entry.set("max_r", wave.max_r);
    entry.set("speed", wave.speed);
    entry.set("gain", wave.gain);
    entry.set("echoes", wave.echoes);
    entry.set("normal", wave.normal);
    entry
}
```

`game/scripts/main.gd` — one line after `observer.inject(level, player.camera)`:

```gdscript
	observer.inject_hero(player)
```

- [ ] **Step 8: Run everything, watch it pass**

Run: `cd rust && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`, rebuild, then the full gdUnit gate. Expected: all green, including the untouched refusal tests (the no-camera refusal still fires before the hero is even consulted).

- [ ] **Step 9: Commit**

Commit the four Rust/GDScript files + both test files. Subject: *"The snapshot learns where the hero stands, in the same instant as everything else"*. Body: the eight-reads problem, the named-absence rule for the hero group (unknown, not refusal — and why that differs from the camera), and that Plan 2's `capture()` will reuse the group.

---

### Task 5: Two boots, one hash — the substrate proves it can repeat itself

Nothing in the repo pins `--fixed-fps`, so `now += dt` rides real frame deltas and two runs never agree. This task delivers the pinned recipe *and its proof*: a headless probe that boots the full game seeded, runs 240 fixed frames (4.0 s simulated — several fan/radio beats, cat paw taps, flicker jitter), and prints one md5 of the full-precision, key-sorted snapshot JSON; a shell gate runs it twice and demands the pair agree. This is the spec's honest measurement of Godot-physics reproducibility: **if the pair disagrees, that is a finding — invoke systematic-debugging and report; do not widen tolerances or drop fields from the hash.**

**Files:**
- Create: `game/tests/probe/determinism_probe.gd`
- Create: `tools/determinism_probe.sh` (mode 0755, like `tools/probe_visibility.sh`)
- Modify: `ci/pipeline.sh` (new stage after the gdUnit stage at `ci/pipeline.sh:117`, before the export stage)

**Interfaces:**
- Consumes: Task 1's `UNSEEING_SEED`, Task 4's hero block (in the hash), `UnseeingMain.now`/`observer`, `WaveObserver.snapshot`.
- Produces: the deterministic-run recipe every Plan-2/Plan-3 harness reuses: `UNSEEING_SEED=1 "$GODOT" --headless --fixed-fps 60 --path game -s res://tests/probe/determinism_probe.gd`, and the stdout contract `DETERMINISM_HASH=<md5>` exactly once on success.

- [ ] **Step 1: Write the probe script**

Create `game/tests/probe/determinism_probe.gd`:

```gdscript
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
	_main = load("res://scenes/main.tscn").instantiate() as UnseeingMain
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
```

- [ ] **Step 2: Write the gate — first WITHOUT `--fixed-fps`, to watch it fail for the right reason**

Create `tools/determinism_probe.sh` (`chmod 755`):

```bash
#!/usr/bin/env bash
# Two seeded headless boots must produce byte-identical state hashes.
# Catches: unseeded randomness, wall-clock leaks into the sim, and
# run-to-run divergence in anything the snapshot can see — the substrate
# every reproduction artifact (capture blob, action tape) rides on.
# A MISSING hash is a failure, never a pass: a probe that crashed or
# refused must not read as "the runs agreed".
set -euo pipefail
DIR="$(cd "$(dirname "$0")/.." && pwd)"
GODOT="${GODOT:-godot}"

run_once() {
  UNSEEING_SEED=1 "$GODOT" --headless --path "$DIR/game" \
    -s res://tests/probe/determinism_probe.gd 2>&1 \
    | grep '^DETERMINISM_HASH=' | head -1
}

A="$(run_once || true)"
B="$(run_once || true)"
[ -n "$A" ] || { echo "determinism: FAILED — no hash from run A (probe crashed or refused)"; exit 1; }
[ -n "$B" ] || { echo "determinism: FAILED — no hash from run B (probe crashed or refused)"; exit 1; }
if [ "$A" != "$B" ]; then
  echo "determinism: FAILED — two seeded boots disagree:"
  echo "  run A: $A"
  echo "  run B: $B"
  exit 1
fi
echo "determinism: OK $A"
```

Note the deliberately missing `--fixed-fps 60` — that is the failing state.

- [ ] **Step 3: Run it, watch it fail for the right reason**

Run: `GODOT=<binary> tools/determinism_probe.sh`

Expected: `determinism: FAILED — two seeded boots disagree` — real frame deltas make `now` (and every birth time riding it) differ between runs. This failure is the proof that the hash is sensitive to timing drift, i.e. that the gate can actually catch what it claims to catch. If the two runs *agree* here, stop: the hash is not covering the clock, and the probe is broken.

- [ ] **Step 4: Add `--fixed-fps 60`, watch it pass**

Change `run_once` to:

```bash
  UNSEEING_SEED=1 "$GODOT" --headless --fixed-fps 60 --path "$DIR/game" \
    -s res://tests/probe/determinism_probe.gd 2>&1 \
    | grep '^DETERMINISM_HASH=' | head -1
```

Run it again. Expected: `determinism: OK DETERMINISM_HASH=<md5>`. Run it three more times — every run must print the same hash. **If it flakes:** that is the Godot-physics finding the spec anticipates. Invoke superpowers:systematic-debugging, identify which snapshot field diverges (diff the two JSON dumps by hand: re-run the probe with the JSON printed instead of the hash), and report the field and the tick — do not mask it.

- [ ] **Step 5: Wire the stage into the pipeline**

In `ci/pipeline.sh`, directly after the gdUnit stage (the `"$GODOT" --headless --path "$DIR/game" -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd …` block ending near line 117), add:

```bash
echo "ci: determinism probe (two seeded fixed-fps boots must agree)"
GODOT="$GODOT" "$DIR/tools/determinism_probe.sh"
```

- [ ] **Step 6: Run the full pipeline**

Run: `ci/pipeline.sh` (with `SKIP_EXPORT=1` if no export templates locally). Expected: the new stage prints `determinism: OK …` between the unit tests and the export; everything green.

- [ ] **Step 7: Lint + commit**

Run `gdformat`/`gdlint` on the probe script. Commit `game/tests/probe/determinism_probe.gd`, `tools/determinism_probe.sh`, `ci/pipeline.sh`. Subject: *"Two boots, one hash: the substrate proves it can repeat itself"*. Body: what the hash covers (full-precision sorted snapshot JSON including the hero block), why the failing-first run without `--fixed-fps` was kept as the sensitivity proof, and the flake-is-a-finding rule.

---

## Self-Review Notes

- **Spec coverage (substrate section):** seed decoupling → Task 1; fixed-timestep recipe → Task 5; hero block in `snapshot()` → Task 4; `tap()` → Task 2; look entry → Task 3. The spec's `?seed` web param ships in Task 1 (`_seed_armed`), tested headless via env and covered on web by the existing smoke boot (a wrong helper would crash `_ready` there).
- **Type consistency:** `QueuedWave.kind: i64` matches `WaveRequest.kind: i64`; the wire key is `"type"` to match the existing `queued_waves()` `#[func]`; `hero["pitch"]` comes from `eye_pitch()` (camera rotation.x), the same number Task 3's clamp law writes.
- **Deliberate non-goals:** no capture, no restore, no tape (Plans 2–3); no `#[func]` for the player accessors (crate-internal); `queue_wave` untouched; the demo tap's arming logic untouched.
