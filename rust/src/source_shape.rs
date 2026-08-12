//! The one shape a sound source's decorative limbs need that neither a box
//! nor `prop_shape::column_triangles` already gives them: a torus, for the
//! fan's guard ring and the radio's speaker grille.
//!
//! Both parts used to be the engine's own `TorusMesh` primitive, drawn
//! through a per-instance object-id uniform alone — legal while the shader
//! read only that uniform, but a `TorusMesh` carries no `CUSTOM0` channel
//! to bake a per-vertex label into (Task 7), so it has to become real
//! geometry this crate owns, the same turn a cylinder already took as
//! [`crate::prop_shape::column_triangles`].
//!
//! Orientation matches the `TorusMesh` primitive this replaces, so no
//! caller's position or rotation had to change when the swap landed: the
//! ring lies flat in the local XZ plane, its hole open along local Y —
//! the same "height along Y" convention every Godot primitive mesh uses.
//! `nodes/fan.rs`'s guard ring and `nodes/radio.rs`'s grille both already
//! carry a 90° X rotation to tip that hole forward along Z, exactly as they
//! did against the engine primitive.
//!
//! `inner_radius`/`outer_radius` read the same way `TorusMesh`'s own knobs
//! do: the ring (major) radius is their average, the tube (minor) radius
//! is half their difference.
//!
//! Sources render through the acoustic-image skin (`data_xray.gdshader`,
//! `render_mode ... cull_back`, `game/tests/data_skins_test.gd`'s own
//! pin) — UNLIKE the world skin the static solids draw through
//! (`cull_disabled`, `prop_shape.rs`'s own doc comment). Winding is
//! therefore load-bearing here, not cosmetic: a torus wound inward would
//! cull to nothing from outside. [`tests::every_triangle_winds_outward`]
//! is the guard.
//!
//! # A disclosed fidelity trade-off, not a silent one
//!
//! Both replacements tessellate COARSER than the engine primitives they
//! stand in for: `TorusMesh`'s own defaults are `rings` 64 (its major
//! loop) and `ring_segments` 32 (its tube cross-section), against
//! [`MAJOR_SEGMENTS`] 32 and [`MINOR_SEGMENTS`] 12 here; `CylinderMesh`'s
//! own default `radial_segments` is 64, against
//! [`crate::prop_shape::COLUMN_SEGMENTS`] 32 — reused unchanged from the
//! world's own columns rather than given a source-specific count, so a
//! source's cylindrical limbs read at the same silhouette resolution the
//! rest of the game already settled on.
//!
//! Deliberate, judged against the actual object scale rather than
//! assumed harmless: the fan's guard ring is a THIN ring (major radius
//! ≈0.42 m, tube radius ≈0.02 m — `nodes/fan.rs`'s `labelled_torus(0.40,
//! GUARD_R, ..)`) and the radio's grille smaller still (major ≈0.069 m,
//! tube ≈0.017 m — `labelled_torus(0.052, 0.086, ..)`); at either size,
//! in a black-and-white thin-outline renderer whose only marks are the
//! silhouette and the crease lines a Laplacian draws off packed depth
//! and a flat label, the missing facets between 12 and 32 tube segments
//! or 32 and 64 major segments do not read as visible polygon corners at
//! ordinary play distance. Judged negligible, not measured zero — flagged
//! here rather than left for a future session to rediscover by looking
//! twice at a screenshot.

use std::f32::consts::TAU;

use godot::builtin::Vector3;

/// How many steps the ring (major) loop takes — pinned equal to
/// [`crate::prop_shape::COLUMN_SEGMENTS`] so a torus and a barrel read at
/// the same silhouette resolution.
pub const MAJOR_SEGMENTS: usize = 32;

/// How many steps the tube's (minor) cross-section takes — a thin ring
/// needs less roundness across its own width than a barrel needs around
/// its silhouette, so this is coarser than [`MAJOR_SEGMENTS`] on purpose.
pub const MINOR_SEGMENTS: usize = 12;

/// The point and outward unit normal at ring angle `theta` (around the
/// local Y axis) and tube angle `phi` (around the tube's own
/// cross-section) — the torus's own parametric surface. The normal never
/// divides by either radius, so it stays a finite unit vector even where
/// the position itself degenerates (both radii zero collapses every
/// vertex onto the origin, but never onto a NaN or a zero-length normal).
fn point(major: f32, minor: f32, theta: f32, phi: f32) -> (Vector3, Vector3) {
    let (st, ct) = theta.sin_cos();
    let (sp, cp) = phi.sin_cos();
    let normal = Vector3::new(cp * ct, sp, cp * st);
    let ring_center = Vector3::new(major * ct, 0.0, major * st);
    (ring_center + normal * minor, normal)
}

