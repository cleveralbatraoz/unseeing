# Walls Stop Every Source Wave — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a wall an absolute barrier to every sound wave, so no sound source reveals geometry or sounds its shell through a wall, for every source kind natively.

**Architecture:** The reveal law moves into one pure cargo-tested function, `sight::reveal_visibility`, which the GLSL `source_reveal_vis` transliterates. Pulse kind stops changing the answer at a wall, so `source_reveal_vis` loses its `typ` parameter and `HUM_THROUGH` is deleted from both languages. The source *silhouette* (`SOURCE_THROUGH`) is untouched — it belongs to a separate approved spec.

**Tech Stack:** Godot 4.7 (GL Compatibility), typed GDScript for tests/probes only, Rust GDExtension via gdext, gdUnit4, plain `cargo test`.

**Spec:** `docs/superpowers/specs/2026-08-14-walls-stop-source-waves-design.md`

## Global Constraints

These apply to EVERY task. They are not optional and not summarised elsewhere.

- **Perception laws.** Render black and white, thin outlines only: no textures, fills, materials, or visual noise. The world is revealed only by sound, touch, and wind waves. Keep UI/UX simple and minimal.
- **Superface merge law.** One silhouette per object. Same-facing coplanar overlapping faces merge into one superface sharing one per-vertex label bit-for-bit. Separate touching solids need labels at least `MIN_SEP` = 0.08 apart. Labels live in the sRGB-safe band `[0.15, 0.96]`, with the one grandfathered `Role::Case` = 0.05 exception. The merge law is `rust/src/render/superface.rs`; colouring and roles are `rust/src/render/labels.rs`. Never assign labels by cycling a list. **No task in this plan changes labels or geometry** — if a change here appears to require it, stop and report.
- **Two code layers.** Law 1: everything a designer meets is a registered Rust node or a `.tscn` composed of them. Law 2: everything else lives in Rust. Shipped GDScript is forbidden; GDScript is tests and probes only, permanently.
- **Architecture laws.** Domain logic pure and engine-free; every function total over its declared input domain (no panics, no NaN/Infinity, no blind indexing); dependencies injected, not reached for; global mutable state forbidden. `#![deny(unsafe_code)]` holds.
- **Platforms.** x86_64 and arm64 both. macOS universal, Windows x86_64 + arm64, Rust targets both desktop architectures plus wasm32. Never write platform-specific implementations; `game/` is the sole project and source of truth.
- **Strict TDD.** For every behaviour change: write the test, observe the correct failure, add minimal code, observe the pass, then refactor. Delete production code written before its test. Every test names the break it catches. No mirror assertions and no constant-change detectors — hand-derive literals. Before finishing a task, mutation-check its constants and branches; each mutation must fail a test.
- **Debug with structured state, never screenshots.** `WaveObserver` and `rust/src/observe/` expose pulse, eviction, occlusion, crossing and reflection state as data — `snapshot` and `explain_ray` are the first tools to reach for on any unexpected result, and a screenshot is a last resort that signals a missing observability surface. Note the standing caveat: `explain_ray` reports what **Rust** believes and can never prove the GLSL agrees; only Task 5's rendered probe closes that gap, and no task may claim it did otherwise.
- **Commits.** Small, self-contained, green: one behaviour each, with its test. Evocative narrative subject matching repository history, body explaining the precise what and why. Repository identity is `Dmitrii Galchenko <dggrus@gmail.com>`. **Never** add `Co-Authored-By`, `Generated with`, or any assistant attribution in commits, code, comments, docs, or PRs. Do not paste literal commit messages from this plan — write your own.
- **Never commit** build output, exports, `.pck`, `.wasm`, `target/`, rendered frames, or reports.
- **Autonomy ends at integration.** Do not merge, push, or deploy. Do not run `deploy.sh`.

### Running the gates

The worktree is at `.claude/worktrees/walls-stop-source-waves`. Run everything from there.

```bash
# Rust — the fast loop
cd rust && cargo test
cd rust && cargo fmt --check && cargo clippy --all-targets -- -D warnings

# GDScript format/lint (required before any commit touching .gd)
gdformat game/tests/<file>.gd && gdlint game/tests/<file>.gd

# gdUnit4 — MUST import first
godot --headless --path game --import >/dev/null 2>&1 || true
./ci/run_gdunit.sh "$PWD/game" godot --headless --path "$PWD/game" \
  -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd --ignoreHeadlessMode -c -a tests

# The whole gate
./ci/pipeline.sh
```

