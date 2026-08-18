# Closing the renderer's last three gaps — design

Frozen 2026-08-18. Follows the design audit recorded in
`docs/superpowers/handoffs/2026-08-18-rendering-design-audit-state.md`, whose
"What remains" section this supersedes.

Three items were carried rather than fixed. Researched together because two of
them turned out to be different problems from the ones recorded, and the third
turned out to be largely already solved.

---

## Finding 0 — the platform gate is cleared

Both remaining designs depended on a number nobody had measured: how many
levels one data channel preserves on the WEB target.
`tools/measure_web_platform.sh` now answers it. **1024, the same as the
desktop**, with the depth texture live there too (0.9490 against an analytic
0.9508). Measured under SwiftShader, so it is the floor a browser guarantees
rather than a ceiling — and a driver giving FEWER levels is the direction that
would break things, which is what the measurement rules out.

This matters most for Gap 2 below, whose whole design turns on the band
holding hundreds of distinguishable codes rather than eleven.

---

## Gap 1 — the label melt. The recorded diagnosis was wrong.

### What was recorded

> The label allocator's adjacency is a physical-contact relation
> (`TOUCH_EPS = 0.01`) while the image needs a screen-adjacency one … Fixing
> it properly means moving the silhouette test to a scale-relative predicate
> AND growing the adjacency relation together — neither works alone.

### What is actually true

**The magnitude encoding of G buys nothing on screen, and costs the entire
palette.** Every label the game ships is a rung of a ladder 0.09 apart, so
`nrm` is always either exactly 0 or at least 0.09, and
`smoothstep(0.04, 0.08, nrm)` therefore returns exactly 0 or exactly 1 in
every shipped frame. The crease is **already binary**. The eleven-label
ceiling, the graph colouring, the palette starvation reports and the
`MIN_SEP` separation law are all machinery for making a magnitude comparison
behave like an equality test that the shader is already performing.

**The melt is worse than recorded.** Forty shipped solid pairs share a label
and present a depth step under the 0.48 m silhouette knee. The worst is
`EastBarrelA`/`EastBarrelB` at **0.036 m** — smaller than one B quantum
(39.1 mm), so it is not merely under the knee, it is not representable in the
channel at all. `PipeA/B/C` read as one fat pipe. Every four-legged prop in
the game — 2 chairs, 2 tables, 2 benches, a workbench — merges its legs
pairwise whenever you sight along an axis of the leg square.

**"Screen adjacency" is not a well-posed law.** From *some* camera any two
mutually visible solids overlap, so the exact relation is the complete graph.
Restricting it to overlaps that *melt* does not help: at the boundary of any
screen overlap the depth step goes continuously to zero, so every overlapping
pair has a sub-knee sliver somewhere. Any implementable version is a distance
heuristic — `TOUCH_EPS` grown — with no law behind the choice of distance.
And the exact form would need the camera, which the derive-time planner does
not have and cannot get: `CUSTOM0` is baked into `ArrayMesh` vertices once.

**A scale-relative silhouette predicate cannot reach the worst cases either.**
Each B tap carries up to half a code of error, so the Laplacian carries up to
4 codes = 0.156 m of pure noise. The knee can come down perhaps 2×, to
~0.24 m, and no further. The four worst pairs (0.036–0.17 m) are inside the
noise band. The information is not in B and no predicate over B can recover
it.

### Decision — re-encode G as an identity

```glsl
float nrm = step(0.5 / 1023.0, abs(c_c.g - c_l.g))
          + step(0.5 / 1023.0, abs(c_c.g - c_r.g))
          + ... ;   // any neighbour differing is a crease
```

Equality is **exactly correct here**, and by construction rather than by luck:

- `screen_tex` is `filter_nearest` (pinned by `shader_contract_test.gd`).
- `CUSTOM0` is piecewise constant per face by construction — a box is built
  with 24 vertices, 4 per face, `vertex_ordinal = vertex / 4`; a column's 12
  vertices per segment split rim/rim/flank so a flank never shares a vertex
  with a rim (`render/paint.rs`). No interpolation gradient exists inside a
  face.
- Nothing can smear it: `msaa_3d = 0`, `screen_space_aa = 0`, `use_taa =
  false`, `use_debanding = false`, `scaling_3d/scale = 1.0`, measured live.

A half-code epsilon rather than a literal `!=`, so future bit noise cannot
manufacture a crease. Centre-vs-neighbour rather than left-vs-right, which
fixes a pre-existing miss: the current form is blind to a one-pixel-wide
sliver of a differing label.

