//! Where the window goes: the monitor's own resolution by default, and the
//! centered box a windowed game falls back to.
//!
//! MEASURED LAW, and the reason this module exists (Godot 4.7.1, macOS):
//! leaving full screen does NOT restore the window that was there before.
//! The mode flips back to windowed while the size stays the monitor's and
//! the position stays the screen corner — forever, unless someone says
//! otherwise. Every windowed plan here therefore carries an explicit size
//! AND position; nothing is left to the platform to remember.
//!
//! DECORATIONS: a title bar is not free. The frame a window occupies is
//! its content plus an overhead the platform picks (0x64 on macOS, other
//! numbers on Windows and Linux — never assumed here, always measured),
//! and that overhead is UNREADABLE while full screen, where the server
//! reports a frame equal to its content. So [`fit`] is idempotent by
//! construction: feeding it its own answer changes nothing. That is what
//! lets the engine layer apply a windowed plan immediately and re-settle
//! one frame later, once the decorations are back, with no risk of a
//! second, different answer.
//!
//! TOTAL: a headless run reports ZERO screens and a zero-sized window, so
//! there is no screen 0 to ask about. [`ScreenMetrics::headless`] stands
//! in the project's own viewport size instead, and every function below
//! answers for it exactly as it would for a real monitor.

use godot::builtin::{Rect2i, Vector2i};

/// The smallest window this game will ask a platform for. A monitor too
/// small to hold it is a monitor we overflow deliberately rather than
/// collapse onto: a 1-pixel window is never the kinder answer.
pub const MIN_CONTENT: Vector2i = Vector2i::new(320, 180);

/// What the display server says about the screen the window lives on.
/// Captured once per apply by the engine layer — the pure plan never asks
/// the server anything itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenMetrics {
    /// The screen's full size: what "native resolution" means, and what a
    /// full-screen window becomes.
    pub size: Vector2i,
    /// The screen minus the furniture — menu bars, docks, taskbars. A
    /// windowed game is centered in HERE, not in `size`.
    pub usable: Rect2i,
    /// Frame minus content: what the title bar and borders cost. Zero
    /// while full screen, where the server reports no decorations.
    pub decorations: Vector2i,
}

impl ScreenMetrics {
    /// Sanitize what the server reported. A screen that claims no area —
    /// a headless server, a monitor mid-unplug — has no usable rect worth
    /// centering in, so the full size stands in for it; negative
    /// decorations are nonsense and read as none.
    pub fn new(size: Vector2i, usable: Rect2i, decorations: Vector2i) -> Self {
        let size = size.coord_max(MIN_CONTENT);
        let usable = if usable.size.x > 0 && usable.size.y > 0 {
            usable
        } else {
            Rect2i::new(Vector2i::ZERO, size)
        };
        Self {
            size,
            usable,
            decorations: decorations.coord_max(Vector2i::ZERO),
        }
    }

    /// The screen a headless run does not have. CI reports zero screens,
    /// so the project's configured viewport stands in as the whole world:
    /// native resolution, usable rect, and no decorations.
    pub fn headless(viewport: Vector2i) -> Self {
        Self::new(
            viewport,
            Rect2i::new(Vector2i::ZERO, viewport),
            Vector2i::ZERO,
        )
    }
}

/// One entry of the resolution row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resolution {
    /// The content size this entry asks for.
    pub size: Vector2i,
    /// True for the monitor's own resolution — the one the game boots at.
    pub native: bool,
}

/// The resolution row's entries, in display order. Today the monitor's own
/// resolution is the only one offered; the row is a list so that adding
/// entries is data, not surgery.
pub fn resolutions(metrics: &ScreenMetrics) -> Vec<Resolution> {
    vec![Resolution {
        size: metrics.size,
        native: true,
    }]
}

/// Which window mode a plan asks for. Deliberately NOT the engine's enum:
/// this module stays free of engine classes, and the engine layer does the
/// one-line translation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanMode {
    /// A window with decorations, sized and placed by us.
    Windowed,
    /// The whole screen. The platform picks the size — it is the monitor's.
    Fullscreen,
}

/// A complete instruction for the display server: the mode, and — when
/// windowed — exactly where the window goes. Full screen carries no
/// geometry because the platform owns it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowPlan {
    /// The mode to switch to.
    pub mode: PlanMode,
    /// Content size to set, or `None` when the platform owns it.
    pub size: Option<Vector2i>,
    /// Frame position to set, or `None` when the platform owns it.
    pub position: Option<Vector2i>,
}

/// What the player chose. Born from reality at boot, never from a file —
/// this game deliberately forgets its settings between sessions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Whether the game covers the screen.
    pub fullscreen: bool,
    /// Index into [`resolutions`]; clamped on read, never trusted.
    pub resolution: usize,
}

