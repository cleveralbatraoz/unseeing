//! Reproduction — one instant of the running world as a value, and the
//! proof that a restored world is that same instant.
//!
//! The blob is ALL-OR-NOTHING by design: a capture that cannot answer for
//! one subsystem is a refusal, never a state with a group missing. What
//! makes that enforceable is [`blob::state_hash`] — one number over EVERY
//! captured field, so "we restored almost everything" cannot pass as
//! "we restored it". When two hashes disagree,
//! [`blob::first_divergence`] names the field, which is the difference
//! between a usable failure and a shrug.
//!
//! Everything here is pure: no engine classes, no Godot runtime, plain
//! `cargo test`. The boundary that fills these structs from live nodes
//! and hands the blob to GDScript lives in `crate::nodes`.

pub mod blob;

pub use self::blob::{
    CaptureState, CatCapture, EnvCapture, HeroCapture, SourceCapture, canonical_bytes,
    first_divergence, fnv1a64, state_hash,
};

/// The canonical format's version, carried INSIDE [`CaptureState`] and
/// hashed as its first field.
///
/// Bump it whenever the encoded byte layout changes — a field added,
/// removed, reordered, retyped, or an enum discriminant renumbered. The
/// hash of an old blob then cannot collide with the hash of a new one by
/// construction, so a stale blob restored into a newer build fails as a
/// version refusal instead of as a mystery divergence twenty fields
/// deep.
pub const FORMAT_VERSION: u32 = 1;
