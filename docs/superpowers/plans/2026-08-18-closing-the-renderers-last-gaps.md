# Closing the Renderer's Last Gaps Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-encode the G channel as an identity so 40 shipped solid pairs stop melting into one silhouette, stop a player's ring washing over a prop-hidden sound source, and put the rendered occlusion probe into CI so its checks stop rotting.

**Architecture:** Three independent changes, deliberately ordered by blast radius. Gap 1 changes how one shader compares two numbers and how `render::labels` allocates them — the allocator is kept, its palette grows from 5 entries to hundreds. Gap 2 adds one pure Rust predicate, one per-frame CPU loop over sources, and one small uniform read by the post pass only. Gap 3 adds no domain logic at all: it wraps an existing script in a software-GL environment.

**Tech Stack:** Godot 4.7.1 (`gl_compatibility`), GDExtension Rust (gdext 0.5.4), typed GDScript for tests and probes only, gdUnit4, plain `cargo test`, POSIX sh.

**Spec:** `docs/superpowers/specs/2026-08-18-closing-the-renderers-last-gaps-design.md`

## Global Constraints

Every task's requirements implicitly include these.

- **Perception laws.** Black and white, thin outlines only — no textures, fills, materials or visual noise. The world is revealed only by sound, touch and wind waves. One silhouette per object.
- **The merge law is untouched.** Faces that are same-facing and coplanar merge into one superface and share one per-vertex label bit-for-bit (`rust/src/render/superface.rs`, `COPLANAR_EPS`, `PATCH_EPS`). Bends, steps and seams between *separate* touching solids still draw.
- **Labels live in the sRGB-safe band `[0.15, 0.96]`**, with one grandfathered exception (`Role::Case` at 0.05). Never assign labels by cycling a list.
- **Two code layers.** Law 1: everything a designer meets is a registered Rust tool node or a `.tscn` composed from them. Law 2: everything else lives in Rust. Registered nodes are thin boundary adapters; pure laws live in cargo-tested engine-free modules. GDScript is tests and probes only, permanently.
- **Platforms.** x86_64 and arm64; macOS universal; Windows x86_64 and arm64; Rust targets both desktop architectures plus wasm32. One Godot project exported to web, macOS and Windows — never platform implementations.
- **Totality.** Every function total over its declared input domain: no panics, no blind indexing, no unbounded loops, no NaN/Infinity, `Option`/`Result` for absence. `#![deny(unsafe_code)]`.
- **No global mutable state.** Dependency injection for clocks, randomness, configuration, world queries.
- **Strict TDD.** Write the test, observe the correct failure, add minimal code, observe the pass, refactor. Every test names the break it catches. No mirror assertions, no constant-change detectors. Mutation-check realistic constants, branches and early returns before finishing.
- **Commits.** Small, self-contained, green — one behaviour each with its test. Evocative narrative subject, body explaining the precise what and why. Repository identity `Dmitrii Galchenko <dggrus@gmail.com>`.
- **Attribution ban.** Never add `Co-Authored-By`, `Generated with`, or any assistant attribution in commits, code, comments, docs or PRs.
- **Never commit build output** — no exports, `.pck`, `.wasm`, `target/`, rendered frames or reports. Commit Godot `.import` and `.uid` sidecars.
- **Autonomy ends at integration.** Present the finish-branch choice; never merge, push or deploy without the user's choice.
- **Measured platform facts** (do not re-derive from memory): one data channel is **RGB10_A2, 1024 codes**, but does not hand back a clean code everywhere — `render::channel::WORST_STEP_CODES` records the widest gap measured, **1.25 nominal codes** on Mesa/AMD desktop GL against 1.02 on SwiftShader and ANGLE/Apple Metal. `DIST_PACK_RANGE = 40.0`; one NOMINAL B code is `40/1023 = 39.1 mm` and the worst real gap is `40 × 1.25/1023 = 48.9 mm`; the silhouette knee `smoothstep(0.012, 0.03, lap)` fires at a `0.012 × 40 = 0.48 m` depth step. **Corrected 2026-08-18** — the plan below was written against 1024 clean levels and every label-spacing figure in it is optimistic by 1.25×.