impl Settings {
    /// Seed the model from what the window ALREADY is. The default full
    /// screen is the project setting's doing, applied by the engine before
    /// a line of this runs; asking the window what happened is how the
    /// menu tells the truth on a platform that refused (the web, where a
    /// browser grants full screen only inside a user gesture) or on a
    /// launch that overrode it (`--windowed` on the command line).
    pub fn boot(fullscreen_now: bool) -> Self {
        Self {
            fullscreen: fullscreen_now,
            resolution: 0,
        }
    }

    /// The chosen entry, whatever the index says. An index past the end of
    /// the row answers with the native resolution rather than nothing.
    pub fn resolution_size(&self, metrics: &ScreenMetrics) -> Vector2i {
        let options = resolutions(metrics);
        options
            .get(self.resolution)
            .map_or(metrics.size, |option| option.size)
    }
}

/// The largest content box no bigger than `request` whose FRAME still fits
/// the usable area, and the frame position that centers it there.
///
/// Idempotent: `fit(fit(r).0) == fit(r)` for every input, which is what
/// lets the engine layer re-run it a frame later once decorations exist.
pub fn fit(request: Vector2i, metrics: &ScreenMetrics) -> (Vector2i, Vector2i) {
    let room = (metrics.usable.size - metrics.decorations).coord_max(MIN_CONTENT);
    let size = request.coord_max(MIN_CONTENT).coord_min(room);
    let frame = size + metrics.decorations;
    // Integer centering floors the half-pixel; a frame too big for the
    // usable area would center to a negative offset, so the origin is the
    // floor — a window is never pushed under a menu bar to center it.
    let slack = (metrics.usable.size - frame).coord_max(Vector2i::ZERO);
    (size, metrics.usable.position + slack / 2)
}