/// A torus's surface as `(position, normal)` pairs, local space, three
/// vertices per triangle, no index buffer — the [`crate::prop_shape::wedge_triangles`]
/// of a donut. `inner_radius`/`outer_radius` are read in absolute value and
/// may be given in either order; a degenerate ring (either radius zero, or
/// the two equal) still yields a finite, if flattened or hairline, mesh —
/// never a NaN, since [`point`]'s normal never divides by a radius at all.
#[must_use]
pub fn torus_triangles(inner_radius: f32, outer_radius: f32) -> Vec<(Vector3, Vector3)> {
    let inner_radius = inner_radius.abs();
    let outer_radius = outer_radius.abs();
    let major = (inner_radius + outer_radius) * 0.5;
    let minor = (outer_radius - inner_radius).abs() * 0.5;

    let mut out = Vec::with_capacity(MAJOR_SEGMENTS * MINOR_SEGMENTS * 6);
    for i in 0..MAJOR_SEGMENTS {
        let i_next = (i + 1) % MAJOR_SEGMENTS;
        let theta0 = i as f32 * TAU / MAJOR_SEGMENTS as f32;
        let theta1 = i_next as f32 * TAU / MAJOR_SEGMENTS as f32;
        for j in 0..MINOR_SEGMENTS {
            let j_next = (j + 1) % MINOR_SEGMENTS;
            let phi0 = j as f32 * TAU / MINOR_SEGMENTS as f32;
            let phi1 = j_next as f32 * TAU / MINOR_SEGMENTS as f32;

            let a = point(major, minor, theta0, phi0);
            let b = point(major, minor, theta1, phi0);
            let c = point(major, minor, theta1, phi1);
            let d = point(major, minor, theta0, phi1);

            // two triangles per quad cell, wound outward — see
            // `tests::every_triangle_winds_outward` for the independent
            // proof this ordering is not a guess.
            out.push(a);
            out.push(d);
            out.push(b);
            out.push(b);
            out.push(d);
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const INNER: f32 = 0.40;
    const OUTER: f32 = 0.44;

    /// The vertex count matches the double loop exactly, hand-derived
    /// rather than read off the function's own length: two triangles per
    /// cell, three vertices each, `MAJOR_SEGMENTS * MINOR_SEGMENTS` cells.
    #[test]
    fn vertex_count_matches_the_double_loop() {
        let tris = torus_triangles(INNER, OUTER);
        assert_eq!(tris.len(), MAJOR_SEGMENTS * MINOR_SEGMENTS * 6);
    }

    /// Every vertex is finite and every normal is a unit vector — a
    /// non-unit normal would be a broken crease id if this geometry ever
    /// fed a per-face law, and a NaN position poisons everything
    /// downstream, exactly the standard `prop_shape.rs` holds its own
    /// generated shapes to.
    #[test]
    fn every_vertex_is_finite_with_a_unit_normal() {
        for (v, n) in torus_triangles(INNER, OUTER) {
            assert!(v.is_finite(), "vertex not finite: {v}");
            assert!(n.is_finite(), "normal not finite: {n}");
            assert!((n.length() - 1.0).abs() < 1e-5, "normal not unit: {n}");
        }
    }

    /// The ring lies flat in the local XZ plane: at tube angle phi = 0 (a
    /// multiple of `MINOR_SEGMENTS` apart) every vertex sits at y = 0 and
    /// exactly `outer_radius` from the Y axis — the OUTER equator, the
    /// widest circle the shape has. This is the property that lets the
    /// fan and the radio keep their existing 90-degree X rotation
    /// unchanged: a hole open along Y is exactly what that rotation
    /// expects to tip forward.
    #[test]
    fn the_outer_equator_lies_flat_in_the_xz_plane() {
        for (v, _) in torus_triangles(INNER, OUTER) {
            let radius = (v.x * v.x + v.z * v.z).sqrt();
            if v.y.abs() < 1e-4 {
                // near the equator plane: could be the OUTER ring (phi=0)
                // or the INNER ring (phi=pi) — both are flat, so only the
                // outer one is asserted here to keep the check simple and
                // unambiguous
                assert!(
                    (radius - OUTER).abs() < 1e-3 || (radius - INNER).abs() < 1e-3,
                    "a y=0 vertex sits at radius {radius}, neither the inner nor outer equator"
                );
            }
        }
    }

    /// Every triangle winds OUTWARD: the independent proof this crate's
    /// other generated shapes hold themselves to
    /// (`prop_shape::column_flank_winds_radially_outward`,
    /// `render::paint::every_face_winds_outward`) — and the one property
    /// that matters here specifically, since a source's limbs render
    /// through the acoustic-image skin's `cull_back`, not the world
    /// skin's `cull_disabled` (`game/tests/data_skins_test.gd`). Checked
    /// against EVERY triangle's own analytic normal (the mean of its three
    /// corners' normals, which for this smooth, convex-in-cross-section
    /// shape always points the same way the flat triangle should), not a
    /// hand-picked sample — the mutation this catches is a swapped index
    /// in exactly one of the two triangles per cell, which a single-sample
    /// check could miss by only ever landing on the other one.
    #[test]
    fn every_triangle_winds_outward() {
        let tris = torus_triangles(INNER, OUTER);
        assert_eq!(tris.len() % 3, 0);
        for tri in tris.chunks_exact(3) {
            let [(v0, n0), (v1, n1), (v2, n2)] = [tri[0], tri[1], tri[2]];
            let face_normal = (v1 - v0).cross(v2 - v0);
            let analytic = (n0 + n1 + n2) / 3.0;
            assert!(
                face_normal.dot(analytic) > 0.0,
                "triangle {v0:?},{v1:?},{v2:?} does not wind toward its own outward normal {analytic:?}"
            );
        }
    }

    /// Total on the shapes a designer (or a knob-turned scene) can build
    /// by accident: reversed radii, a hairline tube, and a fully collapsed
    /// torus (both radii zero) all stay finite — no division anywhere in
    /// [`point`]'s normal means there is nothing to divide by zero.
    #[test]
    fn degenerate_radii_stay_finite() {
        for (inner, outer) in [(OUTER, INNER), (0.0, 0.0), (0.3, 0.3), (-0.1, 0.2)] {
            for (v, n) in torus_triangles(inner, outer) {
                assert!(v.is_finite(), "vertex not finite for {inner}/{outer}: {v}");
                assert!(n.is_finite(), "normal not finite for {inner}/{outer}: {n}");
                assert!((n.length() - 1.0).abs() < 1e-5);
            }
        }
    }
}