---

## File Structure

**Gap 1 — G as identity**
- `game/shaders/hearing_post.gdshader` — the crease comparison becomes an identity test.
- `rust/src/render/crease.rs` — `CreaseKnee` becomes a one-code epsilon, not a `MIN_SEP`-derived pair.
- `rust/src/render/labels.rs` — the ladder becomes a code grid; the palette grows.
- `rust/src/render/paint_plan.rs` — `MAX_PALETTE_VALUES`, band validation.
- `rust/src/observe/oids.rs` — the seam census's `separated` becomes "distinct".
- `game/tests/{shader_contract,data_skins,map,wiring}_test.gd` — the pins follow.

**Gap 2 — the prop ring cut**
- `rust/src/sight.rs` — a new pure predicate for prop occlusion of a source.
- `rust/src/nodes/level.rs` — `tick_sources` computes the flag; a new `#[func]` publishes the table.
- `rust/src/nodes/game.rs` — registers `post_mat` for the table (the level already owns the push).
- `game/shaders/hearing_post.gdshader` — the ring cut ORs the flag.
- `game/tests/probe/occlusion_probe.gd` — a rendered check for the radio behind its pillar.

**Gap 3 — the probe in CI**
- `tools/probe_visibility.sh` — an opt-in software-GL path.
- `ci/pipeline.sh` — a stage that runs it, failing loudly when the environment is absent.

---

### Task 1: The crease becomes an identity test

**Files:**
- Modify: `game/shaders/hearing_post.gdshader` (the `nrm` line and its comment)
- Modify: `rust/src/render/crease.rs`
- Test: `rust/src/render/crease.rs` (its own `mod tests`), `game/tests/shader_contract_test.gd`

**Interfaces:**
- Consumes: `render::channel::CHANNEL_LEVELS` (u32, 1024).
- Produces: `render::crease::LABEL_EPSILON: f64` — half a channel code, `0.5 / (CHANNEL_LEVELS - 1)`. `CreaseKnee::shipped()` is replaced by `crease::label_epsilon() -> f64`.

- [ ] **Step 1: Write the failing test**

In `rust/src/render/crease.rs`'s `mod tests`:

```rust
    /// THE break this catches: a crease that fades with label DISTANCE
    /// rather than answering label IDENTITY.
    ///
    /// The magnitude encoding never bought anything on screen. Every label
    /// the game ships is a rung 0.09 apart, so `nrm` is always either
    /// exactly 0 or at least 0.09, and `smoothstep(0.04, 0.08, nrm)`
    /// therefore returns exactly 0 or exactly 1 in every shipped frame —
    /// while the 0.08 separation it demanded capped the whole band at
    /// eleven labels. Identity draws the same picture and costs one code.
    ///
    /// Hand-derived from the measured channel: 1024 levels give 1023 steps,
    /// so one code is 1/1023 = 0.000978 and half a code — the epsilon that
    /// separates "the same label" from "a different one" without letting
    /// bit noise manufacture a seam — is 0.000489.
    #[test]
    fn a_crease_answers_identity_not_distance() {
        let eps = label_epsilon();
        assert!((eps - 0.000_489).abs() < 1.0e-6, "epsilon moved: {eps}");
        // two labels one RELIABLE gap apart are DIFFERENT and must draw.
        // NOT one nominal code: the channel collapses a nominal step to a
        // single code at some bases, so a palette spaced that way melts
        // exactly where the ladder says it would.
        let one_gap = channel::WORST_STEP_CODES / f64::from(channel::CHANNEL_LEVELS - 1);
        assert!(draws_a_crease(0.15, 0.15 + one_gap));
        // the same label, bit-for-bit, never draws — this is the merge law
        assert!(!draws_a_crease(0.15, 0.15));
        // ...and the old ladder spacing still draws, so nothing regresses
        assert!(draws_a_crease(0.15, 0.24));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --lib render::crease`
