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
//! `Fullscreen`, `Close`, `StartDrag`, `InnerSize`, `OuterPosition`), which
//! eframe forwards to winit on native and ignores headlessly — so every
//! control is testable without a window (tier 0b asserts the queued
//! `FullOutput.viewport_output[...].commands`).
//!
//! **One command per user action.** Move is a drag, and egui reports a drag as
//! *active* for every frame of the gesture: the `StartDrag` command must be
//! sent on `drag_started()` (true for exactly one frame — the press), never on
//! `dragged()`: re-sending it mid-drag makes egui-winit re-invoke winit's
//! `drag_window()`; on X11 those pointer grabs stack and the window keeps
//! following the cursor after release ("never gets undragged").
//!
//! **Resize is app-owned** (B-004 / ADR 017 v3): while a grip is held, the app
//! computes the new size itself each frame and queues `InnerSize` (plus
//! `OuterPosition` to re-anchor north/west drags). The old one-shot
//! `BeginResize` handed the gesture to the compositor (`xdg_toplevel.resize`
//! on Wayland, driven by a button serial winit records on press *and*
//! release) and silently did nothing on KDE Plasma/KWin — see B-004.

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

/// The smallest window size a client-side resize drag may produce (B-004 /
/// ADR 017 v3): small enough for a single-column diagram, large enough that
/// the app bar and canvas stay usable.
pub const MIN_WINDOW_SIZE: egui::Vec2 = egui::vec2(320.0, 240.0);

/// Generous upper bound for resize steps — keeps the gesture arithmetic in
/// comfortable f32 territory no matter how far the pointer wanders.
const MAX_WINDOW_SIZE: egui::Vec2 = egui::vec2(10_000.0, 10_000.0);

/// One in-progress client-side window-resize gesture (B-004 / ADR 017 v3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResizeGrab {
    /// Which edge or corner is being dragged.
    pub direction: egui::viewport::ResizeDirection,
    /// The window's inner size when the gesture started (points).
    pub start_size: egui::Vec2,
    /// The window's outer top-left when the gesture started — `None` where the
    /// backend does not report a position (headless); north/west drags then
    /// resize without re-anchoring.
    pub start_outer: Option<Pos2>,
    /// The pointer position when the gesture started (points).
    pub start_pointer: Pos2,
}

/// Pure geometry of one client-side resize step (B-004 / ADR 017 v3): given
/// the gesture's start state and the pointer's current position, returns the
/// new inner size — clamped to [`MIN_WINDOW_SIZE`]…[`MAX_WINDOW_SIZE`] — and,
/// for north/west edges, the new outer top-left that keeps the opposite
/// edge/corner fixed (`None` when the start position was unknown).
pub fn resize_step(grab: ResizeGrab, pointer: Pos2) -> (egui::Vec2, Option<Pos2>) {
    let d = grab.direction;
    let dx = pointer.x - grab.start_pointer.x;
    let dy = pointer.y - grab.start_pointer.y;
    let mut w = grab.start_size.x;
    let mut h = grab.start_size.y;
    if matches!(
        d,
        egui::viewport::ResizeDirection::East
            | egui::viewport::ResizeDirection::NorthEast
            | egui::viewport::ResizeDirection::SouthEast
    ) {
        w += dx;
    }
    if matches!(
        d,
        egui::viewport::ResizeDirection::West
            | egui::viewport::ResizeDirection::NorthWest
            | egui::viewport::ResizeDirection::SouthWest
    ) {
        w -= dx;
    }
    if matches!(
        d,
        egui::viewport::ResizeDirection::South
            | egui::viewport::ResizeDirection::SouthEast
            | egui::viewport::ResizeDirection::SouthWest
    ) {
        h += dy;
    }
    if matches!(
        d,
        egui::viewport::ResizeDirection::North
            | egui::viewport::ResizeDirection::NorthEast
            | egui::viewport::ResizeDirection::NorthWest
    ) {
        h -= dy;
    }
    let size = egui::Vec2::new(
        w.clamp(MIN_WINDOW_SIZE.x, MAX_WINDOW_SIZE.x),
        h.clamp(MIN_WINDOW_SIZE.y, MAX_WINDOW_SIZE.y),
    );
    let west = matches!(
        d,
        egui::viewport::ResizeDirection::West
            | egui::viewport::ResizeDirection::NorthWest
            | egui::viewport::ResizeDirection::SouthWest
    );
    let north = matches!(
        d,
        egui::viewport::ResizeDirection::North
            | egui::viewport::ResizeDirection::NorthEast
            | egui::viewport::ResizeDirection::NorthWest
    );
    let outer = grab.start_outer.map(|start| {
        Pos2::new(
            if west {
                start.x + (grab.start_size.x - size.x)
            } else {
                start.x
            },
            if north {
                start.y + (grab.start_size.y - size.y)
            } else {
                start.y
            },
        )
    });
    (size, outer)
}