**What identity unlocks.** Labels need only be *distinct*, not `MIN_SEP`
apart. The band `[0.15, 0.96]` at 1024 levels holds ~828 codes, or ~414 at a
safe two-code spacing — against **five** today. The shipped map has 179
superface classes, so every class can hold its own label and the melt
inventory goes to zero **without touching the Laplacian at all**, because the
melts are precisely the small depth steps where B is blind and G is where the
information already was.

> **Corrected 2026-08-18, after the ladder was densified.** The channel does
> not deliver a nominal code everywhere: `render::channel::WORST_STEP_CODES`
> records a widest measured gap of 1.25 nominal codes (Mesa/AMD desktop GL;
> 1.02 on SwiftShader and ANGLE/Metal). So the band holds ~662 reliably
> distinct codes, not ~828, and ~331 at two-code spacing rather than ~414.
> The conclusion is unchanged — 179 classes fit inside 331 with room to
> spare — but any future plan that sizes a palette against 828 is sizing it
> against a number no driver measured.

**And the adjacency question dissolves.** Two distinct classes draw a seam
wherever they meet on screen — touching or 0.4 m apart, it makes no
difference — while coplanar-merged faces keep bit-identical labels and still
melt by construction. The merge law in `render/superface.rs` is untouched.

So the recorded "pair that must land together" is half right: the re-encoding
is the fix, and growing adjacency is not needed at all once it lands. The
scale-relative predicate stays worth doing, but as an independent quality item
for *large* steps between same-class surfaces at distance — not as a
co-requisite.

### Rejected alternatives

| option | why not |
|---|---|
| Grow `TOUCH_EPS` to ~0.45 m | Needs 6–7 world colours against a palette that can gain at most one. At eps > 0.30 a chair becomes a K6 and starves. |
| `TOUCH_EPS` ≈ 0.20 as a stopgap | Fits in 4 colours and does catch the three unfixable-by-B pairs. A heuristic the next authored level breaks; kept only as a fallback if identity is deferred. |
| Lower the absolute silhouette knee to (0.006, 0.015) | Catches 34 of 40 pairs for one constant, but 0.24 m is only 1.5× the worst-case quantisation noise and ~2× the grazing-floor Laplacian. Resolution-dependent, so a knee tuned on a 4K monitor speckles a 720p one. |
| Shrink `DIST_PACK_RANGE` to buy B precision | Unavailable: 39.71 m of required slab diagonal against 40. |

### Residual risk

Graph colouring is **kept**, not deleted — with a large palette and two-code
spacing it never starves (demand is ≤ 9 even at `TOUCH_EPS = 1.0`), and it is
the graceful-degradation path if a platform ever reports fewer codes. The
change is an encoding change, not a removal of the allocator.

---

## Gap 2 — the prop ring cut. The recorded design was the expensive one.

### What is actually true

Measured over the shipped scene on a 0.1 m grid of reachable eye positions:
**1.34 m² of standing area** where the Radio is prop-occluded but wall-clear —
a contiguous wedge you walk through entering the radio room — and **9.66 m²**
for the Fan. At 3.3–4.0 m the pillar covers **100%** of the radio's
silhouette.

The artifact is sharper than "a wash": at those pixels `scene_d` is the
*radio's* 3.3 m, not the pillar's 0.9 m, so every shell root between them is
admitted. A ring interrupted by the pillar everywhere else **continues
through the radio-shaped patch**, and the radio's dim ghost flashes bright as
the ring sweeps. The discontinuity is what reads as broken. Do the same with a
*wall* between you and the radio and the ring is cut dead — props alone leak.

**Nothing the post pass can read answers it.** Exhaustively: B and G hold the
source's values (the source's fragment won the pixel), R is a wall-count
exponent, A is never written but is a write opportunity rather than a read,
and neighbour taps are different rays — useless where the pillar covers the
source completely, which is the whole of the near region.

**The blocker table is the wrong shape.** 106 props against `MAXW = 32`;
125 entries would be **6.6× the hottest loop in the game** — 259 M near-tests
per frame at 1080p — on a dynamically-indexed uniform-array loop, the
construct WebGL2 and mobile drivers handle worst. And a column's bounding
square over-approximates by 41.4% radially, which at 2.3 m is a **35 px false
notch on each side** of every pillar: trading a corner-case leak for a new
artifact everywhere.

### Decision — a per-source CPU visibility flag

`WaveLevel::tick_sources` already walks every source every frame with the eye
in hand and computes a per-source CPU sight answer (`source_muffle`). It
computes one more: is this source occluded from the eye by a prop? The blocked
sources' world boxes go to `post_mat` alone as `u_blocked_src[MAXS]`
(MAXS = 8 → **256 bytes**), and the ring cut becomes
`seen_walled || seen_blocked_source`.