Expected: FAIL — `label_epsilon` and `draws_a_crease` are not defined.

- [ ] **Step 3: Write minimal implementation**

Replace `CreaseKnee` in `rust/src/render/crease.rs` with:

```rust
/// Half a channel code: the epsilon that separates "the same label" from "a
/// different one".
///
/// Half rather than one, so that neither rounding at the write nor bit
/// noise at the read can manufacture a seam between two labels that were
/// assigned the same value, and nothing narrower than a real code can hide
/// one.
#[must_use]
pub fn label_epsilon() -> f64 {
    0.5 / f64::from(channel::CHANNEL_LEVELS - 1)
}

/// Do two labels draw a seam between them?
///
/// Identity, not distance. Equality is exactly correct here and by
/// construction rather than by luck: `screen_tex` is `filter_nearest`,
/// `CUSTOM0` is piecewise constant per face (`render::paint` builds a box
/// with four vertices per face and a column so a flank never shares a
/// vertex with a rim), and nothing can smear it — MSAA, screen-space AA,
/// TAA, debanding and 3D scaling are all off.
///
/// Total over every f64 pair: a non-finite label answers `false`, since a
/// value the channel cannot hold cannot be seen to differ from anything.
#[must_use]
pub fn draws_a_crease(a: f64, b: f64) -> bool {
    a.is_finite() && b.is_finite() && (a - b).abs() >= label_epsilon()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --lib render::crease`
Expected: PASS.

- [ ] **Step 5: Port the comparison to GLSL**

In `game/shaders/hearing_post.gdshader`, replace the `nrm` line:

```glsl
	// IDENTITY, not distance. A label answers "which superface am I", and
	// the correct predicate for that is equality — exact here by
	// construction, because screen_tex is filter_nearest and CUSTOM0 is
	// piecewise constant per face, so the varying has no gradient inside a
	// face for a magnitude test to measure.
	//
	// Centre-vs-neighbour rather than left-vs-right: the old form compared
	// the two OPPOSITE taps and was blind to a one-pixel sliver of a
	// differing label between them.
	float nrm = max(
			max(step(u_label_eps, abs(c_c.g - c_l.g)), step(u_label_eps, abs(c_c.g - c_r.g))),
			max(step(u_label_eps, abs(c_c.g - c_u.g)), step(u_label_eps, abs(c_c.g - c_d.g))));
	float edge = max(smoothstep(0.012, 0.03, lap), nrm);
```

and replace the `u_crease_knee` uniform with:

```glsl
// Half a channel code, derived in Rust from the measured CHANNEL_LEVELS
// (rust/src/render/crease.rs::label_epsilon) and pushed by the composition
// root. The default is deliberately 1.0 — larger than any in-band label
// difference — so a post material nobody pushed draws NO creases at all,
// which is loudly wrong rather than accidentally right.
uniform float u_label_eps = 1.0;
```

- [ ] **Step 6: Update the push and the wiring pin**

In `rust/src/nodes/game.rs`, replace the `u_crease_knee` push with:

```rust
        self.post_mat.set_shader_parameter(
            "u_label_eps",
            &(render::crease::label_epsilon() as f32).to_variant(),
        );
```

In `game/tests/wiring_test.gd`, rename the case to
`test_the_label_epsilon_reaches_the_post_pass_from_the_measured_channel` and
assert the pushed float equals `WaveCore.new().label_epsilon()`, that the
shader reads `u_label_eps` and that `smoothstep(u_crease_knee` appears
nowhere. Add `#[func] fn label_epsilon(&self) -> f64` to `rust/src/ffi.rs`
returning `render::crease::label_epsilon()`.

- [ ] **Step 7: Run every gate**

