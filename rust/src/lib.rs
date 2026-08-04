//! The wave/physics core of Unseeing, as a GDExtension.
//!
//! Layering law: everything below the engine boundary is pure Rust with
//! no engine types beyond godot's glam-backed math builtins — testable
//! under plain `cargo test` with no Godot runtime. Engine classes may
//! appear only in the boundary modules: `ffi` (the wave core the shim
//! wraps) and `nodes` (the registered node classes the Godot layer
//! places).
//!
//! The pure core, one law per module — each a bit-for-bit mirror of the
//! GDScript wave system it will replace, pinned by ported tests:
//! - [`cat_body`] — the companion cat's silhouette skeleton: two-bone
//!   legs, sit blend, ears and whiskers, the lagging follow-chain tail.
//! - [`cat_brain`] — the companion cat's deterministic whimsy: a seeded
//!   PCG32 wanderer that roams, pauses, sits, and abandons blocked paths.
//! - [`cat_gait`] — the companion cat's four-beat lateral-sequence walk:
//!   planted paws, swing arcs, touchdown contacts, the paw-wave voice.
//! - [`pulse_pool`] — the 64 pulse slots both shaders read as uniforms:
//!   packing, lifetimes, eviction, the live count.
//! - [`ray_fan`] — the golden-angle spherical fan that samples the world
//!   for reflections, culled to the birth surface's hemisphere.
//! - [`sight`] — which walls a straight line pierces: the analytic
//!   occluder the acoustic-image shaders count reveal and shells against,
//!   cargo-pinned reference for the GLSL transliteration.
//! - [`clustering`] — how ray hits merge per 0.9 m cell so flat walls
//!   answer as a few points, ranked in a deterministic total order.
//! - [`echo_queue`] — scheduled reflections that fire at the exact
//!   instant the primary wavefront reaches their surface point.
//! - [`fan_wave`] — the oscillating fan's motion curves and whoosh
//!   cadence, the world's one constant sound source.
//! - [`viewmodel`] — the hero's own body as the camera sees it: head-bob
//!   and sway curves, the cane's carry and strike, the footstep cadence.
//! - [`level_plan`] — the level's derived technical contracts: wall box
//!   dimensions, axis snapping, centerlines, the dev demo tap.
//! - [`oid_palette`] — which flat object id each box in the world carries,
//!   colouring the touch graph so every seam between two objects draws.
//!
//! Determinism is construction, not luck: no hashed iteration anywhere
//! near an output (ordered containers only), no system time, no
//! randomness — the same inputs replay into the same waves on every
//! platform, which is what lets desktop and wasm share one truth.
//!
//! Web builds are single-threaded (the export pins thread_support=false):
//! no threads, no rayon, no parking primitives anywhere in this crate.
//!
//! No unsafe Rust: the sole exception is the `unsafe impl ExtensionLibrary`
//! entry point in [`ffi`], whose `unsafe` keyword gdext's API mandates.
#![deny(unsafe_code)]

pub mod cat_body;
pub mod cat_brain;
pub mod cat_gait;
pub mod clustering;
pub mod echo_queue;
pub mod fan_wave;
mod ffi;
pub mod level_plan;
mod nodes;
pub mod oid_palette;
pub mod pulse_pool;
pub mod ray_fan;
pub mod sight;
pub mod viewmodel;
