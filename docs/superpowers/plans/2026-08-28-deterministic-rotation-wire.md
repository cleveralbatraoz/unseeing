# Deterministic rotation wire law — plan

**Spec:** `docs/superpowers/specs/2026-08-28-deterministic-rotation-wire-design.md`
**Branch:** `issue-64-hero-elevation` (existing worktree
`.worktrees/issue-64-hero-elevation`; the fix belongs to PR #81, whose CI it
turns green). No new worktree: this is the same task's branch.

## Global constraints (carry into every implementer brief verbatim)

- **Perception laws:** black and white, thin outlines only; the world is
  revealed by sound, touch and wind waves; reveal composes by `max`; occlusion
  is a `{0,1}` gate; there is no frequency axis and no acoustic derivation may
  justify any constant. Superface merge law: same-facing coplanar faces merge
  and share one label bit-for-bit; separate touching solids need labels
  ≥ `MIN_SEP` = 0.08 apart; labels live in `[0.15, 0.96]`; never assign labels
  by cycling a list.
- **Platforms:** everything must work on x86_64 and arm64; macOS, Windows,
  web (wasm32). No architecture-conditional compilation for behaviour;
  runtime portability checks only.
- **Two layers:** everything a designer meets is a registered Rust tool node
  (Law 1); everything else is pure, cargo-tested Rust (Law 2); registered
  nodes are thin boundary adapters; GDScript is tests and probes only.
- **Four Rust laws:** explicit contracts with dependency injection; totality
  over the declared domain (no panic, no NaN emission, `Option`/`Result` for
  absence/failure); pure domain logic (no engine state, clocks, randomness or
  globals inside laws); no global state. `#![deny(unsafe_code)]`; the gdext
  `unsafe impl ExtensionLibrary` stays the sole exception.
- **Strict TDD:** write the failing test first, observe the correct failure,
  add minimal code, observe the pass, refactor. Every test names the break it
  catches. Hand-derive literals — never copy bits a machine printed, never
  mirror the implementation. Mutation-check constants, branches and early
  returns before claiming done.
- **No new dependencies.** The law uses IEEE 754 basic operations only — no
  trig, no `libm` crate, no new crates.
- **Commits:** small, self-contained, green; evocative narrative subject with
  a precise body; identity `Dmitrii Galchenko <dggrus@gmail.com>`; **never**
  add `Co-Authored-By`, `Generated with`, or any assistant attribution
  anywhere (commits, code, comments, docs, PRs).
- **Verification:** `cargo fmt`, `cargo clippy` (warnings denied),
  `cargo test` in `rust/`; gdUnit via the repo's runner with `--import` first,
  trusting only suite+case counts (a green line can carry failures and an
  empty run exits 0). `ci/pipeline.sh` cannot finish on macOS (issue #83) —
  run stages directly.

## Task 1 — the cat's rotation seam becomes verbatim euler storage

Under the current (old) law, so each step stays green on macOS.

1. Failing cargo test: the cat motion port's rotation write is local euler
   (`set_rotation`), not global; rename/adjust the port method and its
   contract docs so the seam is explicit. `FakeCatMotionPort` follows.
2. Failing gdUnit or cargo evidence for capture: `capture_state` reads
   `get_rotation()` (local), not `get_global_rotation()`.
3. Restore install uses `set_rotation`, not `set_global_rotation`
   (`cat.rs` `install_prepared`).
4. Placement law: a pure, total, cargo-tested predicate over the ancestor
   basis chain (exact identity bases required; origins irrelevant), plus the
   standard dual-channel warning wiring on `WaveCat` (stored editor warning +
   runtime print, virtual and callable forwarder), following the existing
   warning patterns in the class. gdUnit: warning fires under a rotated
   parent, silent at root. Boot-pattern coverage if the class participates in
   the boot-error gate.
5. Full gates: cargo suite, cat/restore gdUnit suites, restore probe. Commit
   per behaviour.

## Task 2 — the arithmetic wire law replaces the trig roundtrip

1. Failing tests first, in `support_motion.rs`: the new law's domain and wrap
   behaviour with hand-derived literals — identity on in-domain bits
   (`0.25/-0.5/0.125` and friends), `±PI_F32` admitted at both closed ends,
   first-f32-above-`PI_F32` wraps into domain, `4.0` wraps to the hand-derived
   `4 − τ` image, zero spellings (`-0.0 → +0.0`, `try_canonical` refuses a
   hand-edited `-0.0`), idempotence over a hand-picked bit sweep including
   `f32::MAX`, subnormals and both zeros, and totality (no panic, no NaN) at
   the domain edges.
2. Replace `canonicalize` internals: per-lane arithmetic wrap exactly as the
   spec defines (domain check first — identity path before any arithmetic;
   `% TAU` in f64; one conditional `±TAU` adjustment; f32 cast; zero
   normalization). Delete the `Basis::from_euler`/`get_euler_with` roundtrip,
   the 16-iteration loop, and the now-unreachable iteration-exhausted error
   path. Public signatures unchanged.
3. Rewrite every test that pinned trig-roundtrip behaviour so it pins the new
   law instead (`support_motion.rs` rotation tests; `nodes/cat.rs`
   `copied_cat_state_requires_the_exact_producer_relationships` — use a
   non-f32-representable f64 yaw such as `0.1` so the cross-owner guard's
   narrowing stays observable). Update stale doc comments that describe the
   old mechanism (`support_motion.rs`, `observer.rs:134/179/2455`,
   test messages claiming "Godot rewrites this angle").
4. Mutation evidence: flip the domain boundary comparison, drop the `±TAU`
   adjustment, reverse its direction, drop zero normalization, reorder wrap
   before the domain check — each must fail a named test.
5. Full gates: cargo, every gdUnit suite touching rotation/restore, restore
   probe. Commit per behaviour.

## Task 3 — cross-platform acceptance fixture

1. Failing gdUnit test: capture a blob live, then overwrite the four
   rotation-validated lanes (hero `yaw`/`pitch`, cat `yaw`, brain `yaw`) with
   hand-derived canonical text values satisfying the cross-owner guard
   (`body_yaw == f64::from(brain_yaw as f32)`; pick a non-dyadic yaw so the
   narrowing is visible), recompute the canonical hash via the observer's
   published hash entry point, restore, and require success plus a green
   verify. This test passing on macOS (local) and Linux (CI) is the two-libm
   proof of #82's acceptance criterion; wasm follows by construction.
2. Full gates one last time; commit.

## After the tasks (orchestrator, not implementer)

- Re-run everything myself; review each task with independent reviewers who
  re-derive rather than re-read (campaign lesson), with an extra
  numerics/design lens on Task 2.
- Push, confirm CI green on PR #81; update the PR body (design summary, add
  `Closes #82`), comment on #82 correcting its "pre-existing" claim.
- Update the held wiki commit (capture/restore page: the law; levels/objects
  page: the cat placement rule) and the campaign ledger.
- Present the result to the user; merging stays theirs.
