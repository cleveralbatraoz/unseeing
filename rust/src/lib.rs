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
//! - [`pulse_pool`] — the 64 pulse slots both shaders read as uniforms:
//!   packing, lifetimes, eviction, the live count.
//! - [`ray_fan`] — the golden-angle spherical fan that samples the world
//!   for reflections, culled to the birth surface's hemisphere.
//! - [`clustering`] — how ray hits merge per 0.9 m cell so flat walls
//!   answer as a few points, ranked in a deterministic total order.
//! - [`echo_queue`] — scheduled reflections that fire at the exact
//!   instant the primary wavefront reaches their surface point.
//! - [`fan_wave`] — the oscillating fan's motion curves and whoosh
//!   cadence, the world's one constant sound source.
//!
//! Determinism is construction, not luck: no hashed iteration anywhere
//! near an output (ordered containers only), no system time, no
//! randomness — the same inputs replay into the same waves on every
//! platform, which is what lets desktop and wasm share one truth.
//!
//! Web builds are single-threaded (the export pins thread_support=false):
//! no threads, no rayon, no parking primitives anywhere in this crate.

pub mod clustering;
pub mod echo_queue;
pub mod fan_wave;
mod ffi;
mod nodes;
pub mod pulse_pool;
pub mod ray_fan;
pub mod viewmodel;
