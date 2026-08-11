# Wall-junction z-fight fix — plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (or
> subagent-driven-development) to implement this plan task-by-task.

**Spec:** `docs/superpowers/specs/2026-08-11-wall-junction-zfight-design.md`
**Branch:** `worktree-issue-14-corner-hazard`

## Global Constraints

- Perception laws: black & white, thin outlines only; the world is revealed
  by waves; one outline per object, and every seam between two touching
  objects draws. Any new solid that can touch another needs an object id at
  least 0.08 clear of it; ids come from `rust/src/oid_palette.rs`'s graph
  colouring, never from cycling a list.
- Two layers: all logic in Rust (`rust/`, pure modules cargo-tested,
  `#![deny(unsafe_code)]`); Godot holds editor-authored scenes and thin
  typed GDScript. Platforms: Windows, macOS, web — one Godot project,
  x86_64 + arm64 + wasm32, no arch-specific code.
- Strict TDD: write the failing test, watch it fail for the right reason,
  minimal code, watch it pass. No mirror assertions, no change detectors;
  run the mutation check. Production code written before its test gets
  deleted.
- Formatters/analyzers before every commit: `cargo fmt`, `cargo clippy`
  (warnings are errors), `cargo test`; `gdformat` + `gdlint` for GDScript.
- Commits: small, self-contained, green; narrative evocative subject with a
  precise technical body. **All work is authored on behalf of the user,
  never an assistant. No Co-Authored-By or "Generated with" trailers; no
  mention of any assistant in commits, code, comments, docs, or PRs.**
  Repo-local identity: Dmitrii Galchenko <dggrus@gmail.com>.
- Never commit build output or binaries; `game/addons/godot_mcp/` and
  `game/override.cfg` stay untracked.

## Tasks

### 1. The z-fight census law (pure Rust)
- Failing tests first, hand-derived: two boxes sharing a same-facing
  vertical coplanar overlapping face pair with ids 0.09 apart → one fight
  reported; opposite-facing (abutting) → none; horizontal plane (floor/
  prop contact) → none; coplanar but disjoint spans → none; ids closer
  than the crease floor → reported quiet / not a fight; separated planes
  → none.
- Implement the law beside the touch census (`rust/src/observe/oids.rs` /
  `rust/src/oid_palette.rs` as fits), total functions, no panics.
- Commit (law + tests together).

### 2. The wall cap inset
- Failing tests first: two perpendicular walls whose centerlines meet
  produce ZERO fights under the new law; the abutting cap plane lies
  strictly inside the partner's box; the junction still touches
  (`TOUCH_EPS`) and still interpenetrates. Update the existing
  `wall_box` padding pins to the new hand-derived literals.
- Implement `CAP_INSET` in `level_plan.rs`; `wall_box` run-axis pad =
  `WALL_T - CAP_INSET`.
- Commit.

### 3. The observer reports fights
- Failing gdUnit test first: `explain_oids()` carries a `fights` array,
  and the shipped map reports zero. (Run `--import` before the suite in a
  fresh checkout; trust suite/case counts, not the exit code.)
- Wire the pure law through `WaveObserver`'s boundary with the censused
  boxes it already reads; extend the in-editor docs.
- Commit.

### 4. The furniture panels
- With task 3's gate red against the shipped map (Shelf/Rack backs), tuck
  `ShelfBack` and `RackBack` 5 mm behind their side panels' plane in
  `game/scenes/level_01.tscn`; the gate goes green. Update any census pins
  the scene edit trips.
- Commit.

### 5. Gates, review, writeback
- Full pipeline: repo hygiene, cargo suite, gdUnit suite (import first),
  determinism probe. Mandatory subagent code review of the whole diff;
  physics/render change also gets the multi-lens design/perf review.
- Wiki: Mechanics — Level and Objects (the wall box law, the new gate);
  Engineering — Debugging and Observability (the fights report). Memory
  writeback. Close-out comment on issue #14 with the evidence.
