//! The engine's registered node classes — the second engine boundary,
//! sibling of [`crate::ffi`]. Every node here is a hidden-machinery organ
//! the Godot layer places and tunes but never implements: scene assembly,
//! designer `#[export]` knobs, and the translation from engine callbacks
//! into the pure modules where the actual laws live. Like `ffi`, these
//! files carry values across the boundary and add no law of their own.

mod cat;
mod fan;
mod hero;
mod level;
mod limbs;
mod player;
mod wall;
