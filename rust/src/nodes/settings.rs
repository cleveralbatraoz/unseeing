//! The settings overlay as an engine node — the one place that touches
//! the display server, the pause flag and the mouse.
//!
//! It carries values across the boundary and adds no law of its own:
//! which rows exist, what a key means and what each row reads come from
//! [`crate::settings_menu`]; where the window goes comes from
//! [`crate::display_plan`]. This file opens and closes the overlay, draws
//! the strings the model hands it, and asks the server for the plan the
//! planner computed.
//!
//! THE OVERLAY OBEYS THE PERCEPTION LAWS. It is thin white line and text
//! on the black the world already is: a one-pixel rectangle around the
//! rows, no backdrop, no panel fill, no focus highlight. The cursor is a
//! pair of brackets around a value, because a filled selection bar is a
//! fill and this game draws none.
//!
//! STATE IS READ, NOT REMEMBERED. Every open re-seeds the model from the
//! window that actually exists. That is what keeps the toggle honest when
//! something outside the game changed the mode — the macOS green button,
//! a window manager's F11, a browser's Escape leaving full screen — and
//! it is why nothing here needs to observe the display server.
//!
//! PAUSE IS OWNED HERE, AND RELEASED ON THE WAY OUT. The overlay runs with
//! `ProcessMode::ALWAYS` so it still hears Escape while the world is
//! frozen; every other node stays pausable, which is exactly why a click
//! aimed at the overlay can never reach the hero's cane. Leaving the tree
//! un-pauses, so a menu that is freed mid-open — a test suite tearing down
//! its scene — cannot strand the tree frozen for whatever runs next.

use godot::classes::{
    CanvasLayer, CenterContainer, Control, DisplayServer, HBoxContainer, ICanvasLayer, IControl,
    Input, InputEvent, Label, ProjectSettings, VBoxContainer, control, display_server, input, node,
};
use godot::global::HorizontalAlignment;
use godot::prelude::*;

use crate::display_plan::{self, PlanMode, ScreenMetrics, Settings, WindowPlan};
use crate::settings_menu::{self, Menu, MenuKey, Outcome, ROWS, Row};

/// The overlay draws above everything the game renders.
const OVERLAY_LAYER: i32 = 128;

/// The rule box's line weight, in pixels, at every resolution: thin means
/// thin, and a hairline that thickens with the monitor is not a hairline.
const RULE: f32 = 1.0;

/// Breathing room between the rows and the rule box, in ems.
const PAD_EM: f32 = 1.4;

/// The rows' column width in ems — fixed, so the panel does not twitch
/// wider and narrower as the bracket cursor moves between rows.
const COLUMN_EM: f32 = 22.0;

/// How long the overlay keeps watching a window plan, in frames — about
/// four seconds. Long enough to outlast a macOS full-screen space
/// animation and the delegate that fires at the end of it, short enough
/// that a platform which simply refuses is left alone rather than fought
/// forever.
const ENFORCE_FRAMES: i32 = 240;

/// Frames between re-assertions while the window disagrees. A toggle every
/// frame is not more insistent, it is less: each one lands inside the
/// transition the last one started.
const RETRY_EVERY: i32 = 20;

/// The thin rule around the overlay's rows. A Control exists here for one
/// reason: `_draw` is the only way to put a one-pixel unfilled rectangle
/// on screen without a StyleBox, and a StyleBox is a fill.
#[derive(GodotClass)]
#[class(init, base=Control)]
pub struct SettingsFrame {
    /// The box to draw around — the rows' container, in this Control's
    /// own coordinates.
    #[var]
    content: Option<Gd<Control>>,
    /// How far the rule stands off the rows, in pixels.
    #[var]
    #[init(val = 24.0)]
    padding: f32,
    base: Base<Control>,
}

#[godot_api]
impl IControl for SettingsFrame {
    fn draw(&mut self) {
        let Some(content) = self.content.clone() else {
            return; // nothing to frame: draw nothing rather than a stray box
        };
        let rect = content.get_rect();
        if rect.size.x <= 0.0 || rect.size.y <= 0.0 {
            // the container has not been laid out yet; drawing now would
            // put a box at the origin around nothing. `item_rect_changed`
            // brings us back the moment it has a real rect.
            return;
        }
        let box_rect = rect.grow(self.padding);
        self.base_mut()
            .draw_rect_ex(box_rect, Color::WHITE)
            .filled(false)
            .width(RULE)
            .done();
    }
}