**The gdUnit gate lies three ways** and you must defend against all three:
1. It exits 0 on a **parse failure**.
2. It can print a green `PASSED` on a line that carries failures.
3. **A fresh worktree with no `game/.godot` cache runs ZERO tests while exiting 0** — an empty run looks exactly like a green one.

Therefore: always run `--import` first, and **trust only the suite and case counts**. Baseline for this branch is **462 cargo tests** (verified green at `e6e5511`). Record the gdUnit suite/case counts on your first run and assert they do not drop.

---

## File Structure

| File | Responsibility | Task |
| --- | --- | --- |
| `rust/src/sight.rs` | Owns the pure reveal law `reveal_visibility` alongside the crossing predicates it already owns. | 1 |
| `rust/src/observe/ray.rs` | Reports the law as `RayExplanation::wave_transmission`. | 1 |
| `rust/src/nodes/observer.rs:1057` | Boundary adapter: publishes the renamed key. | 1 |
| `rust/src/level_plan.rs:31-34,1367` | Loses `HUM_THROUGH` and the doc reference to it. | 1 |
| `game/tests/observer_test.gd:37-40,721-730` | gdUnit half of the observer contract. | 1 |
| `game/shaders/data_core.gdshaderinc:65-86,125-140` | The shipped reveal law. | 2 |
| `game/shaders/hearing_post.gdshader:122-130` | The shell-in-the-air law. | 3 |
| `game/shaders/pulse_pool.gdshaderinc:18-24` | Loses the `HUM_THROUGH` constant. | 3 |
| `game/tests/shader_contract_test.gd:91-101` | Shader-text contract for the reveal law. | 2 |
| `game/tests/data_skins_test.gd:79-85` | Shader-text contract for the muffle vocabulary. | 2, 3 |
| `game/tests/probe/occlusion_probe.gd` | The only test that reads real pixels. | 4 |
| Wiki pages (waves, sound sources, rendering) | Describe the shipped law. | 5 |

---

### Task 1: The pure reveal law and its oracle

Move the reveal law into `sight.rs` as a kind-free pure function, make `explain_ray` report it, and delete the Rust half of `HUM_THROUGH`. No shader changes in this task — the game still renders the old behaviour after it, and that is expected.

**Files:**
- Modify: `rust/src/sight.rs` (add `reveal_visibility` + tests)
- Modify: `rust/src/observe/ray.rs:11,39-42,78-81` and its test module
- Modify: `rust/src/nodes/observer.rs:1057`
- Modify: `rust/src/level_plan.rs:31-34` (delete constant), `:1367` (doc reference)
- Modify: `game/tests/observer_test.gd:37-40,721-730`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces:
  - `pub const fn sight::reveal_visibility(source_crossings: u32) -> f64` — `1.0` when `source_crossings == 0`, else `0.0`.
  - `RayExplanation::wave_transmission: f64` replaces `hum_transmission: f64`.
  - Observer dictionary key `"wave_transmission"` replaces `"hum_transmission"`.

- [ ] **Step 1: Write the failing test for the pure law**

Append to the `#[cfg(test)] mod tests` block in `rust/src/sight.rs`:

```rust
    /// The reveal law is TOTAL and kind-free: a wave reveals fully when no
    /// wall stands between its source and the lit point, and reveals
    /// NOTHING once one does. Catches the break this branch exists to fix —
    /// any per-kind transmission privilege reintroduced here (a hum
    /// surviving at 0.55, say) makes the second assertion fail. The third
    /// pins that more walls cannot resurrect a wave.
    #[test]
    fn a_wall_extinguishes_a_wave_whatever_made_it() {
        assert!((reveal_visibility(0) - 1.0).abs() < 1e-12);
        assert!(reveal_visibility(1).abs() < 1e-12);
        assert!(reveal_visibility(2).abs() < 1e-12);
        assert!(reveal_visibility(u32::MAX).abs() < 1e-12);
    }
```

- [ ] **Step 2: Run it and watch it fail correctly**

Run: `cd rust && cargo test a_wall_extinguishes`
Expected: FAIL to compile — `cannot find function 'reveal_visibility' in this scope`. A compile error is the correct failure here; a passing test would mean the function already existed.

- [ ] **Step 3: Add the pure law**

Add to `rust/src/sight.rs`, near the other predicates:

```rust
/// How much of a wave's REVEAL survives the walls between its source and
/// the lit point.
///
/// A wall is a barrier no sound crosses, so this is a gate and not an
/// attenuation: full reveal with a clear line, nothing at all once any
/// wall stands in the way. Pulse kind is deliberately absent — a cane
/// tap, its echoes, a footstep and a world source's wave all stop at a
/// wall alike, and a parameter that cannot change the answer would be a
/// lie about the domain.
///
/// `source_crossings` comes from [`crossings_from`], which skips the wall
/// a source is born inside, so a sound struck flush on a wall still
/// lights that wall's own near face.
///
/// The GLSL `source_reveal_vis` in `game/shaders/data_core.gdshaderinc`
/// transliterates this function; the two are held in step by
/// `game/tests/shader_contract_test.gd`.
#[must_use]
pub const fn reveal_visibility(source_crossings: u32) -> f64 {
    if source_crossings == 0 { 1.0 } else { 0.0 }
}
```

- [ ] **Step 4: Run it and watch it pass**

Run: `cd rust && cargo test a_wall_extinguishes`
Expected: PASS, 1 passed.

- [ ] **Step 5: Write the failing test for the renamed oracle field**

In `rust/src/observe/ray.rs`, replace the two existing assertions on `hum_transmission`. In `one_wall_is_named_and_its_transmission_derived`, change:

```rust
        assert!((e.hum_transmission - 0.55).abs() < 1e-9);
```

to:

```rust
        // One wall stands between source and lit point, so the wave is
        // extinguished — 0.0, not a surviving fraction. The silhouette is
        // a DIFFERENT law and still dims to 0.30; asserting both here is
        // what keeps a future edit from collapsing the two.
        assert!((e.wave_transmission - 0.0).abs() < 1e-9);
```

Update that test's doc comment: the transmission is now `0.0` for the wave and `0.30^1` for the silhouette.

