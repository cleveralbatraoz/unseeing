# Editor Authoring SP4 — The Rust Composition Root — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `game/scripts/main.gd` (202 lines on the pre-merge branch; grown on main with capture/restore) is absorbed into a registered Rust `UnseeingGame` node; GDScript in the repo shrinks to designer-facing residue plus test helpers; the designer-facing razor is recorded in CLAUDE.md as law.

**Architecture:** A `#[class(init, base=Node3D)] UnseeingGame` in `rust/src/nodes/game.rs` reproduces `main.gd`'s ready-order and per-frame loop exactly (both are pinned by tests and gates); `Flicker` and `DemoTap` become pure Rust modules (the flicker generic over a rand source so cargo can test the law while the game feeds it the same seeded Godot `RandomNumberGenerator` stream); the `Pulses` GDScript shim leaves the shipped game (UnseeingGame holds `WaveCore` directly — every Rust consumer already duck-types the handle, and the observer try-casts `WaveCore` first) and survives only as a test helper; `capture_env`/`apply_env`/`restore_blob` port to `#[func]`s with byte-identical semantics; seven suites and three probes retype from `UnseeingMain` to `UnseeingGame` against a designed observability surface.

**Tech Stack:** Rust (gdext 0.5.4 pinned), Godot 4.7.1.stable.official, gdUnit4, headless probes, POSIX sh.

## Global Constraints

Every task's requirements implicitly include this section. Copied from CLAUDE.md, the campaign spec, and the merged tree's measured state.

