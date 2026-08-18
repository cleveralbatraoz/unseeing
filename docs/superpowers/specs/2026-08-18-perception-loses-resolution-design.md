# The world loses resolution

**Date:** 2026-08-18 · **Status:** approved, implementing

A blind hero's world neither goes dark nor stays lit. It **loses resolution**.

## The one law

`hearing_post.gdshader` already draws with two independent mechanisms and
then throws the distinction away:

```glsl
float lap = /* Laplacian of packed camera distance */;   // gross SHAPE
float nrm = /* difference of per-vertex labels    */;   // fine DETAIL
float edge = max(smoothstep(...lap...), smoothstep(...nrm...));   // <- destroyed here
```

Give the two terms different laws and the whole model follows:

| | SHAPE (silhouette) | DETAIL (crease) |
|---|---|---|
| reach | the wave's full 6.0 m | ~2.75 m |
| lifetime | the full 6.0 s tail | ~1.5 s |
| remembered? | **yes** | **never** |

Derived from shipped constants, not tuned. With a wall tap (gain 1.0, peak
1.0, `atten = 1/(1 + 0.06 d²)`, `pack_data`'s `exp(-0.05 vd)`, the flare
`1.3e^(-t/0.25) + 0.5e^(-t/3)`), reveal reads 1.000 at the struck point,
0.783 at `CANE_REACH`, 0.600 at 2.75 m, 0.303 at 5.1 m, 0.234 at 6.0 m; and
0.599 at t=0.50 s, 0.299 at t=1.57 s.

### The theorem

`DetailKnee` = `(SOURCE_THROUGH, SOURCE_THROUGH / LOW_KNEE_RATIO)` =
`(0.30, 0.60)`. A source's image is `muffle · max(wave, volume)` with both
factors ≤ 1, so **through one wall a source's reveal is at most 0.30 — which
is exactly the knee's floor.**

> A source behind a wall cannot draw a crease, for any wave and any volume,
> by construction rather than by tuning.

You always know a source is sounding there. Past the first wall you stop
knowing it is a *radio*. Identity is lost before presence — the one
perceptual claim in this campaign that survived adversarial review.

## Occlusion is geometry, never node class

`data_core.gdshaderinc`'s "props are transparent to waves — only walls
obstruct" is retired. A world solid occludes waves iff **both**:

- it spans the corridor: `bottom <= SLAB_T + 0.05` and `top >= WALL_H - 0.05`
- it is at least as thick as a wall: min horizontal extent `>= 2·WALL_T = 0.30 m`

On `level_01` this admits **exactly the seven pillars** (span `[0.00, 3.00]`
= floor to `WALL_H`, 0.44–0.50 m across) and refuses **the seven pipes**
(span `[0.00, 2.90]`, 0.14–0.20 m) and **all 62 boxes** (none above 2.00 m
against `EYE = 1.6`). The author had already separated the populations in
both dimensions. Table: 19 → 26 of `MAXW = 32`. No `MAXW` change.

Non-spanning props still take a source's **clarity**:
`PROP_THROUGH = sqrt(SOURCE_THROUGH) = 0.5477` — two props cost exactly one
wall — folded into `SourceImage.muffle`. **Decided: a prop DIMS the player's
ring rather than cutting it.** A pillar is not a barrier, and a full cut
reads as a source-shaped hole.

## Memory: shape only, and never anything alive

A 14×14 floor-plan grid at 2.0 m cells (196 bytes), entering the shader
through a `max` on the **silhouette term only**:

```glsl
col = max(sil * max(reveal, trace), crease * detail * reveal)
```

- `TRACE_CEIL = 0.15` — half the detail knee's floor, so memory is
  *structurally incapable* of naming a thing.
- `MEMORY_CELL = 2.0 m` — coarser than the largest non-pillar solid (1.4 m),
  so the grid cannot carry object detail even if someone wants it to.
- `TRACE_HALF_LIFE = 45 s`. **Decided: slow fade.** Not physics, and not
  claimed as such — echoic memory is 2–4 s (that is the *ring*), survey
  memory is minutes to hours. This is craft, chosen so the twentieth minute
  does not feel like the first.
- **Movers never write the trace.** Falls out of the world-static/creature
  partition `render::labels` already draws. Needs no per-mover state.

> **Decided: a silent creature is ABSENT, not a ghost.** Persistence buys a
> map you can trust about walls and cannot trust about anything alive.

Acceptance gate, falsifiable: with every cell stamped at `TRACE_CEIL`, mean
linear luminance of a `level_01` frame must rise by **< 0.01**. If it rises
more, the trace is lighting the level rather than remembering it.

## A live defect this uncovered

The documented muffle ladder is 0.30 / 0.09 / 0.027. `u_grain_amp` is
0.034, multiplied again by the vignette's 0.45 at the edge. **A three-wall
source is dimmer than the film grain — it is not visible at all**, so the
shipped build already violates the settled law that sources are always
visible. `PRESENCE = 2 · GRAIN_AMP = 0.068`, applied as a `max` on packed R
*after* `pack_data`, so the camera-distance fade cannot defeat it.

## Deliberately not simulated

Frequency of any kind; Fresnel/Maekawa arithmetic; the two-leg
`1/(d1²d2²)` law; per-fragment graded prop occlusion against a per-prop
table; runtime label modulation; per-surface memory; Doppler; binaural
cues; reverberation tails and per-material absorption; air absorption as a
separate term; second-order reflections.

Also refused, though it is the most physically defensible thing the review
surfaced: **relocating a through-wall source to the doorway it is heard
through.** On `level_01`'s geometry the doorway path beats the through-wall
path by 25–30 dB and sits 45–57° off the true bearing — but moving a drawn
node contradicts "a source is visible *as itself*" and would read as a bug.

## Rule changes

"Physics and sound-wave propagation must be exact" is the root cause of this
campaign's failure mode: five claims, all refuted 3/3, each for the same
reason — real acoustics applied to an engine that has none. Restated:

> The laws we model must be exact, pure and cargo-pinned; every stylisation
> must be named, quantified in its own units, and recorded in the wiki, and
> no constant may be justified by an acoustic derivation the engine cannot
> represent.

`MIN_SEP`, `LOW_KNEE_RATIO`, `WORLD_PALETTE` and the `[0.15, 0.96]` band are
**unchanged** — revising them was authorised and this design does not need it.
