//! The engine's registered node classes — the second engine boundary,
//! sibling of [`crate::ffi`]. Every node here is a hidden-machinery organ
//! the Godot layer places and tunes but never implements: scene assembly,
//! designer `#[export]` knobs, and the translation from engine callbacks
//! into the pure modules where the actual laws live. Like `ffi`, these
//! files carry values across the boundary and add no law of their own.
//!
//! Two of them are not classes but ABSTRACTIONS, published to the engine
//! with `#[godot_dyn]` so the level can hold a heterogeneous list of nodes
//! without ever naming a concrete class: [`solid`]'s `WaveSolid` (anything
//! the waves can strike) and [`source`]'s `SoundSource` (anything that
//! makes the world's own sound). gdext cannot derive one registered class
//! from another, so "a radio IS a sound source" is a Rust trait, not
//! inheritance — and adding a shape or a source means adding a file, not
//! editing the level.

mod cat;
mod fan;
mod game;
mod hero;
mod level;
mod limbs;
mod observer;
mod player;
mod props;
mod radio;
mod restorer;
mod run;
mod settings;
mod solid;
mod source;
mod spawn;
mod support;
mod wall;
