# Rendering engine design audit — state at 2026-08-18

Branch `pr47-review`, worktree `.claude/worktrees/pr47-review`, rebased onto
`origin/main` (32 commits of tooling/deploy/docs; main had not touched
`rust/src`, `game/shaders`, `game/scenes`, `game/project.godot` or
`game/tests`, so all prior work applied cleanly).

**Nothing is merged, pushed or deployed.** The finish-branch choice has not
been presented, and integration is the user's call.

## What this branch is

It began as a review of PR 47 ("A wall becomes a barrier no sound may
cross") and became a design-level audit of the whole rendering engine, at
the user's direction: *"fix all gaps until the implementation is ideal /
your goal is to compose a technically perfect rendering engine, not ad-hoc
fixes application."*

The barrier-law work (PR 47 plus its repairs) is the first 18 commits. The
nine after it are the design audit — five fixes, then three more repairing
what an adversarial re-review found in them, then this record — and are what
this document is mainly about.

## Verification state of the tree

Reproduced at commit `45f0536`:

| gate | result |
|---|---|
| `cargo test` | 496 passed |
| `cargo fmt --check`, `clippy -D warnings` | clean |
| gdUnit4 | 333 cases / 31 suites, 0 failures |
| headless boot | no script/shader/engine errors |
| `tools/probe_visibility.sh` | PASS, 10 checks, reproduced across a cold and a warm boot |
| `gdformat --check`, `gdlint` | clean |
| `test/repo_hygiene.sh`, `ci/check_gdscript_policy.sh` | pass |

Rendered probe readings (Mesa / Linux / Godot 4.7.1, AMD Radeon,
Compatibility). The full table with each row's bound is on the wiki's
Build, Test, Deploy page; the two most informative:

| check | reading | bound |
|---|---|---|
| the fan's own body, ABSOLUTE, through one wall | 0.192–0.196 | 0.13 … 0.30 |
| the same shell inside the fan's OWN room | 0.306–0.322 | > 0.05 |

The positive controls are markedly stronger than the pre-audit readings
(0.161–0.314 for the own-face case, now 0.373–0.396). That is the wave-death
gate: the "muted" baseline no longer carries an unexpired front, so the delta
stops understating.

Every row was proven able to FAIL by breaking the law it guards — see the
adversarial-pass section below, which is where two of them turned out not to
be.

**Do not add `--fixed-fps` to the probe.** See the note in
`tools/probe_visibility.sh`; at a fixed 60 fps a 12-frame baseline is 0.2 s
against the fan's own 0.4 s cadence and case 4 collapses from 0.329 to 0.000.

## The five landed design fixes

Each was verified against the code before it was touched — the audit was a
lead, never evidence — and each was written test-first, with the correct
failure observed and the realistic mutations confirmed to fail.

### 1. `84b53bf` — a sound now ends where its own slot ends

`reveal_at` had **no end condition**. `dist > min(radius, d.y)` freezes into
the static `dist > max_r` once the front has run its course, and the decay
`1.3·e^(-ga/0.25) + 0.5·e^(-ga/3)` is a sum of exponentials, so it never
reaches zero. Every point a wave ever swept kept **0.0677** of peak (a tap)
or 0.2568 (a hum) indefinitely, going dark only when an unrelated later
sound claimed the slot. The visible life of a sound was a property of the
slot allocator.

`rust/src/render/reveal.rs` is the new pure home of the law, beside
`sight`'s: sight owns *where* a wave reaches, this owns *when it stops*.
Both are written against seconds-since-the-front-passed, the only coordinate
under which the fade is the same law near the source and at full reach.

The envelope was first brought to zero by subtracting its own value at the
tail. **That was wrong and is fixed** — see the adversarial pass below; it
is now a closing window over the last quarter of each wave's life, which
leaves the shape identical for every kind until then.

**Design note worth keeping:** the audit proposed a new `vec4 u_plife[MAXP]`
lane (1 KB) to carry death times. It is not needed — death is already
derivable from `dat` as `t0 + max_r/speed + fade_tail(kind)`, so the
shader's death is now *exactly* the CPU's `end` with no new uniform.

`pulse_fade_tail` in `game/shaders/pulse_pool.gdshaderinc` now has a real
cross-language gate: `game/tests/shader_contract_test.gd` reads the arms out
of the shipped GLSL and evaluates them against `render::reveal` through
`WaveCore.wave_fade_tail`, kind by kind, including kinds outside the four
`emit` packs. Mutating one arm fails with
`"GLSL grants kind 3 a 2.5 s tail while Rust budgets its slot for 2.0 s"`.

The commit also corrects the early-out comment the barrier campaign
invalidated: with a 0/1 occlusion gate, `bound <= reveal` cannot fire
against a `reveal` still at exactly 0.0, so the fragments paying the full
wall walk are precisely the ones in the room next door. It is not the loop's
affordability.

### 2. `269ae74` — a wall may take something from a source at last

`SOURCE_THROUGH` is documented as the ladder a source's silhouette descends
through walls (0.30, 0.09, ...) and it descended nothing. The level
pre-multiplied volume by muffle into one `u_source_floor`, which reached
`data_xray` as a floor under `reveal_at` — and a floor only ever competes. A
source's hub is unwalled from its own body by construction, so `reveal_at`
reads near full strength there however many walls stand between the source
and the player; `max()` handed back the full strength.

Now delivered as `u_source_volume` and `u_source_muffle` and composed as
`muffle * max(wave, volume)`, with `render::reveal::source_image` as the
pure law. With no wall between, it is exactly the old `max`, so a source in
the player's own room reads bit-for-bit as before.

The debug snapshot reports the halves separately (`source_volume`,
`source_muffle`) because the renderer consumes them separately; their
product is no longer a quantity anything on screen forms.

### 3. `4ed321f` — the acoustic image gets a band it can order in

`SOURCE_BAND = 1.0e-5` against a 24-bit depth buffer — and f32's identical
`2^-24` ULP near 1.0 — carries **168 distinguishable values** across
`DIST_PACK_RANGE = 40`: one code every **0.238 m**. Every limb of the
shipped fan lies inside one code, so housing, guard and blades resolved by
opaque draw order, and since a blade's sort key moves as it spins the
creases on the fan head reshuffled instead of rotating.

`rust/src/render/depth.rs` now states **both** bounds as cargo-tested
functions:

- wide enough to order: `band_resolution(1e-3, 40) = 2.38 mm`, under the
  fan's 0.012 m guard-to-blade gap (`MIN_SOURCE_LIMB_GAP`);
- narrow enough to stay unreachable:
  `deepest_world_fragment_in_band(1e-3, 0.05, 60) = 0.050050 m`, i.e. a
  world surface must stand 0.05 mm past a near plane that already clips
  everything closer.

Four orders of magnitude separate the two bounds, which is why the wrong
value looked right in any screenshot of a single source. The camera's near
and far planes moved into the same module, since the second bound is a
statement about them.

**The gdUnit assertion guarding this was
`assert_bool(1.0e-5 < 1.0 - 0.999999 + 1.0e-5)`, which reduces to
`x < 1e-6 + x` and is true for every x.** It is replaced by the derivation,
read back through `WaveCore`.

### 4. `8a9a9dd` — an anchor names a surface, not a whole slab

One prop set flush into the floor could cost a level every outline it has.

An anchor was written onto every class its entry owned — correct only while
the entry stays a merge singleton. Any coplanar merge splits a slab's faces,
rule (a) separates the pairs sharing an edge, and every one of those classes
still carried the slab's own label, so the check compared 0.15 against 0.15
and rejected the whole request:
`AnchorSeparationConflict { first_class: 0, second_class: 2, first_entry: 0, second_entry: 0 }`.
`WaveLevel::paint_labels` answers a rejection with one warning and `return`,
so every solid keeps its unpainted `BOX_ORDINALS`, which `pack_data` writes
to G unclamped and the LDR target saturates 1..5 to white.

`PaintEntryInput.anchor` is now `Option<FaceAnchor>` — a label plus the
direction of the face it belongs to. Slabs anchor `[0, +1, 0]` (floor) and
`[0, -1, 0]` (ceiling). A lone slab is still a merge singleton, so all six
faces collapse into one class and all six carry the label; that behaviour is
pinned in `level_test.gd`.

Ties between two faces answering a direction equally well resolve in build
order (`faces` builds a box -X, +X, -Y, +Y, -Z, +Z), pinned and
mutation-checked, because the wasm and desktop builds must colour one level
identically.

### 5. `4420bfa` — every label that can share a frame stands on one ladder

The role table was held to no separation law and shipped breaking it twice.
Creatures, the viewmodel and the palette never enter `paint_entries`, so the
graph colouring that enforces `MIN_SEP` everywhere else was structurally
blind to them, and the only test over the table was a per-row mirror
assertion that would agree with any numbers at all.

- `HeroBody` 0.82 vs `Ceiling` 0.90 → **0.079999983** in f32, a hair under
  the knee;
- `Ceiling` 0.90 vs `HeroCane` 0.96 → **0.06**, and the cane reaches the
  ceiling (eye 1.6, `PITCH_LIMIT` 1.35 rad, `CANE_REACH` 1.7 → 3.26 m
  against a 3.0 m ceiling), where the distance Laplacian is dead and the
  label difference draws the seam alone at half strength.

**The repair could not be local, and that is the finding.** Ten labels must
coexist in one frame — floor, five palette entries, cat, hero body, ceiling,
cane — and nine gaps across the 0.81-wide band leave exactly 0.09 each.
Pushing the ceiling clear of the cane pushed the body into the ceiling;
pushing the cat clear of that pushed it into the palette's top entry.

The ladder is `0.15 + 0.09k`, `k = 0..9`:

| rung | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 |
|---|---|---|---|---|---|---|---|---|---|---|
| label | 0.15 | 0.24 | 0.33 | 0.42 | 0.51 | 0.60 | 0.69 | 0.78 | 0.87 | 0.96 |
| owner | Floor | palette | palette | palette | palette | palette | Cat | HeroBody | Ceiling | HeroCane |

`Role::Shell` (0.33) and `Role::Moving` (0.60) deliberately reuse palette
rungs: they are standalone blueprint preview defaults that a level never
renders beside a wall, and twelve distinct labels do not fit in a band that
holds eleven. `Role::Case` (0.05) remains the grandfathered exception below
the band.

`WORLD_OIDS` moved from `nodes/level.rs` into `render::labels::WORLD_PALETTE`,
because the law is only checkable where the whole label universe is visible
at once. The suites now read the table through `WaveCore.role_labels()`,
`WaveCore.world_palette()` and `WaveCore.min_label_separation()` instead of
transcribing it — including `map_test.gd`'s `MIN_OID_SEP`, a third
executable copy of `MIN_SEP` that let a Rust-side change keep every gdUnit
case green while the seams rendered weaker.

## New Rust modules and FFI surface

- `rust/src/render/reveal.rs` — `flare`, `reveal_tail`, `source_image`,
  `SourceImage`. The reveal envelope and its end, and a source's standing
  acoustic image.
- `rust/src/render/depth.rs` — `ALWAYS_ON_TOP`, `SOURCE_BAND`,
  `DEPTH_CODES`, `CAM_NEAR`, `CAM_FAR`, `MIN_SOURCE_LIMB_GAP`,
  `window_depth`, `deepest_world_fragment_in_band`, `source_depth`,
  `band_resolution`.
- `rust/src/render/labels.rs` — gains `LADDER_BASE`/`LADDER_STEP`/
  `LADDER_RUNGS`, `ladder_rung`, `WORLD_PALETTE`, `coexisting_labels`.
- `WaveCore` gains, for the suites to read rather than transcribe:
  `wave_fade_tail`, `source_band`, `source_band_resolution`,
  `min_source_limb_gap`, `deepest_world_fragment_in_band`, `camera_near`,
  `role_labels`, `world_palette`, `min_label_separation`.

## The adversarial pass, and the six repairs it forced

A ten-agent workflow re-reviewed the five landed fixes, each skeptic tasked
with REFUTING rather than confirming, with a second opinion on every
refutation. It found real defects in three of the five. All are now fixed
(commits `c667d0a`, `b932898`, `45f0536`).

**The worst was mine, and it was in a test.** Making the wall muffle
multiply the whole acoustic image shrank every reading on the fan by 3.3x,
while the rendered probe's two hand-derived leak floors (0.12 and 0.08)
stayed where they were. The largest delta a full-strength leak could then
produce was `0.867 x (0.30 - 0.225) = 0.065` — under both floors. **The two
checks that exist to catch a wave reaching through a wall could no longer
fail.** Two agents derived this independently and I confirmed the
arithmetic before touching anything. They are ratios of the fan's own
standing image now, which is scale-free, plus a new ABSOLUTE row that
catches the one failure no unit test can see: an instance uniform that never
reached the GPU, because the suites and the observer read those uniforms
back through the same names that would have been renamed. Verified by
deleting the muffle from the skin — 0.659 against a 0.30 ceiling.

**The envelope repair had over-reached.** Bringing the decay to zero by
subtracting its own value at the tail reached zero correctly and darkened
everything else by a kind-dependent amount: 17.3, 39.7, 55.4 and 65.6 codes
of 255 for kinds 0..3. A cane tap and its own echo striking one surface read
0.3144 and 0.2264 at the same age — a 22-code split with no cause in the
world, only in the slot allocator. The decay is the acoustic law and the
tail is only the budget, so a CLOSING WINDOW now ends the wave instead:
flat at 1.0 through the first three quarters of every wave's life, then a
smoothstep to exactly zero. `CLOSE_FRACTION = 0.25`.

**Two "cargo-pinned references" pinned nothing.** `render::reveal::flare`
and `::source_image` had no non-test caller, and the shipped GLSL was held
to them by substring alone — a `contains()` cannot tell 1.3 from 1.0, or a
3.0 time constant from a 4.0 one. And `ga = age - dist / speed`, the single
time coordinate the whole law is written against, was asserted nowhere: with
`ga = age` the fan (ring time exactly 2.0 s against kind 3's tail of exactly
2.0 s) would stop revealing the outer metre of its own wash at the instant
its front arrived, while the ring kept drawing. Both cross the language
boundary numerically now, through `WaveCore.wave_flare` /
`wave_close_fraction` / `wave_death_time`, with the shader's own constants
parsed out of the GLSL and evaluated. Mutation-checked: the slow time
constant, `CLOSE_FRACTION`, and the time coordinate all fail.

**The probe had a second, older lie.** Its sweep window was 200 frames,
about 3.3 s against the fan's 11.42 s oscillation, so it could land wholly
in a phase where the beam points elsewhere — a POSITIVE control read 0.000
on one boot and 0.322 and 0.310 on the next two. The window is a duration on
the simulated clock now (`SWEEP_SECONDS = 12.0`), because the probe's frame
rate is set by the full-framebuffer readback it does every frame and a fixed
frame count means something different on every machine. Widening it exposed
the second half: the hero is a `CharacterBody3D` running its own physics and
drifts out from under the sample points over a window that long, so
`_peak_r` now re-applies the POSE every sampled frame, not only the aim.

Smaller repairs in the same pass: `paint_plan`'s `LABEL_MIN`/`LABEL_MAX`
were a second executable copy of the ladder endpoints; the Godot-facing role
roster was a hand-written array that a new `Role` could silently miss (it is
paired with an exhaustive `role_name` match now); `SOURCE_THROUGH`'s own
file still documented the law the campaign replaced; `pack_data` wrote G
unclamped while clamping R and B; the anchor direction was a world-up
literal rather than the slab's own basis; and two anchor limits that survive
— a column's curved flank cannot be named by any direction, and two entries
pinned to the SAME label across a seam are still refused outright — are now
written down and tested as the unreachable edges they currently are.

**Two of the five fixes survived refutation intact**: the label ladder and
the face-scoped anchor drew only minor findings, both of which are fixed.

## Gaps identified and NOT yet fixed

Ranked by my own assessment, not the audit's severity labels (which were
uniformly "critical" and are not trustworthy as a ranking).

