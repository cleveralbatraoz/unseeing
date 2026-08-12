//! Smooth cartoon limb geometry, shared by every creature the engine
//! draws — the hero's body and the cat alike. Per-vertex normals mean
//! the edge detector draws only one clean silhouette per shape, never
//! facet lines.
//!
//! Every helper here APPENDS `(position, normal, CUSTOM0 label)` triples
//! to a caller-owned buffer rather than writing straight into a live mesh
//! (Task 7): the cat rebuilds its whole silhouette every physics tick, so
//! the buffer is meant to be cleared and refilled in place, frame after
//! frame — `Vec::clear` keeps its capacity, so a buffer that has settled
//! into its steady-state size after a few frames allocates nothing more
//! for the rest of the cat's life. The caller hands the finished triples
//! to [`crate::render::paint::resize_triangle_surface`], the same
//! derive-time bake every column and wedge already builds its own mesh
//! through.
//!
//! `label` is one constant per whole creature or viewmodel layer — the
//! cat is one silhouette, the hero's arm is another — so every call in a
//! single build passes the identical value; nothing here chooses it.

use std::f64::consts::{PI, TAU};

use godot::prelude::*;

/// One built limb's raw geometry: `(position, normal, CUSTOM0 label)`
/// triples, local space, three per triangle, no index buffer — exactly
/// what [`crate::render::paint::resize_triangle_surface`] takes.
pub(super) type LimbBuf = Vec<(Vector3, Vector3, f32)>;

/// A latitude/longitude sphere fan at the full 6 x 12 tessellation (two
/// triangles per cell, a normal per vertex) — the hero body's spheres,
/// unchanged.
pub(super) fn sphere(buf: &mut LimbBuf, c: Vector3, r: f32, label: f32) {
    sphere_res(buf, c, r, 6, 12, label);
}

/// A sphere whose latitude/longitude tessellation scales with its screen
/// size: a pea-sized joint at outline resolution needs nothing near 6 x
/// 12 = 432 verts to read as one clean contour, and the cat is mostly
/// such joints. Radius-tiered so small spheres cost a fraction of the
/// FFI while the silhouette is identical.
pub(super) fn sphere_lod(buf: &mut LimbBuf, c: Vector3, r: f32, label: f32) {
    let (la, lo) = if r >= 0.05 {
        (6, 12)
    } else if r >= 0.02 {
        (4, 8)
    } else {
        (3, 6)
    };
    sphere_res(buf, c, r, la, lo, label);
}

/// The general lat/long sphere fan at an explicit resolution.
fn sphere_res(buf: &mut LimbBuf, c: Vector3, r: f32, la: i32, lo: i32, label: f32) {
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
                buf.push((c + n * r, n, label));
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
pub(super) fn tube(buf: &mut LimbBuf, a: Vector3, b: Vector3, r1: f32, r2: f32, label: f32) {
    tube_res(buf, a, b, r1, r2, 10, label);
}

/// A tapered tube at an explicit radial segment count — hair-thin parts
/// (whiskers) read as one line at 4 segments and needn't pay for 10.
pub(super) fn tube_res(
    buf: &mut LimbBuf,
    a: Vector3,
    b: Vector3,
    r1: f32,
    r2: f32,
    seg: i32,
    label: f32,
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
            buf.push((vertex, normal, label));
        }
    }
}
