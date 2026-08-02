//! The wave/physics core of Unseeing, as a GDExtension.
//!
//! Layering law: everything below `ffi` is pure Rust with no engine types
//! beyond godot's glam-backed math builtins — testable under plain
//! `cargo test` with no Godot runtime. The `ffi` module is the only place
//! engine classes may appear.
//!
//! Web builds are single-threaded (the export pins thread_support=false):
//! no threads, no rayon, no parking primitives anywhere in this crate.

pub mod echo_queue;
pub mod pulse_pool;
pub mod ray_fan;

mod ffi {
    use godot::prelude::*;

    struct UnseeingCore;

    #[gdextension]
    unsafe impl ExtensionLibrary for UnseeingCore {}

    /// The wave core's engine-facing surface. Grows method by method as the
    /// GDScript pool migrates in; each method is a thin shim over the pure
    /// modules above.
    #[derive(GodotClass)]
    #[class(init, base=RefCounted)]
    pub struct WaveCore {
        base: Base<RefCounted>,
    }

    #[godot_api]
    impl WaveCore {
        /// Proof-of-life for the extension boundary: the number of rays in
        /// the golden-angle reflection fan, served from the pure core.
        #[func]
        fn ray_fan_size(&self) -> i64 {
            crate::ray_fan::RAYS as i64
        }
    }
}
