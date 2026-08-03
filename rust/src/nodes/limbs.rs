//! Smooth cartoon limb geometry, shared by every creature the engine
//! draws — the hero's body and the cat alike. Per-vertex normals mean
//! the edge detector draws only one clean silhouette per shape, never
//! facet lines.

use std::f64::consts::{PI, TAU};

use godot::classes::ImmediateMesh;
use godot::prelude::*;

/// A latitude/longitude sphere fan at the full 6 x 12 tessellation (two
/// triangles per cell, a normal per vertex) — the hero body's spheres,
/// unchanged.
pub(super) fn sphere(mesh: &mut Gd<ImmediateMesh>, c: Vector3, r: f32) {
    sphere_res(mesh, c, r, 6, 12);
}

/// A sphere whose latitude/longitude tessellation scales with its screen
/// size: a pea-sized joint at outline resolution needs nothing near 6 x
/// 12 = 432 verts to read as one clean contour, and the cat is mostly
/// such joints. Radius-tiered so small spheres cost a fraction of the
/// FFI while the silhouette is identical.
pub(super) fn sphere_lod(mesh: &mut Gd<ImmediateMesh>, c: Vector3, r: f32) {
    let (la, lo) = if r >= 0.05 {
        (6, 12)
    } else if r >= 0.02 {
        (4, 8)
    } else {
        (3, 6)
    };
    sphere_res(mesh, c, r, la, lo);
}

/// The general lat/long sphere fan at an explicit resolution.
fn sphere_res(mesh: &mut Gd<ImmediateMesh>, c: Vector3, r: f32, la: i32, lo: i32) {
    for i in 0..la {
        let t0 = f64::from(i) / f64::from(la) * PI;
        let t1 = f64::from(i + 1) / f64::from(la) * PI;
        for j in 0..lo {
            let p0 = f64::from(j) / f64::from(lo) * TAU;
            let p1 = f64::from(j + 1) / f64::from(lo) * TAU;
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

/// A tapered tube between two points at the full 10-segment
/// tessellation (quad split into two triangles, the radial direction as
/// the normal) — the hero body's tubes, unchanged.
pub(super) fn tube(mesh: &mut Gd<ImmediateMesh>, a: Vector3, b: Vector3, r1: f32, r2: f32) {
    tube_res(mesh, a, b, r1, r2, 10);
}

/// A tapered tube at an explicit radial segment count — hair-thin parts
/// (whiskers) read as one line at 4 segments and needn't pay for 10.
pub(super) fn tube_res(
    mesh: &mut Gd<ImmediateMesh>,
    a: Vector3,
    b: Vector3,
    r1: f32,
    r2: f32,
    seg: i32,
) {
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
    for k in 0..seg {
        let a0 = f64::from(k) / f64::from(seg) * TAU;
        let a1 = f64::from(k + 1) / f64::from(seg) * TAU;
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
