//! The custom Material app bar (FR14 / [ADR 017](../../doc/adr/017-material-design-title-bar.md)).
//!
//! On native targets the window runs **undecorated** — this bar *is* the
//! window border: it carries the diagram title, the window controls
//! (minimize / fullscreen / close), the drag region that replaces native
//! move, and the invisible edge/corner grips that replace native resize.
//! On wasm the bar still renders (identical layout, NFR1) but the controls
//! are hidden — the browser keeps its own chrome.
//!
//! Window commands go through egui viewport commands (`Minimized`,
//! `Fullscreen`, `Close`, `StartDrag`, `BeginResize`), which eframe forwards
//! to winit on native and ignores headlessly — so every control is testable
//! without a window (tier 0b asserts the queued
//! `FullOutput.viewport_output[...].commands`).
//!
//! **One command per user action.** Both move and resize are drags, and egui
//! reports a drag as *active* for every frame of the gesture. The commands
//! must therefore be sent on `drag_started()` (true for exactly one frame —
//! the press), never on `dragged()`: re-sending `StartDrag` mid-drag makes
//! egui-winit re-invoke winit's `drag_window()`; on X11 those pointer grabs
//! stack and the window keeps following the cursor after release ("never
//! gets undragged").

use eframe::egui;
use egui::{Align, Layout, Pos2, Rect, RichText, Sense, Ui};

/// The app bar height in points: M3's 64dp top app bar is mobile-sized — a
/// dense desktop tool wants one compact row.
pub const APP_BAR_HEIGHT: f32 = 40.0;

/// The window title when no file is loaded yet.
pub const DEFAULT_TITLE: &str = "ascii-diagram-drawer";

/// The width of the invisible window-edge resize grips in points (FR14 /
/// ADR 017): wide enough to grab with a pointer, narrow enough not to eat
/// canvas space at the edges.
pub const RESIZE_GRIP: f32 = 6.0;

/// Draws the app bar as a top panel. Must be shown *before* any other panel
/// (panels claim their area in show order).
pub fn show(ui: &mut Ui, title: &str) {
    // `Panel::top` is non-resizable by default (egui 0.37) — exactly what a fixed
    // app bar wants. The separator line is off: it would reserve a point of the
    // strip for itself (shrinking the inner rect to 39 pt) and double up with the
    // hairline this bar paints on its own bottom edge.
    egui::Panel::top("appbar")
        .exact_size(APP_BAR_HEIGHT)
        .show_separator_line(false)
        .show(ui, |ui| {
            let visuals = ui.visuals();
            let rect = ui.max_rect();
            // The surface fill + a bottom hairline give the bar its own edge
            // against the menu bar below (elevation without a shadow on the
            // frame's topmost surface).
            ui.painter().rect_filled(rect, 0.0, visuals.panel_fill);
            let line = Pos2::new(rect.min.x, rect.max.y - 1.0);
            ui.painter().line_segment(
                [line, line + egui::vec2(rect.width(), 0.0)],
                egui::epaint::Stroke::new(1.0, visuals.widgets.inactive.bg_stroke.color),
            );

            // Drag region FIRST: egui gives hit-test priority to the LAST added
            // widget, so the title and controls registered after this still win
            // their own rects — only the bar's empty area drags the window.
            let drag = ui.interact(rect, egui::Id::unique("appbar_drag"), Sense::drag());
            if drag.drag_started() {
                // Exactly one `StartDrag` per user drag (see module docs): the
                // press frame only. Sending it on every `dragged()` frame leaves
                // stacked pointer grabs behind on X11 and the window never stops
                // following the cursor after release.
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }

            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(RichText::new(title).strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // NFR1: window controls exist only where a window exists —
                    // the browser target keeps its own chrome (no fake buttons).
                    #[cfg(not(target_arch = "wasm32"))]
                    window_controls(ui);
                });
            });
        });
}

