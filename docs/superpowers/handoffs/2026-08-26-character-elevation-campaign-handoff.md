# Character elevation support (issue #64) — state at 2026-08-26

Branch `issue-64-hero-elevation`, worktree
`.worktrees/issue-64-hero-elevation`, 18 commits ahead of `main`
(72 files, +23595/-5861). `main` is untouched and clean.

**Nothing is merged, pushed or deployed.** The finish-branch choice has not
been presented, and integration is the user's call.

The campaign executes `docs/superpowers/plans/2026-08-21-character-elevation-support.md`
against `docs/superpowers/specs/2026-08-21-character-elevation-support-design.md`,
under subagent-driven development. Seven of its eight tasks are complete and
review-clean, with nothing parked. Work stopped deliberately at that point,
before Task 8.

## What this branch is

Before it, both actors were frozen to the floor: `UnseeingPlayer` and
`WaveCat` overwrote Y velocity with planar motion every tick, so neither
could fall from an unsupported position or leave an edge, and `HeroBody`,
`viewmodel::leg_pose`, `cat_body::skeleton`, `CatGait` and every actor wave
origin rebuilt themselves against absolute world Y near zero. A player
standing on an authored platform kept its feet, its footstep waves and its
cat's paws at the floor.

After it, both actors acquire real support motion: they stand on props, walk
off edges, fall under authored gravity to a terminal speed, land, and voice
that landing — and every visual and every wave origin follows the surface
that actually holds them.

## Verification state of the tree

At `d136993`, on this Mac (the reference platform), all green:

- `cargo test` — **719 passed, 0 failed**
- gdUnit4 census — **476 cases across 35 suites, 0 failures** (campaign
  baseline at start was 354 cases / 32 suites)
- editor source probe — **16 editor checks + 3 run checks**, all pass
  (raised from 11 editor checks by Task 5)
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, release build,
  `gdformat`, `gdlint` — clean

The pinned engine is Godot `4.7.1.stable.official` per `.godot-version`.
During this campaign the previously-pinned copy under `/tmp` was lost to a
temp sweep and re-fetched to the same path;
`/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot` is where
the plan's literal commands expect it. Both Godots otherwise installed on
this machine have drifted to 4.7.2, which the repo's own engine pin
correctly refuses.

### One platform divergence, pre-existing, not caused by this branch

Part of this campaign ran on a Linux box (Debian 13 x86_64) before returning
to the Mac. There, at the branch's own base commit, **4 cargo tests and 1
gdUnit case fail**, all green on macOS:

- `support_motion::tests::godot_rotation_canonicalization_is_idempotent_for_observed_pitch_and_wrapped_yaw`
- `support_motion::tests::godot_rotation_canonicalizing_lane_replacement_allows_owned_ulp_only`
- `support_motion::tests::godot_rotation_lane_replacement_preserves_uncaptured_bits_and_checks_the_complete_yxz_target`
- `nodes::cat::tests::copied_cat_state_requires_the_exact_producer_relationships`
- gdUnit `test_round_trip_capture_restore_capture_is_exact`

Cause: `GodotRotation::canonicalize` (`rust/src/support_motion.rs:322`)
finds its canonical form by round-tripping through gdext's
`Basis::from_euler(...).get_euler_with(...)`, which reaches the platform's
libm. Apple's and glibc's trig differ by 3–4 ULP, so the round trip
converges to different bits per platform, and `try_canonical` demands
bit equality. **The stake is larger than the test names suggest: a capture
blob written on one platform can refuse to restore on another, so the
reproduction system's determinism promise is currently platform-scoped, and
wasm is a third target whose compiled math will differ again.** The fix
direction is deterministic math — a pure-Rust trig path, or a canonical form
defined arithmetically so no libm is involved — not teaching one platform to
imitate another's libm. This deserves its own issue after the campaign; no
work on it has been done here.

## The design decision that changed the plan