/// The cursor icon for a resize direction.
fn cursor_for(dir: egui::viewport::ResizeDirection) -> egui::CursorIcon {
    use egui::viewport::ResizeDirection::*;
    match dir {
        North => egui::CursorIcon::ResizeNorth,
        South => egui::CursorIcon::ResizeSouth,
        East => egui::CursorIcon::ResizeEast,
        West => egui::CursorIcon::ResizeWest,
        NorthEast => egui::CursorIcon::ResizeNorthEast,
        SouthEast => egui::CursorIcon::ResizeSouthEast,
        NorthWest => egui::CursorIcon::ResizeNorthWest,
        SouthWest => egui::CursorIcon::ResizeSouthWest,
    }
}

/// The eight grip zones for a window rect: edges first, corners last (on a
/// hit-test tie the later widget wins, so a corner owns its own square).
fn grip_zones(screen: &Rect) -> [(&'static str, Rect, egui::viewport::ResizeDirection); 8] {
    let (l, t) = (screen.min.x, screen.min.y);
    let (r, b) = (screen.max.x, screen.max.y);
    let g = RESIZE_GRIP;
    [
        (
            "w",
            Rect::from_min_max(Pos2::new(l, t), Pos2::new(l + g, b)),
            egui::viewport::ResizeDirection::West,
        ),
        (
            "e",
            Rect::from_min_max(Pos2::new(r - g, t), Pos2::new(r, b)),
            egui::viewport::ResizeDirection::East,
        ),
        (
            "n",
            Rect::from_min_max(Pos2::new(l, t), Pos2::new(r, t + g)),
            egui::viewport::ResizeDirection::North,
        ),
        (
            "s",
            Rect::from_min_max(Pos2::new(l, b - g), Pos2::new(r, b)),
            egui::viewport::ResizeDirection::South,
        ),
        (
            "nw",
            Rect::from_min_max(Pos2::new(l, t), Pos2::new(l + g, t + g)),
            egui::viewport::ResizeDirection::NorthWest,
        ),
        (
            "ne",
            Rect::from_min_max(Pos2::new(r - g, t), Pos2::new(r, t + g)),
            egui::viewport::ResizeDirection::NorthEast,
        ),
        (
            "sw",
            Rect::from_min_max(Pos2::new(l, b - g), Pos2::new(l + g, b)),
            egui::viewport::ResizeDirection::SouthWest,
        ),
        (
            "se",
            Rect::from_min_max(Pos2::new(r - g, b - g), Pos2::new(r, b)),
            egui::viewport::ResizeDirection::SouthEast,
        ),
    ]
}

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
/// Visual order is left-to-right **minimize ─, fullscreen □, close ×** — close
/// at the window's top-right corner (FR14 / ADR 017 v2). The buttons are added
/// in the *reverse* order below because they live in a `Layout::right_to_left`
/// scope: egui places the first-added widget rightmost, so adding close first
/// puts it at the right end and minimize (added last) at the left.
///
/// Glyphs are monospace by design: the bundled proportional font lacks
/// box-drawing glyphs (verified by `gui_tests::window_control_glyphs_are_in_the_font`),
/// while Hack covers all three.
#[cfg(not(target_arch = "wasm32"))]
fn window_controls(ui: &mut Ui) {
    // No live `ctx` may be held across the `ui.add_sized` calls below (borrow
    // rules): fetch each command through a fresh `ui.ctx()` at the point of use.

    // Close: the app exits after this frame (eframe's winit handling).
    // Added first → rightmost under `right_to_left` (top-right corner, FR14).
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

    // Minimize: the small horizontal line (U+2500).
    // Added last → leftmost of the trio.
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
}

/// The invisible window-edge and corner resize grips (FR14 / ADR 017 v3).
///
/// An undecorated window has no OS border to grab, so the app owns resizing:
/// a [`RESIZE_GRIP`]-pt-wide strip along each screen edge plus one square at
/// each corner. Hovering shows the matching resize cursor; pressing and
/// dragging runs an **app-owned gesture** (B-004): while the button is held,
/// every frame computes the new size from the pointer delta ([`resize_step`])
/// and queues `InnerSize` — plus `OuterPosition` for north/west edges so the
/// opposite edge/corner stays fixed. The compositor-driven alternative
/// (`BeginResize` → `xdg_toplevel.resize`) did not work on KDE Plasma/KWin,
/// see B-004.
///
/// The grips live in their own foreground [`egui::Area`] rather than the app
/// bar's `Ui`: a panel `Ui` clips every interact to its 40-pt strip, which
/// would kill the bottom and side grips. Foreground order also makes them
/// beat the app bar's drag region where they overlap (the top 6-pt strip —
/// resize from the very edge, move from below it), and edges are registered
/// before corners so a corner wins its own zone on hit-test ties.
///
/// `grab` carries the in-flight gesture between frames (`None` = idle).
///
/// Native only: in the browser the page owns the window chrome (NFR1).
#[cfg(not(target_arch = "wasm32"))]
pub fn show_resize_grips(ctx: &egui::Context, grab: &mut Option<ResizeGrab>) {
    // Single-window app: the viewport rect *is* the window (screen) rect.
    let screen = ctx.viewport_rect();
    if screen.width() <= RESIZE_GRIP * 2.0 || screen.height() <= RESIZE_GRIP * 2.0 {
        *grab = None; // no distinct edge zones → no gesture can be in flight
        return;
    }
    let pointer = ctx.input(|i| i.pointer.interact_pos());
    // `button_down` — the HELD state. (`button_pressed` is edge-triggered in
    // egui 0.37: "pressed this frame" — it would end the gesture on the first
    // move frame.)
    let held = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));

    egui::Area::new(egui::Id::unique("window_resize_grips"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            // Edges first, corners last: on a hit-test tie the later widget wins.
            for (name, rect, dir) in grip_zones(&screen) {
                let resp = ui.interact(rect, egui::Id::unique(["win_grip", name]), Sense::drag());
                if resp.drag_started() {
                    // Start the gesture exactly once — on the press frame.
                    if let Some(p0) = pointer {
                        *grab = Some(ResizeGrab {
                            direction: dir,
                            start_size: screen.size(),
                            start_outer: ctx.input(|i| i.viewport().outer_rect).map(|r| r.min),
                            start_pointer: p0,
                        });
                    }
                } else {
                    resp.on_hover_and_drag_cursor(cursor_for(dir));
                }
            }
        });

    // Active gesture: apply one resize step this frame while the button is
    // held. The grab is `take`n and only put back if the gesture continues —
    // a released button ends it with no further commands.
    if let (Some(g), Some(p)) = (grab.take(), pointer)
        && held
    {
        let (size, outer) = resize_step(g, p);
        if (size - screen.size()).length() > 0.5 {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        }
        if let Some(pos) = outer {
            let current = ctx.input(|i| i.viewport().outer_rect).map(|r| r.min);
            if current.is_none_or(|c| (pos - c).length() > 0.5) {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
            }
        }
        *grab = Some(g);
        // Keep the direction cursor even when the pointer wanders off the
        // grip zone mid-drag.
        ctx.set_cursor_icon(cursor_for(g.direction));
    }
}