Run:
```
cargo fmt --manifest-path rust/Cargo.toml --check
cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path rust/Cargo.toml
ci/run_gdunit.sh "$PWD/game" "$GODOT" --headless --path "$PWD/game" -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd --ignoreHeadlessMode -c -a tests
```
Expected: all green. Fix the pins that named `u_crease_knee` or `MIN_SEP`'s knee.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit
```
Subject in the repository's narrative register; body states that the crease was already binary in every shipped frame and that the magnitude encoding cost the entire palette for nothing.

---

### Task 2: The label ladder becomes a code grid

**Files:**
- Modify: `rust/src/render/labels.rs`
- Modify: `rust/src/render/paint_plan.rs:8-9,20` (`LABEL_MIN`, `LABEL_MAX`, `MAX_PALETTE_VALUES`)
- Modify: `rust/src/observe/oids.rs` (its `separated` read)
- Test: `rust/src/render/labels.rs`, `game/tests/map_test.gd`

**Interfaces:**
- Consumes: `render::crease::draws_a_crease`, `render::channel::CHANNEL_LEVELS`.
- Produces: `labels::WORLD_PALETTE` becomes `fn world_palette() -> Vec<f64>` of `PALETTE_SIZE = 64` entries spaced two codes apart from `LADDER_BASE`; `labels::MIN_SEP` is deleted; `labels::separated` delegates to `crease::draws_a_crease`.

- [ ] **Step 1: Write the failing test**

```rust
    /// THE break this catches: a palette still sized for a magnitude
    /// comparison. Under identity a label needs only to be DISTINCT, so the
    /// band holds hundreds rather than eleven — and the shipped map's 179
    /// superface classes stop competing for five slots.
    ///
    /// Hand-derived: the band [0.15, 0.96] is 0.81 wide; at 1024 levels one
    /// code is 1/1023, so the band holds 0.81 * 1023 = 828 codes, and at a
    /// two-code spacing 414 labels. Sixty-four is the size chosen — far
    /// above the 9 colours the shipped graph needs even at TOUCH_EPS = 1.0,
    /// and far below the ceiling, so the allocator never starves and the
    /// spacing stays legible.
    #[test]
    fn the_palette_is_sized_for_identity_not_for_distance() {
        let palette = world_palette();
        assert_eq!(palette.len(), PALETTE_SIZE);
        let two_codes = 2.0 / f64::from(channel::CHANNEL_LEVELS - 1);
        for pair in palette.windows(2) {
            assert!((pair[1] - pair[0] - two_codes).abs() < 1.0e-9);
            assert!(crease::draws_a_crease(pair[0], pair[1]));
        }
        assert!(palette.iter().all(|&l| (LADDER_BASE..=0.96).contains(&l)));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --lib render::labels`
Expected: FAIL — `world_palette`, `PALETTE_SIZE` not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
/// Labels the world palette offers. Sixty-four, spaced two channel codes
/// apart — comfortably above the nine colours the shipped separation graph
/// needs even at a generous adjacency, and comfortably below the ~414 the
/// band could hold, so the allocator never starves and every entry stays
/// distinguishable with a code to spare on either side.
pub const PALETTE_SIZE: usize = 64;

#[must_use]
pub fn world_palette() -> Vec<f64> {
    // two RELIABLE gaps, not two nominal codes: at 1.25 codes per gap this
    // is 2.5 nominal codes per slot, and 64 slots still fit the band
    let step = 2.0 * channel::WORST_STEP_CODES / f64::from(channel::CHANNEL_LEVELS - 1);
    (0..PALETTE_SIZE)
        .map(|slot| LADDER_BASE + step * slot as f64)
        .collect()
}
```

Keep `role_label` and the ten-rung ladder for the fixed roles: creatures and
the viewmodel are few and their spacing is already lawful. Replace
`separated(a, b)` with a delegation to `crease::draws_a_crease(a, b)` and
delete `MIN_SEP`, updating `coexisting_labels`'s test to assert every pair
*draws a crease* rather than clears 0.08.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml`
Expected: PASS, after `paint_plan::MAX_PALETTE_VALUES` is raised to `PALETTE_SIZE` and `LABEL_MIN`/`LABEL_MAX` are re-derived.

- [ ] **Step 5: Prove the melt is gone, on the shipped map**

Add to `game/tests/map_test.gd`:

```gdscript
## THE break this catches: two solids that never touch, share a label, and
## melt into one silhouette on screen.
##
## Forty shipped pairs did. The worst, EastBarrelA against EastBarrelB, is
## 0.036 m apart — smaller than one B quantum (39.1 mm), so the silhouette
## Laplacian cannot see it at any knee, and a shared label left nothing else
## to draw. Under an identity crease every distinct class draws its seam
## wherever it meets another on screen, touching or not.
func test_no_two_separate_solids_in_the_shipped_map_share_a_label() -> void:
	var level: WaveLevel = auto_free(LEVEL_SCENE.instantiate() as WaveLevel)
	level.inject(ShaderMaterial.new(), ShaderMaterial.new(), Pulses.new())
	add_child(level)
	var e: Dictionary = level.get_node("/root").get_tree()  # placeholder — see step 6
```

- [ ] **Step 6: Finish that test against the real observer**

Read the labels through `WaveObserver.explain_oids()`, which already returns
`names` and `oids`. Assert that for every pair of DISTINCT solids whose world
AABBs do not touch within `TOUCH_EPS`, the labels differ by at least
`WaveCore.new().label_epsilon()`. Derive the pair list from
`level.paint_entry_boxes()` if it exists; otherwise add a `#[func]` returning
each entry's world box, since the test must not recompute geometry the engine
already owns.

- [ ] **Step 7: Run it and watch it fail on the pre-Task-2 build, then pass**

Run the gdUnit suite. Expected before Task 2's palette: FAIL naming
`EastBarrelA`/`EastBarrelB`. After: PASS.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit
```

---

### Task 3: A source hidden by a prop stops being washed by rings

**Files:**
- Modify: `rust/src/sight.rs` (a new pure predicate)
- Modify: `rust/src/nodes/level.rs` (`tick_sources`, a new `#[func]`)
- Modify: `rust/src/nodes/game.rs` (register `post_mat`)
- Modify: `game/shaders/hearing_post.gdshader`
- Test: `rust/src/sight.rs`, `game/tests/map_test.gd`, `game/tests/probe/occlusion_probe.gd`

**Interfaces:**
- Consumes: `sight::Occluder`.
- Produces: `sight::blocked_by_any(from: Vector3, to: Vector3, blockers: &[Occluder]) -> bool`; `WaveLevel::blocked_source_boxes() -> PackedVector4Array` (xz min/max per flagged source) plus `blocked_source_spans() -> PackedVector2Array`; shader uniforms `u_blocked_src[MAXS]`, `u_blocked_src_y[MAXS]`, `u_blocked_src_count`, `MAXS = 8`.

- [ ] **Step 1: Write the failing test**

```rust
    /// THE break this catches: a source hidden behind a PROP being treated
    /// as visible, so a player's ring washes over it.
    ///
    /// Props are transparent to WAVES, deliberately — that law is not in
    /// question. This is the CAMERA's question, and it has a different
    /// answer. Hand-derived: an eye at x = 0 looking at a source at x = 4
    /// with a 0.5 m prop centred at x = 2 is blocked; step the eye aside to
    /// z = 2 and the same prop is missed entirely.
    #[test]
    fn a_prop_between_the_eye_and_a_source_blocks_the_camera_not_the_wave() {
        let prop = Occluder::new(Vector4::new(1.75, -0.25, 2.25, 0.25), 0.0, 2.0)
            .expect("a finite prop box");
        let eye = Vector3::new(0.0, 1.0, 0.0);
        let source = Vector3::new(4.0, 1.0, 0.0);
        assert!(blocked_by_any(eye, source, &[prop]));
        assert!(!blocked_by_any(Vector3::new(0.0, 1.0, 2.0), source, &[prop]));
        // ...and an empty table blocks nothing, so a level that pushes no
        // props degrades to exactly its former behaviour
        assert!(!blocked_by_any(eye, source, &[]));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml --lib sight::`
Expected: FAIL — `blocked_by_any` not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
/// Does any of `blockers` stand between `from` and `to`?
///
/// The CAMERA's question, and deliberately NOT the wave's: `blocked_from`
/// answers for waves and walls, skips the wall a sound is born inside, and
/// must never see a prop, because props are transparent to sound by design.
/// This one sees props and has no birth-wall rule, because an eye is not
/// born anywhere.
///
/// Total on any input: an empty table answers `false`, so a level that
/// pushes nothing degrades to exactly its former behaviour.
#[must_use]
pub fn blocked_by_any(from: Vector3, to: Vector3, blockers: &[Occluder]) -> bool {
    blockers.iter().any(|occ| crosses(from, to, *occ))
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path rust/Cargo.toml --lib sight::`
Expected: PASS.

- [ ] **Step 5: Compute the flag per source, per frame**

In `rust/src/nodes/level.rs`'s `tick_sources`, after the muffle is computed,
test `sight::blocked_by_any(eye, hub, &self.prop_blockers)` and collect the
world box of each blocked source into `self.blocked_sources`. Build
`self.prop_blockers` in `derive()` from the census's props, columns and
wedges, using each node's `world_shape()` through `render::faces::bounds` —
the same path the wall occluders take. Cap at `MAXS = 8` blocked sources and
say so through `level_plan`'s existing `Budget`/`Severity` channel when the
cap is hit.

- [ ] **Step 6: Push it and read it**

`WaveLevel::push_wall_table` also pushes `u_blocked_src`, `u_blocked_src_y`
and `u_blocked_src_count` to the registered `post_mat`. In
`hearing_post.gdshader` the ring cut becomes:

```glsl
			if (t >= scene_d || seen_walled || seen_blocked_source) { continue; }
```

with `seen_blocked_source` computed once per fragment, gated behind
`seen_image` so it costs nothing off a source's own pixels:

```glsl
	bool seen_blocked_source = false;
	if (seen_image) {
		for (int i = 0; i < MAXS; i++) {
			if (i >= u_blocked_src_count) { break; }
			if (wall_contains(u_blocked_src[i], seen_pt, u_blocked_src_y[i])) {
				seen_blocked_source = true;
			}
		}
	}
```

- [ ] **Step 7: A rendered check on the shipped geometry**

Add to `game/tests/probe/occlusion_probe.gd` a pose in the radio room's
prop shadow — the wedge x 24.5…26.9, z 4.1…6.0 — and assert that a cane tap's
ring does not lift the radio's own pixels, as a RATIO of the radio's standing
image, with a non-vacuity guard on the denominator. Prove it can fail by
reverting `seen_blocked_source` to `false` and watching the reading rise.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit
```

---

### Task 4: The rendered probe runs in CI

**Files:**
- Modify: `tools/probe_visibility.sh`
- Modify: `ci/pipeline.sh`
- Test: `test/ci_probe_gate.sh` (new self-test, in the style of `test/ci_boot_error_gate.sh`)

**Interfaces:**
- Consumes: nothing new.
- Produces: `PROBE_SOFTWARE_GL=1` env knob on `tools/probe_visibility.sh`.

- [ ] **Step 1: Write the failing self-test**

`test/ci_probe_gate.sh` asserts that `ci/pipeline.sh` contains a stage
invoking `tools/probe_visibility.sh`, and that the stage fails loudly rather
than skipping when `xvfb-run` is absent — the same law `test/web_smoke.sh`
applies to a missing browser.

- [ ] **Step 2: Run it to verify it fails**

Run: `test/ci_probe_gate.sh`
Expected: FAIL — no such stage.

- [ ] **Step 3: Add the software-GL path**

In `tools/probe_visibility.sh`, when `PROBE_SOFTWARE_GL=1`, wrap `run_scene`:

```sh
run_scene() {
  if [ "${PROBE_SOFTWARE_GL:-}" = "1" ]; then
    LIBGL_ALWAYS_SOFTWARE=1 UNSEEING_SEED=1 $KEEP_AWAKE \
      dbus-run-session xvfb-run -a "$GODOT" --path "$DIR/game" "$@"
  else
    UNSEEING_SEED=1 $KEEP_AWAKE "$GODOT" --path "$DIR/game" "$@"
  fi
}
```

`dbus-run-session` is not decoration: AccessKit aborts without a session bus.

- [ ] **Step 4: Re-measure the thresholds under llvmpipe**

Run the probe with `PROBE_SOFTWARE_GL=1` and record every reading. The
windows are durations on the SIMULATED clock, so a slower rasteriser collects
fewer samples rather than taking longer — but the noise floor and the
warm-boot-pair law were both established on a real GPU and must be
re-established here. Record the numbers in the probe's own docstring beside
the real-GPU ones; do not silently widen a floor.

- [ ] **Step 5: Add the CI stage**

In `ci/pipeline.sh`, after the gdUnit stage:

```sh
echo "ci: rendered occlusion probe (software GL)"
command -v xvfb-run >/dev/null 2>&1 || {
  echo "ci: FAILED xvfb-run not found (apt install xvfb) — an absent display is a broken host, not a pass"
  exit 2
}
PROBE_SOFTWARE_GL=1 GODOT="$GODOT" "$DIR/tools/probe_visibility.sh"
```

- [ ] **Step 6: Run the self-test and the pipeline**

Run: `test/ci_probe_gate.sh` then `ci/pipeline.sh`
Expected: both green.

- [ ] **Step 7: Commit**

```bash
git add -A && git commit
```

---

### Task 5: Retire what these changes made obsolete

**Files:**
- Modify: `docs/superpowers/specs/2026-08-11-pixel-oracle-gate-design.md` (mark superseded)
- Modify: `docs/superpowers/handoffs/2026-08-18-rendering-design-audit-state.md`
- Modify: the wiki's `Mechanics-Rendering.md`, `Engineering-Build-Test-Deploy.md`

- [ ] **Step 1: Mark the oracle spec superseded**

State that `expect_lit(walls_between, kind)` degenerated to `walls == 0` with
a dead `kind` parameter when the barrier campaign deleted `HUM_THROUGH`, that
its comparison is already implemented inline at `occlusion_probe.gd`'s checks
10 and 11, and that `rust/src/observe/oracle.rs` is deliberately not built.

- [ ] **Step 2: Rewrite the wiki's label section**

The ladder is a code grid; the crease is an identity; `MIN_SEP` is gone. Say
what the magnitude encoding cost and why it bought nothing.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit
```

---

## Self-Review

**Spec coverage.** Gap 1 → Tasks 1 and 2. Gap 2 → Task 3. Gap 3 → Task 4.
Finding 0 is already landed (`7ced8a0`). The spec's "what this does not cover"
items are deliberately absent from this plan.

**Placeholder scan.** Task 2 Step 5 contains a deliberate placeholder line
marked as such and resolved in Step 6 — the test needs an accessor whose
existence must be checked in the tree first. That is the one place this plan
tells the implementer to look rather than showing the code, and it is flagged.

**Type consistency.** `label_epsilon()` is used by Tasks 1 and 2 and exposed
through `ffi.rs` under the same name. `draws_a_crease` replaces `separated`
everywhere. `blocked_by_any` is named identically in Task 3's test and
implementation. `MAXS = 8` is used consistently in the Rust cap and the GLSL
array bound.

**Known weakness.** Task 2 Step 6 depends on an accessor that may not exist;
if it does not, that step grows a `#[func]` and its own test. Task 4 Step 4
cannot be planned further without running it — the thresholds are a
measurement, not a derivation, and the plan says so rather than inventing
numbers.
