//! How the world is SEEN, replacing the per-instance object-id crease with
//! per-vertex superface labels (`docs/superpowers/specs/2026-08-12-superface-outline-rendering-design.md`).
//! Object logic loses all id knowledge; a paint pass bakes a label into
//! every vertex instead, so two overlapping solids agree on the G channel
//! by CONSTRUCTION rather than by a shader tie-break.
//!
//! - [`channel`] — what one data channel actually holds, measured rather
//!   than assumed, and the geometric tolerance that turns on it: the
//!   hearing pass reconstructs a world point from B, and only
//!   `sight::RECT_SHRINK` keeps that point outside the wall it stands
//!   against.
//! - [`crease`] — the rendered response to a label difference: the
//!   hearing pass's crease knee, derived from `labels::MIN_SEP` rather than
//!   retyped in GLSL, so the law that ALLOCATES separations and the law
//!   that DRAWS them cannot drift apart.
//! - [`knee`] — the one `smoothstep` contract every rendered fade obeys:
//!   finite as f32 and strictly ordered in the width the GPU will use. The
//!   crease, detail and silhouette knees each wrap it in their own units, so
//!   the argument is written once and a knee in metres cannot be pushed
//!   where a knee in label differences belongs.
//! - [`depth`] — the acoustic-image depth band: how a sound source's skin
//!   rides over the world without losing its own front-to-back order, with
//!   the band's width derived from the depth buffer's quantisation and the
//!   camera's frustum rather than asserted.
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
//! - [`paint_plan`] — the pure, atomic level decision: validates the complete
//!   face/source request and returns positional relabel/keep commands,
//!   ownership, and faults before any Godot resource changes.
//! - [`paint`] — the thin mesh boundary that bakes world-face or semantic-role
//!   labels into `ArrayMesh` `CUSTOM0`; derivation and runtime builders share
//!   it while the geometry and label decisions remain pure.
//! - [`silhouette`] — SHAPE: the outline a break in the world's distance
//!   draws, with its knee stated in METRES of depth step rather than in
//!   channel units, so that raising the packing range — which
//!   `level_plan::pack_range_budget` actively tells a designer to do — can no
//!   longer retune the outline behind their back.
//! - [`reveal`] — how long a swept surface keeps hearing the wave that swept
//!   it: the decay envelope and its end, as a pure function of time since
//!   the wavefront passed. `sight` says where a wave reaches; this says when
//!   it stops.

pub mod channel;
pub mod crease;
pub mod depth;
pub mod detail;
pub mod faces;
pub mod grain;
pub mod knee;
pub mod labels;
pub mod paint;
pub mod paint_plan;
pub mod reveal;
pub mod silhouette;
pub mod superface;

pub use faces::{Face, Shape, faces};
pub use labels::{Labelling, MIN_SEP, Role, role_label, separated};
pub use superface::{COPLANAR_EPS, PATCH_EPS, Superfaces, superfaces};
