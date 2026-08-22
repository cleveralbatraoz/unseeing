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
//! `cargo test`. The Godot boundary that fills these structs from live
//! nodes and carries the blob to Godot-side callers lives in `crate::nodes`.

pub mod blob;

pub use self::blob::{
    CaptureState, CatCapture, EnvCapture, HeroCapture, SourceCapture, canonical_bytes,
    first_divergence, fnv1a64, state_hash,
};

/// The canonical format's version, carried INSIDE [`CaptureState`] and
/// hashed as its first field.
///
/// Bump it whenever the encoded byte layout changes — a field added,
/// removed, reordered, retyped, an enum discriminant renumbered, or a
/// fixed ARITY changed: `pulse_pool::MAXP` (the 64 slots written with no
/// length prefix), `cat_body::TAIL_N`, and `cat_gait::LEGS` are all part
/// of the layout, and a blob written under a different one would decode
/// its own fields at the wrong offsets. The
/// hash of an old blob then cannot collide with the hash of a new one by
/// construction, so a stale blob restored into a newer build fails as a
/// version refusal instead of as a mystery divergence twenty fields
/// deep.
pub const FORMAT_VERSION: u32 = 2;

/// A checked restore owner's one diagnostic carrier. Validation laws stay in
/// the owner that reads the value; this type only preserves its dotted field
/// path and stable rule text across the prepared transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreValueError {
    pub path: String,
    pub rule: &'static str,
}

impl RestoreValueError {
    #[must_use]
    pub fn new(path: impl Into<String>, rule: &'static str) -> Self {
        Self {
            path: path.into(),
            rule,
        }
    }

    #[must_use]
    pub fn prefixed(self, prefix: &str) -> Self {
        Self {
            path: if self.path.is_empty() {
                prefix.to_string()
            } else {
                format!("{prefix}.{}", self.path)
            },
            rule: self.rule,
        }
    }
}

impl std::fmt::Display for RestoreValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "field {}: {}", self.path, self.rule)
    }
}

impl std::error::Error for RestoreValueError {}