Why this wins:

- **The cost leaves the hot path.** CPU work is bounded by *sources* × *props*
  = 212 tests per frame, unmeasurable. GPU work is bounded by the **source**
  count (2), not the prop count (106, and growing with every crate a designer
  drags in). The table option's cost grows with content; this does not.
- **The shape problem evaporates.** On the CPU the exact cylinder and the
  exact wedge hull are available (`prop_shape::wedge_hull` is right there).
  No circumscribed-vs-inscribed trade, no 35 px notches.
- **It cannot touch `source_muffle`.** Props never enter `self.occluders`, so
  the silhouette muffle law is untouched — which putting props in the shared
  table would have silently broken, dimming every prop-shadowed source to 0.3.
- **It is where this project already puts such decisions**: a pure function
  over immutable inputs in a cargo-tested module, applied by a thin boundary
  adapter, with the verdict pushed as data.
- **ORed, so it cannot regress.** A level that pushes nothing degrades to
  exactly today's behaviour — the same argument the outline cap already banked.

### What it does not fix, stated

- **All-or-nothing per source.** A source half behind a pillar edge flips
  entirely on or off. `source_muffle` already accepts precisely this,
  deliberately, so the precedent stands — but the boundary is where it will
  pop. Mitigation if a playtest dislikes it: test the hub plus a few body
  points and require a majority.
- **The per-pixel over-kill it inherits.** `seen_walled` is fragment-constant,
  so it kills *all* roots at that pixel including a shell a metre in front of
  the player's face. That is a **pre-existing** defect of the wall path, found
  during this research and worth recording: a source seen through a wall
  punches a source-shaped hole in near rings. Fixing it properly means testing
  per-root against the occluder, which is a separate and larger change.

### Taste, not correctness — needs a playtest

Whether a prop-hidden source should cut the ring **completely**, matching a
wall, or merely attenuate it. A wall is a barrier; a pillar is not, and the
fully-cut version may itself read as a hole. **Planned as a knob, not a
constant.**

---

## Gap 3 — the executable oracle. Mostly already built, and misnamed.

### What is actually true

`rust/src/observe/oracle.rs` is not missing so much as **obsolete**. The
2026-08-11 spec designed `expect_lit(walls_between, kind) -> Verdict` on the
premise that player sounds are cut at a wall while a world source is muffled
but not silenced. The barrier campaign deleted `HUM_THROUGH` and made a wall
absolute for every kind, so that function now degenerates to `walls == 0` with
a dead `kind` parameter, and its constants no longer exist.

The comparison it was designed to make — Rust's belief against real pixels —
**is already implemented**, inline, at `occlusion_probe.gd`'s checks 10 and
11: `explain_ray(...)["camera_crossings"] == 1`, then the pixel held to a
hand-derived window `[0.13, 0.30]`. That is the oracle pattern, shipped.

**What is genuinely missing is a scheduler and a host, not an oracle.** The
rot is measurable: two of the eleven checks had silently become unfailable and
were caught only because a human went looking.

### Decision — automate the probe, drop `oracle.rs`

The stated obstacle ("headless CI cannot see shader-reveal leaks") is true of
`--headless`, which admits only the `dummy` driver. It is **not** true of
CI-without-a-GPU. This repository's own `Research-Linux-CI-and-Shipping.md`
already documents the recipe:

```sh
LIBGL_ALWAYS_SOFTWARE=1 dbus-run-session xvfb-run -a godot --path game …
```

and the project pins `gl_compatibility`, so llvmpipe suffices — no Vulkan
stack. `test/web_smoke.sh` already proves a software rasteriser is affordable
here: it runs SwiftShader on every deploy, executing every fragment shader and
screenshotting the result.

Wall-clock cost is roughly fixed rather than proportional to rasteriser speed,
because the probe's windows are durations on the **simulated** clock: a slower
rasteriser collects fewer samples inside the same window, it does not stretch
it.

Two real residual obstacles, neither the stated one: the GitHub workflow runs
`SKIP_EXPORT=1` with a 10-minute cap, and sampling density under llvmpipe
needs the noise floor and the warm-boot-pair law re-validated.

`rust/src/observe/oracle.rs` is **not built**, and the 2026-08-11 spec is
marked superseded.

---

## What this spec does not cover

- Per-root ring occlusion (the pre-existing fragment-constant over-kill).
- The scale-relative silhouette predicate, now independent of Gap 1.
- Re-measuring the web on a GPU-backed browser rather than SwiftShader.
