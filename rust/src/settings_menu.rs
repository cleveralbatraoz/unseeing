//! The settings overlay's model: which rows exist, which one the player is
//! on, what a key does to it, and the exact text each row shows.
//!
//! Everything the overlay KNOWS lives here, in plain Rust the cargo suite
//! drives without a Godot runtime; the engine layer only draws these
//! strings and forwards key presses. That split is what lets the menu's
//! behaviour be pinned without a window, a screen, or a rendered frame —
//! and CI has none of the three.
//!
//! Two laws shape the key handling:
//!   - a press that changes nothing reports [`Outcome::Unchanged`], never
//!     a change. The engine layer applies a window plan only on a real
//!     change, which keeps the web build from firing a redundant browser
//!     full-screen request it would have to be granted a gesture for;
//!   - the row is a LIST, not a pair of hard-coded controls, so the day a
//!     second resolution is offered the menu needs no new branch.
//!
//! Presentation lives here too — the bracket markers around the selected
//! row's value are text, and text is testable. Nothing in the engine layer
//! decides what a row reads.

use crate::display_plan::{ScreenMetrics, Settings, resolutions};

/// The viewport height the overlay's type was drawn for.
pub const BASE_HEIGHT: i32 = 720;

/// The overlay's type size at [`BASE_HEIGHT`].
pub const BASE_FONT: i32 = 18;

/// The overlay's type size for a viewport of this height.
///
/// The game sets no content scale — the viewport IS the window, so a
/// full-screen 4K monitor renders four times the pixels a windowed 720p
/// one does, and type pinned to a pixel size would shrink to a thread.
/// The overlay therefore scales with the viewport, clamped at both ends so
/// a degenerate height (headless reports a 100-pixel root) still produces
/// something legible rather than nothing.
pub fn font_size(viewport_height: i32) -> i32 {
    let scaled = i64::from(BASE_FONT) * i64::from(viewport_height.max(0)) / i64::from(BASE_HEIGHT);
    scaled.clamp(12, 96) as i32
}

/// Whether losing the mouse should raise the overlay by itself.
///
/// A browser reserves Escape as its OWN gesture: while the pointer is
/// locked, the first Escape exits pointer lock in the browser process and
/// is never delivered to the page, so the key the overlay listens for
/// simply does not arrive. The player presses Escape, the cursor comes
/// back, and nothing opens. What the browser DOES give us is the unlock
/// itself — so on the web, losing a capture the game was holding is read
/// as the request Escape would have been.
///
/// Web only, deliberately. On a desktop the window manager can drop a
/// capture for its own reasons (a focus change, a task switch), and a
/// settings panel appearing because the player alt-tabbed would be a
/// surprise, not a service.
pub fn capture_loss_opens(had_capture: bool, captured_now: bool, on_web: bool) -> bool {
    on_web && had_capture && !captured_now
}

/// The overlay's rows, in display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    /// Whether the game covers the screen.
    Fullscreen,
    /// Which resolution the window asks for.
    Resolution,
}

/// Every row, top to bottom — the menu's whole surface.
pub const ROWS: [Row; 2] = [Row::Fullscreen, Row::Resolution];

/// A key press the overlay understands, named by intent rather than by
/// keycode: the engine layer owns the keyboard, this module owns meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuKey {
    /// Previous row.
    Up,
    /// Next row.
    Down,
    /// Previous value on this row.
    Left,
    /// Next value on this row.
    Right,
    /// Next value on this row — the same as [`MenuKey::Right`], for
    /// players who reach for Enter or Space.
    Accept,
    /// Close the overlay and resume the game.
    Cancel,
}

/// What a press did — the engine layer's instruction for what to do next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The selection moved; nothing about the display changed.
    Moved,
    /// A setting changed; the engine layer must apply the new plan.
    Changed,
    /// The press did nothing — a row with one value, pressed sideways.
    Unchanged,
    /// Close the overlay.
    Close,
}