1. **The wall occluder lives in a different coordinate frame than the wall.**
   `sight::crosses` hard-codes `lo = [rect.x, 0.0, rect.y]`
   (`rust/src/sight.rs:91`), mirrored at
   `game/shaders/pulse_pool.gdshaderinc:62`, with one global `u_wall_top`.
   Nothing constrains a wall's Y: `level_plan::plan_wall_transform`
   normalises the **basis** but leaves `origin.y` entirely free. Lift a
   `WaveWall` a metre and the drawn box spans [1, 4] while the occluder
   spans [0, 3] — a phantom barrier beneath it and an unoccluded strip over
   it. Lift the level root and the barrier law **fails open across the whole
   map, silently**. Verified by reading; the fix was not started.
2. **The crease knee is a bare GLSL literal.** `MIN_SEP` governs
   allocation in Rust; `smoothstep(0.04, 0.08, nrm)` in
   `hearing_post.gdshader` governs the rendered response, and nothing
   compares them. `labels.rs`'s own doc comment says it "cannot be
   single-sourced from Rust", which the shipped code refutes —
   `nodes/game.rs` already pushes `u_base`/`u_breath`/`u_grain_t` into the
   post material. `(MIN_SEP/2, MIN_SEP)` reproduces 0.04/0.08 exactly, so
   the change is behaviour-preserving.