/// The settings overlay. Placed by the composition root, driven by
/// Escape, and otherwise silent.
#[derive(GodotClass)]
#[class(init, base=CanvasLayer)]
pub struct SettingsMenu {
    /// The model: cursor and chosen settings.
    #[init(val = Menu::new(Settings::boot(false)))]
    menu: Menu,
    /// The screen as last measured — refreshed on every open and every
    /// apply, never cached across them.
    #[init(val = ScreenMetrics::headless(Vector2i::new(1280, 720)))]
    metrics: ScreenMetrics,
    /// Whether the overlay is up (and therefore whether the world is
    /// frozen and the mouse is free).
    open: bool,
    /// Frames left to keep re-asserting the applied plan (see
    /// `enforce_plan`); zero when the window is where it should be.
    enforce: i32,
    /// Frames until the next re-assertion while the window disagrees.
    retry: i32,
    /// The mouse mode the game was using before the overlay took it, so
    /// closing restores what was there rather than assuming capture.
    #[init(val = input::MouseMode::VISIBLE)]
    mouse_before: input::MouseMode,
    /// The viewport height the type was last scaled for.
    last_height: i32,
    frame: Option<Gd<SettingsFrame>>,
    /// One (name, value) label pair per row of [`ROWS`], in order.
    rows: Vec<(Gd<Label>, Gd<Label>)>,
    /// Each row's line box, whose width is pinned so the rule does not
    /// twitch as the bracket cursor moves between rows.
    lines: Vec<Gd<HBoxContainer>>,
    /// Every label, for re-scaling type when the viewport changes.
    labels: Vec<Gd<Label>>,
    base: Base<CanvasLayer>,
}

#[godot_api]
impl ICanvasLayer for SettingsMenu {
    fn ready(&mut self) {
        self.base_mut().set_layer(OVERLAY_LAYER);
        // the world freezes, the overlay does not: without this the menu
        // would pause itself and never hear the Escape that closes it
        self.base_mut().set_process_mode(node::ProcessMode::ALWAYS);
        self.base_mut().set_process(true);
        self.base_mut().set_visible(false);
        self.build();
        self.metrics = Self::capture();
        self.menu = Menu::new(Settings::boot(Self::window_is_fullscreen()));
        self.relayout();
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        if event.is_action_pressed("ui_cancel") {
            if self.open {
                self.shut();
            } else {
                self.raise();
            }
            self.consume();
            return;
        }
        if !self.open {
            return; // closed: the overlay hears nothing but Escape
        }
        let Some(key) = Self::menu_key(&event) else {
            // a closed door: while the overlay is up, no key reaches the
            // world behind it, whether the overlay understood it or not
            self.consume();
            return;
        };
        match self.menu.press(key, &self.metrics) {
            Outcome::Close => self.shut(),
            Outcome::Changed => self.apply(),
            Outcome::Moved | Outcome::Unchanged => self.refresh(),
        }
        self.consume();
    }

    fn process(&mut self, _dt: f64) {
        self.enforce_plan();
        if self.open && self.viewport_height() != self.last_height {
            self.relayout();
        }
    }

    fn exit_tree(&mut self) {
        // a menu freed while open must not strand the tree frozen
        if self.open {
            self.open = false;
            self.set_paused(false);
        }
    }
}

#[godot_api]
impl SettingsMenu {
    /// Whether the overlay is up — the observable face of the pause.
    #[func]
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Frames of re-assertion left before the overlay stops insisting the
    /// window match the plan. Zero when settled. Observable so a rendered
    /// probe can tell "the frame loop never ran" from "the platform
    /// refused".
    #[func]
    pub fn enforce_left(&self) -> i32 {
        self.enforce
    }

    /// Whether the player's chosen settings ask for full screen. What the
    /// MODEL wants, which a headless suite can check and a window cannot.
    #[func]
    pub fn wants_fullscreen(&self) -> bool {
        self.menu.settings().fullscreen
    }