Task 3 was found mid-flight with a deterministic red:
`prepared_visual_adds_support_once_to_every_body_vertex` failed by ~1.34 ULP
against a ~1.125 ULP budget. Root cause was not a slip but the construction
the plan itself prescribed: support was threaded through the
hip→knee→ankle→shoe chain, so each joint rounded to f32 in turn and the tube
emitter added one more rounding. A raised silhouette therefore drifted
several roundings away from the translated flat silhouette, and the exact
translation law could not hold per-vertex.

**The user chose the transport law** (2026-08-25, an explicit design choice,
not a controller ruling): `viewmodel::leg_pose` became the flat leg law with
no support parameter (hip `0.90`, ankle floor `0.07`, shoe floor `0.065`),
and `hero_visual` adds `support.y()` exactly once to every emitted body
vertex and both shoes in a single transport pass — the same shape the cat
already shipped through `translate_skeleton_y`/`transport_y`. A single f32
add bounds every lane at half an output ULP, so "a raised silhouette is the
translated flat silhouette" is now exact by construction rather than
approximately true.

Commit `398649d` carries that decision into plan Steps 1, 2, 3, 7, 15 and
into the spec's player coordinate-law section. A derived consequence
recorded there: with the flat law, `leg_pose` is total at the actor envelope
edge (the pose envelope exceeds the actor envelope by 2 m while joint
offsets are bounded near 1.25 m), so what had been an envelope-refusal test
became a totality proof, and the extreme-support refusal moved to the
candidate validator.

## Task state

Dependency order is `1 → 6 → 4 → 2 → 3 → 5 → 7 → 8`.

| Task | Subject | Commits | State |
|---|---|---|---|
| 1 | Pure support motion and landing response | `5d1c429`, `a819079` | complete |
| 6 | Format-2 capture/restore schema (dormant) | `7e81251`, `4356109` | complete |
| 4 | Cat gait, skeleton and tail share one elevation | `b4afe1b` | complete |
| 2 | Player physical support, layers, Inspector injection | `5aac99d` | complete |
| 3 | Player silhouette, footsteps, landing voice, cane | `6c82982`, `6df3303` | complete |
| 5 | Cat physical adapter, controls, elevated voices | `695cc14`, `ba2e694`, `0388333`, `c1cffa0` | complete |
| 7 | Structured motion observability and fixtures | `8d0bd8c`, `d136993` | complete |
| 8 | Final gates, evidence, wiki rewrite | — | **not started** |

Tasks 1, 6, 4 and 2 were completed in earlier sessions; `de8eebf` and
`398649d` are docs-only commits that froze and then amended Task 3's
transaction.

### Task 3 — the player (`6c82982`, `6df3303`)

Implemented all 15 steps: the flat leg law, the atomic value-only
`prepare_hero_visual` transaction, `HeroBody` wiring that deleted the
migration shim, footstep provenance (`ControlledContact` gate plus two
distinct admission proofs), the fresh-landing voice, cane elevation through
one support datum, and format-2 activation with no wire change. 45 mutations,
each killed by a named test.

Four review seats (spec+quality, architecture, wave/performance,
restore-transaction) produced 1 Critical and 4 Important findings, all fixed
in `6df3303` and confirmed by a scoped re-review:

1. **Critical, totality.** `prepare_cane_tap` called `Vector3::normalized()`
   on the horizontal aim. In gdext 0.5.4 that is
   `try_normalized().expect(...)` — an abort across FFI — and a camera
   pitched to ±π/2 is a *legal* value under finiteness-only validation. Fixed
   with a total `hero_visual::horizontal_aim` and an explicit refusal.
2. **Important, placement.** The cane's whole decision law (reach shortening,
   the settle offset, the floorish predicate, the raised-pitch
   classification, the swish geometry, and the authored voice table) had
   stayed in `nodes/player.rs` although the brief required pure functions.
   Moved into named `hero_visual` functions; the adapter now sequences port
   calls only. *This is the campaign's most repeated lesson: authored values
   and predicates belong in a pure cargo-tested owner.*
