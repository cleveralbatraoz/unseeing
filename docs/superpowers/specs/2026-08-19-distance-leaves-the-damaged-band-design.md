# Distance leaves the damaged band

Status: DESIGNED, measurement in progress. Approved 2026-08-19.
Supersedes the reconstruction-tolerance reasoning in
`2026-08-18-closing-the-renderers-last-gaps-design.md` §"the channel's
accuracy", which rests on a premise this document refutes.

## The defect, measured

Godot 4.7.1's `gl_compatibility` pass puts every value a spatial shader
writes to `ALBEDO` through an sRGB pair whose two halves are not inverses:

    returned(v) = linear_to_srgb_exact( srgb_to_linear_cubic(v) )

with `srgb_to_linear_cubic(c) = c*(c*(c*0.305306011 + 0.682171111) +
0.012522878)` — Godot's polynomial approximation — against the exact power
law on the way out. Fitted to measurement within ~1 nominal 10-bit code
across the range.

It is not the driver: **AMD radeonsi (Mesa 25.0.7) and llvmpipe software
agree to the code**. It is not storage quantisation: a `SubViewport` with
`use_hdr_2d` — a half-float target — shows the transfer PURE (0.0274 →
0.0008, 0.05 → 0.0300, 0.25 → 0.2491, 0.50 → 0.5010, 0.75 → 0.7495,
matching the model to four decimals). **There is no render-target escape
hatch in this renderer.**

Everything at or below **28 nominal codes (v ≈ 0.0274)** comes back as
exactly zero.

Measured through the SHIPPED `data_pass.gdshader`, a real quad at a real
distance, read exactly as `hearing_post` reads it (`B = clamp(vd/40, 0, 1)`):

| true distance | what B says | error |
|---|---|---|
| 0.5 m | 0.11 m | −0.39 |
| **1.0 m** | **~0.0 m** | **−1.02** |
| 2.0 m | 1.29 m | −0.71 |
| 4.0 m | 3.61 m | −0.39 |
| 8.0 m | 7.92 m | −0.08 |
| ≥12 m | exact within readback resolution | 0 |

Worst error **1.02 m against a `sight::RECT_SHRINK` of 0.03 m**, and it is a
systematic UNDERSHOOT, never an overshoot. `RECT_SHRINK` was raised from 0.02
to 0.03 to absorb an overshoot that cannot occur.

## What this does and does not touch

- **G is unharmed.** Labels live in `[0.15, 0.96]`, where the transfer's
  slope is 1.00–1.05; `MIN_SEP = 0.08` comes back as 0.084 at the bottom of
  the band. The superface and crease laws are safe and are not changed here.
- **R is grazed.** `SOURCE_THROUGH^3 = 0.027` sits exactly on the cliff;
  `reveal::PRESENCE = 0.068` survives at 0.052. The presence floor works,
  but its "twice the grain" margin is 1.5× on screen. Recorded, not fixed
  here.
- **B is the defect.** Five of its six reads in `hearing_post` need ABSOLUTE
  metric distance (`seen_pt`, `wall_first_entry`, `air_d`, and the four-tap
  source-reveal borrow); the sixth, the silhouette Laplacian, needs a
  correct local GAIN and an absolute knee. None is merely monotone.

## The decision

**Pack distance into the part of the channel the pipeline gives back, and
corroborate the centre tap with the depth buffer.**

Write end: `pack_data` maps `0..DIST_PACK_RANGE` affinely into
`[SAFE_FLOOR, 1]` — one fused multiply-add replacing one clamp. Read end:
every B read becomes `unpack_distance(b)`. The Laplacian's coefficients sum
to zero, so `SAFE_FLOOR` cancels exactly and only the in-band gain survives
— which is why this closes the sub-1.1 m hole at its root rather than
moving it.

Centre tap only, the depth buffer is asked as a WITNESS: world fragments
unprojected through `INV_PROJECTION_MATRIX` (no `CAM_NEAR`, no `CAM_FAR`, no
reversed-Z assumption — unprojecting `vec4(uv*2−1, depth, 1)` and taking
`length()` IS the radial distance), acoustic images through
`render::depth::source_depth`'s band inverse. Accepted only when it agrees
with the coarse reading within `WITNESS_TOL`, so a dead depth texture
degrades to exactly the colour channel and never below it. That is the same
discipline `pulse_pool.gdshaderinc` already states for `seen_image`, and it
is corroboration rather than a new dependency.

Godot's Compatibility depth attachment is `GL_DEPTH24_STENCIL8`
(`drivers/gles3/storage/texture_storage.cpp` at 4.7.1-stable) with
`GL_GEQUAL` — 24-bit reversed-Z. Unprojected, half an LSB is 0.6 µm at 1 m,
14.9 µm at 5 m, 0.24 mm at 20 m and 0.95 mm at 40 m. `hearing_post` already
fetches `depth_tex` at all five taps, so the witness costs **no new texture
fetch**.

Rejected: pre-compensating the write with the inverse transfer. It
hard-codes an internal Godot polynomial that can change between engine
versions, and it cannot recover the bottom 28 codes, which are destroyed.

## Why the constant is not in this document

`SAFE_FLOOR` is the smallest representable code at or above the measured
transfer floor, rounded UP, never to a prettier decimal. **No implementer
may type it until the probe has read it.**

The measurement method is settled by an experiment run while writing this:
laying the sweep out SPATIALLY down a screen column, and comparing each row
against the base recomputed from `uv.y`, disagrees with the constant-per-
frame method by more than the quantity being measured — the spatial version
reported −16.28 codes at base 0.93 where the constant-base version measured
−0.05. `tap_error_probe`'s own header already records why: with a base that
moves 1.28 codes per screen row, a reading about sampling wears a reading
about the channel's clothes. **The floor is therefore measured by
`tap_error_probe`, which holds one base per frame, not by a region added to
`platform_probe`.**

## Migration, five commits, each green alone

1. **The measurement.** `tap_error_probe` sweeps the whole channel (it began
   at 0.30 and so could not see the part that matters: 1 m of distance IS
   0.025) and reports the safe floor against a tolerance ladder. Reports
   only — the constant a gate would check does not exist yet. Deliverable:
   the floor on desktop and, through `tools/measure_web_platform.sh`, on web.
2. **The Rust laws.** `render::channel` gains `TRANSFER_FLOOR` (commit 1's
   reading), `SAFE_FLOOR`, `pack_distance`, `unpack_distance`,
   `witness_tol`, `witnessed_distance`; `recon_eps` and `quantum` are
   redefined over the band. `render::depth` gains `source_distance`.
   `render::silhouette` is new, owning the knee. Nothing calls any of it yet.
3. **The channel moves.** `pack_data` writes `pack_distance(vd)`; the six
   `hearing_post` reads decode; `lap` scales; `u_sil_knee` is pushed and
   defaults wrong. This is the behaviour commit.
4. **The witness.** The centre tap gains the depth corroboration.
5. **The deletions and the record.** `max_safe_range`,
   `reconstruction_budget` and its dead runtime call, the refuted paragraphs
   in `channel.rs` and `sight.rs`, and the wiki.

## The one thing most likely to make this wrong

The transfer model was fitted to the HDR path. It predicts `T(0.0274) =
0.0111`; the shipped LDR path returns exactly **0**. Something beyond the
fitted transfer crushes that end, and it is not yet identified. If the same
something also bites inside the packed band, the real floor is higher and
every number moves with it. That is why commit 1 is the probe and nothing
else, and why the floor ships as measured rather than as modelled.
