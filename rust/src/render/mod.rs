//! How the world is SEEN, replacing the per-instance object-id crease with
//! per-vertex superface labels (`docs/superpowers/specs/2026-08-12-superface-outline-rendering-design.md`).
//! Object logic loses all id knowledge; a paint pass bakes a label into
//! every vertex instead, so two overlapping solids agree on the G channel
//! by CONSTRUCTION rather than by a shader tie-break.
//!
//! - `faces` — pure geometry: a solid's shape becomes its world-space
//!   planar faces, the vocabulary every later stage (the merge law, the
//!   label colouring) is built from.
//! - [`paint`] — the derive-time pass that turns a shape's faces into an
//!   `ArrayMesh` carrying the label as a per-vertex `CUSTOM0` float.

pub mod faces;
pub mod paint;

pub use faces::{Face, Shape, faces};