3. **Important, ambient precondition.** `commit_hero_frame` wrote the camera
   only `if let Some(camera)` and otherwise silently committed a
   half-applied frame, trusting the caller to have run `owns_visual_camera`.
   Replaced with a consumed `VisualCameraProof` token, so a proof-less call
   is now a compile error.
4. **Important, unpinned proofs.** The cane strike's echo budget, origin
   normal and both retained proofs were pinned by nothing; three
   brief-required mutations survived. Now pinned, and those rows re-executed.
5. **Important, false witness.** A restore-atomicity test named "a later
   group refuses" poisoned an *earlier* group (restorer order is
   waves → hero → cats → sources), so it passed vacuously. Re-pointed at a
   cat lane and witnessed against an early-write mutant.

Plus a consensus item all four seats raised independently: both emit sites
discarded the request's proven `PreparedTime` and re-read the raw clock, so
the wave reaching the pool was not the wave the proof admitted. Now threaded
through, including across the tick for queued footsteps (`prepared_at`,
runtime-only, wire untouched).

### Task 5 — the cat (`695cc14`, `ba2e694`, `0388333`, `c1cffa0`)

The cat's counterpart: its own `CatMotionPort` transaction, all eleven solver
values set explicitly, phase-derived layers, brain and yaw frozen while
airborne with gait/tail/presence continuing from *achieved* displacement, six
exported per-cat settings with exact Inspector hints, elevated paw/presence
voices and a direct omnidirectional landing voice (distinct from the player's
reflecting one), and the editor probe extended to the cat's capsule datum and
both warning channels.

Three seats reviewed it; three fix rounds followed.

1. **Important, found independently by two seats.** The early return in
   `read_cat_post_move_support` that implements "every floor contact came
   from another actor, so claim no support and never probe" was reached by no
   test — the nearest test supplied a real world floor in a later slide, so
   the branch was stepped over. Deleting the line left the entire suite
   green. Closed by a targeted port-trace test that arms the probe to succeed
   if wrongly reached and asserts its absence from the trace.
2. **Important, evidence accuracy.** A mutation row credited a test that
   structurally could not see the fault: it drops a fresh cat whose brain
   starts paused at bit-zero speed and stays frozen through the fall, so no
   stride completes and the gated loop body never runs. Corrected; the
   implementer found a second such row unprompted while re-walking.
3. **Two rounds on one tolerance.** A dead constant `F32_ULP_AT_2` was
   "fixed" by wiring it into the collider-datum check whose lanes are 0.17
   and 0.0 — a magnitude-2 ULP guarding a magnitude-0.17 lane, ~16× looser
   than a correct derivation and looser than the literal it replaced. Then
   the *same* defect was found one file over, in the CI-gating editor probe,
   under a comment claiming the value was derived "at the cat's own scale."
   Both now hold the plain `1.0e-7` cross-language convention with honest
   derivation comments, and the probe's check-message strings no longer claim
   a ULP bound they never had.

The reasoning for not asserting the mathematically tighter `2^-26` bound is
recorded in the code: it would require two independently-rounded values, one
computed in Rust and one authored in GDScript, to agree bit-for-bit, which
nothing proves.

### Task 7 — observability (`8d0bd8c`, `d136993`)

Adds `ActorMotionObservation`/`CatMotionObservation` (pure, in
`rust/src/observe/mod.rs`), the hero motion dictionary and ordered cat motion
dictionaries (boundary shaping in `rust/src/nodes/observer.rs`), structured
cross-checks layered onto the elevation suites, and a test-only movie scene
and probe.

The architecture/evidence-law seat approved outright with nothing Critical or
Important. Its central finding is worth keeping: **every new observation
field reads data produced by something other than the code under test** —
phase against engine facts fetched through a different FFI path, landing
against the pulse pool written by a separate emitter, cat order traced to
`level.rs`'s documented deterministic depth-first walk with no hash-ordered
collection in the chain. It also verified in source that the capture-identity
governor is structural: `HeroCapture`/`CatCapture` carry no identity field
and all six construction and destructuring sites use exhaustive field lists,
so adding one would fail to compile in six places. Transient collider IDs
reach observation and nothing else.

