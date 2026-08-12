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
//! - [`display_plan`] — where the window goes: the monitor's own
//!   resolution as the default, and the centered, decoration-aware box a
//!   windowed game falls back to when full screen is switched off.
//! - [`clustering`] — how ray hits merge per 0.9 m cell so flat walls
//!   answer as a few points, ranked in a deterministic total order.
//! - [`echo_queue`] — scheduled reflections that fire at the exact
//!   instant the primary wavefront reaches their surface point.
//! - [`sound_source`] — what a world sound source IS: the volume ladder
//!   (amplitude is gain, reach is linear in it), even spread against
//!   directed cone, and the cadence gate every source's clock runs through.
//! - [`fan_wave`] — the oscillating fan's motion curves and its shipped
//!   voice: the world's DIRECTED source, a cone swept by a pivoting head.
//! - [`radio_wave`] — the radio's shipped voice: the world's LOUDEST and
//!   EVEN source, and the pinned ladder between the two.
//! - [`viewmodel`] — the hero's own body as the camera sees it: head-bob
//!   and sway curves, the cane's carry and strike, the footstep cadence.
//! - [`settings_menu`] — the settings overlay's model: its rows, its
//!   cursor, what a key press means, and the exact text each row shows.
//! - [`level_plan`] — the level's derived technical contracts: wall box
//!   dimensions, axis snapping, centerlines, the dev demo tap.
//! - [`prop_shape`] — the geometry of the shapes a prop can be: the
//!   generated triangular prism the engine ships no primitive for.
//! - [`oid_palette`] — which flat object id each box in the world carries,
//!   colouring the touch graph so every seam between two objects draws.
//! - [`reproduce`] — one instant of the running world as a value: the
//!   capture format, its hand-derived byte layout, the FNV-1a state hash,
//!   and the field-naming diff a divergent restore is proved against.
//! - [`observe`] — the wave engine described to an agent as data: pool,
//!   eviction, occlusion, the object-id touch graph, the reflection fan —
//!   snapshot and explain, never a rendered frame.
//! - [`render`] — how the world is SEEN: per-vertex superface labels
//!   replacing the per-instance object id, so overlapping solids agree on
//!   the outline's G channel by construction. Mostly pure face/label law,
//!   cargo-tested; [`render::paint`] is the one impure edge, the
//!   derive-time pass that bakes a label into an `ArrayMesh`'s `CUSTOM0`.
//! - [`source_shape`] — the one generated shape a sound source's limbs
//!   need beyond a box or [`prop_shape::column_triangles`]: a torus, for
//!   the fan's guard ring and the radio's speaker grille.
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
pub mod display_plan;
pub mod echo_queue;
pub mod fan_wave;
mod ffi;
pub mod level_plan;
mod nodes;
pub mod observe;
pub mod oid_palette;
pub mod prop_shape;
pub mod pulse_pool;
pub mod radio_wave;
pub mod ray_fan;
pub mod render;
pub mod reproduce;
pub mod settings_menu;
pub mod sight;
pub mod sound_source;
pub mod source_shape;
pub mod viewmodel;