3. **`RECT_SHRINK` and the B-channel quantum guard the same boundary and do
   not know each other exist.** `hearing_post` reconstructs a world point
   from B and asks the wall table about it; the only thing keeping a real
   surface's reconstructed point outside its own wall is that
   `RECT_SHRINK = 0.02 m` exceeds B's half-LSB. `pack_range_budget`
   explicitly advises raising `DIST_PACK_RANGE` while listing three
   consequences of which this is not one. **The channel depth is
   UNVERIFIED** — the brief says 8-bit LDR, one audit probe measured
   `1/1023` (RGB10_A2). At 8 bits the half-LSB is 78 mm and this is a live
   bug; at 10 bits it is 19.55 mm and the shipped build is correct by
   0.45 mm. Settle the channel depth first.
4. **The x-ray layer is inferred, not written.** `seen_walled` reconstructs
   a point from B and asks the **wall** table — but `WaveProp`,
   `WaveColumn` and `WaveWedge` are in no occluder table anywhere, so a
   source hidden behind a prop is misclassified and both corrections fail
   open. The natural fix is for `data_xray` to write a layer sentinel into
   the unclaimed A lane; **whether writing `ALPHA` in a Godot 4.7
   Compatibility spatial shader moves the material into the transparent pass
   is UNVERIFIED and must be settled before designing around it** — if it
   does, it would destroy the always-on-top depth trick.