/// The whole instruction for a set of settings on a given screen.
pub fn plan(settings: &Settings, metrics: &ScreenMetrics) -> WindowPlan {
    if settings.fullscreen {
        return WindowPlan {
            mode: PlanMode::Fullscreen,
            size: None,
            position: None,
        };
    }
    let (size, position) = fit(settings.resolution_size(metrics), metrics);
    WindowPlan {
        mode: PlanMode::Windowed,
        size: Some(size),
        position: Some(position),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The measured macOS screen this module was designed against: a
    /// Retina panel reporting logical points, a menu bar and a Dock eating
    /// the edges, and a 64-pixel title bar.
    fn mac() -> ScreenMetrics {
        ScreenMetrics::new(
            Vector2i::new(2816, 1762),
            Rect2i::from_components(98, 60, 2718, 1702),
            Vector2i::new(0, 64),
        )
    }

    /// Native resolution IS the screen size — the row's one entry, and
    /// what the game boots at.
    #[test]
    fn the_only_resolution_offered_is_the_monitors_own() {
        let options = resolutions(&mac());
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].size, Vector2i::new(2816, 1762));
        assert!(options[0].native);
    }

    /// Full screen hands the platform the screen: a mode and nothing else.
    #[test]
    fn a_full_screen_plan_carries_no_geometry() {
        let plan = plan(
            &Settings {
                fullscreen: true,
                resolution: 0,
            },
            &mac(),
        );
        assert_eq!(plan.mode, PlanMode::Fullscreen);
        assert_eq!(plan.size, None);
        assert_eq!(plan.position, None);
    }

    /// THE measured bug this module exists for: leaving full screen never
    /// restores a window, so a windowed plan must always say where the
    /// window goes.
    #[test]
    fn a_windowed_plan_always_carries_size_and_position() {
        let plan = plan(
            &Settings {
                fullscreen: false,
                resolution: 0,
            },
            &mac(),
        );
        assert_eq!(plan.mode, PlanMode::Windowed);
        assert!(plan.size.is_some());
        assert!(plan.position.is_some());
    }

    /// Native windowed on the measured Mac: exactly the box Godot's own
    /// maximize picks — the usable area minus the title bar — seated at
    /// the usable origin.
    #[test]
    fn native_windowed_fills_the_usable_area_minus_the_title_bar() {
        let (size, position) = fit(Vector2i::new(2816, 1762), &mac());
        assert_eq!(size, Vector2i::new(2718, 1638));
        assert_eq!(position, Vector2i::new(98, 60));
    }

    /// A window smaller than the screen is centered in the USABLE rect,
    /// frame included — the title bar counts toward the centering, or the
    /// window sits low.
    #[test]
    fn a_smaller_window_is_centered_by_its_frame() {
        let (size, position) = fit(Vector2i::new(1280, 720), &mac());
        assert_eq!(size, Vector2i::new(1280, 720));
        // frame 1280x784 in a 2718x1702 usable rect at (98,60)
        assert_eq!(position, Vector2i::new(98 + 719, 60 + 459));
    }

    /// Feeding the fit its own answer changes nothing — the property that
    /// lets the engine re-settle a frame later, when the decorations the
    /// server hid during full screen have come back.
    #[test]
    fn the_fit_is_idempotent() {
        for request in [
            Vector2i::new(2816, 1762),
            Vector2i::new(1280, 720),
            Vector2i::new(99999, 99999),
            Vector2i::new(1, 1),
        ] {
            let once = fit(request, &mac());
            let twice = fit(once.0, &mac());
            assert_eq!(once, twice, "fit not idempotent for {request:?}");
        }
    }

    /// The same request answered while FULL SCREEN — where the server
    /// reports no decorations — and then again once the title bar is back.
    /// The second answer is the one that fits; the first never overflows
    /// the usable area either.
    #[test]
    fn a_fit_measured_without_decorations_still_fits_the_usable_area() {
        let undecorated = ScreenMetrics::new(
            Vector2i::new(2816, 1762),
            Rect2i::from_components(98, 60, 2718, 1702),
            Vector2i::ZERO,
        );
        let (size, position) = fit(Vector2i::new(2816, 1762), &undecorated);
        assert_eq!(size, Vector2i::new(2718, 1702));
        assert_eq!(position, Vector2i::new(98, 60));
        // and once the decorations are measurable, the settle shrinks it
        let (settled, settled_pos) = fit(size, &mac());
        assert_eq!(settled, Vector2i::new(2718, 1638));
        assert_eq!(settled_pos, Vector2i::new(98, 60));
    }

    /// A screen too small for the floor size overflows rather than
    /// collapsing, and the window still starts inside the usable rect
    /// instead of being centered to a negative corner.
    #[test]
    fn a_tiny_screen_overflows_instead_of_collapsing() {
        let tiny = ScreenMetrics::new(
            Vector2i::new(320, 180),
            Rect2i::from_components(0, 20, 320, 100),
            Vector2i::new(0, 30),
        );
        let (size, position) = fit(Vector2i::new(320, 180), &tiny);
        assert_eq!(size, MIN_CONTENT);
        assert_eq!(position, Vector2i::new(0, 20));
    }

    /// Headless: zero screens, so the project's viewport is the world. The
    /// plan answers for it without ever naming a screen index.
    #[test]
    fn headless_stands_the_projects_viewport_in_for_a_monitor() {
        let metrics = ScreenMetrics::headless(Vector2i::new(1280, 720));
        assert_eq!(metrics.size, Vector2i::new(1280, 720));
        assert_eq!(resolutions(&metrics)[0].size, Vector2i::new(1280, 720));
        let (size, position) = fit(Vector2i::new(1280, 720), &metrics);
        assert_eq!(size, Vector2i::new(1280, 720));
        assert_eq!(position, Vector2i::ZERO);
    }

    /// A server that reports an empty usable rect (headless, or a monitor
    /// mid-unplug) is not allowed to produce a zero-sized window.
    #[test]
    fn an_empty_usable_rect_falls_back_to_the_whole_screen() {
        let broken = ScreenMetrics::new(
            Vector2i::new(1920, 1080),
            Rect2i::from_components(0, 0, 0, 0),
            Vector2i::ZERO,
        );
        assert_eq!(broken.usable, Rect2i::from_components(0, 0, 1920, 1080));
        let (size, _) = fit(Vector2i::new(1920, 1080), &broken);
        assert_eq!(size, Vector2i::new(1920, 1080));
    }

    /// Boot reads the window, not a wish: whatever mode the engine
    /// actually gave us is what the menu shows.
    #[test]
    fn boot_settings_mirror_the_window_that_exists() {
        assert!(Settings::boot(true).fullscreen);
        assert!(!Settings::boot(false).fullscreen);
        assert_eq!(Settings::boot(true).resolution, 0);
    }

    /// A resolution index past the end of the row still answers — with the
    /// monitor's own size, never a panic and never nothing.
    #[test]
    fn an_out_of_range_resolution_index_answers_native() {
        let settings = Settings {
            fullscreen: false,
            resolution: 99,
        };
        assert_eq!(settings.resolution_size(&mac()), Vector2i::new(2816, 1762));
    }

    /// Negative decorations are nonsense a server should never send, and
    /// are read as none rather than growing the window.
    #[test]
    fn negative_decorations_read_as_none() {
        let metrics = ScreenMetrics::new(
            Vector2i::new(1920, 1080),
            Rect2i::from_components(0, 0, 1920, 1080),
            Vector2i::new(-10, -40),
        );
        assert_eq!(metrics.decorations, Vector2i::ZERO);
    }
}