In `the_two_transmissions_use_their_own_occluder`, change `e.hum_transmission` to `e.wave_transmission` in both the assertion and its message; the expected value stays `1.0` (that fixture's source sits inside the wall it would otherwise cross, so the SOURCE occluder skips it while the CAMERA occluder still counts it). Update the doc comment to say a version exponentiating both by `camera_crossings` would report `0.0` here instead of `1.0`.

In `two_walls_compose_their_transmission`, the wave half no longer composes — two walls extinguish exactly as one does. Rewrite its wave assertion to `0.0`, keep the silhouette assertion at `0.09`, and rewrite the doc comment to say that composition is now the SILHOUETTE's law alone and the wave's answer is a gate.

- [ ] **Step 6: Run and watch it fail correctly**

Run: `cd rust && cargo test --lib observe::ray`
Expected: FAIL to compile — `no field 'wave_transmission' on type 'RayExplanation'`.

- [ ] **Step 7: Rename and redefine the field**

In `rust/src/observe/ray.rs`:
- Line 11: change the import to `use crate::level_plan::SOURCE_THROUGH;` and add `reveal_visibility` to the `crate::sight` import list.
- Replace the `hum_transmission` field and its doc with:

```rust
    /// How much of the source's WAVE survives — the shader's
    /// `source_reveal_vis`, keyed to the SOURCE occluder so a sound born
    /// flush on a wall still lights its own face. A gate, not a fade: a
    /// wall stops a wave whatever kind made it.
    pub wave_transmission: f64,
```

- Replace the field's construction and comment with:

```rust
        // sight::reveal_visibility is the law the GLSL source_reveal_vis
        // transliterates; it reads off the SOURCE occluder, which skips
        // the wall a source is born inside.
        wave_transmission: reveal_visibility(source_crossings),
```

- [ ] **Step 8: Run and watch it pass**

Run: `cd rust && cargo test --lib observe::ray`
Expected: PASS.

- [ ] **Step 9: Update the boundary adapter and delete the Rust constant**

In `rust/src/nodes/observer.rs:1057`, change the key and value:

```rust
    entry.set("wave_transmission", explanation.wave_transmission);
```

In `rust/src/level_plan.rs`, delete the `HUM_THROUGH` constant and its whole doc comment (lines 31-34 including the `///` block above it). In the `DIST_PACK_RANGE` doc at line ~1367, the sentence `Held to it by game/tests/shader_contract_test.gd, exactly as [`HUM_THROUGH`] is.` now names a deleted item and would break `cargo doc`; rewrite it to `Held to it by \`game/tests/shader_contract_test.gd\`.`

- [ ] **Step 10: Verify the whole Rust gate**

Run: `cd rust && cargo fmt && cargo test && cargo clippy --all-targets -- -D warnings`
Expected: PASS, **463 tests** (the 462 baseline plus the new `a_wall_extinguishes_a_wave_whatever_made_it`), 0 failures, no clippy warnings. If the count differs from 463, stop and report — a test was lost.

- [ ] **Step 11: Update the gdUnit observer contract**

In `game/tests/observer_test.gd`, delete the `HUM_THROUGH` constant (line ~40) and its doc comment (lines ~38-39). Replace the assertion at line ~730:

```gdscript
	assert_float(e["wave_transmission"]).is_equal_approx(0.0, 0.0001)
```

Rewrite the test's doc comment (lines ~719-722) to state the new law: the line crosses the one wall this fixture holds exactly once, born well clear of it, so the fan's WAVE is extinguished (0.0) while its silhouette survives at SOURCE_THROUGH — two different laws, which is the break this assertion catches.

- [ ] **Step 12: Format, lint, and run the gdUnit suite**

```bash
gdformat game/tests/observer_test.gd && gdlint game/tests/observer_test.gd
godot --headless --path game --import >/dev/null 2>&1 || true
./ci/run_gdunit.sh "$PWD/game" godot --headless --path "$PWD/game" \
  -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd --ignoreHeadlessMode -c -a tests
```

Expected: green. **Record the suite and case counts** in your report — they are the only trustworthy signal. A run reporting 0 suites is a failed run, not a pass.

- [ ] **Step 13: Mutation-check this task**

Prove the new tests bite. Apply each mutation, run the named command, confirm FAIL, then revert:
1. `reveal_visibility` returns `0.55` instead of `0.0` → `cargo test a_wall_extinguishes` must FAIL.
2. `reveal_visibility` uses `source_crossings <= 1` instead of `== 0` → `cargo test a_wall_extinguishes` must FAIL.
3. `wave_transmission: reveal_visibility(camera_crossings)` (wrong occluder) → `cargo test --lib observe::ray` must FAIL on `the_two_transmissions_use_their_own_occluder`.

Report the three FAIL lines verbatim.

- [ ] **Step 14: Commit**

Stage `rust/src/sight.rs`, `rust/src/observe/ray.rs`, `rust/src/nodes/observer.rs`, `rust/src/level_plan.rs`, `game/tests/observer_test.gd`. Write your own narrative subject and a body explaining that the wave's transmission became a gate owned by one pure function, that kind no longer reaches it, and that the silhouette law is deliberately untouched.

---

### Task 2: The shipped reveal law drops its kind privilege

Change the GLSL that actually renders. `HUM_THROUGH` stays declared in `pulse_pool.gdshaderinc` after this task because `hearing_post.gdshader` still references it; Task 3 removes both together so every commit compiles.

**Files:**
- Modify: `game/shaders/data_core.gdshaderinc:65-86` (the law), `:125-140` (the `bound` comment and the call site)
- Modify: `game/tests/shader_contract_test.gd:91-101`
- Modify: `game/tests/data_skins_test.gd:84`

**Interfaces:**
- Consumes: `sight::reveal_visibility` from Task 1 as the reference this GLSL transliterates.
- Produces: `float source_reveal_vis(vec3 src, vec3 world)` — note the dropped `typ` parameter; any later call site must not pass a kind.

- [ ] **Step 1: Write the failing shader-text contract**

In `game/tests/shader_contract_test.gd`, replace the body of `test_data_core_occludes_reveal_by_the_wall_table`:

```gdscript
func test_data_core_occludes_reveal_by_the_wall_table() -> void:
	var core := _read(CORE_PATH)
	assert_str(core).contains("float source_reveal_vis(vec3 src, vec3 world)")
	assert_str(core).contains("wall_crossings_from(src, world)")
	(
		assert_bool(core.contains("HUM_THROUGH"))
		. append_failure_message(
			"data_core still grants a wave kind a transmission privilege; a wall stops every sound"
		)
		. is_false()
	)
	var pool := _include_text()
	assert_str(pool).contains("int wall_crossings_from(vec3 from, vec3 to)")
	assert_str(pool).contains("bool wall_contains(vec4 rect, vec3 p, float top)")
```

Rewrite the test's doc comment above it: the data core counts the walls between a source and the lit point and extinguishes the reveal once there is one, for every kind alike; pinned as source text so the GLSL cannot drift from its cargo-pinned reference, `rust/src/sight.rs::reveal_visibility`.

- [ ] **Step 2: Run it and watch it fail correctly**

```bash
godot --headless --path game --import >/dev/null 2>&1 || true
./ci/run_gdunit.sh "$PWD/game" godot --headless --path "$PWD/game" \
  -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd --ignoreHeadlessMode -c -a tests/shader_contract_test.gd
```
Expected: FAIL — the signature assertion fails (the file still declares `float source_reveal_vis(float typ, ...)`) and the `HUM_THROUGH` absence assertion fails with the message above.

- [ ] **Step 3: Rewrite the law in the shader**

In `game/shaders/data_core.gdshaderinc`, replace lines 65-86 (the comment block and the function) with:

```glsl
// How much a wave still reveals `world`, given the WALLS between it and
// its source (props are transparent to waves — only walls obstruct). A
// wall is a BARRIER, not a muffle: the wave flows through openings and
// lights the next room through a doorway, and stops dead at a wall. Kind
// does not enter — a cane tap, its echoes, a footstep and a world
// source's hum are all cut crisp alike, which is why this takes no `typ`.
// EVERY wave obeys it, echoes included: an echo is a reflected wave, and
// a reflection is stopped by a wall like any sound — the reflection ray
// only decides where the echo is BORN, never that its reveal may pass
// through a wall.
// crossings_from skips the wall the source is born inside, so a tap or an
// echo struck ON a wall still lights that wall's own near face.
// rust/src/sight.rs::reveal_visibility is the cargo-pinned reference this
// transliterates.
float source_reveal_vis(vec3 src, vec3 world) {
	return wall_crossings_from(src, world) == 0 ? 1.0 : 0.0;
}
```

- [ ] **Step 4: Fix the call site and the `bound` comment**

At what is currently line 140, the call passes a kind. Change:

```glsl
			reveal = max(reveal, bound * source_reveal_vis(typ, u_ppos[i], world));
```

to:

```glsl
			reveal = max(reveal, bound * source_reveal_vis(u_ppos[i], world));
```

In the `bound` comment block (currently ~line 127), the phrase `a value in [0, 1] (1, 0, or HUM_THROUGH^walls)` names a range that no longer exists. Change that clause to `a value in [0, 1] (1 with a clear line, 0 behind a wall)`. Leave the rest of the early-out reasoning intact — it is still valid and still load-bearing.

- [ ] **Step 5: Check whether `typ` is now unused in `reveal_at`**

`typ` is still read at the `peak` line just above, so it stays. Confirm by eye that no other use of `typ` in `reveal_at` was tied only to the reveal gate. If the compiler or a lint reports an unused variable, remove it; otherwise change nothing.

- [ ] **Step 6: Update the muffle-vocabulary contract**

In `game/tests/data_skins_test.gd`, the test at ~line 79 asserts `HUM_THROUGH` appears in all three shader files. The data core no longer mentions it. Delete only this line for now (line ~84):

```gdscript
	assert_str(_text(CORE_PATH)).contains("pow(HUM_THROUGH, float(blocked))")
```

Leave the `POOL_PATH` and `POST_PATH` assertions — Task 3 removes them with the constant itself. Add a line to that test's doc comment noting the data core has left the muffle vocabulary because a wall now stops its wave outright.

- [ ] **Step 7: Run the suite and watch it pass**

```bash
gdformat game/tests/shader_contract_test.gd game/tests/data_skins_test.gd
gdlint game/tests/shader_contract_test.gd game/tests/data_skins_test.gd
godot --headless --path game --import >/dev/null 2>&1 || true
./ci/run_gdunit.sh "$PWD/game" godot --headless --path "$PWD/game" \
  -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd --ignoreHeadlessMode -c -a tests
```
Expected: green, with suite/case counts at or above Task 1's recorded numbers.

- [ ] **Step 8: Prove the shader still compiles in the real engine**

A GLSL signature change that breaks compilation does not always fail the text tests. Run the boot check:

```bash
. ./ci/boot_error_pattern.sh
OUT="$(godot --headless --path game --quit-after 30 2>&1)"
printf '%s' "$OUT" | grep -iE "$BOOT_ERROR_PATTERN" | head -10
```
Expected: no matching lines. Any shader compile error appears here and must be fixed before committing.

- [ ] **Step 9: Commit**

Stage the two shader/test groups. Narrative subject; body explains that the reveal gate stopped asking what made the wave.

---

### Task 3: The shell stops at the wall too, and the muffle vocabulary dies

**Files:**
- Modify: `game/shaders/hearing_post.gdshader:122-130`
- Modify: `game/shaders/pulse_pool.gdshaderinc:18-24`
- Modify: `game/tests/data_skins_test.gd:79-85`

**Interfaces:**
- Consumes: Task 2's completed reveal law (the two must not disagree about whether a wall is a barrier).
- Produces: no `HUM_THROUGH` identifier anywhere in the repository.

- [ ] **Step 1: Write the failing contract**

In `game/tests/data_skins_test.gd`, replace the remaining two `HUM_THROUGH` assertions in that test with an absence check across all three files:

```gdscript
	for path: String in [POOL_PATH, CORE_PATH, POST_PATH]:
		(
			assert_bool(_text(path).contains("HUM_THROUGH"))
			. append_failure_message(
				"%s still speaks the muffle vocabulary; a wall stops a wave outright" % path
			)
			. is_false()
		)
```

Rewrite the test's doc comment and rename the test to say what it now pins: no shader grants a sound a way through a wall. Use a name such as `test_no_shader_lets_a_wave_through_a_wall`.

- [ ] **Step 2: Run it and watch it fail correctly**

```bash
godot --headless --path game --import >/dev/null 2>&1 || true
./ci/run_gdunit.sh "$PWD/game" godot --headless --path "$PWD/game" \
  -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd --ignoreHeadlessMode -c -a tests/data_skins_test.gd
```
Expected: FAIL twice — `pulse_pool.gdshaderinc` and `hearing_post.gdshader` both still contain `HUM_THROUGH`.

- [ ] **Step 3: Collapse the shell branch**

In `game/shaders/hearing_post.gdshader`, replace lines 122-130:

```glsl
				float mute = 1.0;
				if (typ < 2.5) {
					// a player-made ring dies at the world, and never washes an
					// x-rayed source seen through a wall — the hero's sound is in
					// another room from it
					if (t >= scene_d || seen_walled) { continue; }
				} else if (t >= scene_d) {
					mute = HUM_THROUGH;  // the hum passes the world, muffled
				}
```

with:

```glsl
				// EVERY ring dies at the world, whatever made it, and none
				// washes an x-rayed source seen through a wall. seen_walled is
				// not redundant with the depth test: the always-on-top source
				// skins corrupt packed depth at their own pixels, so scene_d
				// alone cannot be trusted on a ray that reaches a source
				// through a wall.
				if (t >= scene_d || seen_walled) { continue; }
```

Then delete `mute` from the accumulation at what is currently line 139 — `env * mute * cone * ...` becomes `env * cone * ...`.

- [ ] **Step 4: Delete the constant**

In `game/shaders/pulse_pool.gdshaderinc`, delete lines 18-24 — the `HUM_THROUGH` declaration and its entire comment block.

- [ ] **Step 5: Run the suite and the boot check**

```bash
gdformat game/tests/data_skins_test.gd && gdlint game/tests/data_skins_test.gd
godot --headless --path game --import >/dev/null 2>&1 || true
./ci/run_gdunit.sh "$PWD/game" godot --headless --path "$PWD/game" \
  -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd --ignoreHeadlessMode -c -a tests
. ./ci/boot_error_pattern.sh
OUT="$(godot --headless --path game --quit-after 30 2>&1)"
printf '%s' "$OUT" | grep -iE "$BOOT_ERROR_PATTERN" | head -10
```
Expected: suite green at recorded counts; boot check prints nothing. An undeclared-identifier error here means a `HUM_THROUGH` reference was missed.

- [ ] **Step 6: Confirm the identifier is gone repository-wide**

Run: `grep -rn "HUM_THROUGH" --include="*.rs" --include="*.gd" --include="*.gdshader" --include="*.gdshaderinc" . | grep -v '\.godot/'`
Expected: no output. Any hit outside `docs/` is a miss; hits inside `docs/superpowers/specs/` are historical record and correct to leave.

- [ ] **Step 7: Watch for the `seen_walled` risk**

The spec flags one uncertainty here: `seen_walled` now suppresses source shells too. If the suite is green but the probe in Task 4 shows a source's shell wrongly vanishing **inside the hero's own room**, the recorded fallback is to gate only on `t >= scene_d` for kind 3 and record why in the spec. Do not pre-emptively apply the fallback — it needs the probe's evidence first. Note this open question in your task report.

- [ ] **Step 8: Commit**

Narrative subject; body explains that the last privilege a standing source held over the hero's own sounds is gone and the muffle vocabulary with it.

---

### Task 4: The rendered probe covers the source-side leak

The bug the user reported is a **source** revealing through a wall; the existing probe only taps that wall with a **player** sound. This task adds the missing case and is the only evidence that GLSL agrees with Rust.

**Files:**
- Modify: `game/tests/probe/occlusion_probe.gd`

**Interfaces:**
- Consumes: Tasks 2 and 3's shipped shader laws.
- Produces: a recorded pass/fail with numbers, for the branch report.

- [ ] **Step 1: Read the probe end to end before editing**

Read `game/tests/probe/occlusion_probe.gd` completely and `tools/probe_visibility.sh` completely. The probe boots `main.tscn` windowed, positions the hero, samples named world points projected to screen, and counts checks. Follow its existing structure exactly — do not invent a new harness.

- [ ] **Step 2: Understand two hazards before writing the check**

**Hazard A — a vacuous pass.** `_peak_r` clamps each unprojected sample to the image bounds (`clampi(cx + dx, 0, img.get_width() - 1)`). A sample point that lands **off-screen therefore silently reads a border pixel**, which is black, and the check passes while measuring nothing. Step 4 is the only guard against this and is not optional.

**Hazard B — the existing tap contaminates the sample.** Case 1 and 2 strike `WALL_TAP = (6.25, 1.5, 4.06)`, which is *on the divider's west face* — exactly the surface a source-side check wants to read. A player tap lights that face legitimately (the birth-wall skip means a tap struck on a wall lights its own near face). The new check must therefore sample **away from the tap point** and settle long enough for the tap's ring and its echoes to die: range 6.0 m at 5.5 m/s is ~1.1 s of ring plus its fade tail.

Unlike cases 1 and 2, this check is **absolute, not a delta** — the fan hums continuously, so there is no "before" to subtract.

- [ ] **Step 3: Add the failing source-side check**

Add these constants next to the existing ones, with the doc comments:

```gdscript
## The hero at the spawn marker, in the room WEST of the Divider (x = 6.4,
## whose doorway spans z in [8, 12.4]). The fan at (8.6, 4.4) is east of
## it, and the fan's sight line to every point below crosses SOLID divider
## well clear of that doorway. Nothing the fan emits may light any of them.
const AT_SPAWN := Vector3(3.0, 0.9, 4.0)
## What the hero looks at from the spawn: the divider face, north of the
## tap point so the earlier cases' strike cannot be mistaken for a leak.
const SPAWN_AIM := Vector3(6.25, 1.0, 6.5)
## Spawn-room surfaces the fan must never reveal. Both are chosen so the
## fan-to-point line pierces the divider: (8.6, 4.4) -> (5.0, 6.0) crosses
## x = 6.4 at z ~= 5.38, inside the divider's z in [0.6, 8] run.
const SPAWN_SIDE: Array[Vector3] = [
	Vector3(6.25, 1.5, 6.5),  # the divider's WEST face, fan directly behind it
	Vector3(5.0, 0.0, 6.0),  # spawn-room floor
]
```

Add the third case immediately before the `_report()` call, after the existing case 2. The hearing quad is already hidden by then, so the read is the data pass:

```gdscript
	# 3 — SOURCE REVEAL: the fan hums untouched behind the divider and the
	# hero stands in the room beyond it. No tap, no footstep — the only
	# wave in flight is the fan's own, and a wall stops it dead. Absolute,
	# not a delta: a continuous source has no "before" to subtract.
	main.player.position = AT_SPAWN
	main.player.camera.look_at(SPAWN_AIM, Vector3.UP)
	# outlast the case-2 tap and its echoes (6.0 m at 5.5 m/s, plus tail)
	await _settle(200)
	var leak := await _peak_r(main, SPAWN_SIDE, 26)
	print("# occlusion @spawn: fan lifts spawn-room reveal %.3f" % leak)
	_check(
		"the fan does NOT reveal the spawn room through the divider (%.3f < 0.08)" % leak,
		leak < 0.08
	)
```

- [ ] **Step 4: Verify the check fails on the OLD law — the anti-vacuity guard**

This step proves the probe measures anything at all, and is the only defence against Hazard A. Temporarily restore the old law in `game/shaders/data_core.gdshaderinc`: signature `float source_reveal_vis(float typ, vec3 src, vec3 world)` returning `pow(HUM_THROUGH, float(blocked))` for `typ > 2.5`, with a local `const float HUM_THROUGH = 0.55;` so it compiles, and pass `typ` at the call site again. Then:

```bash
GODOT=godot ./tools/probe_visibility.sh
```

Expected: the new spawn check FAILS with a leak **at or near 0.55 × the local attenuation** — a clearly non-zero number. **Record it.** Then restore the fixed law:

```bash
git checkout -- game/shaders/data_core.gdshaderinc
```

If the check **passes** under the old law, the probe is measuring nothing — the sample points are off-screen, occluded by nearer geometry, or the camera is aimed wrong. Fix the probe and repeat until it fails here. Do not proceed to Step 5 until this failure has been observed; a check that cannot fail is worse than no check, because it reports safety.

- [ ] **Step 5: Verify it passes on the new law**

Run: `GODOT=godot ./tools/probe_visibility.sh`
Expected: PASS, all three checks, both boots agreeing. The script runs twice by design — the warm-boot law says the first boot after a shader edit compiles different GL programs than every boot after it, so only a reproduced PASS counts. **Record both verdicts and the measured numbers.**

- [ ] **Step 6: Debug with structured state if it disagrees**

If the probe fails on the new law, do **not** reach for screenshots. Use `WaveObserver.explain_ray` from a dump scene to ask what Rust believes about the fan-hub-to-sample-point line, and compare `wave_transmission` and `source_crossings` against the probe's pixel. A Rust answer of `0.0` with a bright pixel localises the bug to the GLSL; matching answers localise it to the sample points. Record which.

- [ ] **Step 7: Format, lint, commit**

```bash
gdformat game/tests/probe/occlusion_probe.gd && gdlint game/tests/probe/occlusion_probe.gd
```
Narrative subject; body explains that the probe now watches the source-side leak the report actually described, and records that it was proven to fail against the old law.

---

### Task 5: Prove the gate bites, then rewrite the wiki

**Files:**
- Modify: wiki pages covering waves, sound sources, and rendering
- Modify: `docs/superpowers/specs/2026-08-14-walls-stop-source-waves-design.md` (only if the `seen_walled` fallback was taken)

- [ ] **Step 1: Run the full gate**

Run: `./ci/pipeline.sh`
Expected: every stage green. Record the cargo count (**463**), the gdUnit suite and case counts, and confirm no stage was skipped. If the pipeline exits 0 having run 0 gdUnit suites, that is a FAILED run — re-run after `--import`.

- [ ] **Step 2: Mutation-check the shipped shader law**

Each mutation must be caught by a named test. Apply, run, confirm FAIL, revert:
1. `data_core.gdshaderinc`: restore `pow(0.55, float(blocked))` for `typ > 2.5` → `shader_contract_test.gd` and `data_skins_test.gd` must FAIL.
2. `data_core.gdshaderinc`: flip the gate to `!= 0` → the **rendered probe** must FAIL (the text tests will not catch this, which is exactly why Task 4 exists — state this plainly in the report).
3. `hearing_post.gdshader`: delete the `|| seen_walled` term → record whether any test catches it. If none does, say so explicitly rather than claiming coverage.

Report each result verbatim, including any mutation that **nothing** catches.

- [ ] **Step 3: Rewrite the wiki**

Read the wiki pages on wave propagation, sound sources, and rendering. Every passage describing the hum passing walls muffled at 0.55 is now false. Rewrite them to state the shipped law: a wall is an absolute barrier to every sound wave regardless of kind; a wave lights the next room only through a doorway; the reveal gate is owned by `rust/src/sight.rs::reveal_visibility` and transliterated by `game/shaders/data_core.gdshaderinc`. Name the file owning each quoted constant, as the project requires.

State plainly on the sound-sources page what did **not** change: a source's own silhouette still shows through walls at `SOURCE_THROUGH` = 0.3 (`rust/src/level_plan.rs`), so a source in another room is visible but silent — and that gating it on having once been heard is a separate planned change.

- [ ] **Step 4: Record the coverage finding**

Add a short note to the build/test wiki page: the automated suite cannot see a shader-reveal leak. Every automated occlusion test asserts a crossing count in Rust or a substring of shader source; only `game/tests/probe/occlusion_probe.gd` reads pixels, and `tools/probe_visibility.sh` excludes it from CI because headless renders nothing. Anyone changing the reveal or shell law must run that probe by hand.

- [ ] **Step 5: Commit**

Narrative subject; body explains the documentation now matches the shipped law and records the gate's known blind spot.

---

## Definition of Done

- [ ] `cargo test` green at 463, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean.
- [ ] gdUnit4 green with suite and case counts recorded and not below baseline.
- [ ] `./ci/pipeline.sh` green end to end.
- [ ] `tools/probe_visibility.sh` run by hand, passing twice, with numbers recorded — **and** proven to fail against the old law. The failure must be attributable to the wall law: an absolute reading of a swept source can go dark for a phase of the sweep, and an uninvited tap can light the room, so a run that fails for either reason does not discharge this box.
- [ ] `grep -rn "HUM_THROUGH"` returns nothing outside `docs/` **and** `game/tests/`, whose two absence assertions must name the identifier in order to forbid it (Task 3 Step 3 adds them). Note this grep is a tripwire, not the guard — see Task 3.
- [ ] Mutation evidence recorded for every item in Task 1 Step 13 and Task 5 Step 2, including any mutation nothing catches.
- [ ] Wiki rewritten to the shipped law.
- [ ] Code review requested and its findings verified against the codebase before acceptance.
- [ ] **Not merged, not pushed, not deployed** — the finish-branch choice goes to the user.