/// The overlay's whole state: where the cursor is and what is chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Menu {
    selected: usize,
    settings: Settings,
}

impl Menu {
    /// Open on the first row with the settings the window already has.
    pub fn new(settings: Settings) -> Self {
        Self {
            selected: 0,
            settings,
        }
    }

    /// The row the cursor is on.
    pub fn selected(&self) -> Row {
        ROWS[self.selected.min(ROWS.len() - 1)]
    }

    /// The chosen settings — what the engine layer turns into a plan.
    pub fn settings(&self) -> Settings {
        self.settings
    }

    /// Drive the menu with a key. Total: every key has an answer, and no
    /// press can move the cursor off the row list or the resolution index
    /// past the row's end.
    pub fn press(&mut self, key: MenuKey, metrics: &ScreenMetrics) -> Outcome {
        match key {
            MenuKey::Cancel => Outcome::Close,
            MenuKey::Up => {
                self.selected = (self.selected + ROWS.len() - 1) % ROWS.len();
                Outcome::Moved
            }
            MenuKey::Down => {
                self.selected = (self.selected + 1) % ROWS.len();
                Outcome::Moved
            }
            MenuKey::Left => self.adjust(-1, metrics),
            MenuKey::Right | MenuKey::Accept => self.adjust(1, metrics),
        }
    }

    /// Step the selected row's value, reporting whether anything moved.
    fn adjust(&mut self, delta: isize, metrics: &ScreenMetrics) -> Outcome {
        let before = self.settings;
        match self.selected() {
            Row::Fullscreen => self.settings.fullscreen = !self.settings.fullscreen,
            Row::Resolution => {
                let count = resolutions(metrics).len();
                // A row with nothing to cycle through cannot move; the
                // modulo below would divide by zero on an empty row.
                if count > 0 {
                    let current = self.settings.resolution.min(count - 1) as isize;
                    let count = count as isize;
                    self.settings.resolution = (current + delta).rem_euclid(count).unsigned_abs();
                }
            }
        }
        if self.settings == before {
            Outcome::Unchanged
        } else {
            Outcome::Changed
        }
    }

    /// Adopt the window's actual full-screen-ness WITHOUT disturbing the
    /// cursor. The mode can change behind the overlay's back while it is
    /// open — a browser's own Escape leaves full screen, a window manager
    /// has its own shortcut — and a row that kept claiming otherwise would
    /// need two presses to do one thing. Reports whether anything moved.
    pub fn adopt_fullscreen(&mut self, fullscreen: bool) -> bool {
        if self.settings.fullscreen == fullscreen {
            return false;
        }
        self.settings.fullscreen = fullscreen;
        true
    }