- **Perception laws** (B/W thin outlines, waves reveal, one outline per object, id clearance 0.08, `oid_palette` colouring — never cycle a list). SP4 must not alter any rendering behaviour: the five-material protocol (data/source/cane/body/post, priorities 0/20, cane `u_base` 0.85, per-instance `u_source_floor`), the post quad recipe (QuadMesh 2×2, `extra_cull_margin` 16384.0, position (0,0,-1), camera child, `material_override`), and the post-mat wall table hand-off are transliterated, not redesigned.
- **Platforms:** Windows/macOS/web from one project; **arch-independent** — web-only code paths guard on `Os::singleton().has_feature("web")` at runtime, **never** `cfg(target_arch)`/`cfg(target_os = "emscripten")` conditional compilation for behaviour (the JavaScriptBridge class does not exist in desktop bindings — reach it dynamically via `Engine::singleton().get_singleton("JavaScriptBridge")` under the feature guard).
- **The two layers + the designer-facing razor** (campaign spec): all logic in the single Rust crate; GDScript residue = designer-facing scenes/knobs + tests/probes only.
- **No unsafe Rust** (crate-level deny; the one `ffi.rs` exception stands).
- **Strict TDD** (test first, fail right, minimal code, mutation check; no mirror assertions; no change detectors). The one sanctioned substitute where a law moves languages: an **equivalence harness** that runs old (GDScript) and new (Rust) side by side on identical inputs and asserts identical outputs, committed green BEFORE the old implementation is deleted.
- **Formatters/analyzers before every commit:** `cargo fmt` + `cargo clippy --all-targets -- -D warnings` + `cargo test`; `gdformat` + `gdlint` on changed `.gd`.
- **Commits:** small, self-contained, green; narrative house-style subjects; **no attribution of any kind — no Co-Authored-By, no "Generated with", no assistant mentions anywhere**. Identity `Dmitrii Galchenko <dggrus@gmail.com>` is configured.
- **Gate honesty:** `--import` before any gdUnit run; trust suite/case counts. **Merged-tree baselines at e0c0250: 304 cargo / 259 gdUnit cases / 31 suites.** Tasks that migrate suites must state the expected count delta in the report (a silent shrink is a failed task); the full pipeline (SKIP_EXPORT=1) must print the expected counts and every probe/gate verdict.
- **Boot-gate contract:** any class-style diagnostic opening (`"UnseeingGame: "`) must be a literal in `rust/src` AND `ERROR: UnseeingGame` must join `ci/boot_error_pattern.sh:38` in the same commit; synthesised prefixes (`format!("{cls}: …")`) are banned by the gate.
- **Injection order is law, enforced by tests and the boot gate:** `level.inject(...)` strictly BEFORE `add_child(level)` (godot_error + unrepairable derive otherwise); player/hero/observer wired before their `add_child`; **SettingsMenu added LAST** (bottom-up unhandled input gives it Escape first — `settings_test.gd`'s isolation law depends on it); observer in the same tree as the level (its `space_state()` needs a physics world).
- **The clock is simulated** (`now += delta` per process frame), never wall time. Demo waves fire through `player.queue_wave(...)` (physics-context law), never a direct emit from `process`.
- **Seed/demo are two separate switches** (`UNSEEING_SEED` / `UNSEEING_DEMO` env; web URL `?seed` / `?demo`): either arms the 0x5EED flicker seed; only DEMO arms the tap. `seed_test.gd` pins all three combinations; the web `?demo` path is the browser smoke gate's entire light source.
- **Blob compatibility:** `rust/src/reproduce/blob.rs` and the observer's `capture`/`env_of` formats are untouched; `capture_env()` keeps exactly its 9 keys with the same value types; `restore_blob()` keeps its full semantic shape (pause bracket, refusal rollback, post-write hash mismatch → one-key `unavailable` WITHOUT rollback).
- **Determinism gates:** `tools/determinism_probe.sh` and `tools/restore_probe.sh` (both `UNSEEING_SEED=1 --fixed-fps 60`, booting `main.tscn`) must pass; they compare a build against itself, so ALSO keep the flicker equivalence harness honest — self-consistency is not behaviour preservation.
- **Class rosters:** a new registered class must be added to BOTH hand-written 15-name rosters (`game/tests/engine_binary_test.gd:25-41` and `game/tests/probe/engine_census_probe.gd`) — they are deliberately duplicated. Do NOT add an `[icons]` entry (icon_manifest_test uses contains_exactly over eight designer classes; UnseeingGame is not designer-facing).
- **Lint-scope sentinel:** `test/ci_gdscript_lint_scope.sh:73` greps for `/game/scripts/main\.gd$` — deleting main.gd without repointing the sentinel fails the pipeline's second-cheapest gate.
- **Typed, not stringly:** every private `#[func]` the root must call gets a visibility bump to `pub(super)` (the `spawn_pos`/`wall_rects` precedent) and is called through `bind_mut()`. No new stringly `.call()` wiring except where the codebase already committed to it (the shared pool handle, cat `#[var]` property sets).
- **Full `SKIP_EXPORT=1 ci/pipeline.sh` green before claiming any task done.** Task 8 additionally runs the FULL pipeline (no SKIP_EXPORT: wasm build + web export + browser smoke) — the only gate that exercises the web `?demo` arming.

## Decisions Locked by Research (do not re-litigate; each traces to a measured fact)

1. **Settings:** the spec sentence "the overlay's layout stays a .tscn; its logic moves to Rust" was written on a false premise — no settings `.gd` or `.tscn` exists; logic AND layout are already fully Rust (`rust/src/nodes/settings.rs`, 643 lines). SP4's settings work is exactly one line: construct `SettingsMenu` last. Task 9 records the erratum in the spec file.
2. **Pulses:** UnseeingGame holds `Gd<WaveCore>` (typed) and hands it around `upcast::<RefCounted>()` — the existing `inject` signatures don't change; every Rust consumer calls the handle stringly already and `WaveCore` carries the identical method names; `observer.pulse_core` try-casts `WaveCore` first (measured). The shim's `apply()` loop moves into `UnseeingGame::process`; the MAXP mirror moves to a `WaveCore` `#[func]`; `pulses.gd` relocates to `game/tests/` as a test helper (`class_name Pulses` is location-independent; the wasm export excludes `tests/*`, so the shipped game stops carrying it).
3. **Flicker determinism:** the Rust flicker consumes the SAME seeded `RandomNumberGenerator` with the SAME call sequence (1–3 `randf()` per frame, f32 → f64 cast) so the 0x5EED stream is bit-identical; the law itself is a pure module generic over a `Randf` source, cargo-tested with a stub.
4. **One pool handle:** exactly one `WaveCore` instance flows to player, hero, level (sources/cats), and the process loop — never two references to one pool.
5. **`ensure_actions()` stays called twice** (root ready + player ready) — idempotent by design; a bare player in a test scene depends on the second call.

## File Structure

- Create: `rust/src/flicker.rs` (pure law + `Randf` trait), `rust/src/demo_tap.rs` (pure), `rust/src/nodes/game.rs` (UnseeingGame).
- Modify: `rust/src/ffi.rs` (WaveCore `max_pulses`), `rust/src/nodes/mod.rs` (module wiring), visibility bumps in `level.rs`/`player.rs`/`hero.rs`/`observer.rs`/`restorer.rs`, `game/scenes/main.tscn`, `ci/boot_error_pattern.sh`, `test/ci_gdscript_lint_scope.sh`, both class rosters, `CLAUDE.md`, campaign spec (erratum), wiki-debt file.
- Migrate: `game/tests/{wiring,seed,settings,observer,restore,restore_transaction}_test.gd`, `game/tests/probe/{determinism,occlusion,display,restore}_probe.gd`.
- Create then retire: equivalence harness suites; retire `game/scripts/{main,flicker,demo_tap}.gd` + `game/tests/{flicker,demo_tap}_test.gd` (laws re-homed to cargo); move `game/scripts/pulses.gd` → `game/tests/pulses.gd`.

---

### Task 1: WaveCore owns the MAXP mirror

**Files:** Modify `rust/src/ffi.rs`, `game/tests/shader_contract_test.gd`.
**Interfaces:** Produces `#[func] fn max_pulses() -> i64` on `WaveCore` (= `pulse_pool::MAXP as i64`). Consumed by shader_contract_test now, by Task 6's shim relocation later.

- [ ] Failing gdUnit first: in `shader_contract_test.gd`, next to the existing `Pulses.MAXP` mirror assertion, add `assert_int(WaveCore.new().max_pulses()).is_equal(_shader_const("MAXP"))` (match the file's existing `_shader_const` idiom). Run single-suite after `--import`: red (no such method).
- [ ] Implement the `#[func]` in `ffi.rs` beside the existing ones; `cargo fmt/clippy/test`; suite green. Keep the `Pulses.MAXP` assertion — it dies with the shim's relocation, not before.
- [ ] Mutation: return `MAXP - 1` — the new assertion must fail. Restore. Full SKIP_EXPORT pipeline. Commit.

### Task 2: The flicker law moves to Rust, bit-for-bit

**Files:** Create `rust/src/flicker.rs`; modify `rust/src/lib.rs` (module); create `game/tests/flicker_parity_test.gd`.
**Interfaces:** Produces `pub trait Randf { fn randf(&mut self) -> f64; }` and `pub struct Flicker` with `pub fn new() -> Flicker` (constants LEVEL_MIN 0.72, LEVEL_MAX 1.2, DROP_DEPTH 0.55, relax 0.12, jitter 0.09, first drop at 9.0, spacing 8.0+rand*10.0, length 0.08+rand*0.1 — transcribe from `game/scripts/flicker.gd:33-43` exactly, same rand-draw ORDER) and `pub fn next(&mut self, dt: f64, rng: &mut impl Randf) -> f64`; plus state accessors Task 5 needs for `capture_env`: `pub fn state(&self) -> FlickerState { t, level, drop_until, next_drop }` and `pub fn restore(&mut self, s: FlickerState)`. Also a registered thin wrapper is NOT created — UnseeingGame will own a `Flicker` + a `Gd<RandomNumberGenerator>` adapter implementing `Randf` (`rng.randf() as f64`).

- [ ] Cargo tests first (hand-derived): with a scripted stub Randf returning a fixed sequence, assert the exact envelope values for the first N steps (derive them by hand from the law — no mirror), the clamp bounds, the dropout compounding, and that a drop consumes exactly the draws flicker.gd would (call-count parity matters for the shared stream). Red (module absent) → implement → green.
- [ ] The equivalence harness, gdUnit: `flicker_parity_test.gd` seeds two `RandomNumberGenerator`s with 0x5EED, drives `Flicker.new(rng_a)` (the GDScript one) and a small registered test door — add `#[func] fn flicker_probe(seed: i64, frames: i64, dt: f64) -> PackedFloat64Array` on `WaveCore` (test-only surface, cheap, documented) that seeds its own RNG and runs the Rust law — and asserts the two arrays match exactly over 600 frames of varying dt (e.g. alternating 1/60 and 1/45). This is the bit-exactness proof; it must land while flicker.gd still exists.
- [ ] Mutation: flip the f32→f64 cast to a truncation or reorder two draws — parity must fail. Restore. Gates + pipeline. Commit.

### Task 3: The demo tap schedule moves to Rust

**Files:** Create `rust/src/demo_tap.rs`; modify `rust/src/lib.rs`.
**Interfaces:** Produces `pub struct DemoTap { pub armed: bool, pub point: Vector3, pub normal: Vector3 }` + `pub fn new(point, normal)`, `pub fn fire_due(&mut self, now: f64) -> bool` (FIRST_AT 0.6, REPEAT_EVERY 4.0, `_next = now + REPEAT_EVERY` on fire), and `pub fn next_at(&self) -> f64` / `pub fn restore_next(&mut self, next: f64)` for `capture_env`. Consumed by Task 4.

- [ ] Cargo tests first, transcribing `demo_tap_test.gd`'s law into hand-derived cargo form: armed schedule fires exactly 3 times in 10 s at ~0.6/4.6/8.6; unarmed fires 0; `_next` rides actual fire time. Red → implement → green. Mutation: make `_next` ride the due date instead of fire time — a test must fail. Restore. Commit. (The gdUnit `demo_tap_test.gd` stays green untouched until Task 7 retires it with the .gd.)

### Task 4: UnseeingGame — ready side

**Files:** Create `rust/src/nodes/game.rs`; modify `rust/src/nodes/mod.rs`; visibility bumps (`pub(super)`) on: `WaveLevel::{inject, tick_sources, cats, demo_tap, demo_tap_normal}` (level.rs), `UnseeingPlayer::{tick, ensure_actions}` (player.rs), `HeroBody::update` (hero.rs), `WaveObserver::{inject, inject_hero, inject_body}` (observer.rs), `WaveRestorer::{inject, restore}` (restorer.rs); add `ERROR: UnseeingGame` to `ci/boot_error_pattern.sh:38`; add `UnseeingGame` to both class rosters; create `game/tests/game_root_test.gd`.
**Interfaces:** Produces the `UnseeingGame` class (`#[class(init, base=Node3D)]`, NOT tool, in `game.rs`): fields for the five materials, `wave_core: Option<Gd<WaveCore>>`, `level/player/hero/settings/observer/restorer` handles, `#[var] now: f64` (writable — seed_test's contract), `flicker: Flicker` + `rng: Option<Gd<RandomNumberGenerator>>`, `demo: DemoTap`, `demo_checked: bool`. Observability surface (all `#[func]` unless noted): `wave_mats() -> Array<Gd<ShaderMaterial>>` AND `#[var]`-readable `data_mat/source_mat/cane_mat/body_mat/post_mat/level/player/hero/settings/observer/restorer` (use `#[var(get = …)]` computed getters returning the stored `Gd` clones so `is_same` identity holds), `flicker_seed() -> i64` (the RNG's `get_seed()`), `demo_armed() -> bool`, `seeded() -> bool`. Tasks 5–7 consume all of this.

`ready()` transliterates `main.gd:64-134` (merged shape — includes `observer.inject_body(hero)` and the restorer construction between observer and settings) in EXACTLY this order, with loud totality on every `try_load` (a failed shader load prints `"UnseeingGame: …"` and returns): ensure_actions → RNG (+0x5EED iff seeded — env `UNSEEING_SEED`/`UNSEEING_DEMO` or, under `has_feature("web")`, the JavaScriptBridge singleton's `window.location.search` containing "seed"/"demo") → materials (priorities 0/20, cane u_base 0.85) → `WaveCore::new_gd()` → level `try_load::<PackedScene>("res://scenes/level_01.tscn")` → `try_instantiate_as::<WaveLevel>` → `inject(data, source, core.upcast())` BEFORE add_child → DemoTap from the level's tap plan → post-mat wall table (u_walls/u_wall_count/u_wall_top from `level_plan::WALL_H`) → cats → player (pulses set, spawn pos/yaw, add) → hero (five fields, add) → post quad on camera → observer (inject, inject_hero, inject_body, add) → restorer (inject(level, player, hero, observer), add) → settings LAST.

- [ ] Failing gdUnit first: `game_root_test.gd` builds `UnseeingGame.new()` directly (no scene), adds it, and asserts the wiring contract synchronously: five mats with correct shaders and priorities and `is_same` identity between named properties and `wave_mats()` entries in order; `level` present with `wall_segments()` non-empty; `player.camera` live; `hero` wired; `settings` is the LAST child; `observer.snapshot(0.0)` not unavailable; `now == 0.0` and writable. Red (class absent) → implement → green. (This new suite is the migration's safety net BEFORE any old suite is touched.)
- [ ] Boot-gate self-test green (new pattern entry + literal openings); both rosters updated (16 names); cargo gates; full SKIP_EXPORT pipeline (old suites untouched and green — main.tscn still boots main.gd; UnseeingGame exists but nothing instantiates it in the shipped path yet). Mutation: reorder settings before observer — game_root_test's last-child assertion must fail. Restore. Commit.

### Task 5: UnseeingGame — process side and the env trio

**Files:** Modify `rust/src/nodes/game.rs`; extend `game/tests/game_root_test.gd`.
**Interfaces:** Produces `process()` transliterating `main.gd:137-155` order exactly (now += dt; player.tick; flicker → u_time/u_flick on all five; post u_breath `1.0 + sin(now*0.5)*0.045` and u_grain_t `fmod(now,1.0)*61.7`; `level.tick_sources(now, camera.global_position)` — the CAMERA eye; cat.tick loop; the apply loop: `core.tick(now)` then u_count/u_ppos/u_pdat/u_pdir to all five mats; hero.update; demo arming at `now >= 0.5` + fire via `player.queue_wave(0, point, 6.0, 5.5, 1.0, 6, normal)`). Plus the env trio as `#[func]`s with EXACT main.gd semantics (merged blob a42e8b5): `capture_env() -> Dictionary` (exactly 9 keys: now, demo_checked, demo_armed, demo_next, flicker_t, flicker_level, flicker_drop_until, flicker_next_drop, flicker_rng_state — sourced from the Rust `Flicker::state()`, `DemoTap::next_at()`, `rng.get_state()`), `apply_env(env: Dictionary)`, `restore_blob(blob: Dictionary) -> Dictionary` (pause bracket; `observer.env_of` refusal → early return; capture previous env; apply; `restorer.restore(blob, capture_env())`; on refusal roll previous back; post-write hash comparison against the blob's `hash` key → one-key `{"unavailable": …}` WITHOUT rollback; restore pause state).

- [ ] Failing tests first, extended in `game_root_test.gd`: (a) two process frames advance `now` and set `u_time` on all five mats; (b) `capture_env()` returns exactly the 9 keys with plausible values; (c) `apply_env(capture_env())` round-trips (capture → mutate now → apply → capture equals first); (d) a `restore_blob` of a fresh `observer.capture(now, capture_env())` blob returns `{"restored": true, "hash": …}` and a doctored blob hash returns one-key unavailable (mirror the assertions of main's `restore_transaction_test.gd` — read it first and reuse its fixture recipe). Red → implement → green.
- [ ] Mutations: (a) swap tick_sources' eye to the player body position — no current test catches it, so ADD the assertion that does: after positioning camera and body apart beside a wall, `source_muffle`-affected image differs (derive the fixture by hand from `source_test.gd:145-170`'s pattern); (b) reorder apply loop before tick_sources — the same-frame emission test must fail (adapt `source_test.gd:112-137`'s one-tick law against the root). Restore both. Gates + pipeline. Commit.

### Task 6: The switchover — main.tscn boots Rust

**Files:** Modify `game/scenes/main.tscn` (root node `type="UnseeingGame"`, script removed, ext_resource dropped); migrate `game/tests/wiring_test.gd`, `seed_test.gd`, `settings_test.gd`, `observer_test.gd`, `restore_test.gd`, `restore_transaction_test.gd`, `game/tests/probe/determinism_probe.gd`, `occlusion_probe.gd`, `display_probe.gd`, `restore_probe.gd` (retype `as UnseeingMain` → `as UnseeingGame`; private reaches become the designed surface: `main._flicker._rng.seed` → `main.flicker_seed()`, `main._demo.armed` → `main.demo_armed()`; `main.now = 1.0` stays — `#[var]` write); move `game/scripts/pulses.gd` → `game/tests/pulses.gd` (unchanged content, header comment gains one line: test-facing shim; engine uses WaveCore directly); delete `game/scripts/main.gd`; repoint `test/ci_gdscript_lint_scope.sh:73`'s sentinel from `game/scripts/main.gd` to `game/tests/pulses.gd` with the same rationale comment.
**Interfaces:** Consumes everything Tasks 4–5 produced. After this task the shipped game contains zero GDScript.

- [ ] Watch the failure first: flip main.tscn alone and run `wiring_test.gd` — red on the `UnseeingMain` cast (the right failure). Then migrate suites one file at a time, running each single-suite after `--import`.
- [ ] The boot check, determinism probe (two identical hashes), and restore probe (capture/restore legs) all run inside the full SKIP_EXPORT pipeline — every one must pass on the Rust boot. Expected counts: suites/cases unchanged from 31/259 (this task retypes, it does not add or remove).
- [ ] Verify by hand-run: `tools/probe_editor_sources.sh` etc. still green (they never touch main); `git grep -l "UnseeingMain"` returns nothing.
- [ ] Mutation: comment out the `ensure_actions()` call in `UnseeingGame::ready` — settings_test's click-tap case must fail (actions unregistered). Restore. Commit (one commit for the switchover; the suite migrations land with it — they are one behaviour: the boot changed owners).

### Task 7: The GDScript laws retire

**Files:** Delete `game/scripts/flicker.gd`, `game/scripts/demo_tap.gd`, `game/tests/flicker_test.gd`, `game/tests/demo_tap_test.gd`, `game/tests/flicker_parity_test.gd` (its job — proving the port against the original — is done and its subject is gone); remove the `flicker_probe` test door from `ffi.rs` if nothing else uses it.
**Interfaces:** none new. Expected counts SHRINK deliberately: state the exact before/after suite/case numbers in the report and the commit body (the laws now live in cargo — name the cargo tests that carry each retired assertion).

- [ ] Confirm each retired gdUnit assertion has a named cargo heir (Task 2/3 test lists); delete; `--import`; full suite run — counts match the predicted shrink exactly, nothing else red. Full pipeline. Commit.

### Task 8: The web path proven end to end

**Files:** none (verification task; fixes only if red).

- [ ] Run the FULL pipeline — `ci/pipeline.sh` with no SKIP_EXPORT: wasm core build, strict web export, browser smoke (`index.html?demo` in headless Chrome — the only gate on the JavaScriptBridge `?demo`/`?seed` arming, which desktop tests structurally cannot reach). Expected: `smoke: OK` with lit pixels; treat a silent Chrome-missing skip as FAILURE for this task (check the log line, not the exit code).
- [ ] If the smoke fails on the web arming, fix within this task (the dynamic-singleton eval path), re-run, and only then commit whatever fix was needed. Report the smoke transcript either way.

### Task 9: The razor becomes law, the docs catch up

**Files:** Modify `CLAUDE.md` (two-layers section: record the designer-facing razor as the criterion, note GDScript residue = designer scenes/knobs + tests; update the "3,814 GDScript lines" strain note which is now obsolete), campaign spec (append a short **Erratum** section: the settings sentence's false premise, resolved as decision 1 of this plan), wiki-debt file (append SP4's page deltas: Mechanics Overview's file map, Engineering pages' boot description, the new class in Sound-Sources/Level pages where they mention main.gd).

- [ ] Write all three; every claim carries file:line; commit. (Wiki push still deferred to campaign merge.)

---

## Self-Review

1. **Spec coverage:** razor enforced by structure (T4-6), main.gd gone (T6), residue minimized (T6-7), CLAUDE.md law (T9) — spec §SP4 complete; the settings sentence resolved as an erratum, not silently.
2. **Placeholders:** none; where a task depends on merged-main code the implementer must read (`restore_transaction_test.gd` fixtures), the task says read-first explicitly.
3. **Type consistency:** `Randf`/`Flicker::state()`/`DemoTap::next_at()` (T2/3) consumed by T5's env trio; T4's observability surface consumed by T6's migrations; `max_pulses` (T1) referenced by T6's shim move.

## Post-rebase supersession (2026-08-13)

This plan is the frozen migration record. `UnseeingGame`, pure Flicker/DemoTap
laws, restore semantics, and the code-free `main.tscn` root remain current.

- The policy/razor references to `CLAUDE.md` are historical. `AGENTS.md` now
  owns the enforceable engine/content split and says GDScript is tests and
  probes only; `CLAUDE.md` merely includes it.
- A selected level remains content under `UnseeingGame`, not an independently
  playable raw `WaveLevel`. The later `level_scene` property preserves one
  inject-before-add path and is the code-free F6 authoring boundary.
- Class/count predictions inside individual tasks record their execution
  moment. The source-role checkpoint is 407 Cargo tests and 329 gdUnit cases
  in 31 suites, with 19 registered classes and ten icons; closeout recomputes
  final totals.
- Task 9's wiki work remains debt. Integration does not authorize wiki
  publication, deployment, or issue closure; each requires its own user
  decision.