Two seats found four items, fixed in `d136993`: the cat's capped-landing and
edge-departure cases lacked their structured cross-checks, the enumerated
"capture/restore future" case was unpinned entirely (now
`test_a_restored_player_landing_reports_the_captured_motion`), and one more
mutation row was inflated. Asked to re-walk the matrix, the implementer found
a row whose credited test could never have failed — the assertion was an
`is_not_equal` check that a null value satisfies — plus four undercounted
rows.

**A controller error is recorded here too:** the edge-case finding was
dispatched with the justification that the player's departure test had a
cross-check the cat's lacked. It does not; the coordinator misread which
function a line belonged to in diff context, and the implementer pushed back
correctly. The finding stood anyway on the brief's own Step 3 taxonomy, and
the cat's cross-check was built from that instead of from a nonexistent twin.

The scoped re-review of `d136993` verdicted all four ADDRESSED with no new
breakage, and audited seven of the eight mutation rows — not the three
demanded — against the raw gdUnit4 XML each run left behind, matching every
claimed failing-test list name-for-name. The restore test's own mutation
proof is corroborated by an artifact whose single failure carries the exact
string `rust/src/nodes/restorer.rs:121` emits. The fix diff is purely
additive: 76 lines across two test files, no production code.

One accepted limit is worth knowing: the cat's capped-landing cross-check
asserts `impact_speed > 5.0` rather than the authored `0.60`/`2.5` caps,
because the motion channel's landing struct carries only
`{impact_speed, point, normal}` — gain and range are pulse-lane concepts that
do not exist in that dictionary. The caps stay pinned on the pulse lane; the
motion lane pins the speed that produced them.

### Three engine facts discovered by Task 7, all adjudicated

The implementer hit three behaviors and worked around all three in test code
only. A dedicated review seat read production source to judge each:

1. **`WaveLevel` implicitly builds floor and ceiling slabs** — legitimate,
   documented Law-1 behavior (`level.rs` `build_slabs`, `WALL_H = 3.0`). The
   movie probe correctly places its airborne lane outside `LEVEL_EXTENTS`.
2. **The `WaveLevel` census runs once at `ready()` and never again** —
   legitimate. `derive()` only rebuilds bookkeeping; the property writes that
   actually mutate a cat live in `inject()`, so the fixtures' `rederive()`
   call cannot perturb established cat motion state, and it is the same
   public door the editor's drag-watch uses.
3. **`relocate()` leaves `is_on_floor()` stale for one tick** — a real Godot
   quirk, but a **test-only exposure today, not a product defect**: the seat
   traced every production reader and found `is_on_floor()` is consulted in
   exactly one place per actor, always after `move_and_slide_once()` within
   the same tick, and `physics_process` runs that path unconditionally every
   tick. `relocate()` is also not yet wired to any gameplay trigger.
   **It would become a real defect the moment a designer-facing teleport
   trigger reads `is_on_floor()` in the same frame — so it belongs in the
   wiki as a gotcha, which Task 8 must write.**

## What remains

**Task 8 is untouched.** Its brief is extracted at
`.superpowers/sdd/2026-08-21-character-elevation-support/task-8-brief.md`.
It runs the complete mutation matrix again, all Rust and GDScript gates, the
repository and platform gates, visual/performance evidence, and then rewrites
the wiki in a fresh external clone.

Then: the final whole-branch review (on the most capable model, pointed at
the deferred minors below), and the finish-branch choice, which is the user's
to make. The web build deploys automatically from `main`'s HEAD, so the
decision to merge *is* the deploy gate.

### Wiki preparation, and a warning about it

A scouting pass inventoried the wiki (17 pages, clone HEAD `5182df9`) into
`.superpowers/sdd/2026-08-21-character-elevation-support/wiki-inventory.md`.
Its page/heading/constant map is sound and worth reusing.