    /// The row the cursor is on, as its index in [`ROWS`].
    #[func]
    pub fn cursor_row(&self) -> i32 {
        ROWS.iter()
            .position(|row| *row == self.menu.selected())
            .unwrap_or(0) as i32
    }

    /// A row's name as drawn — the suites read the overlay's own text
    /// rather than a parallel description of it.
    #[func]
    pub fn row_label(&self, row: i32) -> GString {
        match Self::row_at(row) {
            Some(row) => GString::from(Menu::row_label(row)),
            None => GString::new(),
        }
    }

    /// A row's value as drawn, brackets and all.
    #[func]
    pub fn row_value(&self, row: i32) -> GString {
        match Self::row_at(row) {
            Some(row) => GString::from(self.menu.row_value(row, &self.metrics).as_str()),
            None => GString::new(),
        }
    }

    /// How many rows the overlay offers.
    #[func]
    pub fn row_count() -> i32 {
        ROWS.len() as i32
    }

    /// The row at an index, or nothing — an index from GDScript is never
    /// trusted to be in range.
    fn row_at(row: i32) -> Option<Row> {
        usize::try_from(row).ok().and_then(|i| ROWS.get(i).copied())
    }

    /// Open: freeze the world, free the mouse, and re-read the window so
    /// the rows describe what IS rather than what was last asked for.
    fn raise(&mut self) {
        self.metrics = Self::capture();
        self.menu = Menu::new(Settings::boot(Self::window_is_fullscreen()));
        self.open = true;
        self.mouse_before = Input::singleton().get_mouse_mode();
        Input::singleton().set_mouse_mode(input::MouseMode::VISIBLE);
        self.base_mut().set_visible(true);
        self.set_paused(true);
        self.relayout();
    }

    /// Close: thaw the world and give the mouse back to whatever had it.
    fn shut(&mut self) {
        self.open = false;
        self.base_mut().set_visible(false);
        self.set_paused(false);
        Input::singleton().set_mouse_mode(self.mouse_before);
    }

    /// Apply the chosen settings to the window, then arrange to re-fit
    /// next frame if this was a windowed plan.
    fn apply(&mut self) {
        self.metrics = Self::capture();
        let plan = display_plan::plan(&self.menu.settings(), &self.metrics);
        Self::drive(&plan);
        // Asking once is not enough — see `enforce_plan`.
        self.enforce = ENFORCE_FRAMES;
        self.retry = RETRY_EVERY;
        self.relayout();
    }

    /// Keep insisting, for a bounded while, that the window is what the
    /// player asked for.
    ///
    /// A mode change CAN be silently dropped, which is measured, not
    /// defensive: macOS ignores a full-screen toggle issued while another
    /// full-screen transition is still animating — and worse, its own
    /// `windowDidEnterFullScreen` delegate then writes the old mode back
    /// over the one Godot recorded, so the request vanishes without an
    /// error and the menu is left describing a window that does not exist.
    /// Toggling within a second of launch, while the game is still sliding
    /// into its full-screen space, does exactly that.
    ///
    /// The budget therefore runs to its END rather than stopping at the
    /// first sign of agreement. Measured, again: a toggle sent during the
    /// boot transition DOES land for a moment — the window really goes
    /// windowed — and then, forty frames later, the delegate for the
    /// transition that was still finishing flips it back. Anything that
    /// stopped watching once reality agreed would have stopped exactly in
    /// that window and left the game full screen with the row reading OFF.
    ///
    /// Re-asserting is rate-limited to one attempt per [`RETRY_EVERY`]
    /// frames, because a toggle per frame is not more insistent, it is
    /// less: each one lands in the transition the last one started, and
    /// the window never comes to rest.
    ///
    /// This subsumes the decoration settle: a window that just left full
    /// screen reports no title bar yet, so its first fit is provisional,
    /// and [`display_plan::fit`] is idempotent precisely so that a later
    /// pass either changes nothing or corrects the title bar it could not
    /// see. Geometry is written only when it actually differs, so a
    /// settled window is never touched again.
    fn enforce_plan(&mut self) {
        if self.enforce <= 0 {
            return;
        }
        self.enforce -= 1;
        self.metrics = Self::capture();
        let plan = display_plan::plan(&self.menu.settings(), &self.metrics);
        if Self::window_agrees(&plan) {
            Self::place(&plan);
            return;
        }
        if self.retry > 0 {
            self.retry -= 1;
            return;
        }
        self.retry = RETRY_EVERY;
        Self::drive(&plan);
    }