/// The minimize / fullscreen / close controls, right-aligned in the bar.
///
/// Glyphs are monospace by design: the bundled proportional font lacks
/// box-drawing glyphs (verified by `gui_tests::window_control_glyphs_are_in_the_font`),
/// while Hack covers all three.
#[cfg(not(target_arch = "wasm32"))]
fn window_controls(ui: &mut Ui) {
    // No live `ctx` may be held across the `ui.add_sized` calls below (borrow
    // rules): fetch each command through a fresh `ui.ctx()` at the point of use.

    // Minimize: the small horizontal line (U+2500).
    if ui
        .add_sized(
            [APP_BAR_HEIGHT - 8.0; 2],
            egui::Button::new(RichText::new("─").monospace()),
        )
        .on_hover_text("Minimize")
        .clicked()
    {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }

    // Fullscreen toggle: reads the current state from the viewport info
    // (`None` headless / unknown → enters fullscreen).
    let is_fullscreen = ui.ctx().input(|i| i.viewport().fullscreen).unwrap_or(false);
    if ui
        .add_sized(
            [APP_BAR_HEIGHT - 8.0; 2],
            egui::Button::new(RichText::new("□").monospace()),
        )
        .on_hover_text(if is_fullscreen {
            "Exit fullscreen"
        } else {
            "Fullscreen"
        })
        .clicked()
    {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
    }

    // Close: the app exits after this frame (eframe's winit handling).
    if ui
        .add_sized(
            [APP_BAR_HEIGHT - 8.0; 2],
            egui::Button::new(RichText::new("×").monospace()),
        )
        .on_hover_text("Close")
        .clicked()
    {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

/// The invisible window-edge and corner resize grips (FR14 / ADR 017).
///
/// An undecorated window has no OS border to grab, so the app owns resizing:
/// a [`RESIZE_GRIP`]-pt-wide strip along each screen edge plus one square at
/// each corner. Pressing and dragging a grip queues exactly one
/// [`egui::ViewportCommand::BeginResize`] (the same one-shot rule as the app
/// bar's `StartDrag`, see module docs), which egui-winit turns into a native
/// `drag_resize_window`. Hovering shows the matching resize cursor.
///
/// The grips live in their own foreground [`egui::Area`] rather than the app
/// bar's `Ui`: a panel `Ui` clips every interact to its 40-pt strip, which
/// would kill the bottom and side grips. Foreground order also makes them
/// beat the app bar's drag region where they overlap (the top 6-pt strip —
/// resize from the very edge, move from below it), and edges are registered
/// before corners so a corner wins its own zone on hit-test ties.
///
/// Native only: in the browser the page owns the window chrome (NFR1).
#[cfg(not(target_arch = "wasm32"))]
pub fn show_resize_grips(ctx: &egui::Context) {
    // Single-window app: the viewport rect *is* the window (screen) rect.
    let screen = ctx.viewport_rect();
    if screen.width() <= RESIZE_GRIP * 2.0 || screen.height() <= RESIZE_GRIP * 2.0 {
        return; // too small for distinct edge zones
    }
    let (l, t) = (screen.min.x, screen.min.y);
    let (r, b) = (screen.max.x, screen.max.y);
    let g = RESIZE_GRIP;

    egui::Area::new(egui::Id::unique("window_resize_grips"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            // Edges first, corners last: on a hit-test tie the later widget wins.
            let grips: [(
                &'static str,
                Rect,
                egui::viewport::ResizeDirection,
                egui::CursorIcon,
            ); 8] = [
                (
                    "w",
                    Rect::from_min_max(Pos2::new(l, t), Pos2::new(l + g, b)),
                    egui::viewport::ResizeDirection::West,
                    egui::CursorIcon::ResizeWest,
                ),
                (
                    "e",
                    Rect::from_min_max(Pos2::new(r - g, t), Pos2::new(r, b)),
                    egui::viewport::ResizeDirection::East,
                    egui::CursorIcon::ResizeEast,
                ),
                (
                    "n",
                    Rect::from_min_max(Pos2::new(l, t), Pos2::new(r, t + g)),
                    egui::viewport::ResizeDirection::North,
                    egui::CursorIcon::ResizeNorth,
                ),
                (
                    "s",
                    Rect::from_min_max(Pos2::new(l, b - g), Pos2::new(r, b)),
                    egui::viewport::ResizeDirection::South,
                    egui::CursorIcon::ResizeSouth,
                ),
                (
                    "nw",
                    Rect::from_min_max(Pos2::new(l, t), Pos2::new(l + g, t + g)),
                    egui::viewport::ResizeDirection::NorthWest,
                    egui::CursorIcon::ResizeNorthWest,
                ),
                (
                    "ne",
                    Rect::from_min_max(Pos2::new(r - g, t), Pos2::new(r, t + g)),
                    egui::viewport::ResizeDirection::NorthEast,
                    egui::CursorIcon::ResizeNorthEast,
                ),
                (
                    "sw",
                    Rect::from_min_max(Pos2::new(l, b - g), Pos2::new(l + g, b)),
                    egui::viewport::ResizeDirection::SouthWest,
                    egui::CursorIcon::ResizeSouthWest,
                ),
                (
                    "se",
                    Rect::from_min_max(Pos2::new(r - g, b - g), Pos2::new(r, b)),
                    egui::viewport::ResizeDirection::SouthEast,
                    egui::CursorIcon::ResizeSouthEast,
                ),
            ];
            for (name, rect, dir, icon) in grips {
                let resp = ui.interact(rect, egui::Id::unique(["win_grip", name]), Sense::drag());
                if resp.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(dir));
                }
                resp.on_hover_and_drag_cursor(icon);
            }
        });
}
