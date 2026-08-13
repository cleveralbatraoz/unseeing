//! How the world is SEEN, replacing the per-instance object-id crease with
//! per-vertex superface labels (`docs/superpowers/specs/2026-08-12-superface-outline-rendering-design.md`).
//! Object logic loses all id knowledge; a paint pass bakes a label into
//! every vertex instead, so two overlapping solids agree on the G channel
//! by CONSTRUCTION rather than by a shader tie-break.
//!
//! - `faces` — pure geometry: a solid's shape becomes its world-space
//!   planar faces, the vocabulary every later stage (the merge law, the
//!   label colouring) is built from.
//! - `superface` — the merge law over faces and the label-separation
//!   graph: which coplanar overlapping faces become one class, and which
//!   resulting classes must take separated labels.
//! - `labels` — colouring the unified world-face/source-role graph against
//!   the palette and role table: creatures and slabs take fixed numeric role
//!   labels, while world faces and each source instance take separated labels
//!   from a small reusable palette.
//! - [`paint`] — the thin mesh boundary that bakes world-face or semantic-role
//!   labels into `ArrayMesh` `CUSTOM0`; derivation and runtime builders share
//!   it while the geometry and label decisions remain pure.

pub mod faces;
pub mod labels;
pub mod paint;
pub mod paint_plan;
pub mod superface;

pub use faces::{Face, Shape, faces};
pub use labels::{Labelling, MIN_SEP, Role, role_label, separated};
pub use superface::{COPLANAR_EPS, PATCH_EPS, Superfaces, superfaces};
