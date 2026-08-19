//! What the per-frame CPU paths actually cost, measured rather than argued.
//!
//! This is not a test and never runs in CI. Timing is not a postcondition —
//! it varies with the machine, the governor and the neighbours — so asserting
//! on it would produce a suite that fails for reasons nobody can act on. It
//! is a MEASUREMENT, run by hand:
//!
//!     cargo run --release --example hot_paths
//!
//! WHAT IS MEASURED, and why these and not others. `WaveGame::process` does
//! four kinds of work every frame: it drives the clockwork of each source,
//! asks the wall table how muffled each one is from the eye, advances the
//! pulse pool, and pushes uniforms across the FFI. Only the first three are
//! ours to make fast; the fourth belongs to Godot and is measured by the
//! frame-time probe instead. Of the three, the wall queries are the ones
//! that grow with the level — one sight line per source against every
//! occluder — and the same law runs per FRAGMENT in GLSL, where the cost is
//! multiplied by every pixel on screen. So they are measured against the
//! shipped table size rather than a toy one.
//!
//! `black_box` on both the input and the result: without it the optimiser is
//! entitled to hoist a pure call with constant arguments out of the loop and
//! report a number that measures nothing at all.

use std::hint::black_box;
use std::time::Instant;

use godot::builtin::{Vector3, Vector4};
use unseeing_core::level_plan;
use unseeing_core::render;
use unseeing_core::sight::{self, Occluder};

/// The shipped map's own wall table, built the way the level builds it: a
/// centerline per wall, inflated by `wall_rect`, swept floor to ceiling.
/// Measuring against four toy walls would flatter every number here.
fn shipped_walls() -> Vec<Occluder> {
    let room = |x0: f64, z0: f64, x1: f64, z1: f64| {
        Vector4::new(x0 as f32, z0 as f32, x1 as f32, z1 as f32)
    };
    let mut segments = vec![
        room(0.6, 0.6, 27.4, 0.6),
        room(27.4, 0.6, 27.4, 27.4),
        room(27.4, 27.4, 0.6, 27.4),
        room(0.6, 27.4, 0.6, 0.6),
    ];
    // interior runs, spaced the way the shipped level spaces them
    for i in 0..11 {
        let x = 3.0 + f64::from(i) * 2.2;
        segments.push(room(x, 2.0, x, 9.0));
    }
    for i in 0..11 {
        let z = 12.0 + f64::from(i) * 1.3;
        segments.push(room(4.0, z, 18.0, z));
    }
    segments.truncate(sight::MAXW);
    segments
        .into_iter()
        .filter_map(|s| Occluder::new(s, 0.0, level_plan::WALL_H))
        .collect()
}

fn bench(label: &str, iters: u32, mut body: impl FnMut()) {
    // one untimed warm pass: the first call pays for cold branch predictors
    // and a cold cache, and reporting that as the steady-state cost is how a
    // micro-benchmark lies in the pessimistic direction.
    body();
    let start = Instant::now();
    for _ in 0..iters {
        body();
    }
    let elapsed = start.elapsed();
    let per = elapsed.as_secs_f64() / f64::from(iters) * 1.0e9;
    println!("{label:<44} {per:>9.1} ns/op   ({iters} iters)");
}

fn main() {
    let walls = shipped_walls();
    println!("# hot paths, {} occluders in the table", walls.len());

    let eye = Vector3::new(2.0, 1.6, 2.0);
    let far_corner = Vector3::new(26.0, 1.2, 26.0);
    let near_point = Vector3::new(4.0, 1.2, 3.0);

    bench("sight::crossings, across the map", 200_000, || {
        black_box(sight::crossings(
            black_box(eye),
            black_box(far_corner),
            black_box(&walls),
        ));
    });
    bench("sight::crossings, one room over", 200_000, || {
        black_box(sight::crossings(
            black_box(eye),
            black_box(near_point),
            black_box(&walls),
        ));
    });
    bench("sight::blocked_from, across the map", 200_000, || {
        black_box(sight::blocked_from(
            black_box(eye),
            black_box(far_corner),
            black_box(&walls),
        ));
    });
    bench("sight::visible_air, across the map", 200_000, || {
        black_box(sight::visible_air(
            black_box(eye),
            black_box(far_corner),
            black_box(&walls),
        ));
    });
    bench("level_plan::source_muffle", 2_000_000, || {
        black_box(level_plan::source_muffle(black_box(2), black_box(3)));
    });
    bench("render::reveal::source_image", 2_000_000, || {
        black_box(render::reveal::source_image(
            black_box(0.4),
            black_box(render::reveal::SourceImage {
                volume: 0.7,
                muffle: 0.3,
            }),
        ));
    });
}
