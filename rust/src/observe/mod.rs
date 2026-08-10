//! Debug observability — the wave engine described to an agent as data.
//!
//! Four verbs, per `docs/superpowers/specs/2026-08-10-debug-observability-design.md`:
//! SNAPSHOT (state now), DIFF (the caller's job — sample and compare),
//! EXPLAIN (pure re-computations that answer "why"), and DIGEST (the pixel
//! reduction, Plan 2).
//!
//! Everything here is pure and engine-free. The boundary that hands these
//! results to Godot is `crate::nodes::observer`.

pub mod evict;
pub mod pool;
pub mod ray;