    /// Whether the window's full-screen-ness is what the plan asked for.
    ///
    /// Judged on full-screen-ness ALONE, never on the exact mode: macOS
    /// answers MAXIMIZED for a window asked to fill the usable area, and
    /// demanding the WINDOWED enum there would re-issue a zoom forever and
    /// leave the window flapping.
    fn window_agrees(plan: &WindowPlan) -> bool {
        let server = DisplayServer::singleton();
        if server.get_screen_count() <= 0 {
            return true; // no window to disagree with
        }
        Self::window_is_fullscreen() == (plan.mode == PlanMode::Fullscreen)
    }

    /// Write the plan's geometry, and only what actually differs — so a
    /// window already where it belongs is left alone and a player dragging
    /// it is not fought.
    fn place(plan: &WindowPlan) {
        let mut server = DisplayServer::singleton();
        if server.get_screen_count() <= 0 {
            return;
        }
        if let Some(size) = plan.size
            && server.window_get_size() != size
        {
            server.window_set_size(size);
        }
        // set and get disagree by design here: setting a position places
        // the DECORATED frame, while window_get_position reports the client
        // area below the title bar — so the comparison must be against the
        // decorated origin or it would never match and never settle.
        if let Some(position) = plan.position
            && server.window_get_position_with_decorations() != position
        {
            server.window_set_position(position);
        }
    }

    /// Hand a whole plan to the display server: the mode, then the
    /// geometry. Silent on a server with no screens — a headless run has
    /// no window to place.
    fn drive(plan: &WindowPlan) {
        let mut server = DisplayServer::singleton();
        if server.get_screen_count() <= 0 {
            return;
        }
        server.window_set_mode(match plan.mode {
            // borderless full screen, not exclusive: it changes no video
            // mode, survives a second monitor, and does not fight screen
            // recorders — the modern default
            PlanMode::Fullscreen => display_server::WindowMode::FULLSCREEN,
            PlanMode::Windowed => display_server::WindowMode::WINDOWED,
        });
        Self::place(plan);
    }

    /// What the window is right now. Maximized is NOT full screen — macOS
    /// lands there on its own when a window is asked to fill the usable
    /// area, and a menu that called that full screen would lie.
    fn window_is_fullscreen() -> bool {
        let server = DisplayServer::singleton();
        if server.get_screen_count() <= 0 {
            return false;
        }
        matches!(
            server.window_get_mode(),
            display_server::WindowMode::FULLSCREEN
                | display_server::WindowMode::EXCLUSIVE_FULLSCREEN
        )
    }

    /// Measure the screen the window lives on. Asks about the MAIN
    /// WINDOW's screen, never screen zero, so a second monitor is
    /// measured correctly; and asks nothing at all of a server that
    /// reports no screens.
    fn capture() -> ScreenMetrics {
        let server = DisplayServer::singleton();
        if server.get_screen_count() <= 0 {
            return ScreenMetrics::headless(Self::project_viewport());
        }
        let decorations = server.window_get_size_with_decorations() - server.window_get_size();
        ScreenMetrics::new(
            server.screen_get_size(),
            server.screen_get_usable_rect(),
            decorations,
        )
    }

    /// The project's configured viewport — the monitor a headless run
    /// does not have.
    fn project_viewport() -> Vector2i {
        let settings = ProjectSettings::singleton();
        let read = |key: &str, fallback: i32| {
            settings
                .get_setting(key)
                .try_to::<i32>()
                .unwrap_or(fallback)
        };
        Vector2i::new(
            read("display/window/size/viewport_width", 1280),
            read("display/window/size/viewport_height", 720),
        )
    }

    /// Translate a key press into the menu's vocabulary.
    fn menu_key(event: &Gd<InputEvent>) -> Option<MenuKey> {
        for (action, key) in [
            ("ui_up", MenuKey::Up),
            ("ui_down", MenuKey::Down),
            ("ui_left", MenuKey::Left),
            ("ui_right", MenuKey::Right),
            ("ui_accept", MenuKey::Accept),
        ] {
            if event.is_action_pressed(action) {
                return Some(key);
            }
        }
        None
    }