**Its list of "soon-to-be-false statements" is over-flagged and must not be
handed to Task 8 as-is.** Five of eight flags contradict this campaign's own
constraints, which leave wall geometry and the occlusion law untouched:
claims about wall height, shared corners, doorways, the column/wedge
origin law, and the lifted-wall phantom-barrier bug all describe behavior
this campaign does not change — walls must still span floor to ceiling, and
that bug stays a bug. The floor-at-y-0 / ceiling-at-`WALL_H` statement also
stays true: actors gain elevated *support* from props, not new floors.

Genuinely false after this branch: the documented hero spawn Y and
`SPAWN_LIFT`, versus Task 2's authored `0.9 m` standing root and `-0.05 m`
capsule centre. Genuinely needing rewrites: **Waves** (support-relative
footstep and paw origins, the new landing voices, `ControlledContact`
gating), **Overview** (support motion in the frame loop), **Level and
Objects** (props as standable support, the root datum), **Debugging and
Observability** (Task 7's surfaces), **Build/Test/Deploy** (the new suites
and probe counts). The diff and the brief are the authorities, not the
inventory.

### Deferred minors, for the final whole-branch review to triage

From Task 5:

- The post-move support-scan law is duplicated verbatim between
  `nodes/player.rs` and `nodes/cat.rs` (same bounds, branch order and
  floor-angle formula, differing only in type names). The brief sanctioned
  either sharing or duplicating; two hand-maintained copies invite drift.
- `cat_control_policy`, a pure decision law, lives in the adapter file rather
  than beside `Mood` in `cat_brain.rs` (follows Task 2's own precedent).
- The presence cadence advance moved from the semi-pure tick into
  `physics_process`; numerically inert, same once-per-successful-tick
  semantics.

From Task 7:

- `read_support_collider_id` is generic over any `Inherits<Node>` though only
  two call sites register that `#[func]`; totality rests on caller discipline
  rather than the type. It also round-trips identity through `i64`, so an
  instance ID above `i64::MAX` would read as "no identity" rather than
  erroring.
- The `actual_velocity` cross-checks prove lossless plumbing, not physics
  (correctly not over-claimed in the report).
- The movie probe waits a fixed `SETTLE_TICKS` for its flat and elevated
  marks. The brief specifies fixed frames and those lanes spawn at rest, so
  it is not a violation, but it sits near the no-sleep line; the `landed`
  mark polls a bounded condition properly.
- `game/tests/character_elevation_fixture.gd` is modified but absent from
  Task 7's Files list — a brief omission rather than scope creep.

## A pattern worth carrying to the next campaign

Across Tasks 3, 5 and 7, **every first review failed on a confidently false
claim rather than broken code** — a mutation row crediting a test that
cannot observe its fault, a test named for a break it does not exercise, a
comment asserting a derivation the number does not have. The code was
usually right; the evidence about the code was not. Reviewers were therefore
briefed to re-derive numerics by hand and to ask, for every claimed mutation
kill, whether the named test can actually see the mutated value in the
scenario it constructs. That question found real holes in all three tasks and
should stay in every review brief.

## Where to pick up

1. Dispatch Task 8 from its extracted brief, carrying the wiki ruling above.
2. Final whole-branch review on the most capable model, pointed at the
   deferred minors.
3. Present the finish-branch choice. Do not merge, push, or deploy without it.

`game/reports/` holds dozens of untracked gdUnit4 artifacts from this
session's mutation runs — gitignored, so harmless, but worth clearing before
Task 8's own gate runs so its evidence is unambiguous.

Working state lives in the git-ignored SDD workspace
`.superpowers/sdd/2026-08-21-character-elevation-support/`: `progress.md` is
the ledger of record (every ruling, every review verdict, every gate number),
alongside per-task briefs, implementer reports, review packages, and the wiki
inventory. That directory is deleted when the campaign closes, which is why
this handoff exists.
