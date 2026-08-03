//! Smooth cartoon limb geometry, shared by every creature the engine
//! draws — the hero's body and the cat alike. Per-vertex normals mean
//! the edge detector draws only one clean silhouette per shape, never
//! facet lines.

use std::f64::consts::{PI, TAU};

use godot::classes::ImmediateMesh;
use godot::prelude::*;

/// A latitude/longitude sphere fan, tessellated exactly like the script's
/// `_sphere` (6 x 12, two triangles per cell, normal per vertex).
pub(super) fn sphere(mesh: &mut Gd<ImmediateMesh>, c: Vector3, r: f32) {
    const LA: i32 = 6;
    const LO: i32 = 12;
    for i in 0..LA {
        let t0 = f64::from(i) / f64::from(LA) * PI;
        let t1 = f64::from(i + 1) / f64::from(LA) * PI;
        for j in 0..LO {
            let p0 = f64::from(j) / f64::from(LO) * TAU;
            let p1 = f64::from(j + 1) / f64::from(LO) * TAU;
            let n00 = lat_lon_normal(t0, p0);
            let n01 = lat_lon_normal(t0, p1);
            let n10 = lat_lon_normal(t1, p0);
            let n11 = lat_lon_normal(t1, p1);
            for n in [n00, n10, n11, n00, n11, n01] {
                mesh.surface_set_normal(n);
                mesh.surface_add_vertex(c + n * r);
            }
        }
    }
}

/// The unit normal at sphere coordinates (theta from the pole, phi around
/// the equator) — the script's inline Vector3, narrowed per component.
fn lat_lon_normal(theta: f64, phi: f64) -> Vector3 {
    Vector3::new(
        (theta.sin() * phi.cos()) as f32,
        theta.cos() as f32,
        (theta.sin() * phi.sin()) as f32,
    )
}

/// A tapered tube between two points, tessellated exactly like the
/// script's `_tube` (10 segments, quad split into two triangles, the
/// radial direction as the normal).
pub(super) fn tube(mesh: &mut Gd<ImmediateMesh>, a: Vector3, b: Vector3, r1: f32, r2: f32) {
    const SEG: i32 = 10;
    let span = b - a;
    let len = span.length();
    let axis = if f64::from(len) > 0.0001 {
        span / len
    } else {
        Vector3::UP
    };
    let reference = if f64::from(axis.y.abs()) > 0.9 {
        Vector3::new(1.0, 0.0, 0.0)
    } else {
        Vector3::new(0.0, 1.0, 0.0)
    };
    let u = axis.cross(reference).normalized();
    let v = axis.cross(u);
    for k in 0..SEG {
        let a0 = f64::from(k) / f64::from(SEG) * TAU;
        let a1 = f64::from(k + 1) / f64::from(SEG) * TAU;
        let d0 = u * (a0.cos() as f32) + v * (a0.sin() as f32);
        let d1 = u * (a1.cos() as f32) + v * (a1.sin() as f32);
        let p00 = a + d0 * r1;
        let p01 = a + d1 * r1;
        let p10 = b + d0 * r2;
        let p11 = b + d1 * r2;
        for (vertex, normal) in [
            (p00, d0),
            (p10, d0),
            (p11, d1),
            (p00, d0),
            (p11, d1),
            (p01, d1),
        ] {
            mesh.surface_set_normal(normal);
            mesh.surface_add_vertex(vertex);
        }
    }
}