    /// Stop the event here, so nothing behind the overlay sees it.
    fn consume(&mut self) {
        if let Some(mut viewport) = self.base().get_viewport() {
            viewport.set_input_as_handled();
        }
    }

    /// Freeze or thaw the world.
    fn set_paused(&mut self, paused: bool) {
        if let Some(mut tree) = self.base().get_tree_or_null() {
            tree.set_pause(paused);
        }
    }

    /// The height the type is scaled against.
    fn viewport_height(&self) -> i32 {
        self.base()
            .get_viewport()
            .map_or(settings_menu::BASE_HEIGHT, |viewport| {
                viewport.get_visible_rect().size.y as i32
            })
    }

    /// Re-scale the type to the viewport, then rewrite every row.
    fn relayout(&mut self) {
        let height = self.viewport_height();
        self.last_height = height;
        let font = settings_menu::font_size(height);
        for label in self.labels.clone() {
            let mut label = label;
            label.add_theme_font_size_override("font_size", font);
        }
        let em = font as f32;
        for line in self.lines.clone() {
            let mut line = line;
            line.set_custom_minimum_size(Vector2::new(em * COLUMN_EM, 0.0));
        }
        if let Some(mut frame) = self.frame.clone() {
            frame.bind_mut().padding = em * PAD_EM;
            frame.queue_redraw();
        }
        self.refresh();
    }

    /// Rewrite every row from the model, and redraw the rule around them.
    fn refresh(&mut self) {
        for (index, row) in ROWS.iter().enumerate() {
            let Some((name, value)) = self.rows.get(index).cloned() else {
                continue;
            };
            let (mut name, mut value) = (name, value);
            name.set_text(Menu::row_label(*row));
            value.set_text(&self.menu.row_value(*row, &self.metrics));
        }
        if let Some(mut frame) = self.frame.clone() {
            frame.queue_redraw();
        }
    }

    /// Build the overlay's controls once: a rule, and the rows inside it.
    fn build(&mut self) {
        let mut frame = SettingsFrame::new_alloc();
        frame.set_anchors_preset(control::LayoutPreset::FULL_RECT);
        frame.set_mouse_filter(control::MouseFilter::IGNORE);
        let mut center = CenterContainer::new_alloc();
        center.set_anchors_preset(control::LayoutPreset::FULL_RECT);
        center.set_mouse_filter(control::MouseFilter::IGNORE);
        let mut column = VBoxContainer::new_alloc();
        column.set_mouse_filter(control::MouseFilter::IGNORE);

        let mut title = Label::new_alloc();
        title.set_text("SETTINGS");
        title.set_horizontal_alignment(HorizontalAlignment::CENTER);
        self.labels.push(title.clone());
        column.add_child(&title);

        for row in ROWS {
            let mut line = HBoxContainer::new_alloc();
            line.set_mouse_filter(control::MouseFilter::IGNORE);
            let mut name = Label::new_alloc();
            name.set_text(Menu::row_label(row));
            name.set_h_size_flags(control::SizeFlags::EXPAND_FILL);
            let mut value = Label::new_alloc();
            value.set_horizontal_alignment(HorizontalAlignment::RIGHT);
            line.add_child(&name);
            line.add_child(&value);
            column.add_child(&line);
            self.lines.push(line);
            self.labels.push(name.clone());
            self.labels.push(value.clone());
            self.rows.push((name, value));
        }

        let mut hint = Label::new_alloc();
        hint.set_text("ESC  RESUME");
        hint.set_horizontal_alignment(HorizontalAlignment::CENTER);
        self.labels.push(hint.clone());
        column.add_child(&hint);

        center.add_child(&column);
        frame.add_child(&center);
        // The rule is drawn around the rows' own box, so it fits whatever
        // the labels turn out to measure at this resolution. Containers lay
        // their children out AFTER the first draw, though, so the frame
        // must be told to draw again once the column has a real rect —
        // otherwise the rule is a box at the origin around nothing.
        let redraw = Callable::from_object_method(&frame, "queue_redraw");
        column.connect("item_rect_changed", &redraw);
        frame.bind_mut().content = Some(column.clone().upcast());
        self.base_mut().add_child(&frame);
        self.frame = Some(frame);
    }
}