    /// The row's name, as shown.
    pub fn row_label(row: Row) -> &'static str {
        match row {
            Row::Fullscreen => "FULLSCREEN",
            Row::Resolution => "RESOLUTION",
        }
    }

    /// The row's value, as shown — bracketed on the row the cursor is on,
    /// bare on the others. The brackets ARE the cursor: a filled highlight
    /// would be a fill, and this game draws none.
    pub fn row_value(&self, row: Row, metrics: &ScreenMetrics) -> String {
        let value = self.raw_value(row, metrics);
        if row == self.selected() {
            format!("< {value} >")
        } else {
            format!("  {value}  ")
        }
    }

    /// The row's value without cursor markers.
    fn raw_value(&self, row: Row, metrics: &ScreenMetrics) -> String {
        match row {
            Row::Fullscreen => {
                if self.settings.fullscreen {
                    "ON".to_string()
                } else {
                    "OFF".to_string()
                }
            }
            Row::Resolution => {
                let options = resolutions(metrics);
                let index = self
                    .settings
                    .resolution
                    .min(options.len().saturating_sub(1));
                match options.get(index) {
                    Some(option) => {
                        let size = format!("{} x {}", option.size.x, option.size.y);
                        if option.native {
                            format!("NATIVE  {size}")
                        } else {
                            size
                        }
                    }
                    // An empty row can only happen on a server with no
                    // screens at all; say so rather than showing nothing.
                    None => "UNAVAILABLE".to_string(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use godot::builtin::{Rect2i, Vector2i};

    fn mac() -> ScreenMetrics {
        ScreenMetrics::new(
            Vector2i::new(2816, 1762),
            Rect2i::from_components(98, 60, 2718, 1702),
            Vector2i::new(0, 64),
        )
    }

    fn menu(fullscreen: bool) -> Menu {
        Menu::new(Settings {
            fullscreen,
            resolution: 0,
        })
    }

    /// The overlay opens on the first row, showing the window that exists.
    #[test]
    fn the_menu_opens_on_the_first_row() {
        let menu = menu(true);
        assert_eq!(menu.selected(), Row::Fullscreen);
        assert!(menu.settings().fullscreen);
    }

    /// Up and down wrap, so the cursor can never leave the list.
    #[test]
    fn the_cursor_wraps_around_the_rows() {
        let mut menu = menu(true);
        assert_eq!(menu.press(MenuKey::Down, &mac()), Outcome::Moved);
        assert_eq!(menu.selected(), Row::Resolution);
        menu.press(MenuKey::Down, &mac());
        assert_eq!(menu.selected(), Row::Fullscreen);
        menu.press(MenuKey::Up, &mac());
        assert_eq!(menu.selected(), Row::Resolution);
    }

    /// Either sideways key toggles full screen, and so does Accept — the
    /// row has two values, so every direction leads to the other one.
    #[test]
    fn every_change_key_toggles_full_screen() {
        for key in [MenuKey::Left, MenuKey::Right, MenuKey::Accept] {
            let mut menu = menu(true);
            assert_eq!(menu.press(key, &mac()), Outcome::Changed);
            assert!(!menu.settings().fullscreen);
            assert_eq!(menu.press(key, &mac()), Outcome::Changed);
            assert!(menu.settings().fullscreen);
        }
    }

    /// Moving the cursor is not a settings change: the engine layer must
    /// not touch the window because the player looked at another row.
    #[test]
    fn moving_the_cursor_changes_no_setting() {
        let mut menu = menu(true);
        let before = menu.settings();
        assert_eq!(menu.press(MenuKey::Down, &mac()), Outcome::Moved);
        assert_eq!(menu.settings(), before);
    }

    /// A row with a single value reports UNCHANGED, not a change. This is
    /// what keeps the web build from firing a browser full-screen request
    /// that nothing asked for.
    #[test]
    fn a_single_valued_resolution_row_reports_no_change() {
        let mut menu = menu(true);
        menu.press(MenuKey::Down, &mac());
        assert_eq!(menu.selected(), Row::Resolution);
        assert_eq!(menu.press(MenuKey::Right, &mac()), Outcome::Unchanged);
        assert_eq!(menu.press(MenuKey::Left, &mac()), Outcome::Unchanged);
        assert_eq!(menu.settings().resolution, 0);
    }

    /// Cancel closes from any row, changing nothing on the way out.
    #[test]
    fn cancel_closes_from_any_row() {
        for steps in 0..ROWS.len() {
            let mut menu = menu(true);
            for _ in 0..steps {
                menu.press(MenuKey::Down, &mac());
            }
            let before = menu.settings();
            assert_eq!(menu.press(MenuKey::Cancel, &mac()), Outcome::Close);
            assert_eq!(menu.settings(), before);
        }
    }

    /// The cursor is the bracket pair, and only the selected row wears it.
    #[test]
    fn only_the_selected_row_is_bracketed() {
        let menu = menu(true);
        assert_eq!(menu.row_value(Row::Fullscreen, &mac()), "< ON >");
        assert_eq!(
            menu.row_value(Row::Resolution, &mac()),
            "  NATIVE  2816 x 1762  "
        );
    }

    /// The rows read as the design draws them: names on the left, the
    /// monitor's own resolution named as native.
    #[test]
    fn the_rows_read_as_designed() {
        assert_eq!(Menu::row_label(Row::Fullscreen), "FULLSCREEN");
        assert_eq!(Menu::row_label(Row::Resolution), "RESOLUTION");
        let mut menu = menu(false);
        assert_eq!(menu.row_value(Row::Fullscreen, &mac()), "< OFF >");
        menu.press(MenuKey::Down, &mac());
        assert_eq!(
            menu.row_value(Row::Resolution, &mac()),
            "< NATIVE  2816 x 1762 >"
        );
    }

    /// A resolution index left over from a bigger row (or from nowhere)
    /// still renders and still cycles, instead of panicking on a slice.
    #[test]
    fn an_out_of_range_resolution_index_still_renders_and_cycles() {
        let mut menu = Menu::new(Settings {
            fullscreen: false,
            resolution: 7,
        });
        assert_eq!(
            menu.row_value(Row::Resolution, &mac()),
            "  NATIVE  2816 x 1762  "
        );
        menu.press(MenuKey::Down, &mac());
        // one option: any step lands back on it, and reports no change
        assert_eq!(menu.press(MenuKey::Right, &mac()), Outcome::Changed);
        assert_eq!(menu.settings().resolution, 0);
        assert_eq!(menu.press(MenuKey::Right, &mac()), Outcome::Unchanged);
    }

    /// Type scales with the viewport, because the viewport IS the window:
    /// the same overlay must read on a 720p window and a full-screen 4K
    /// monitor.
    #[test]
    fn type_scales_with_the_viewport() {
        assert_eq!(font_size(BASE_HEIGHT), BASE_FONT);
        assert_eq!(font_size(1440), BASE_FONT * 2);
        assert_eq!(font_size(1762), 44);
    }

    /// Degenerate heights still produce legible type instead of a thread
    /// or a wall — headless roots report 100 pixels, and monitors get big.
    #[test]
    fn type_is_clamped_at_both_ends() {
        assert_eq!(font_size(0), 12);
        assert_eq!(font_size(-5000), 12);
        assert_eq!(font_size(100), 12);
        assert_eq!(font_size(i32::MAX), 96);
    }

    /// On the web, a lost capture IS the Escape the browser ate: while the
    /// pointer is locked the first Escape never reaches the page, so the
    /// unlock is the only signal the overlay will ever get.
    #[test]
    fn losing_a_web_capture_stands_in_for_the_escape_the_browser_ate() {
        assert!(capture_loss_opens(true, false, true));
    }

    /// And nothing else does: not a capture that is still held, not one the
    /// game never had, and never on a desktop — where a window manager
    /// dropping a capture on a task switch would pop the settings open for
    /// no reason the player could name.
    #[test]
    fn nothing_else_raises_the_overlay_by_itself() {
        assert!(!capture_loss_opens(true, true, true), "still captured");
        assert!(!capture_loss_opens(false, false, true), "never had it");
        assert!(!capture_loss_opens(true, false, false), "desktop");
        assert!(!capture_loss_opens(false, true, false));
    }

    /// The mode can change behind the overlay's back; adopting it moves the
    /// row without moving the cursor, and says whether it moved at all.
    #[test]
    fn adopting_reality_leaves_the_cursor_where_it_was() {
        let mut menu = menu(true);
        menu.press(MenuKey::Down, &mac());
        assert_eq!(menu.selected(), Row::Resolution);
        assert!(menu.adopt_fullscreen(false), "should report the change");
        assert!(!menu.settings().fullscreen);
        assert_eq!(menu.selected(), Row::Resolution, "cursor must not jump");
        assert!(!menu.adopt_fullscreen(false), "already agreed");
    }

    /// Headless has no monitor, and the row still says something true.
    #[test]
    fn the_resolution_row_renders_headless() {
        let metrics = ScreenMetrics::headless(Vector2i::new(1280, 720));
        let menu = menu(false);
        assert_eq!(
            menu.row_value(Row::Resolution, &metrics),
            "  NATIVE  1280 x 720  "
        );
    }
}