5. **Larger, deliberately not attempted**: the label allocator's adjacency
   is a physical-contact relation (`TOUCH_EPS = 0.01`) while the image needs
   a screen-adjacency one, and the B Laplacian is identically zero below a
   0.48 m depth step — so contact-free props 0.4 m apart can share a label
   and melt. Fixing this properly means moving the silhouette test to a
   scale-relative predicate **and** growing the adjacency relation together;
   neither works alone. Related: there is no executable oracle for the
   renderer's arithmetic — `docs/superpowers/specs/2026-08-11-pixel-oracle-gate-design.md`
   designed one and `rust/src/observe/oracle.rs` was never built.

A running workflow (`w3cuaz725`, run `wf_0baad7f2-f50`) was launched to
adversarially refute the five landed fixes and to design items 1–4. **Its
results are not yet folded into this document.** Read its transcript at
`.claude/projects/.../subagents/workflows/wf_0baad7f2-f50/journal.jsonl`
before acting on items 1–4, and verify anything it claims against the
codebase before accepting it.

## Documentation state

- The wiki checkout at `/home/albatraoz/unseeing.wiki` has **3 local commits
  that are not pushed** (`29302ef`, `03e976d`, `c199f4f`). The first two
  cover the barrier law; the third rewrites `Mechanics-Rendering.md`,
  `Mechanics-Waves.md`, `Mechanics-Sound-Sources.md` and
  `Engineering-Build-Test-Deploy.md` for this campaign — the derived depth
  band, the label ladder, the reveal's end, the muffle as a multiplier, and
  the probe's ten-row evidence table with the measured value that proves
  each row can still fail. **Pushing is outward-facing and is the user's
  call.**
- Two specs are amended in-repo rather than rewritten, since a spec freezes
  what was decided: `2026-08-12-superface-outline-rendering-design.md` (the
  label numbers it named broke their own separation law; the design it
  describes is unchanged apart from anchors becoming face-scoped) and
  `2026-08-10-debug-observability-design.md` (`u_source_floor` no longer
  exists). Plans are left untouched: they record what was planned.
- No NEW spec was written for the audit campaign. If this work continues,
  one under `docs/superpowers/specs/` should freeze the label ladder in
  particular — it is a forced arrangement rather than a taste, and the
  derivation deserves its own page.

## Where to pick up

1. Read the workflow result; verify its claims against the code.
2. Repair anything the refutation pass genuinely found in the five landed
   fixes.
3. Settle the two UNVERIFIED platform facts (screen-texture channel depth;
   `ALPHA` in the Compatibility opaque pass), because items 3 and 4 above
   cannot be designed correctly without them.
4. Then items 1 and 2, which are self-contained and need no platform fact.
5. Rewrite the wiki pages listed above.
6. Present the finish-branch choice. Do not merge, push or deploy without
   the user's explicit selection.
