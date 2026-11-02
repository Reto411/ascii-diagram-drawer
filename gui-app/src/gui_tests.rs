//! Headless GUI-level integration tests.
//!
//! These drive the *real* eframe UI code — [`DrawingApp::show`] — through
//! `egui::Context::run_ui`, the same entry point native eframe calls each frame,
//! but without a window or a GPU. Raw pointer/key/text events are injected per
//! frame; assertions then check (a) the resulting diagram model and (b) the drawn
//! shapes in [`egui::FullOutput`] — text-row positions, grid lines, selection
//! outline, anchor dots, the resize-handle marker, connect drag lines, dialog
//! behaviour.
//!
//! This campaign targets the reported bug classes:
//! * canvas content invisible / misplaced (screen-space drawing, square cells);
//! * save/open prompts losing typed input;
//! * Delete/Backspace deleting canvas objects while typing in a field;
//! * the resize-handle marker drawn where it does not work;
//! * anchor dots / connect line drawn at wrong offsets.

use crate::app::{DrawingApp, PathPrompt};
use crate::config::Theme;
use crate::dialogs::{DialogKind, DialogResult, FileDialogBackend};
use crate::template_editor::TemplateEditorSession;
use core_services::Services;
use diagram_model::objects::{ConnectorInstance, Object, TipStyle};
// The app depends on eframe only; egui is reached through its re-export.
use eframe::egui;
use egui::{Color32, Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2};
use std::path::PathBuf;

const WIN_W: f32 = 1024.0;
const WIN_H: f32 = 768.0;
/// Must match `MARGIN_PX` in `app.rs`.
const MARGIN: f32 = 8.0;
/// Must match `SIDEBAR_PX` in `app.rs`.
const SIDEBAR: f32 = 180.0;
/// Must match `PREVIEW_DEFAULT_PX` in `app.rs` (S-026): the SVG preview pane is
/// visible by default, so the canvas no longer spans to the window's right edge.
const PREVIEW: f32 = 380.0;
/// Must match `ANCHOR_COLOR` / `SELECTION_COLOR` in `app.rs`.
const ANCHOR: Color32 = Color32::from_rgb(255, 196, 87);
const SELECTION: Color32 = Color32::from_rgb(90, 160, 255);
/// Must match `RESIZE_COLOR` in `app.rs` (FR21 / CR-018).
const RESIZE: Color32 = Color32::from_rgb(76, 175, 80);

// ─────────────────────────── test harness ───────────────────────────

/// S-039 — fake dialog backend for the tier-0b tests: every `show` returns the
/// next queued result (default `Unavailable` = today's headless behavior, so all
/// pre-S-039 tests keep their inline-prompt flows) and records the request, so
/// tests can assert which kind was asked for and with which prefilled file.
#[derive(Debug, Default)]
struct FakeDialogs {
    /// Results returned by successive `show` calls; once empty it stays Unavailable.
    results: std::sync::Mutex<std::collections::VecDeque<DialogResult>>,
    /// Every request that arrived: `(kind, prefilled file)` in call order.
    calls: std::sync::Mutex<Vec<(DialogKind, Option<PathBuf>)>>,
}

impl FileDialogBackend for FakeDialogs {
    fn show(&self, kind: DialogKind, default_file: Option<&std::path::Path>) -> DialogResult {
        self.calls
            .lock()
            .unwrap()
            .push((kind, default_file.map(PathBuf::from)));
        self.results
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(DialogResult::Unavailable("fake backend".to_owned()))
    }
}

/// Shares a [`FakeDialogs`] with the app under test.
struct SharedFake(std::sync::Arc<FakeDialogs>);

impl FileDialogBackend for SharedFake {
    fn show(&self, kind: DialogKind, default_file: Option<&std::path::Path>) -> DialogResult {
        self.0.show(kind, default_file)
    }
}

/// One headless egui context driving a [`DrawingApp`], frame by frame.
struct Gui {
    ctx: egui::Context,
    app: DrawingApp,
    ts: f64,
    /// S-039 — the fake dialog backend installed on `app` (deterministic dialogs).
    dialogs: std::sync::Arc<FakeDialogs>,
}

impl Gui {
    fn new(app: DrawingApp) -> Self {
        // Install the fake dialog backend: tier-0b must never open a real system
        // dialog (it would block on a desktop and is meaningless headless).
        let dialogs = std::sync::Arc::new(FakeDialogs::default());
        let app = app.with_dialog_backend(Box::new(SharedFake(std::sync::Arc::clone(&dialogs))));
        let ctx = egui::Context::default();
        // Keep the built-in fonts: eframe (without `system_fonts`) lays out with exactly
        // these defaults — Hack monospace + Ubuntu proportional — so every metric the
        // app computes here matches the native app's.
        ctx.set_fonts(egui::FontDefinitions::default());
        // FR14 (CR-006, ADR 017): install the Material palettes BEFORE the first UI
        // pass — mirroring main.rs. egui snapshots the active style into each Ui at
        // creation, so a mid-pass install would make frame 1 render with default
        // spacing (20px button rows) and settle to the Material metrics (32px rows)
        // over the next two frames; every geometry probe in this harness assumes the
        // layout is stable from frame one.
        crate::theme::install(&ctx, app.config().theme);
        Self {
            ctx,
            app,
            ts: 1.0,
            dialogs,
        }
    }

    /// S-039 — queue the result the next dialog request will return.
    fn queue_dialog(&self, result: DialogResult) {
        self.dialogs.results.lock().unwrap().push_back(result);
    }

    /// S-039 — every dialog request so far: `(kind, prefilled file)` in call order.
    fn dialog_calls(&self) -> Vec<(DialogKind, Option<PathBuf>)> {
        self.dialogs.calls.lock().unwrap().clone()
    }

    /// Pump one frame with the given events and return what was drawn.
    fn pump(&mut self, events: Vec<Event>) -> egui::FullOutput {
        let raw = egui::RawInput {
            time: Some(self.ts),
            screen_rect: Some(Rect::from_min_max(Pos2::ZERO, Pos2::new(WIN_W, WIN_H))),
            events,
            ..Default::default()
        };
        self.ts += 1.0 / 60.0;
        let app = &mut self.app;
        let mut out = self.ctx.run_ui(raw, |ui| app.show(ui));
        // Headless: there is no GPU to upload glyph textures to — clear the pending
        // deltas or egui panics when they are dropped unapplied.
        out.textures_delta.clear();
        out
    }

    /// Pump one idle frame.
    fn frame(&mut self) -> egui::FullOutput {
        self.pump(Vec::new())
    }

    /// Pump one frame reporting a `(w, h)` viewport — for the resize grips'
    /// degenerate-window guard (FR14 / ADR 017).
    fn pump_screen(&mut self, w: f32, h: f32, events: Vec<Event>) -> egui::FullOutput {
        let raw = egui::RawInput {
            time: Some(self.ts),
            screen_rect: Some(Rect::from_min_max(Pos2::ZERO, Pos2::new(w, h))),
            events,
            ..Default::default()
        };
        self.ts += 1.0 / 60.0;
        let app = &mut self.app;
        let mut out = self.ctx.run_ui(raw, |ui| app.show(ui));
        out.textures_delta.clear();
        out
    }

    /// Pump one frame reporting a `(w, h)` viewport whose outer top-left sits at
    /// `origin` — simulates a real window whose backend reports its position on
    /// screen (B-004 / ADR 017 v3 re-anchoring). The plain harness leaves the
    /// `ViewportInfo` rects unknown (`None`), which would skip the anchor step.
    fn pump_viewport_at(
        &mut self,
        origin: Pos2,
        w: f32,
        h: f32,
        events: Vec<Event>,
    ) -> egui::FullOutput {
        let mut raw = egui::RawInput {
            time: Some(self.ts),
            screen_rect: Some(Rect::from_min_max(Pos2::ZERO, Pos2::new(w, h))),
            events,
            ..Default::default()
        };
        let rect = Rect::from_min_max(origin, origin + egui::vec2(w, h));
        // `ViewportIdMap` is a plain `HashMap` (read-only index): mutate via
        // `get_mut`. The ROOT entry always exists — `RawInput::default()` seeds it.
        let root = raw
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .expect("ROOT viewport");
        root.inner_rect = Some(rect);
        root.outer_rect = Some(rect);
        self.ts += 1.0 / 60.0;
        let app = &mut self.app;
        let mut out = self.ctx.run_ui(raw, |ui| app.show(ui));
        out.textures_delta.clear();
        out
    }

    // ── event constructors ──

    fn press_at(pos: Pos2) -> Vec<Event> {
        vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::default(),
            },
        ]
    }

    fn release_at(pos: Pos2) -> Vec<Event> {
        vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::default(),
        }]
    }

    /// A full click (press frame + release frame). Returns the last frame's output.
    /// A full click: press, release, plus one settle frame. The extra frame
    /// matters: egui processes the release during the release frame, but that
    /// frame's rendering still contains widgets laid out from the pre-click
    /// state (e.g. a prompt row drawn before its OK button ran). One idle
    /// frame later the output reflects the click's effect.
    fn click_at(&mut self, pos: Pos2) -> egui::FullOutput {
        let _ = self.pump(Self::press_at(pos));
        let _ = self.pump(Self::release_at(pos));
        self.frame()
    }

    /// A full click with Ctrl held (S-035 FR20): the modifier event is pumped on its
    /// own frame before the press and dropped afterwards, so the app's modifier probe
    /// sees Ctrl exactly on the press frame.
    fn ctrl_click_at(&mut self, pos: Pos2) -> egui::FullOutput {
        let _ = self.pump(vec![Event::ModifiersChanged(Modifiers::CTRL)]);
        let out = self.click_at(pos);
        let _ = self.pump(vec![Event::ModifiersChanged(Modifiers::default())]);
        out
    }

    /// Two quick clicks at `pos` (double-click).
    fn double_click_at(&mut self, pos: Pos2) -> egui::FullOutput {
        let _ = self.click_at(pos);
        self.click_at(pos)
    }

    /// Press at `from`, move through the midpoint to `to`, release there.
    fn drag(&mut self, from: Pos2, to: Pos2) -> egui::FullOutput {
        let _ = self.pump(Self::press_at(from));
        let mid = Pos2::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0);
        let _ = self.pump(vec![Event::PointerMoved(mid)]);
        let _ = self.pump(vec![Event::PointerMoved(to)]);
        self.pump(Self::release_at(to))
    }

    /// Press at `from`, move to `to`, and *hold* (no release). Returns the last output.
    fn press_and_hold_to(&mut self, from: Pos2, to: Pos2) -> egui::FullOutput {
        let _ = self.pump(Self::press_at(from));
        self.pump(vec![Event::PointerMoved(to)])
    }

    /// Send a key press (and release on the next frame).
    fn key(&mut self, key: Key) -> egui::FullOutput {
        self.key_with_mods(key, Modifiers::default())
    }

    /// Send a key press with modifier keys held. egui 0.37 ignores the `modifiers`
    /// field on `Event::Key` — modifier state only changes via
    /// `Event::ModifiersChanged`, so we emit that around the press/release.
    fn key_with_mods(&mut self, key: Key, mods: Modifiers) -> egui::FullOutput {
        let _ = self.pump(vec![Event::ModifiersChanged(mods)]);
        let _ = self.pump(vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: mods,
        }]);
        let out = self.pump(vec![Event::Key {
            key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: mods,
        }]);
        // Drop the modifiers again so later frames see a clean keyboard state.
        let _ = self.pump(vec![Event::ModifiersChanged(Modifiers::default())]);
        // The key-release frame — commands (e.g. clipboard) drain into it.
        out
    }

    /// Ctrl+S (press + release).
    fn ctrl_s(&mut self) -> egui::FullOutput {
        self.key_with_mods(Key::S, Modifiers::CTRL)
    }

    /// Type a string into whatever text field has focus (one event per frame).
    fn type_text(&mut self, s: &str) -> egui::FullOutput {
        let mut out = self.frame();
        for c in s.chars() {
            if c == '\n' {
                // `Event::Text("\n")` is ignored by TextEdit — newlines are key
                // events (multiline fields insert a line break on Enter).
                let enter = |pressed: bool| Event::Key {
                    key: Key::Enter,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: Modifiers::default(),
                };
                let _ = self.pump(vec![enter(true)]);
                out = self.pump(vec![enter(false)]);
            } else {
                out = self.pump(vec![Event::Text(c.to_string())]);
            }
        }
        out
    }

    // ── geometry helpers ──

    /// Top-left corner of the canvas (central) area, measured from the grid lines
    /// drawn at the viewport margin. Only valid while the diagram is empty.
    fn canvas_min(&mut self) -> Pos2 {
        let out = self.frame();
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        for (a, b) in line_points(&out) {
            if (a.x - b.x).abs() < 0.5 && a.x > SIDEBAR {
                min_x = min_x.min(a.x); // vertical grid line
            }
            if (a.y - b.y).abs() < 0.5 && a.x > SIDEBAR && b.x > SIDEBAR {
                min_y = min_y.min(a.y); // horizontal grid line
            }
        }
        assert!(
            min_x.is_finite() && min_y.is_finite(),
            "empty canvas draws grid lines"
        );
        Pos2::new(min_x - MARGIN, min_y - MARGIN)
    }

    /// Measured cell size in pixels — the same computation the app uses.
    fn cell_metrics(&self) -> (f32, f32) {
        let painter = egui::Painter::new(
            self.ctx.clone(),
            egui::LayerId::new(egui::Order::Middle, egui::Id::unique("gui_tests")),
            Rect::EVERYTHING,
        );
        let galley = painter.layout(
            "MMMMMMMMMM".to_owned(),
            egui::FontId::monospace(14.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        let w = (galley.size().x / 10.0).max(4.0);
        (w, galley.size().y.max(w))
    }

    /// Absolute screen position of the center of model cell `(col, row)`.
    fn cell_center(&self, min: Pos2, col: u32, row: u32) -> Pos2 {
        let (cw, ch) = self.cell_metrics();
        Pos2::new(
            min.x + MARGIN + (col as f32 + 0.5) * cw,
            min.y + MARGIN + (row as f32 + 0.5) * ch,
        )
    }

    /// Absolute screen position of the top-left corner of model cell `(col, row)`.
    fn cell_corner(&self, min: Pos2, col: u32, row: u32) -> Pos2 {
        let (cw, ch) = self.cell_metrics();
        Pos2::new(
            min.x + MARGIN + col as f32 * cw,
            min.y + MARGIN + row as f32 * ch,
        )
    }

    // ── model helpers ──

    fn object_count(&self) -> usize {
        self.app.diagram.objects().len()
    }

    /// `(x, y, width, height)` of the first placed shape instance.
    fn first_box(&self) -> (u32, u32, u32, u32) {
        for obj in self.app.diagram.objects() {
            if let Object::Shape(s) = obj {
                return (s.pos.x, s.pos.y, s.size.width, s.size.height);
            }
        }
        panic!("no shape instance in diagram");
    }

    /// Place one "box" shape on the canvas at model cell `(col, row)` via the UI:
    /// click its sidebar label, then the target cell. Returns the canvas origin.
    fn place_box_via_ui(&mut self, min: Pos2, col: u32, row: u32) {
        let out = self.frame();
        let label = find_text_exact(&out, "box").expect("sidebar lists the box template");
        let _ = self.click_at(label.center());
        let _ = self.click_at(self.cell_center(min, col, row));
        assert_eq!(self.object_count(), 1, "clicking the canvas placed one box");
    }

    /// Select the first shape by clicking its body center.
    fn select_first_box(&mut self, min: Pos2) {
        let (x, y, w, h) = self.first_box();
        let _ = self.click_at(self.cell_center(min, x + w / 2, y + h / 2));
    }

    /// Click a top-bar button whose galley text is exactly `label`.
    fn click_topbar_button(&mut self, label: &str) -> egui::FullOutput {
        let out = self.frame();
        let rect = find_text_exact(&out, label).unwrap_or_else(|| {
            panic!(
                "top-bar button {label:?} not found; texts: {:?}",
                texts(&out)
            )
        });
        // The top bar is the only exact-match `+`/`−`/Save/Open… in the upper strip.
        assert!(rect.min.y < 60.0, "button {label:?} is in the top bar");
        self.click_at(rect.center())
    }

    /// Click an item inside a top-bar dropdown (FR6, S-025): open the menu by
    /// clicking its button, settle a frame so the popup lays out, then click the
    /// item. Returns the release frame of the item click.
    fn click_menu_item(&mut self, menu: &str, item: &str) -> egui::FullOutput {
        let _ = self.click_topbar_button(menu);
        let out = self.frame();
        let rect = find_text_exact(&out, item).unwrap_or_else(|| {
            panic!(
                "menu {menu:?} does not show item {item:?}; texts: {:?}",
                texts(&out)
            )
        });
        self.click_at(rect.center())
    }

    /// Open a path prompt via the File menu and return its kind after it appears.
    /// S-039 / CR-001 — with no dialog available (the fake's default), `Save As…`
    /// and `Export…` open the in-app format choice first; this helper clicks the
    /// matching option so callers keep their old one-call flow. Logical names:
    /// `Open…`, `Save As…`, `Export SVG…`, `Export PNG…`.
    fn open_prompt(&mut self, item: &str) -> PathPrompt {
        match item {
            "Open…" => {
                let _ = self.click_menu_item("File ▾", "Open…");
            }
            "Save As…" => {
                let _ = self.click_menu_item("File ▾", "Save As…");
                self.click_window_button("Diagram document (.ad)");
            }
            "Export SVG…" | "Export PNG…" => {
                let label = if item == "Export SVG…" {
                    "SVG image (.svg)"
                } else {
                    "PNG image (.png)"
                };
                let _ = self.click_menu_item("File ▾", "Export…");
                self.click_window_button(label);
            }
            other => panic!("open_prompt: unknown item {other:?}"),
        };
        let prompt = self.app.path_prompt();
        assert!(
            prompt.is_some(),
            "clicking File ▾ → {item:?} opens a path prompt"
        );
        prompt.expect("checked above")
    }

    /// Click a button in a top-level dialog window (e.g. the S-039 format choice).
    fn click_window_button(&mut self, label: &str) -> egui::FullOutput {
        let out = self.frame();
        let rect = find_text_exact(&out, label)
            .unwrap_or_else(|| panic!("window does not show {label:?}; texts: {:?}", texts(&out)));
        self.click_at(rect.center())
    }

    /// Click the OK button of the currently open path prompt.
    fn click_ok(&mut self) -> egui::FullOutput {
        let out = self.frame();
        let rect = find_text_exact(&out, "OK").expect("prompt shows an OK button");
        self.click_at(rect.center())
    }
}

// ─────────────────────────── shape scanning ───────────────────────────

/// All laid-out texts with their absolute rects.
fn texts(out: &egui::FullOutput) -> Vec<(String, Rect)> {
    out.shapes
        .iter()
        .filter_map(|cs| match &cs.shape {
            egui::epaint::Shape::Text(t) => Some((
                t.galley.text().to_owned(),
                Rect::from_min_size(t.pos, t.galley.size()),
            )),
            _ => None,
        })
        .collect()
}

/// First galley whose text contains `needle`.
fn find_text(out: &egui::FullOutput, needle: &str) -> Option<Rect> {
    texts(out)
        .into_iter()
        .find(|(t, _)| t.contains(needle))
        .map(|(_, r)| r)
}

/// First galley whose text is exactly `label`.
fn find_text_exact(out: &egui::FullOutput, label: &str) -> Option<Rect> {
    texts(out)
        .into_iter()
        .find(|(t, _)| t == label)
        .map(|(_, r)| r)
}

/// All line segments as `((a, b), stroke_width, color)`.
fn colored_line_segments(out: &egui::FullOutput) -> Vec<((Pos2, Pos2), f32, Color32)> {
    out.shapes
        .iter()
        .filter_map(|cs| match &cs.shape {
            egui::epaint::Shape::LineSegment { points, stroke } => {
                Some(((points[0], points[1]), stroke.width, stroke.color))
            }
            _ => None,
        })
        .collect()
}

/// All line segments as `(a, b)` point pairs.
fn line_points(out: &egui::FullOutput) -> Vec<(Pos2, Pos2)> {
    out.shapes
        .iter()
        .filter_map(|cs| match &cs.shape {
            egui::epaint::Shape::LineSegment { points, .. } => Some((points[0], points[1])),
            _ => None,
        })
        .collect()
}

/// All filled rectangles as `(rect, fill)`.
fn filled_rects(out: &egui::FullOutput) -> Vec<(Rect, Color32)> {
    out.shapes
        .iter()
        .filter_map(|cs| match &cs.shape {
            egui::epaint::Shape::Rect(r) if r.fill != Color32::TRANSPARENT => {
                Some((r.rect, r.fill))
            }
            _ => None,
        })
        .collect()
}

/// All outlined rectangles as `(rect, stroke_color)`.
fn stroked_rects(out: &egui::FullOutput) -> Vec<(Rect, Color32)> {
    out.shapes
        .iter()
        .filter_map(|cs| match &cs.shape {
            egui::epaint::Shape::Rect(r) if r.stroke.color != Color32::TRANSPARENT => {
                Some((r.rect, r.stroke.color))
            }
            _ => None,
        })
        .collect()
}

fn close(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

/// Assert the drawn text status bar contains `needle`.
fn assert_status(out: &egui::FullOutput, needle: &str) {
    assert!(
        find_text(out, needle).is_some(),
        "status bar should contain {needle:?}; texts: {:?}",
        texts(out)
            .iter()
            .map(|(t, _)| t.as_str())
            .collect::<Vec<_>>()
    );
}

// ─────────────────────── WYSIWYG geometry (S-012/S-013) ───────────────────────

#[test]
fn canvas_text_is_drawn_inside_the_canvas_area() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 5, 3);
    let out = gui.frame();

    // Border rows drawn by the canvas itself (sidebar previews live at x < SIDEBAR).
    let lines: Vec<Rect> = texts(&out)
        .into_iter()
        .filter(|(t, _)| t.contains('+') && t.contains('-'))
        .map(|(_, r)| r)
        .filter(|r| r.min.x > SIDEBAR)
        .collect();
    assert!(
        !lines.is_empty(),
        "the placed box draws border text inside the canvas"
    );
    for r in &lines {
        assert!(
            r.min.x >= min.x + 2.0 && r.min.y >= min.y + 2.0,
            "canvas text at {r:?} is inside the canvas area ({min:?}) — not under the \
             sidebar/top bar"
        );
    }
    // Model cell (5,3) must map to where it was clicked, not to a screen corner.
    let (cw, _ch) = gui.cell_metrics();
    let expected_x = min.x + MARGIN + 5.0 * cw;
    assert!(
        close(lines[0].min.x, expected_x, 2.0),
        "box left edge at column 5"
    );
}

#[test]
fn canvas_rows_are_spaced_by_line_height_and_do_not_overlap() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 2, 1);
    let out = gui.frame();

    // The five border rows of the single box (canvas-drawn only: inside the canvas
    // area, not top-bar chrome like the "+" scale button).
    let rows: Vec<Rect> = texts(&out)
        .into_iter()
        .filter(|(t, _)| t.starts_with('+') || t.starts_with('|'))
        .map(|(_, r)| r)
        .filter(|r| r.min.x > SIDEBAR && r.min.y >= min.y - 0.5)
        .collect();
    assert_eq!(rows.len(), 5, "a 9x5 box draws five text rows");
    let (cw, ch) = gui.cell_metrics();
    // The reported bug: spacing by the glyph *advance width* made rows overlap.
    assert!(
        ch > cw + 2.0,
        "line height ({ch}) exceeds advance width ({cw})"
    );
    for pair in rows.windows(2) {
        let (top, bottom) = (&pair[0], &pair[1]);
        let dy = bottom.min.y - top.min.y;
        assert!(close(dy, ch, 0.6), "row spacing {dy} == line height {ch}");
        assert!(
            top.max.y <= bottom.min.y + 0.5,
            "rows do not overlap: {top:?} vs {bottom:?}"
        );
    }
    // Vertical position matches the model row (1).
    let expected_y = min.y + MARGIN + 1.0 * ch;
    assert!(
        close(rows[0].min.y, expected_y, 2.5),
        "first row at model row 1"
    );
}

#[test]
fn grid_lines_stay_inside_the_canvas_area() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    let out = gui.frame();
    for (a, b) in line_points(&out) {
        if a.x < SIDEBAR || (a.y < min.y - 0.5 && b.y < min.y - 0.5) {
            continue; // top-bar / sidebar chrome
        }
        // S-026: the preview pane's separator/resize edge is canvas-right chrome,
        // not a grid line.
        if a.x > WIN_W - PREVIEW || b.x > WIN_W - PREVIEW {
            continue;
        }
        assert!(
            a.x >= min.x - 0.5 && a.y >= min.y - 0.5 && b.x >= min.x - 0.5 && b.y >= min.y - 0.5,
            "line ({a:?}..{b:?}) stays inside the canvas area"
        );
    }
}

#[test]
fn empty_canvas_shows_a_centered_hint_and_full_grid() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    let out = gui.frame();

    // The hint sits in the canvas, not under the chrome.
    let hint = find_text(&out, "Pick a shape").expect("empty canvas shows the hint");
    assert!(
        hint.min.x > min.x && hint.max.y < WIN_H - 20.0,
        "hint {hint:?} is inside the canvas area ({min:?})"
    );

    // The grid covers the whole canvas, not just a content extent: with nothing on
    // the diagram there are still many lines in every direction.
    let (cw, ch) = gui.cell_metrics();
    let mut verticals = 0;
    let mut horizontals = 0;
    for (a, b) in line_points(&out) {
        if a.x < SIDEBAR || a.y < min.y - 0.5 {
            continue; // chrome
        }
        if (a.x - b.x).abs() < 0.5 && (a.y - b.y).abs() > 100.0 {
            verticals += 1;
        } else if (a.y - b.y).abs() < 0.5 && (a.x - b.x).abs() > 100.0 {
            horizontals += 1;
        }
    }
    // S-026: the canvas is ~464 px wide (window − sidebar − default preview pane),
    // still full height → dozens of lines each way.
    assert!(
        verticals >= ((WIN_W - SIDEBAR - PREVIEW) / cw) as usize - 2,
        "full-width vertical grid: {verticals}"
    );
    assert!(
        horizontals >= (700.0 / ch) as usize - 2,
        "full-height horizontal grid: {horizontals}"
    );

    // Placing anything makes the hint go away.
    gui.place_box_via_ui(min, 5, 3);
    let out = gui.frame();
    assert!(
        find_text(&out, "Pick a shape").is_none(),
        "hint hides once content exists"
    );
}

#[test]
fn grid_extends_beyond_a_small_content_extent() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 2, 1);
    let out = gui.frame();

    // The box occupies model columns 2..11 / rows 1..6. Grid lines must exist well
    // past its right and bottom edges (the old grid stopped at the content extent).
    let (cw, ch) = gui.cell_metrics();
    let beyond_right = min.x + MARGIN + 20.0 * cw;
    let beyond_bottom = min.y + MARGIN + 15.0 * ch;
    let mut right_line = false;
    let mut bottom_line = false;
    for (a, b) in line_points(&out) {
        if a.x < SIDEBAR || a.y < min.y - 0.5 {
            continue; // chrome
        }
        if (a.x - b.x).abs() < 0.5 && a.x > beyond_right {
            right_line = true;
        }
        if (a.y - b.y).abs() < 0.5 && a.y > beyond_bottom {
            bottom_line = true;
        }
    }
    assert!(right_line, "grid continues past the box's right edge");
    assert!(bottom_line, "grid continues past the box's bottom edge");
}

#[test]
fn status_message_lives_in_the_bottom_bar_not_the_toolbar() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 4, 2); // sets a status message ("Placed box as s1")
    let out = gui.frame();

    let status = find_text(&out, "Placed box as").expect("placement status is shown");
    let scale = find_text_exact(&out, "Scale ×1").expect("scale label in toolbar");
    // The bottom bar sits below the canvas; the scale label lives in the top
    // toolbar. Different rows, unambiguously.
    assert!(
        status.min.y > scale.max.y,
        "status {status:?} is in the bottom bar, below the toolbar's {scale:?}"
    );
    assert!(
        status.max.y < WIN_H,
        "status {status:?} stays inside the window"
    );
}

#[test]
fn zoom_control_reads_minus_label_plus_left_to_right() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.frame();
    let out = gui.frame();

    let minus = find_text_exact(&out, "−").expect("minus button label");
    let plus = find_text_exact(&out, "+").expect("plus button label");
    // Both are single-glyph button labels laid out the same way, so their rects
    // compare fairly: minus must sit left of plus (screen order “− Scale ×N +”).
    assert!(
        minus.min.x < plus.min.x,
        "zoom reads − … + left to right: minus {minus:?}, plus {plus:?}"
    );
    // The scale label sits between them. Its galley anchor is not guaranteed, so
    // accept either hypothesis for its horizontal span.
    let scale = find_text_exact(&out, "Scale ×1").expect("scale label");
    let left_anchored = (scale.min.x, scale.max.x);
    let right_anchored = (scale.min.x - scale.width(), scale.min.x);
    let between = |&(l, r): &(f32, f32)| minus.max.x < l && r < plus.min.x;
    assert!(
        between(&left_anchored) || between(&right_anchored),
        "scale label {scale:?} sits between the zoom buttons (minus {minus:?}, plus {plus:?})"
    );
}

#[test]
fn selection_outline_covers_exactly_the_selected_cells() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 4, 2);
    gui.select_first_box(min);
    let out = gui.frame();

    let (x, y, w, h) = gui.first_box();
    let tl = gui.cell_corner(min, x, y);
    let (cw, ch) = gui.cell_metrics();
    let expected = Rect::from_min_max(tl, Pos2::new(tl.x + w as f32 * cw, tl.y + h as f32 * ch));

    let sel = stroked_rects(&out)
        .into_iter()
        .find(|(_, c)| *c == SELECTION)
        .map(|(r, _)| r);
    let sel = sel.expect("selection outline is drawn");
    assert!(
        close(sel.min.x, expected.min.x, 2.0),
        "outline left: {sel:?} vs {expected:?}"
    );
    assert!(close(sel.min.y, expected.min.y, 2.0), "outline top");
    assert!(close(sel.max.x, expected.max.x, 2.0), "outline right");
    assert!(close(sel.max.y, expected.max.y, 2.0), "outline bottom");
}

/// S-035 AC1–AC3 (FR20) through the real UI: Ctrl+click extends the selection to a
/// group (status bar reports the set size, both members get outlines), and dragging
/// any member moves the WHOLE group by the same delta — relative positions kept.
#[test]
fn ctrl_click_builds_a_group_and_drag_moves_it() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 2, 2); // A at (2,2), spans cols 2..10
    // Second box B at (15,2): sidebar label again, then the target cell.
    let out = gui.frame();
    let label = find_text_exact(&out, "box").expect("sidebar lists the box template");
    let _ = gui.click_at(label.center());
    let _ = gui.click_at(gui.cell_center(min, 15, 2)); // B at (15,2), spans cols 15..23
    assert_eq!(gui.object_count(), 2);

    // Ctrl+click A's body center (6,4): extends the selection to {A, B}.
    let out = gui.ctrl_click_at(gui.cell_center(min, 6, 4));
    assert_eq!(
        gui.app.selection().len(),
        2,
        "Ctrl+click extended the selection"
    );
    assert!(
        find_text_exact(&out, "2 objects selected").is_some(),
        "status bar reports the set size"
    );
    // Both members carry a selection outline.
    let outlines = stroked_rects(&out)
        .into_iter()
        .filter(|(_, c)| *c == SELECTION)
        .count();
    assert_eq!(outlines, 2, "one outline per selected member");

    // Drag B from (19,4) to (27,6): model delta (+8,+2) for the whole group.
    let _ = gui.drag(gui.cell_center(min, 19, 4), gui.cell_center(min, 27, 6));
    let mut positions: Vec<(u32, u32)> = gui
        .app
        .diagram
        .objects()
        .iter()
        .filter_map(|o| match o {
            Object::Shape(s) => Some((s.pos.x, s.pos.y)),
            _ => None,
        })
        .collect();
    positions.sort();
    assert_eq!(
        positions,
        vec![(10, 4), (23, 4)],
        "both members moved by the same (+8,+2) delta"
    );
}

/// S-035 AC1 (FR20) — a Ctrl+click on empty canvas space leaves the selection
/// untouched; only a plain click deselects.
#[test]
fn ctrl_click_on_empty_space_keeps_the_selection() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 4, 2);

    let _ = gui.ctrl_click_at(gui.cell_center(min, 30, 15)); // far from the box
    assert_eq!(
        gui.app.selection().len(),
        1,
        "Ctrl+click on empty space keeps it"
    );
}

/// Regression (B-003): at scale ×N the selection outline, the rendered text and
/// hit-testing must all agree on the FR8 layout — a model cell renders as an N×N
/// block starting at (col·N, row·N), so the outline covers exactly the drawn extent
/// and clicking anywhere inside it selects the object.
#[test]
fn selection_outline_and_hit_test_follow_the_scaled_extent() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    place_text_object(&mut gui, min); // text at (5,3), inline editor open
    // The draft starts as the placeholder content "text": click its right edge to put
    // the cursor at the end, clear it, then type the real content.
    let out = gui.frame();
    let draft = find_text_exact(&out, "text").expect("the draft shows its default content");
    let _ = gui.click_at(Pos2::new(draft.max.x + 3.0, draft.center().y));
    for _ in 0..4 {
        let _ = gui.key(Key::Backspace);
    }
    let _ = gui.type_text("hello world"); // 11 cells wide
    let _ = gui.key(Key::Enter); // commit → extent (5,3) 11×1
    assert_eq!(text_content(&gui).as_deref(), Some("hello world"));

    // Select by clicking the body.
    let _ = gui.click_at(gui.cell_center(min, 10, 3));
    assert!(
        !gui.app.selection().is_empty(),
        "body click selects the text"
    );

    // Scale ×2 via the top bar (a toolbar click keeps the selection).
    let out = gui.click_topbar_button("+");

    let (cw, ch) = gui.cell_metrics();
    // The scaled block of model cell (5,3): starts at screen cells (10, 6), spans 22×2.
    let tl = Pos2::new(min.x + MARGIN + 10.0 * cw, min.y + MARGIN + 6.0 * ch);
    let expected = Rect::from_min_max(tl, Pos2::new(tl.x + 22.0 * cw, tl.y + 2.0 * ch));

    // (a) The selection outline covers exactly the scaled extent…
    let sel = stroked_rects(&out)
        .into_iter()
        .find(|(_, c)| *c == SELECTION)
        .map(|(r, _)| r);
    let sel = sel.expect("selection outline is drawn");
    assert!(
        close(sel.min.x, expected.min.x, 2.0),
        "outline left: {sel:?} vs {expected:?}"
    );
    assert!(close(sel.min.y, expected.min.y, 2.0), "outline top");
    assert!(close(sel.max.x, expected.max.x, 2.0), "outline right");
    assert!(close(sel.max.y, expected.max.y, 2.0), "outline bottom");

    // (b) …and the rendered text starts where the outline starts.
    let galley = find_text_exact(&out, "h e l l o   w o r l d").expect("scaled text is drawn");
    assert!(
        close(galley.min.x, expected.min.x, 2.0),
        "text left: {galley:?} vs {expected:?}"
    );
    assert!(close(galley.min.y, expected.min.y, 2.0), "text top");

    // (c) A click in the right half of the scaled text selects it — deselect first so
    // the assertion is meaningful. Model col 13 → block center at screen cell 27/7.
    let _ = gui.click_at(Pos2::new(
        min.x + MARGIN + 41.0 * cw,
        min.y + MARGIN + 17.0 * ch,
    ));
    assert!(gui.app.selection().is_empty(), "empty-cell click deselects");
    let right_half = Pos2::new(min.x + MARGIN + 27.0 * cw, min.y + MARGIN + 7.0 * ch);
    let _ = gui.click_at(right_half);
    assert!(
        !gui.app.selection().is_empty(),
        "clicking the scaled text selects it"
    );

    // (d) The graph paper follows the rendered block size: at ×2 a line sits on every
    // second screen cell — on the block boundary, never inside a rendered block.
    let has_v_line = |x: f32| {
        line_points(&out).iter().any(|(a, b)| {
            (a.x - b.x).abs() < 0.5
                && (a.y - b.y).abs() > 100.0 // full-height canvas grid lines only
                && (a.x - x).abs() < 1.0
        })
    };
    assert!(
        has_v_line(min.x + MARGIN + 2.0 * cw),
        "grid line on the block boundary"
    );
    assert!(
        !has_v_line(min.x + MARGIN + 1.0 * cw),
        "no grid line inside a rendered block"
    );
}

#[test]
fn resize_symbol_sits_on_the_inner_bottom_right_cell() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 3, 2);
    gui.select_first_box(min);
    let out = gui.frame();

    let (x, y, w, h) = gui.first_box();
    // FR21 / CR-018 — the symbol is a double-headed diagonal arrow in RESIZE
    // color: one long shaft line plus four short chevron lines at its ends.
    let resize_lines: Vec<(Pos2, Pos2)> = colored_line_segments(&out)
        .into_iter()
        .filter(|((_, _), _, c)| *c == RESIZE)
        .map(|(p, _, _)| p)
        .collect();
    assert!(!resize_lines.is_empty(), "the resize symbol is drawn");
    // The shaft is the longest segment; its midpoint must sit on exactly the
    // cell `hit_target` answers with a resize gesture for (marker/hit parity).
    let shaft = resize_lines
        .iter()
        .max_by(|a, b| {
            let la = (a.0 - a.1).length().round() as i64;
            let lb = (b.0 - b.1).length().round() as i64;
            la.cmp(&lb)
        })
        .expect("the shaft segment is present");
    let mid = shaft.0.lerp(shaft.1, 0.5);
    let expected_center = gui.cell_center(min, x + w - 1, y + h - 1);
    assert!(
        close(mid.x, expected_center.x, 2.0) && close(mid.y, expected_center.y, 2.0),
        "symbol centered at {mid:?} is on the inner bottom-right cell \
         {expected_center:?} — where resizing actually works",
    );
    // AC2: no filled square marker in anchor color remains anywhere.
    assert!(
        !filled_rects(&out).iter().any(|(r, c)| {
            *c == ANCHOR && (2.0..=8.0).contains(&r.width()) && (2.0..=8.0).contains(&r.height())
        }),
        "the old amber square marker is gone"
    );
}

#[test]
fn resize_symbol_absent_for_multi_selection() {
    // AC3 — the symbol (and its hit target) belong to single-shape selections
    // only (S-035 rule); a multi-selection shows anchors without any resize mark.
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 2, 2);
    let out = gui.frame();
    let label = find_text_exact(&out, "box").expect("sidebar lists the box template");
    let _ = gui.click_at(label.center());
    let _ = gui.click_at(gui.cell_center(min, 15, 2));
    let _ = gui.ctrl_click_at(gui.cell_center(min, 6, 4));
    let out = gui.frame();

    assert!(
        colored_line_segments(&out)
            .iter()
            .all(|((_, _), _, c)| *c != RESIZE),
        "no resize symbol under a multi-selection"
    );
}

#[test]
fn anchor_arrows_point_outward_from_the_selection_edges() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 0, 0);
    gui.select_first_box(min);
    let out = gui.frame();

    let (x, y, w, h) = gui.first_box();
    // Default box: 9 wide, 5 tall → anchors n(4,0) s(4,4) w(0,2) e(8,2). CR-008 AC3:
    // the markers are arrow glyphs pointing outward from each side.
    let expected = [
        (gui.cell_center(min, x + w / 2, y), '^'),
        (gui.cell_center(min, x + w / 2, y + h - 1), 'v'),
        (gui.cell_center(min, x, y + h / 2), '<'),
        (gui.cell_center(min, x + w - 1, y + h / 2), '>'),
    ];
    let arrows: Vec<(char, Pos2)> = texts(&out)
        .into_iter()
        .filter(|(t, _)| t.chars().count() == 1 && matches!(t.as_str(), "<" | ">" | "^" | "v"))
        .map(|(t, r)| (t.chars().next().expect("single char"), r.center()))
        .collect();
    assert_eq!(arrows.len(), 4, "four anchor arrows are drawn");
    for (e, glyph) in expected {
        assert!(
            arrows
                .iter()
                .any(|(g, p)| *g == glyph && close(p.x, e.x, 2.0) && close(p.y, e.y, 2.0)),
            "anchor arrow '{glyph}' at {e:?} (edge midpoint cell), arrows: {arrows:?}"
        );
    }
}

#[test]
fn connect_drag_line_runs_from_the_source_anchor_to_the_cursor() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 2, 1);
    gui.select_first_box(min);

    let (x, y, w, h) = gui.first_box();
    let e_anchor = gui.cell_center(min, x + w - 1, y + h / 2); // east anchor cell
    let cursor = gui.cell_center(min, x + w + 4, y); // well past the shape
    let out = gui.press_and_hold_to(e_anchor, cursor);

    let hit = line_points(&out).into_iter().find(|&(a, b)| {
        (close(a.x, e_anchor.x, 2.0)
            && close(a.y, e_anchor.y, 2.0)
            && close(b.x, cursor.x, 1.5)
            && close(b.y, cursor.y, 1.5))
            || (close(b.x, e_anchor.x, 2.0)
                && close(b.y, e_anchor.y, 2.0)
                && close(a.x, cursor.x, 1.5)
                && close(a.y, cursor.y, 1.5))
    });
    assert!(
        hit.is_some(),
        "connect line runs from the anchor to the cursor"
    );
}

// ─────────────────────── file operations via prompts ───────────────────────

#[test]
fn save_as_types_path_survives_frames_and_writes_the_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("gui_save.ad");

    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 1, 1);
    assert_eq!(gui.open_prompt("Save As…"), PathPrompt::SaveAs);

    let _ = gui.type_text(path.to_str().unwrap());
    // The reported bug: typed characters vanished between frames.
    for _ in 0..3 {
        gui.frame();
    }
    let out = gui.click_ok();

    assert!(path.exists(), "the file written by Save As exists");
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.starts_with("ad 1.1"), "valid .ad header");
    assert!(content.contains("shape box"), "diagram content serialized");
    assert_eq!(gui.app.current_file().unwrap(), path.as_path());
    assert_status(&out, "Saved");
}

#[test]
fn open_prompt_loads_a_fixture_diagram() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/one-box.ad");
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.canvas_min();
    assert_eq!(gui.open_prompt("Open…"), PathPrompt::Open);

    let _ = gui.type_text(fixture.to_str().unwrap());
    let out = gui.click_ok();

    assert_eq!(gui.object_count(), 1, "the fixture's box is in the model");
    assert_eq!(gui.app.current_file().unwrap(), fixture.as_path());
    assert_status(&out, "Opened");
}

#[test]
fn ctrl_s_after_edit_rewrites_the_current_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("one-box.ad");
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/one-box.ad"),
        &path,
    )
    .unwrap();

    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    assert_eq!(gui.open_prompt("Open…"), PathPrompt::Open);
    let _ = gui.type_text(path.to_str().unwrap());
    let _ = gui.click_ok();
    assert_eq!(gui.object_count(), 1);

    // Add a second box through the UI, then Ctrl+S.
    let out_pre = gui.frame();
    let label = find_text_exact(&out_pre, "box").expect("sidebar lists box");
    let _ = gui.click_at(label.center());
    let _ = gui.click_at(gui.cell_center(min, 20, 5));
    assert_eq!(gui.object_count(), 2);
    let out = gui.ctrl_s();

    let content = std::fs::read_to_string(&path).unwrap();
    let shape_lines = content.lines().filter(|l| l.starts_with("shape ")).count();
    assert_eq!(shape_lines, 2, "the saved file contains both shapes");
    assert_status(&out, "Saved");
}

#[test]
fn export_svg_prefills_the_sibling_path_and_ok_writes_svg() {
    let tmp = tempfile::tempdir().unwrap();
    let ad_path = tmp.path().join("pic.ad");
    std::fs::write(&ad_path, "ad 1.0\nshape box \"A\" as s1 at 1 1 w 9 h 5\n").unwrap();

    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.canvas_min();
    assert_eq!(gui.open_prompt("Open…"), PathPrompt::Open);
    let _ = gui.type_text(ad_path.to_str().unwrap());
    let _ = gui.click_ok();

    // Export SVG: the prompt must prefill `<name>.svg` next to the current file —
    // clicking OK without typing anything writes exactly that.
    assert_eq!(gui.open_prompt("Export SVG…"), PathPrompt::ExportSvg);
    let out = gui.click_ok();

    let svg_path = tmp.path().join("pic.svg");
    assert!(
        svg_path.exists(),
        "the prefilled sibling .svg path was used"
    );
    let svg = std::fs::read_to_string(&svg_path).unwrap();
    assert!(svg.starts_with("<svg"), "valid SVG output");
    assert_status(&out, "SVG saved");
}

#[test]
fn export_png_completes_and_writes_a_valid_png() {
    let tmp = tempfile::tempdir().unwrap();
    let ad_path = tmp.path().join("pic.ad");
    std::fs::write(&ad_path, "ad 1.0\nshape box \"A\" as s1 at 1 1 w 9 h 5\n").unwrap();

    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.canvas_min();
    assert_eq!(gui.open_prompt("Open…"), PathPrompt::Open);
    let _ = gui.type_text(ad_path.to_str().unwrap());
    let _ = gui.click_ok();

    assert_eq!(gui.open_prompt("Export PNG…"), PathPrompt::ExportPng);
    let mut out = gui.click_ok();
    // Exporting is a blocking background job — pump until it reports done.
    for _ in 0..600 {
        if !find_text(&out, "Exporting").is_some() {
            break;
        }
        out = gui.frame();
    }

    let png_path = tmp.path().join("pic.png");
    assert!(png_path.exists(), "the .png was written");
    let bytes = std::fs::read(&png_path).unwrap();
    assert_eq!(&bytes[0..4], &[0x89, b'P', b'N', b'G'], "PNG magic bytes");
    assert!(bytes.len() > 100, "non-trivial PNG");
}

#[test]
fn open_missing_path_keeps_the_model_and_reports_an_error() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 1, 1);
    assert_eq!(gui.open_prompt("Open…"), PathPrompt::Open);

    let _ = gui.type_text("/definitely/not/here.ad");
    let out = gui.click_ok();

    assert_eq!(gui.object_count(), 1, "the previous model is kept");
    assert!(
        gui.app.current_file().is_none(),
        "no file association on failure"
    );
    assert_status(&out, "Open failed");
}

#[test]
fn escape_closes_the_path_prompt_without_side_effects() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.canvas_min();
    gui.open_prompt("Save As…");
    assert!(gui.app.path_prompt().is_some());

    let _ = gui.key(Key::Escape);

    assert!(gui.app.path_prompt().is_none(), "Escape closed the prompt");
    assert!(gui.app.current_file().is_none());
}

#[test]
fn backspace_in_the_prompt_does_not_touch_the_canvas_selection() {
    // The reported bug: typing in a field deleted canvas objects, because the
    // global key handler saw Delete/Backspace while a text field had focus.
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 2, 2);
    gui.select_first_box(min);
    assert_eq!(gui.object_count(), 1);

    gui.open_prompt("Save As…");
    let _ = gui.type_text("a");
    let out = gui.key(Key::Backspace); // must edit the field, not the diagram

    assert_eq!(
        gui.object_count(),
        1,
        "Backspace in a field does not delete objects"
    );
    let _ = gui.key(Key::Enter); // empty path → prompt stays open, nothing written
    assert!(gui.app.path_prompt().is_some(), "empty path is rejected");
    let _ = out;
}

// ─────────────────────── canvas gestures (S-013) ───────────────────────

#[test]
fn sidebar_click_then_canvas_click_places_the_shape() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();

    let out = gui.frame();
    let label = find_text_exact(&out, "box").expect("sidebar lists the box template");
    let _ = gui.click_at(label.center());
    // A click *outside* the canvas (in the sidebar) must not place anything.
    let _ = gui.click_at(Pos2::new(60.0, 400.0));
    assert_eq!(
        gui.object_count(),
        0,
        "clicking outside the canvas places nothing"
    );

    let _ = gui.click_at(gui.cell_center(min, 5, 3));
    assert_eq!(gui.object_count(), 1);
    let (x, y, _, _) = gui.first_box();
    assert_eq!((x, y), (5, 3), "the shape landed on the clicked cell");
}

#[test]
fn body_drag_moves_the_shape_grid_aligned() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 2, 2);
    let (x0, y0, w, h) = gui.first_box();

    let from = gui.cell_center(min, x0 + w / 2, y0 + h / 2); // body center
    let (cw, _ch) = gui.cell_metrics();
    let to = from + Vec2::new(2.0 * cw, 0.0); // exactly two cells right
    let _ = gui.drag(from, to);

    let (x1, y1, _, _) = gui.first_box();
    assert_eq!(
        (x1, y1),
        (x0 + 2, y0),
        "drag moved the shape by exactly 2 cells"
    );
}

#[test]
fn resize_handle_drag_grows_the_shape() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 1, 1);
    gui.select_first_box(min);
    let (x, y, w0, h0) = gui.first_box();

    // Drag from the marker's cell (inner bottom-right) outward by 2x1 cells.
    let from = gui.cell_center(min, x + w0 - 1, y + h0 - 1);
    let (cw, ch) = gui.cell_metrics();
    let to = from + Vec2::new(2.0 * cw, 1.0 * ch);
    let _ = gui.drag(from, to);

    let (_, _, w1, h1) = gui.first_box();
    assert_eq!(
        (w1, h1),
        (w0 + 2, h0 + 1),
        "resize grew the shape by the drag delta"
    );
}

#[test]
fn anchor_drag_creates_a_connector_between_two_shapes() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 2, 1); // A: 9 wide → east anchor at (10, 3)
    let out = gui.frame();
    let label = find_text_exact(&out, "box").expect("sidebar lists box");
    let _ = gui.click_at(label.center());
    let _ = gui.click_at(gui.cell_center(min, 20, 1)); // B at (20, 1)
    assert_eq!(gui.object_count(), 2);

    // Re-select A (clicking B's cell selected B), then drag A's east anchor onto B.
    let (ax, ay, aw, ah) = gui.first_box();
    let _ = gui.click_at(gui.cell_center(min, ax + aw / 2, ay + ah / 2));
    let from = gui.cell_center(min, ax + aw - 1, ay + ah / 2); // A's east anchor cell
    let to = gui.cell_center(min, 20, 1 + 5 / 2); // B's west edge (cell 20, row 3)
    let _ = gui.drag(from, to);

    assert!(
        gui.app
            .diagram
            .objects()
            .iter()
            .any(|o| matches!(o, Object::Connector(_))),
        "a connector object now links the two shapes"
    );
}

/// CR-008 AC2/AC3: pressing the target tip cell of a straight connector cycles its
/// style — hollow arrow → empty rhombus → filled rhombus → bare — and the canvas glyph
/// follows each step.
#[test]
fn pressing_the_connector_tip_cycles_its_style() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 2, 1); // A: east anchor at (10, 3)
    let out = gui.frame();
    let label = find_text_exact(&out, "box").expect("sidebar lists box");
    let _ = gui.click_at(label.center());
    let _ = gui.click_at(gui.cell_center(min, 20, 1)); // B at (20, 1)
    assert_eq!(gui.object_count(), 2);

    // Drag A's east anchor onto B's west edge → straight connector (route row 3).
    let (ax, ay, aw, ah) = gui.first_box();
    let _ = gui.click_at(gui.cell_center(min, ax + aw / 2, ay + ah / 2));
    let from = gui.cell_center(min, ax + aw - 1, ay + ah / 2);
    let to = gui.cell_center(min, 20, 1 + 5 / 2);
    let _ = gui.drag(from, to);

    let tip_cell = gui.cell_center(min, 19, 3); // one cell before B's west anchor (20, 3)
    // The connector starts with the default hollow arrow; each press advances it.
    let expected_after_presses = [
        TipStyle::EmptyRhombus,
        TipStyle::FilledRhombus,
        TipStyle::Bare,
    ];

    for next in &expected_after_presses {
        let out = gui.click_at(tip_cell);
        // The status bar names the new style.
        assert_status(&out, &format!("target tip set to {next}"));
        // The model follows.
        let connector: &ConnectorInstance = gui
            .app
            .diagram
            .objects()
            .iter()
            .find_map(|o| match o {
                Object::Connector(c) => Some(c),
                _ => None,
            })
            .expect("the connector exists");
        assert_eq!(connector.to_tip, *next, "target tip cycled to {next}");
    }

    // The glyph on the canvas follows: bare stamps nothing, so the last press left no
    // rhombus/arrow content on the tip cell — the row is plain dashes again.
    let out = gui.frame();
    assert!(
        !texts(&out)
            .iter()
            .any(|(t, _)| t.contains('◇') || t.contains('◆')),
        "no rhombus glyph remains after cycling to bare"
    );
}

#[test]
fn arrow_tool_draws_a_standalone_arrow_via_the_sidebar() {
    use diagram_model::anchors::ConnectorEndpoint;
    use diagram_model::geometry::CellPos;

    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();

    // Arm the arrow tool from the sidebar (CR-010 AC5).
    let out = gui.frame();
    let label = find_text_exact(&out, "↘  Arrow").expect("sidebar lists the arrow tool");
    let _ = gui.click_at(label.center());

    // Press-drag-release across empty canvas space.
    let from = gui.cell_center(min, 3, 6);
    let to = gui.cell_center(min, 14, 9);
    let _ = gui.drag(from, to);
    // One settle frame: the release frame processes the drop, but its rendering
    // already contains the pre-release status bar text.
    let out = gui.frame();

    // The status bar reports the new connector…
    assert!(
        texts(&out)
            .iter()
            .any(|(t, _)| t.starts_with("Connector added as")),
        "status bar names the new arrow"
    );
    // …and the model holds a standalone arrow with both ends free at the
    // press/drop cells.
    let connector: &ConnectorInstance = gui
        .app
        .diagram
        .objects()
        .iter()
        .find_map(|o| match o {
            Object::Connector(c) => Some(c),
            _ => None,
        })
        .expect("the arrow exists");
    assert_eq!(connector.from, ConnectorEndpoint::Free(CellPos::new(3, 6)));
    assert_eq!(connector.to, ConnectorEndpoint::Free(CellPos::new(14, 9)));
}

#[test]
fn arrow_drag_line_runs_from_the_attached_source_to_the_cursor() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 2, 1);

    // Arm the arrow tool from the sidebar.
    let out = gui.frame();
    let label = find_text_exact(&out, "↘  Arrow").expect("sidebar lists the arrow tool");
    let _ = gui.click_at(label.center());

    // Press exactly on the box's east anchor and hold past the shape: the drag
    // line must run from the anchor to the cursor (same feedback as S-013).
    let (x, y, w, h) = gui.first_box();
    let e_anchor = gui.cell_center(min, x + w - 1, y + h / 2); // east anchor cell
    let cursor = gui.cell_center(min, x + w + 4, y);
    let out = gui.press_and_hold_to(e_anchor, cursor);

    assert!(
        line_points(&out).into_iter().any(|(a, b)| {
            (close(a.x, e_anchor.x, 2.0)
                && close(a.y, e_anchor.y, 2.0)
                && close(b.x, cursor.x, 1.5)
                && close(b.y, cursor.y, 1.5))
                || (close(b.x, e_anchor.x, 2.0)
                    && close(b.y, e_anchor.y, 2.0)
                    && close(a.x, cursor.x, 1.5)
                    && close(a.y, cursor.y, 1.5))
        }),
        "arrow drag line runs from the attached source anchor to the cursor"
    );
}

#[test]
fn delete_key_removes_the_selected_object() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 2, 2);
    gui.select_first_box(min);
    assert_eq!(gui.object_count(), 1);

    let _ = gui.key(Key::Delete);

    assert_eq!(gui.object_count(), 0, "Delete removed the selection");
}

#[test]
fn escape_cancels_placement_mode() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    let out = gui.frame();
    let label = find_text_exact(&out, "box").expect("sidebar lists box");
    let _ = gui.click_at(label.center());

    let _ = gui.key(Key::Escape);
    let _ = gui.click_at(gui.cell_center(min, 5, 3));
    assert_eq!(
        gui.object_count(),
        0,
        "after Escape the canvas click placed nothing"
    );
}

#[test]
fn text_tool_places_and_inline_edit_commits_on_enter() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    let out = gui.frame();
    let label = find_text_exact(&out, "T  Text").expect("sidebar lists the text tool");
    let _ = gui.click_at(label.center());

    let _ = gui.click_at(gui.cell_center(min, 5, 3));
    assert_eq!(gui.object_count(), 1, "clicking placed a text object");
    let out = gui.frame();
    assert!(
        find_text(&out, "type its content").is_some(),
        "the inline editor opened with its hint"
    );

    // The draft starts as the placeholder content "text": click at its right edge to
    // put the cursor at the end, clear it, then type the real content.
    let label_rect = find_text_exact(&out, "text").expect("the draft shows its default content");
    let _ = gui.click_at(Pos2::new(label_rect.max.x + 3.0, label_rect.center().y));
    for _ in 0..5 {
        let _ = gui.key(Key::Backspace);
    }
    let _ = gui.type_text("hello world");
    let out = gui.key(Key::Enter); // commit (not cancel)

    let text_obj = gui
        .app
        .diagram
        .objects()
        .iter()
        .find_map(|o| match o {
            Object::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .expect("a Text object exists");
    assert_eq!(text_obj, "hello world", "Enter committed the typed content");
    let _ = out;
}

#[test]
fn double_click_on_existing_text_reopens_editing() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    let out = gui.frame();
    let label = find_text_exact(&out, "T  Text").expect("sidebar lists the text tool");
    let _ = gui.click_at(label.center());
    let _ = gui.click_at(gui.cell_center(min, 5, 3));

    // The draft starts as the placeholder "text": cursor to its end, clear it, type.
    let out = gui.frame();
    let label_rect = find_text_exact(&out, "text").expect("the draft shows its default content");
    let _ = gui.click_at(Pos2::new(label_rect.max.x + 3.0, label_rect.center().y));
    for _ in 0..5 {
        let _ = gui.key(Key::Backspace);
    }
    let _ = gui.type_text("hello");
    let _ = gui.key(Key::Enter);

    // Double-click the text object's cell to edit it again.
    let pos = gui
        .app
        .diagram
        .objects()
        .iter()
        .find_map(|o| match o {
            Object::Text(t) => Some((t.pos.x, t.pos.y)),
            _ => None,
        })
        .expect("text object");
    let out = gui.double_click_at(gui.cell_center(min, pos.0, pos.1));
    assert!(
        gui.app.is_editing(),
        "double-click reopened the inline editor"
    );
    assert!(
        find_text(&out, "Editing").is_some(),
        "status reports editing"
    );

    // The draft shows the existing content; click its right edge so the appended
    // character lands at the end (a fresh focus starts with the cursor at 0).
    let hello_rect = find_text_exact(&out, "hello").expect("the draft holds the old content");
    let _ = gui.click_at(Pos2::new(hello_rect.max.x + 3.0, hello_rect.center().y));
    let _ = gui.type_text("!");
    let _ = gui.key(Key::Enter);
    let content = gui
        .app
        .diagram
        .objects()
        .iter()
        .find_map(|o| match o {
            Object::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(content, "hello!");
}

/// Places a text object (opening the inline editor).
fn place_text_object(gui: &mut Gui, min: Pos2) {
    let out = gui.frame();
    let label = find_text_exact(&out, "T  Text").expect("sidebar lists the text tool");
    let _ = gui.click_at(label.center());
    let _ = gui.click_at(gui.cell_center(min, 5, 3));
    assert_eq!(gui.object_count(), 1, "clicking placed a text object");
    assert!(gui.app.is_editing(), "placing opens the inline editor");
}

/// The content of the first (only) text object in the diagram.
fn text_content(gui: &Gui) -> Option<String> {
    gui.app.diagram.objects().iter().find_map(|o| match o {
        Object::Text(t) => Some(t.text.clone()),
        _ => None,
    })
}

/// Regression: while an inline text edit is open, the path prompt must take the
/// keyboard when it appears — the canvas field used to re-request focus every frame
/// (drawn last, so its request won), and typed characters landed in the draft.
/// Taking the keyboard blurs the edit, which commits it (S-018 AC1).
#[test]
fn path_prompt_keeps_the_keyboard_over_an_open_inline_edit() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    place_text_object(&mut gui, min);

    // Opening the prompt takes the keyboard; that blur commits the inline edit.
    let _ = gui.open_prompt("Open…");
    assert!(!gui.app.is_editing(), "blur committed the inline edit");
    assert_eq!(
        text_content(&gui).as_deref(),
        Some("text"),
        "the untouched draft was committed"
    );

    // Typing must land in the prompt field, not a (now closed) canvas draft.
    let _ = gui.type_text("ab");
    assert_eq!(gui.app.path_prompt_value().as_deref(), Some("ab"));

    // Escape closes the prompt.
    let _ = gui.key(Key::Escape);
    assert!(gui.app.path_prompt().is_none(), "Escape closed the prompt");
}

/// Regression: for one frame after clicking a file action, both the path prompt and
/// an inline edit are open (the blur commit happens on the settle frame). Escape in
/// that window must close the innermost thing — the prompt — not cancel the edit.
#[test]
fn escape_while_prompt_opens_closes_the_prompt_not_the_edit() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    place_text_object(&mut gui, min);

    // Click File ▾ → Open…, but press Escape before the settle frame: both are open now.
    let out = gui.frame();
    let file = find_text_exact(&out, "File ▾").expect("top bar has File ▾");
    let _ = gui.click_at(file.center());
    let out = gui.frame();
    let rect = find_text_exact(&out, "Open…").expect("the File menu shows Open…");
    let _ = gui.pump(Gui::press_at(rect.center()));
    let _ = gui.pump(Gui::release_at(rect.center()));
    assert!(gui.app.path_prompt().is_some(), "the prompt opened");
    assert!(
        gui.app.is_editing(),
        "the edit is still open in the transient window"
    );

    let _ = gui.pump(vec![Event::Key {
        key: Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::default(),
    }]);
    assert!(gui.app.path_prompt().is_none(), "Escape closed the prompt");
    assert!(gui.app.is_editing(), "…not the inline edit");

    // Settle: the edit is still open; a later Escape cancels it.
    let _ = gui.frame();
    assert!(gui.app.is_editing(), "the edit survives the settle frame");
    let _ = gui.key(Key::Escape);
    assert!(
        !gui.app.is_editing(),
        "a later Escape cancels the surviving edit"
    );
}

/// Regression: while an inline text edit is open, clicking a template-editor field
/// must keep the keyboard in that field — the canvas draft used to steal it back
/// every frame (same root cause as the path-prompt case). The blur commits the edit.
#[test]
fn template_editor_field_keeps_the_keyboard_over_an_open_inline_edit() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    place_text_object(&mut gui, min);

    // Open the template editor window.
    let out = gui.frame();
    let new_btn = find_text(&out, "New template").expect("sidebar has '＋ New template…'");
    let _ = gui.click_at(new_btn.center());
    assert!(
        gui.app.template_editor_name().is_some(),
        "the editor opened"
    );

    // Click the Name field: it takes the keyboard, blurring (committing) the edit.
    let out = gui.frame();
    let name_label = find_text_exact(&out, "Name:").expect("the editor shows a Name field");
    let _ = gui.click_at(Pos2::new(name_label.max.x + 10.0, name_label.center().y));
    assert!(!gui.app.is_editing(), "blur committed the inline edit");

    // Typing must land in the Name field (the click places the cursor at an
    // arbitrary offset inside "new", so assert on containment).
    let _ = gui.type_text("xy");
    let name = gui.app.template_editor_name().expect("the editor is open");
    assert!(
        name.contains("xy"),
        "typing reached the Name field: {name:?}"
    );
}

#[test]
fn scale_buttons_change_the_diagram_scale() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.canvas_min();

    let _ = gui.click_topbar_button("+");
    assert_eq!(gui.app.diagram.scale(), 2, "+ doubled the scale");
    let _ = gui.click_topbar_button("−");
    assert_eq!(gui.app.diagram.scale(), 1, "− halved it back");
}

#[test]
fn copy_button_puts_the_exact_canvas_bytes_on_the_clipboard() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 3, 2);
    let expected = gui.app.canvas_frame().expect("canvas renders").0;

    // The CopyText command is drained into the *release* frame's FullOutput (egui
    // takes it out of the context at end of pass), so capture that frame directly
    // instead of going through `click_at`'s settle frame. Copy is a top-level File ▾
    // item (S-039 flattened the one-item Output submenu): open the menu, then
    // press/release on the item.
    let out = gui.frame();
    let file = find_text_exact(&out, "File ▾").expect("top bar has File ▾");
    let _ = gui.click_at(file.center());
    let out = gui.frame();
    let rect = find_text_exact(&out, "Copy").expect("the File menu shows Copy");
    let _ = gui.pump(Gui::press_at(rect.center()));
    let out = gui.pump(Gui::release_at(rect.center()));
    let copied = out.platform_output.commands.iter().find_map(|c| match c {
        egui::OutputCommand::CopyText(t) => Some(t.clone()),
        _ => None,
    });
    assert_eq!(
        copied.as_deref(),
        Some(expected.as_str()),
        "clipboard == canvas text"
    );
}

// ─────────────────────── robustness / exploratory ───────────────────────

/// Deterministic LCG so the fuzz test is reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    fn below(&mut self, n: u32) -> u32 {
        (self.next() % n as u64) as u32
    }
}

#[test]
fn random_event_stream_never_panics_and_keeps_the_model_valid() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.frame();
    let mut rng = Rng(0x5eed_cafe);
    let keys = [
        Key::Delete,
        Key::Backspace,
        Key::Escape,
        Key::Enter,
        Key::A,
        Key::M,
    ];

    for _ in 0..400 {
        let events = match rng.below(6) {
            0 => vec![Event::PointerMoved(Pos2::new(
                rng.below(WIN_W as u32) as f32,
                rng.below(WIN_H as u32) as f32,
            ))],
            1 => press_events(
                rng.below(WIN_W as u32) as f32,
                rng.below(WIN_H as u32) as f32,
            ),
            2 => vec![Event::PointerButton {
                pos: Pos2::new(
                    rng.below(WIN_W as u32) as f32,
                    rng.below(WIN_H as u32) as f32,
                ),
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::default(),
            }],
            3 => vec![Event::Key {
                key: keys[rng.below(6) as usize],
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::default(),
            }],
            4 => vec![Event::Text("x".to_owned())],
            _ => Vec::new(),
        };
        let out = gui.pump(events);
        // Invariants must hold after every frame.
        assert!((1..=8).contains(&gui.app.diagram.scale()));
        assert!(gui.object_count() < 500, "object explosion");
        let _ = out;
    }
    // The canvas still renders valid text at the end.
    let (text, _, _) = gui.app.canvas_frame().expect("canvas renders");
    let _ = text;
}

fn press_events(x: f32, y: f32) -> Vec<Event> {
    Gui::press_at(Pos2::new(x, y))
}

#[test]
fn releasing_outside_the_canvas_cancels_a_body_drag() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 6, 3);
    let (x0, y0, w, h) = gui.first_box();

    let from = gui.cell_center(min, x0 + w / 2, y0 + h / 2);
    let to_somewhere_else = Pos2::new(50.0, 400.0); // over the sidebar
    let _ = gui.press_and_hold_to(from, to_somewhere_else);
    let _ = gui.pump(Gui::release_at(to_somewhere_else));

    let (x1, y1, _, _) = gui.first_box();
    assert_eq!(
        (x1, y1),
        (x0, y0),
        "drag released off-canvas did not move the shape"
    );
}

#[test]
fn template_editor_save_registers_a_placeable_template() {
    let tmp = tempfile::tempdir().unwrap();
    let (services, _warnings) = Services::native_with_collections(Some(tmp.path()));
    let mut gui = Gui::new(DrawingApp::new(services));
    let _ = gui.canvas_min();

    // Open the editor from the sidebar.
    let out = gui.frame();
    let new_btn = find_text(&out, "New template").expect("sidebar has '＋ New template…'");
    let _ = gui.click_at(new_btn.center());
    let out = gui.frame();
    assert!(
        find_text(&out, "Template editor").is_some(),
        "the editor window opened"
    );

    // Select the first grid cell (normal mode) — gives the glyph editor a target.
    let dots: Vec<(f32, f32, Rect)> = texts(&out)
        .into_iter()
        .filter(|(t, _)| t == "·")
        .map(|(_, r)| (r.min.y, r.min.x, r))
        .collect();
    assert!(!dots.is_empty(), "the template grid is drawn");
    let mut dots = dots;
    dots.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let first_cell = dots.into_iter().next().unwrap().2;
    let _ = gui.click_at(first_cell.center()); // selects (0, 0)

    // Type a glyph into the selected cell: focus the small single-line editor that
    // sits right after the "cell (x, y)" hint label, then send the character.
    let out = gui.frame();
    let hint = find_text(&out, "cell (0, 0)").expect("the clicked cell is selected");
    let _ = gui.click_at(Pos2::new(hint.max.x + 20.0, hint.center().y));
    let out = gui.pump(vec![Event::Text("X".to_owned())]);

    // Enable anchor mode (click its checkbox label) and click the same cell again to
    // declare an anchor there — connectors bind to anchors, and validation requires
    // at least one.
    let anchor_label = find_text(&out, "Anchor mode").expect("anchor-mode checkbox is shown");
    let _ = gui.click_at(anchor_label.center());
    gui.frame();
    let _ = gui.click_at(first_cell.center()); // toggles an anchor at (0, 0)

    // Save via the window's own Save button — the only visible "Save" (the
    // top-bar one lives inside the closed File menu, S-025).
    let out = gui.frame();
    let saves: Vec<Rect> = texts(&out)
        .into_iter()
        .filter(|(t, _)| t == "Save")
        .map(|(_, r)| r)
        .collect();
    assert_eq!(saves.len(), 1, "only the window's Save button is visible");
    let out = gui.click_at(saves[0].center());

    // The template is registered under the default collection "myshapes".
    assert!(
        gui.app.services().catalog.get("myshapes/new").is_some(),
        "the saved template is in the catalog"
    );
    assert!(
        tmp.path().join("myshapes.adc").exists(),
        "the .adc file was written"
    );
    assert!(
        find_text(&out, "Template editor").is_none(),
        "the window closed after saving"
    );

    // And it now shows up in the sidebar's template list: the collection group
    // header plus the template's own name. The group is below the fold with the
    // full built-in catalog, so scroll to it first (egui culls off-clip labels).
    let out = scroll_sidebar_until(&mut gui, "new");
    assert!(
        find_text(&out, "myshapes").is_some(),
        "sidebar has the collection group"
    );
    assert!(
        find_text_exact(&out, "new").is_some(),
        "sidebar lists the new template by name"
    );
}

/// S-024 AC6 — the art text box and resize toolset work end to end: pasted art
/// replaces the grid, complete rules unlock a resized live preview without error,
/// the saved template is resizable in the catalog with its computed minimum, and a
/// placed instance resizes on the canvas by re-rasterization (AC5).
#[test]
fn template_editor_art_box_and_resize_rules_save_a_resizable_template() {
    let tmp = tempfile::tempdir().unwrap();
    let (services, _warnings) = Services::native_with_collections(Some(tmp.path()));
    let mut gui = Gui::new(DrawingApp::new(services));
    let _ = gui.canvas_min();

    // Open the editor from the sidebar.
    let out = gui.frame();
    let new_btn = find_text(&out, "New template").expect("sidebar has '＋ New template…'");
    let _ = gui.click_at(new_btn.center());
    let out = gui.frame();
    assert!(
        find_text(&out, "Template editor").is_some(),
        "the editor window opened"
    );

    // The blank 12×8 grid: every visible cell is a faint dot (the grid scrolls).
    let dots_before = texts(&out).iter().filter(|(t, _)| t == "·").count();
    assert!(dots_before > 0, "the blank grid shows empty cells");

    // Paste art into the text box: focus it just below its caption, drag-select the
    // whole (blank) buffer — egui 0.37's TextEdit has no select-all shortcut — and
    // type the 5×3 box over the selection.
    let cap = find_text(&out, "Art (type or paste").expect("the art caption is shown");
    let top_left = Pos2::new(cap.min.x + 2.0, cap.max.y + 4.0);
    let bottom_right = Pos2::new(cap.min.x + 418.0, cap.max.y + 134.0);
    let _ = gui.drag(top_left, bottom_right); // click-drag selects the whole buffer
    let out = gui.type_text("+---+\n|   |\n+---+");

    // The grid re-parsed to 5×3: both the art box and the live preview (rules still
    // off → natural-size stamp rendered from `draft.rows`) show the exact three-line
    // pattern. A rejected or newline-mangled input can never produce this galley —
    // on rejection the buffer reverts to the previous grid, and dropped newlines
    // yield a single 15-character row instead.
    let shown = texts(&out);
    assert!(
        shown.iter().any(|(t, _)| t == "+---+\n|   |\n+---+"),
        "the grid and preview mirror the pasted 5×3 art"
    );
    // Only the three interior spaces of the middle row remain empty cells.
    assert_eq!(
        shown.iter().filter(|(t, _)| t == "·").count(),
        3,
        "the grid mirrors the pasted 5×3 art"
    );

    // Enable the resize toolset and fill every edge's insert/remove pair.
    let rules_label = find_text(&out, "Resize rules").expect("the resize checkbox is shown");
    let _ = gui.click_at(rules_label.center());
    let out = gui.frame();
    assert!(find_text(&out, "Top").is_some(), "the edge rows appeared");

    let field_texts = texts(&out);
    let inserts: Vec<Rect> = field_texts
        .iter()
        .filter(|(t, _)| t == "insert")
        .map(|(_, r)| *r)
        .collect();
    let removes: Vec<Rect> = field_texts
        .iter()
        .filter(|(t, _)| t == "remove")
        .map(|(_, r)| *r)
        .collect();
    assert_eq!(inserts.len(), 4, "one insert field per edge");
    assert_eq!(removes.len(), 4, "one remove field per edge");
    // Row order in the UI: Top, Bottom, Left, Right — sort by y to be safe.
    let mut inserts = inserts;
    inserts.sort_by(|a, b| a.min.y.total_cmp(&b.min.y));
    let mut removes = removes;
    removes.sort_by(|a, b| a.min.y.total_cmp(&b.min.y));
    // Fill all eight fields: top/bottom use '-', left/right use '|'.
    let fill = |gui: &mut Gui, field: Rect, c: char| {
        let _ = gui.click_at(Pos2::new(field.max.x + 12.0, field.center().y));
        let _ = gui.type_text(&c.to_string());
    };
    fill(&mut gui, inserts[0], '-'); // top insert
    fill(&mut gui, removes[0], '-'); // top remove
    fill(&mut gui, inserts[1], '-'); // bottom insert
    fill(&mut gui, removes[1], '-'); // bottom remove
    fill(&mut gui, inserts[2], '|'); // left insert
    fill(&mut gui, removes[2], '|'); // left remove
    fill(&mut gui, inserts[3], '|'); // right insert
    fill(&mut gui, removes[3], '|'); // right remove

    // With complete rules the live preview rasterizes at its current target size —
    // no error hint may appear.
    let out = gui.frame();
    assert!(
        find_text(&out, "preview error").is_none(),
        "the preview renders"
    );

    // Declare one anchor (validation requires at least one): anchor mode on, then
    // click the top-left corner cell of the grid.
    let anchor_label = find_text(&out, "Anchor mode").expect("anchor-mode checkbox is shown");
    let _ = gui.click_at(anchor_label.center());
    let out = gui.frame();
    // The toolbar's scale control is also a "+" — it lives on the right side of
    // the top chrome (x ≈ 1000), while the grid sits in the window on the left;
    // filter by x, not y (the S-027 app bar pushed the toolbar below y = 20).
    let corner = texts(&out)
        .into_iter()
        .filter(|(t, r)| t == "+" && r.min.x < 400.0 && r.min.y > 20.0)
        .min_by(|a, b| {
            a.1.min
                .y
                .total_cmp(&b.1.min.y)
                .then_with(|| a.1.min.x.total_cmp(&b.1.min.x))
        })
        .map(|(_, r)| r)
        .expect("the grid shows the box's top-left corner");
    let _ = gui.click_at(corner.center());

    // Save via the window's own Save button — the only visible "Save" (the
    // top-bar one lives inside the closed File menu, S-025).
    let out = gui.frame();
    let saves: Vec<Rect> = texts(&out)
        .into_iter()
        .filter(|(t, _)| t == "Save")
        .map(|(_, r)| r)
        .collect();
    assert_eq!(saves.len(), 1, "only the window's Save button is visible");
    let out = gui.click_at(saves[0].center());

    // The template is registered and — because it carries resize rules — the catalog
    // marks it resizable with a minimum extent (S-024 AC5/AC6).
    let info = gui
        .app
        .services()
        .catalog
        .get("myshapes/new")
        .expect("the saved template is in the catalog");
    assert!(info.resizable, "a rule-bearing custom shape is resizable");
    // The 5×3 box shrinks to `++`/`||` — both axes down to 2 (the corners are fixed,
    // the middle row is a pure edge+fill row).
    assert_eq!(
        info.min_size,
        diagram_model::geometry::CellSize::new(2, 2).expect("valid"),
        "the minimum extent is computed by shrink simulation"
    );
    assert!(
        find_text(&out, "Template editor").is_none(),
        "the window closed after saving"
    );

    // AC6 end: the saved template is immediately placeable from the sidebar, and a
    // placed instance resizes by re-rasterization (AC5) — not a fixed-size stamp.
    // The group is below the fold with the full built-in catalog: scroll to it
    // first (egui culls off-clip labels).
    let out = scroll_sidebar_until(&mut gui, "new");
    let label = find_text_exact(&out, "new").expect("sidebar lists the new template");
    let _ = gui.click_at(label.center());
    let min = gui.canvas_min();
    let _ = gui.click_at(gui.cell_center(min, 2, 2)); // place at (2, 2)
    assert_eq!(gui.object_count(), 1);

    // Natural size on the canvas: the pipeline trims to the content box, so a lone
    // 5×3 box renders exactly its three art rows.
    let (text, w, h) = gui.app.canvas_frame().expect("renders");
    assert_eq!((w, h), (5, 3));
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        vec!["+---+", "|   |", "+---+"],
        "natural-size stamp at first placement"
    );

    // Select it and drag the resize handle (inner bottom-right cell) out by 2×1.
    let (x, y, w0, h0) = gui.first_box();
    let _ = gui.click_at(gui.cell_center(min, x + w0 / 2, y + h0 / 2));
    let from = gui.cell_center(min, x + w0 - 1, y + h0 - 1);
    let (cw, ch) = gui.cell_metrics();
    let _ = gui.drag(from, from + Vec2::new(2.0 * cw, 1.0 * ch));

    let (_, _, w1, h1) = gui.first_box();
    assert_eq!((w1, h1), (7, 4), "the resize gesture grew the instance");
    // The canvas now shows exactly the re-rasterized 7×4 box — a fixed-size stamp
    // would still render the original 5×3 art.
    let (text, w, h) = gui.app.canvas_frame().expect("renders");
    assert_eq!((w, h), (7, 4));
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        vec!["+-----+", "|     |", "|     |", "+-----+"],
        "the canvas re-rasterizes at the instance size"
    );
}

// ─────────────────────── visual dump (opt-in) ───────────────────────
//
// GUI_DUMP=1 cargo test -p gui-app visual_dump
// Rasterizes representative UI states to /tmp/gui-dump/*.png with the
// headless software renderer (`screenshot.rs`) — no GPU/X11 needed.

#[test]
fn visual_dump() {
    if std::env::var("GUI_DUMP").is_err() {
        return; // opt-in: keep plain `cargo test` from writing files
    }
    let dir = std::path::PathBuf::from("/tmp/gui-dump");
    std::fs::create_dir_all(&dir).unwrap();

    let shot = |ctx: &egui::Context, out: &egui::FullOutput, name: &str| {
        crate::screenshot::capture_to_png(ctx, out, WIN_W as u32, WIN_H as u32, &dir.join(name))
            .unwrap_or_else(|e| panic!("rasterize {name}: {e}"));
    };

    let load_fixture = |name: &str| -> DrawingApp {
        let mut app = DrawingApp::new(Services::native());
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{name}"));
        match app.open(&path) {
            crate::fileio::OpenOutcome::Loaded { .. } => app,
            other => panic!(
                "fixture {name:?} failed to load: {}",
                crate::fileio::open_message(&other)
            ),
        }
    };

    // 1. Empty canvas.
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let out = gui.frame();
    shot(&gui.ctx, &out, "01-empty.png");

    // 2. Text fixture (shape + single/multi-line text).
    {
        let mut gui = Gui::new(load_fixture("text-sample.ad"));
        let out = gui.frame();
        shot(&gui.ctx, &out, "02-text-sample.png");
    }

    // 3. Two boxes with a connector.
    {
        let mut gui = Gui::new(load_fixture("two-boxes-connector.ad"));
        let out = gui.frame();
        shot(&gui.ctx, &out, "03-two-boxes.png");
    }

    // 4. Box selected: outline, anchor dots, resize handle.
    {
        let mut gui = Gui::new(DrawingApp::new(Services::native()));
        let min = gui.canvas_min();
        gui.place_box_via_ui(min, 2, 1);
        gui.select_first_box(min);
        let out = gui.frame();
        shot(&gui.ctx, &out, "04-selected.png");
    }

    // 5. Connect drag in progress (line from anchor to cursor).
    {
        let mut gui = Gui::new(DrawingApp::new(Services::native()));
        let min = gui.canvas_min();
        gui.place_box_via_ui(min, 2, 1);
        gui.select_first_box(min);
        let (x, y, w, h) = gui.first_box();
        let e_anchor = gui.cell_center(min, x + w - 1, y + h / 2);
        let cursor = gui.cell_center(min, x + w + 4, y);
        let out = gui.press_and_hold_to(e_anchor, cursor);
        shot(&gui.ctx, &out, "05-connect-drag.png");
    }

    // 6. Template editor window open (name typed, pattern cells painted).
    {
        let mut gui = Gui::new(DrawingApp::new(Services::native()));
        let out = gui.frame();
        let new_btn = find_text(&out, "New template").expect("sidebar has '＋ New template…'");
        let _ = gui.click_at(new_btn.center());
        assert!(gui.app.template_editor_name().is_some(), "editor opened");
        // Type a name and paint a few pattern cells so the preview shows.
        let out = gui.frame();
        let name_label = find_text_exact(&out, "Name:").expect("editor shows Name field");
        let _ = gui.click_at(Pos2::new(name_label.max.x + 10.0, name_label.center().y));
        let _ = gui.type_text("star");
        let out = gui.frame();
        shot(&gui.ctx, &out, "06-template-editor.png");
    }

    // 7. Path prompt open with a typed path.
    {
        let mut gui = Gui::new(DrawingApp::new(Services::native()));
        let _ = gui.open_prompt("Open…");
        let _ = gui.type_text("/tmp/diagrams/next.ad");
        let out = gui.frame();
        shot(&gui.ctx, &out, "07-path-prompt.png");
    }

    // 8. Inline text editing (floating editor over the object).
    {
        let mut gui = Gui::new(DrawingApp::new(Services::native()));
        let min = gui.canvas_min();
        place_text_object(&mut gui, min);
        let out = gui.frame();
        shot(&gui.ctx, &out, "08-inline-edit.png");
    }

    // 9. File menu open (FR6 / CR-002): the grouped dropdown over the canvas.
    {
        let mut gui = Gui::new(DrawingApp::new(Services::native()));
        let _ = gui.click_topbar_button("File ▾");
        let out = gui.frame();
        shot(&gui.ctx, &out, "09-file-menu.png");
    }

    // 10. Material dark palette (FR14 / CR-006): app bar + themed chrome.
    {
        let mut app = DrawingApp::with_config(
            Services::native(),
            crate::config::AppConfig {
                theme: Theme::Dark,
                ..Default::default()
            },
            None,
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/two-boxes-connector.ad");
        match app.open(&path) {
            crate::fileio::OpenOutcome::Loaded { .. } => {}
            other => panic!(
                "fixture failed to load: {}",
                crate::fileio::open_message(&other)
            ),
        }
        let mut gui = Gui::new(app);
        let out = gui.frame();
        shot(&gui.ctx, &out, "10-material-dark.png");
    }

    // 11. Material light palette: same fixture under Light (AC2/AC5 — only the
    // chrome re-colors, the canvas pixels are identical to shot 10's).
    {
        let mut app = DrawingApp::with_config(
            Services::native(),
            crate::config::AppConfig {
                theme: Theme::Light,
                ..Default::default()
            },
            None,
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/two-boxes-connector.ad");
        match app.open(&path) {
            crate::fileio::OpenOutcome::Loaded { .. } => {}
            other => panic!(
                "fixture failed to load: {}",
                crate::fileio::open_message(&other)
            ),
        }
        let mut gui = Gui::new(app);
        let out = gui.frame();
        shot(&gui.ctx, &out, "11-material-light.png");
    }

    // 12. Diagram colors dialog: all roles at the medium default (FR15 / CR-009).
    {
        let mut gui = Gui::new(DrawingApp::new(Services::native()));
        let out = gui.open_color_dialog();
        shot(&gui.ctx, &out, "12-color-dialog-default.png");
    }

    // 13. Diagram colors dialog with two roles set — swatches + recent strip.
    {
        let mut gui = Gui::new(DrawingApp::new(Services::native()));
        let out = gui.open_color_dialog();
        gui.click_color_row_field(&out, 1);
        let _ = gui.type_text("#ff0000");
        let out = gui.frame();
        let out = gui.click_color_row_action(&out, 1, "Set");
        gui.click_color_row_field(&out, 0);
        let _ = gui.type_text("#3c6ef5");
        let out = gui.frame();
        let out = gui.click_color_row_action(&out, 0, "Set");
        shot(&gui.ctx, &out, "13-color-dialog-filled.png");
    }

    // 14. Multi-selection (S-035 FR20): two boxes selected via Ctrl+click — one
    // outline per member, anchor dots on both, no resize handle.
    {
        let mut gui = Gui::new(DrawingApp::new(Services::native()));
        let min = gui.canvas_min();
        gui.place_box_via_ui(min, 2, 2);
        let out = gui.frame();
        let label = find_text_exact(&out, "box").expect("sidebar lists the box template");
        let _ = gui.click_at(label.center());
        let _ = gui.click_at(gui.cell_center(min, 15, 2));
        let out = gui.ctrl_click_at(gui.cell_center(min, 6, 4));
        shot(&gui.ctx, &out, "14-multi-select.png");
    }

    eprintln!("visual dump written to {dir:?}");
}

#[test]
fn dbg_layout_dump() {
    if std::env::var("GUI_DEBUG_LAYOUT").is_err() {
        return;
    }
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    if std::env::var("GUI_DEBUG_LAYOUT_PROMPT").is_ok() {
        let _ = gui.click_menu_item("File ▾", "Save As…");
    }
    let out = gui.frame();
    println!(
        "=== galleys ({} state) ===",
        if std::env::var("GUI_DEBUG_LAYOUT_PROMPT").is_ok() {
            "prompt"
        } else {
            "empty"
        }
    );
    for (t, r) in texts(&out) {
        println!(
            "text {:?} rect=({:.0},{:.0})-({:.0},{:.0})",
            t, r.min.x, r.min.y, r.max.x, r.max.y
        );
    }
    println!("=== filled rects ===");
    for (r, c) in filled_rects(&out) {
        println!(
            "fill ({:.0},{:.0})-({:.0},{:.0}) color={c:?}",
            r.min.x, r.min.y, r.max.x, r.max.y
        );
    }
    println!("=== stroked rects ===");
    for (r, c) in stroked_rects(&out) {
        println!(
            "stroke ({:.0},{:.0})-({:.0},{:.0}) color={c:?}",
            r.min.x, r.min.y, r.max.x, r.max.y
        );
    }
}

// ─────────────────────── B-002: scale control ───────────────────────

/// All text galleys drawn inside the canvas area — for a non-empty diagram these
/// are exactly the diagram's rows (grid lines are line segments; selection/anchor
/// overlays are strokes and fills, never text). The canvas bounds come from the
/// outermost grid lines, which span the whole canvas rect.
fn canvas_lines(out: &egui::FullOutput) -> Vec<String> {
    let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);
    for (a, b) in line_points(out) {
        if (a.x - b.x).abs() < 0.5 && a.x > SIDEBAR {
            min_x = min_x.min(a.x); // vertical grid lines span the canvas height
            max_x = max_x.max(a.x);
        }
        if (a.y - b.y).abs() < 0.5 && a.x > SIDEBAR && b.x > SIDEBAR {
            min_y = min_y.min(a.y); // horizontal grid lines span the canvas width
            max_y = max_y.max(a.y);
        }
    }
    assert!(
        min_x.is_finite() && max_x.is_finite() && min_y.is_finite() && max_y.is_finite(),
        "canvas draws grid lines"
    );
    texts(out)
        .into_iter()
        .filter(|(_, r)| {
            r.min.x >= min_x - 1.0
                && r.min.y >= min_y - 1.0
                && r.max.x <= max_x + 1.0
                && r.min.y < max_y - 1.0
        })
        .map(|(t, _)| t)
        .collect()
}

/// FR8 expansion at text level: each glyph becomes an `s × s` block; trailing
/// whitespace is re-trimmed per line (the `to_text` rule).
fn expand_lines(lines: &[String], s: u32) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len() * s as usize);
    for line in lines {
        for _ in 0..s {
            let row: String = line
                .chars()
                .flat_map(|c| std::iter::repeat_n(c, s as usize))
                .collect();
            out.push(row.trim_end().to_owned());
        }
    }
    out
}

/// B-002: pressing the toolbar "+" scale control must render each shape exactly
/// once — scaled (FR8), with no unscaled remnant on the canvas — and every press
/// must register. Driven through the real UI: place an ellipse, then three "+"
/// clicks; after each click the drawn rows must equal the FR8 expansion of the
/// ×1 baseline.
#[test]
fn b002_scale_presses_render_exactly_one_expanded_copy() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min(); // measured while empty — the documented contract
    // Place one ellipse via the UI: sidebar label, then a canvas cell.
    let out = gui.frame();
    let label = find_text_exact(&out, "ellipse").expect("sidebar lists the ellipse template");
    let _ = gui.click_at(label.center());
    let _ = gui.click_at(gui.cell_center(min, 4, 2));
    assert_eq!(
        gui.object_count(),
        1,
        "clicking the canvas placed one ellipse"
    );

    // Deselect: anchor arrows are drawn as text on the selection (CR-008) and must
    // not pollute the baseline rows.
    let _ = gui.click_at(gui.cell_center(min, 30, 20));

    // Baseline: the exact rows drawn at scale 1.
    let baseline = canvas_lines(&gui.frame());
    assert_eq!(baseline.len(), 5, "the ellipse template is 5 rows tall");

    // Three "+" presses (B-002 steps 2–3): each must register and the canvas must
    // show exactly one scaled copy.
    for (press, label) in ["Scale ×2", "Scale ×3", "Scale ×4"].into_iter().enumerate() {
        let out = gui.click_topbar_button("+");
        assert!(
            find_text_exact(&out, label).is_some(),
            "toolbar shows {label} after press {}; texts: {:?}",
            press + 1,
            texts(&out)
        );
        let s = (label
            .chars()
            .next_back()
            .expect("scale label ends in a digit") as u32)
            - ('0' as u32);
        let lines = canvas_lines(&gui.frame());
        assert_eq!(
            lines,
            expand_lines(&baseline, s),
            "canvas at {label} is exactly the FR8 expansion of the ×1 rows"
        );
    }
}

// ─────────────────────────── panning (S-022) ───────────────────────────

/// Middle-button press at `pos` (with the pointer already there).
fn middle_press(pos: Pos2) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos),
        Event::PointerButton {
            pos,
            button: PointerButton::Middle,
            pressed: true,
            modifiers: Modifiers::default(),
        },
    ]
}

/// Middle-button release at `pos`.
fn middle_release(pos: Pos2) -> Vec<Event> {
    vec![Event::PointerButton {
        pos,
        button: PointerButton::Middle,
        pressed: false,
        modifiers: Modifiers::default(),
    }]
}

/// The drawn rect of the placed box's top border row inside the canvas area.
/// (The built-in box template uses ASCII corners: `+-------+`.)
fn box_top_row(out: &egui::FullOutput) -> Rect {
    texts(out)
        .into_iter()
        .find(|(t, r)| t.contains("+-------+") && r.min.x > SIDEBAR)
        .map(|(_, r)| r)
        .expect("placed box draws its top border in the canvas")
}

/// One scroll frame at `pos`. Point-unit deltas below 8 px are applied in full on
/// the event's frame by egui (no smoothing tail), so these pan deterministically.
fn scroll_at(pos: Pos2, delta: Vec2) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: Modifiers::default(),
        },
    ]
}

/// Scrolls the template sidebar down until `label` is laid out *fully above the
/// bottom status bar*, returning that frame. egui culls layout widgets outside
/// the sidebar's clip rect, and with the full built-in catalog (CR-015) collection
/// groups sit below the 768 px fold — a real user reaches them by scrolling, and so
/// do these tests. A label merely peeking above the status bar is not clickable
/// (the panel owns that strip), so the row must clear it with margin. Point-unit
/// deltas below 8 px apply in full on their own frame (no smoothing tail), so the
/// scroll is deterministic and leaves no pending wheel to re-pan the canvas later.
fn scroll_sidebar_until(gui: &mut Gui, label: &str) -> egui::FullOutput {
    let over_sidebar = Pos2::new(SIDEBAR / 2.0, 400.0);
    let safe_bottom = WIN_H - 60.0;
    let mut out = gui.frame();
    for _ in 0..600 {
        if find_text_exact(&out, label).is_some_and(|r| r.max.y <= safe_bottom) {
            return out;
        }
        out = gui.pump(scroll_at(over_sidebar, Vec2::new(0.0, -4.0)));
    }
    panic!("sidebar never scrolled [{label}] into view within 600 wheel frames")
}

#[test]
fn middle_drag_pans_the_canvas_and_hit_testing_follows() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 6, 3);

    // Baseline: where the box's top border row is drawn (LEFT_TOP text anchor).
    let before = box_top_row(&gui.frame());

    // Middle-drag left/up by (60, 30) over two move frames.
    let _ = gui.pump(middle_press(Pos2::new(560.0, 290.0)));
    let _ = gui.pump(vec![Event::PointerMoved(Pos2::new(530.0, 275.0))]);
    let _ = gui.pump(vec![Event::PointerMoved(Pos2::new(500.0, 260.0))]);
    let _ = gui.pump(middle_release(Pos2::new(500.0, 260.0)));
    assert_eq!(
        gui.app.pan(),
        Vec2::new(60.0, 30.0),
        "middle-drag left/up pans the view right/down"
    );

    // The whole canvas content shifted by exactly the drag delta.
    let after = box_top_row(&gui.frame());
    assert!(
        close(before.min.x - after.min.x, 60.0, 0.5),
        "content moved left by the drag: {before:?} -> {after:?}"
    );
    assert!(
        close(before.min.y - after.min.y, 30.0, 0.5),
        "content moved up by the drag: {before:?} -> {after:?}"
    );

    // Hit-testing follows the panned origin: clicking where the box body is NOW
    // (same model cell as before the pan) selects it.
    let (x, y, w, h) = gui.first_box();
    let (cw, ch) = gui.cell_metrics();
    let body_now = Pos2::new(
        min.x + MARGIN - 60.0 + (x + w / 2) as f32 * cw + cw / 2.0,
        min.y + MARGIN - 30.0 + (y + h / 2) as f32 * ch + ch / 2.0,
    );
    let out = gui.click_at(body_now);
    assert!(
        stroked_rects(&out).iter().any(|(_, c)| *c == SELECTION),
        "clicking the panned body position selects the box; rects: {:?}",
        stroked_rects(&out)
    );
}

#[test]
fn scroll_pans_the_canvas_but_not_over_the_sidebar() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 6, 3);
    let before = box_top_row(&gui.frame());

    // 15 × −4 px point deltas over the canvas: content moves up by 60 px (egui's
    // delta says how the CONTENT should move).
    let over_canvas = Pos2::new(min.x + 400.0, min.y + 300.0);
    for _ in 0..15 {
        let _ = gui.pump(scroll_at(over_canvas, Vec2::new(0.0, -4.0)));
    }
    assert_eq!(
        gui.app.pan(),
        Vec2::new(0.0, 60.0),
        "scrolling over the canvas pans the view"
    );
    let after = box_top_row(&gui.frame());
    assert!(
        close(before.min.y - after.min.y, 60.0, 0.5),
        "content moved up: {before:?} -> {after:?}"
    );

    // The same scroll over the sidebar must not pan the canvas.
    let over_sidebar = Pos2::new(80.0, 300.0);
    for _ in 0..5 {
        let _ = gui.pump(scroll_at(over_sidebar, Vec2::new(0.0, -4.0)));
    }
    assert_eq!(
        gui.app.pan(),
        Vec2::new(0.0, 60.0),
        "scrolling over the sidebar does not pan"
    );
}

#[test]
fn pan_is_clamped_at_the_model_origin() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    gui.frame();

    // Middle-drag right/down: the view tries to pan past cell 0 — clamped at zero.
    let _ = gui.pump(middle_press(Pos2::new(400.0, 300.0)));
    let _ = gui.pump(vec![Event::PointerMoved(Pos2::new(600.0, 400.0))]);
    let _ = gui.pump(middle_release(Pos2::new(600.0, 400.0)));
    assert_eq!(
        gui.app.pan(),
        Vec2::ZERO,
        "panning past the model origin clamps at zero"
    );
}

#[test]
fn opening_a_document_resets_the_pan() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 6, 3);

    // Pan first.
    let _ = gui.pump(middle_press(Pos2::new(500.0, 300.0)));
    let _ = gui.pump(vec![Event::PointerMoved(Pos2::new(400.0, 250.0))]);
    let _ = gui.pump(middle_release(Pos2::new(400.0, 250.0)));
    assert_eq!(
        gui.app.pan(),
        Vec2::new(100.0, 50.0),
        "panned before opening"
    );

    // Opening a real document replaces the model and returns the view to the origin.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/one-box.ad");
    let outcome = gui.app.open(&path);
    assert!(
        matches!(outcome, crate::fileio::OpenOutcome::Loaded { .. }),
        "fixture loads: {outcome:?}"
    );
    assert_eq!(
        gui.app.pan(),
        Vec2::ZERO,
        "opening a document resets the pan"
    );
}

// ─────────────────── menu bar (FR6 / CR-002, S-025) ───────────────────

/// AC1 — the top bar is a menu bar: File ▾ and Options ▾ plus the Scale group; no
/// flat action buttons remain while the menus are closed.
#[test]
fn toolbar_is_a_menu_bar_without_flat_action_buttons() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let out = gui.frame();
    for label in ["File ▾", "Options ▾", "Scale ×1"] {
        assert!(
            find_text_exact(&out, label).is_some(),
            "top bar has {label:?}"
        );
    }
    // The one-shot actions live inside the dropdowns, not on the bar itself.
    for label in ["Open…", "Save As…", "Copy", "Export…"] {
        assert!(
            find_text_exact(&out, label).is_none(),
            "{label:?} is a menu item, not a flat button"
        );
    }
}

/// AC1/AC3 (S-039 / CR-001) — opening File ▾ lists the file actions top-level in
/// group order: file ops | clipboard | one Export… entry. The old Output and
/// Export submenus are gone (their only surviving members were flattened), and
/// `Text .txt` no longer exists as an item (its function is inside Save As…).
#[test]
fn file_menu_lists_the_grouped_actions_flat() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.click_topbar_button("File ▾");
    let out = gui.frame();

    // Top level: file ops, clipboard, export — in group order.
    let mut last_y = f32::MIN;
    for label in ["Open…", "Save", "Save As…", "Copy", "Export…"] {
        let rect = find_text_exact(&out, label)
            .unwrap_or_else(|| panic!("the File menu shows {label:?}; texts: {:?}", texts(&out)));
        assert!(rect.min.y >= last_y, "{label:?} keeps group order");
        last_y = rect.min.y;
    }
    // The submenu entries and the removed/absorbed items are gone.
    for label in [
        "Output",
        "Export",
        "Text .txt",
        "Export SVG…",
        "Export PNG…",
    ] {
        assert!(
            find_text_exact(&out, label).is_none(),
            "{label:?} is no longer a File menu entry"
        );
    }
}

/// AC2/AC3 — selecting a File item performs the action and closes the menu.
/// S-039 / CR-001 AC5 — with no dialog available, `Save As…` opens the in-app
/// format choice; picking the document option then opens the save-as prompt.
#[test]
fn file_menu_item_performs_and_closes_the_menu() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let out = gui.click_menu_item("File ▾", "Save As…");
    // No system dialog in the test harness → the format choice window is open.
    assert!(find_text_exact(&out, "What do you want to save?").is_some());
    assert_eq!(gui.app.path_prompt(), None);
    let out = gui.click_window_button("Diagram document (.ad)");
    assert_eq!(gui.app.path_prompt(), Some(PathPrompt::SaveAs));
    // The menu closed: its items are gone from the frame (the prompt is open instead).
    for label in ["Open…", "Copy", "Export…"] {
        assert!(find_text_exact(&out, label).is_none(), "{label:?} is gone");
    }
}

/// AC3 — Escape closes the File menu without performing anything.
#[test]
fn escape_closes_the_file_menu_without_performing() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.click_topbar_button("File ▾");
    assert!(
        find_text_exact(&gui.frame(), "Open…").is_some(),
        "menu is open"
    );

    let _ = gui.key(Key::Escape);
    let out = gui.frame();
    assert!(gui.app.path_prompt().is_none(), "nothing was performed");
    assert!(find_text_exact(&out, "Open…").is_none(), "the menu closed");
}

/// AC2 — while a PNG export is in flight, the Export… item is disabled
/// (clicking it does nothing); once the job completes the item is offered again.
/// The in-flight state is driven deterministically through the S-015 test seam:
/// a held reply channel keeps the job Pending until the test releases it.
#[test]
fn export_png_item_is_disabled_while_an_export_runs() {
    let tmp = tempfile::tempdir().unwrap();
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.canvas_min();

    // In-flight job: the sender is held, so every tick reports Pending.
    let (tx, rx) = std::sync::mpsc::channel::<Result<Vec<u8>, ui_api::ExportError>>();
    gui.app.set_inflight_png_export(
        core_services::PngJob::from_reply(rx),
        tmp.path().join("held.png"),
    );

    // S-039 — the single Export… item is listed but disabled while a job runs:
    // clicking it does nothing.
    let _ = gui.click_topbar_button("File ▾");
    let out = gui.frame();
    let item = find_text_exact(&out, "Export…").expect("the Export… item is listed");
    let _ = gui.click_at(item.center());
    assert!(
        gui.app.path_prompt().is_none(),
        "no new export prompt while one is running"
    );

    // Release the job: the next frame ticks it to Done, writes the bytes and clears
    // the exporting state — after which the item opens a prompt again.
    let _ = tx.send(Ok(vec![0x89, b'P', b'N', b'G']));
    let out = gui.frame();
    assert!(
        !gui.app.png_export_inflight(),
        "the job was ticked to completion"
    );
    assert!(
        tmp.path().join("held.png").exists(),
        "the job's bytes were written"
    );
    assert!(
        find_text(&out, "Exporting").is_none(),
        "the exporting state is cleared"
    );
    assert_eq!(gui.open_prompt("Export PNG…"), PathPrompt::ExportPng);
}

// ─────────────── system file dialogs (S-039 / CR-001) ───────────────

/// AC1 — Open… with a dialog: the chosen path goes through the existing open flow.
#[test]
fn open_dialog_chosen_path_replaces_the_model() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/one-box.ad");
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 1, 1);
    gui.queue_dialog(DialogResult::Chosen(fixture.clone()));

    let _ = gui.click_menu_item("File ▾", "Open…");

    assert_eq!(
        gui.object_count(),
        1,
        "the fixture's box replaced the model"
    );
    assert_eq!(gui.app.current_file().unwrap(), fixture.as_path());
    assert_eq!(gui.dialog_calls(), vec![(DialogKind::Open, None)]);
}

/// AC1 — Open… cancelled: nothing changes and no fallback prompt appears.
#[test]
fn open_dialog_cancelled_changes_nothing() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 1, 1);
    gui.queue_dialog(DialogResult::Cancelled);

    let _ = gui.click_menu_item("File ▾", "Open…");

    assert_eq!(gui.object_count(), 1, "the model is unchanged");
    assert!(gui.app.current_file().is_none());
    assert!(
        gui.app.path_prompt().is_none(),
        "no fallback prompt on cancel"
    );
}

/// AC2 — Save with a dialog: prefilled with the current file; the chosen path is
/// written through the existing save flow.
#[test]
fn save_dialog_is_prefilled_and_writes_the_chosen_path() {
    let tmp = tempfile::tempdir().unwrap();
    let ad_path = tmp.path().join("pic.ad");
    std::fs::write(&ad_path, "ad 1.0\nshape box \"A\" as s1 at 1 1 w 9 h 5\n").unwrap();

    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.app.open(&ad_path);
    let new_path = tmp.path().join("pic-renamed.ad");
    gui.queue_dialog(DialogResult::Chosen(new_path.clone()));

    let out = gui.click_menu_item("File ▾", "Save");

    assert!(new_path.exists(), "the chosen path was written");
    assert_eq!(gui.app.current_file().unwrap(), &new_path);
    // The dialog was prefilled with the current file (name + directory).
    assert_eq!(gui.dialog_calls(), vec![(DialogKind::Save, Some(ad_path))]);
    assert_status(&out, "Saved");
}

/// AC2 — Save cancelled: the document is untouched.
#[test]
fn save_dialog_cancelled_keeps_the_document() {
    let tmp = tempfile::tempdir().unwrap();
    let ad_path = tmp.path().join("pic.ad");
    std::fs::write(&ad_path, "ad 1.0\nshape box \"A\" as s1 at 1 1 w 9 h 5\n").unwrap();

    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.app.open(&ad_path);
    gui.queue_dialog(DialogResult::Cancelled);

    let out = gui.click_menu_item("File ▾", "Save");

    assert_eq!(
        gui.app.current_file().unwrap(),
        &ad_path,
        "still the old file"
    );
    assert_status(&out, "Save cancelled");
}

/// AC3 — Save As… with a dialog: a `.txt` choice writes exactly the canvas bytes
/// and does not touch the current document.
#[test]
fn save_as_dialog_txt_writes_text_output_without_touching_the_document() {
    let tmp = tempfile::tempdir().unwrap();
    let ad_path = tmp.path().join("pic.ad");
    std::fs::write(&ad_path, "ad 1.0\nshape box \"A\" as s1 at 1 1 w 9 h 5\n").unwrap();

    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.app.open(&ad_path);
    let expected = gui.app.canvas_frame().expect("canvas renders").0;
    let txt_path = tmp.path().join("out.txt");
    gui.queue_dialog(DialogResult::Chosen(txt_path.clone()));

    let out = gui.click_menu_item("File ▾", "Save As…");

    assert_eq!(
        std::fs::read_to_string(&txt_path).unwrap_or_else(|e| panic!("no txt: {e}")),
        expected,
        "the .txt holds exactly the canvas bytes"
    );
    assert_eq!(
        gui.app.current_file().unwrap(),
        &ad_path,
        "the document is untouched"
    );
    assert_status(&out, "Text output saved");
}

/// AC3 — Save As… with a dialog: a `.ad` choice runs the save-as flow (new current file).
#[test]
fn save_as_dialog_ad_runs_the_save_as_flow() {
    let tmp = tempfile::tempdir().unwrap();
    let ad_path = tmp.path().join("pic.ad");
    std::fs::write(&ad_path, "ad 1.0\nshape box \"A\" as s1 at 1 1 w 9 h 5\n").unwrap();

    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.app.open(&ad_path);
    let new_path = tmp.path().join("copy.ad");
    gui.queue_dialog(DialogResult::Chosen(new_path.clone()));

    let out = gui.click_menu_item("File ▾", "Save As…");

    assert!(
        new_path.exists(),
        "the document was saved to the chosen path"
    );
    assert_eq!(
        gui.app.current_file().unwrap(),
        &new_path,
        "the current file moved"
    );
    assert_status(&out, "Saved");
}

/// AC4 — Export… with a dialog: the `.svg` choice writes the exporter bytes.
#[test]
fn export_dialog_svg_writes_the_exporter_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let ad_path = tmp.path().join("pic.ad");
    std::fs::write(&ad_path, "ad 1.0\nshape box \"A\" as s1 at 1 1 w 9 h 5\n").unwrap();

    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.app.open(&ad_path);
    let svg_path = tmp.path().join("out.svg");
    gui.queue_dialog(DialogResult::Chosen(svg_path.clone()));

    let out = gui.click_menu_item("File ▾", "Export…");

    let svg = std::fs::read_to_string(&svg_path).unwrap_or_else(|e| panic!("no svg: {e}"));
    assert!(svg.starts_with("<svg"), "valid SVG output");
    assert_status(&out, "SVG saved");
}

/// AC4 — Export… with a dialog: the `.png` choice starts the non-blocking worker.
#[test]
fn export_dialog_png_runs_the_non_blocking_worker() {
    let tmp = tempfile::tempdir().unwrap();
    let ad_path = tmp.path().join("pic.ad");
    std::fs::write(&ad_path, "ad 1.0\nshape box \"A\" as s1 at 1 1 w 9 h 5\n").unwrap();

    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.app.open(&ad_path);
    let png_path = tmp.path().join("out.png");
    gui.queue_dialog(DialogResult::Chosen(png_path.clone()));

    let _ = gui.click_menu_item("File ▾", "Export…");
    assert!(gui.app.png_export_inflight(), "the PNG job is in flight");

    // Pump until the worker completes (same loop as the S-015 test).
    for _ in 0..600 {
        if !gui.app.png_export_inflight() {
            break;
        }
        let _ = gui.frame();
    }
    let bytes = std::fs::read(&png_path).unwrap_or_else(|e| panic!("no png: {e}"));
    assert_eq!(&bytes[0..4], &[0x89, b'P', b'N', b'G'], "PNG magic bytes");
}

/// AC5 — fallback (no dialog): Save As… offers both options; the `.txt` choice
/// opens the prefilled text-output prompt and OK writes exactly the canvas bytes.
#[test]
fn fallback_save_as_offers_both_options_and_txt_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let ad_path = tmp.path().join("pic.ad");
    std::fs::write(&ad_path, "ad 1.0\nshape box \"A\" as s1 at 1 1 w 9 h 5\n").unwrap();

    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.app.open(&ad_path);
    let expected = gui.app.canvas_frame().expect("canvas renders").0;

    // No queued result → the fake reports Unavailable → the format choice shows.
    let out = gui.click_menu_item("File ▾", "Save As…");
    assert!(find_text_exact(&out, "Diagram document (.ad)").is_some());
    assert!(find_text_exact(&out, "Text output (.txt)").is_some());

    let _ = gui.click_window_button("Text output (.txt)");
    assert_eq!(gui.app.path_prompt(), Some(PathPrompt::SaveAsText));
    // Prefilled with the sibling .txt of the current file.
    assert_eq!(
        gui.app.path_prompt_value().as_deref(),
        Some(tmp.path().join("pic.txt").to_str().unwrap()),
        "the text prompt is prefilled next to the document"
    );
    let out = gui.click_ok();

    let txt_path = tmp.path().join("pic.txt");
    assert_eq!(
        std::fs::read_to_string(&txt_path).unwrap_or_else(|e| panic!("no txt: {e}")),
        expected,
        "the .txt holds exactly the canvas bytes"
    );
    assert_status(&out, "Text output saved");
}

/// AC5 — fallback (no dialog): Export… offers both formats; each choice opens its
/// prompt with the sibling prefill.
#[test]
fn fallback_export_offers_both_formats_with_prefill() {
    let tmp = tempfile::tempdir().unwrap();
    let ad_path = tmp.path().join("pic.ad");
    std::fs::write(&ad_path, "ad 1.0\nshape box \"A\" as s1 at 1 1 w 9 h 5\n").unwrap();

    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.app.open(&ad_path);

    let out = gui.click_menu_item("File ▾", "Export…");
    assert!(find_text_exact(&out, "SVG image (.svg)").is_some());
    assert!(find_text_exact(&out, "PNG image (.png)").is_some());

    let _ = gui.click_window_button("SVG image (.svg)");
    assert_eq!(gui.app.path_prompt(), Some(PathPrompt::ExportSvg));
    assert_eq!(
        gui.app.path_prompt_value().as_deref(),
        Some(tmp.path().join("pic.svg").to_str().unwrap()),
        "the svg prompt is prefilled next to the document"
    );
    let _ = gui.key(Key::Escape);

    // Second round: PNG.
    let out = gui.click_menu_item("File ▾", "Export…");
    let _ = gui.click_window_button("PNG image (.png)");
    assert_eq!(gui.app.path_prompt(), Some(PathPrompt::ExportPng));
    assert_eq!(
        gui.app.path_prompt_value().as_deref(),
        Some(tmp.path().join("pic.png").to_str().unwrap()),
        "the png prompt is prefilled next to the document"
    );
    let _ = gui.key(Key::Escape);
    assert!(gui.app.path_prompt().is_none());
    let _ = out;
}

// ─────────────── options menu + theme persistence (FR13 / CR-005, S-023) ───────────────

/// AC1/AC3 — the Options menu lists all three themes; selecting one applies it live
/// and persists it to the app config file.
#[test]
fn options_menu_selects_light_theme_and_persists_it() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    let mut gui = Gui::new(DrawingApp::with_config(
        Services::native(),
        crate::config::AppConfig::default(),
        Some(config_path.clone()),
    ));

    // Open the Options menu (top-bar button).
    let _ = gui.click_topbar_button("Options ▾");

    // The open menu lists all three theme choices; select Light.
    let out = gui.frame();
    for label in ["Light", "Dark", "Native (system)"] {
        assert!(
            find_text_exact(&out, label).is_some(),
            "menu lists {label:?}"
        );
    }
    let light = find_text_exact(&out, "Light").expect("the Light choice");
    let _ = gui.click_at(light.center());

    // The selection applies live (the context switches to light) and is recorded in
    // the app state…
    assert_eq!(
        gui.app.config().theme,
        Theme::Light,
        "state holds the choice"
    );
    assert_eq!(gui.ctx.theme(), egui::Theme::Light, "light theme is active");
    // …and is persisted to the config file.
    let saved = std::fs::read_to_string(&config_path).expect("the config file was written");
    assert!(
        saved.contains(r#""theme": "light""#),
        "file holds the theme: {saved}"
    );
    assert_eq!(gui.app.status, "Theme: light (saved)");
}

/// AC2 — a theme stored in the config file is applied at startup, before any click.
#[test]
fn the_stored_theme_is_applied_at_startup() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    std::fs::write(&config_path, r#"{"theme": "light"}"#).unwrap();

    let mut gui = Gui::new(DrawingApp::with_config(
        Services::native(),
        crate::config::AppConfig::load(Some(&config_path)),
        Some(config_path),
    ));
    assert_eq!(gui.app.config().theme, Theme::Light, "the file was read");
    let _ = gui.frame(); // first frame
    assert_eq!(
        gui.ctx.theme(),
        egui::Theme::Light,
        "stored light theme is active before any click"
    );
}

/// AC1 — the default theme is native; in the headless harness no system appearance
/// is fed through `RawInput::system_theme`, so it resolves to egui's dark fallback
/// (CR-005's allowed fallback).
#[test]
fn the_default_native_theme_renders_dark() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    assert_eq!(gui.app.config().theme, Theme::Native);
    let _ = gui.frame();
    assert_eq!(
        gui.ctx.theme(),
        egui::Theme::Dark,
        "native falls back to dark without a system theme"
    );
}

// ── Material app bar & window controls (S-027 / CR-006 / ADR 017) ───────────

/// The viewport commands queued for the root window in this frame's output.
/// egui drains them at the end of each pass, so a command is visible only in
/// the frame it was sent on — which is exactly what makes the one-shot drag
/// rule testable.
fn window_commands(out: &egui::FullOutput) -> Vec<egui::ViewportCommand> {
    out.viewport_output
        .get(&egui::viewport::ViewportId::ROOT)
        .map(|vo| vo.commands.clone())
        .unwrap_or_default()
}

/// The bundled monospace font must cover the window-control glyphs, or the app
/// bar would draw tofu boxes for minimize / fullscreen / close (the proportional
/// font genuinely lacks them — that is why the controls are monospace by design,
/// see `appbar::window_controls`).
#[test]
fn window_control_glyphs_are_in_the_font() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.frame(); // build the font atlas
    let font_id = egui::FontId::monospace(14.0);
    gui.ctx.fonts_mut(|fonts| {
        for c in ['─', '□', '×'] {
            assert!(
                fonts.has_glyph(&font_id, c),
                "window control glyph '{c}' missing from the bundled monospace font"
            );
        }
    });
}

/// Regression (user report: “grabbing the window never undrags it”): the app bar
/// must queue `StartDrag` exactly once per drag — on the press frame. The old code
/// sent it on every frame while `dragged()`, and egui-winit re-invokes winit's
/// `drag_window()` for each command; on X11 those pointer grabs stack, so the
/// window keeps following the cursor after release.
#[test]
fn app_bar_drag_queues_start_drag_exactly_once() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    // A point in the app bar strip that is over neither the title nor a control.
    let pos = Pos2::new(500.0, 20.0);

    let count = |out: &egui::FullOutput| -> usize {
        window_commands(out)
            .iter()
            .filter(|c| matches!(c, egui::ViewportCommand::StartDrag))
            .count()
    };

    // Hovering queues no drag/resize command (the first frame may carry the
    // one-shot `SetTheme`, which is unrelated to dragging).
    let out = gui.pump(vec![Event::PointerMoved(pos)]);
    assert!(
        !window_commands(&out).iter().any(|c| matches!(
            c,
            egui::ViewportCommand::StartDrag | egui::ViewportCommand::BeginResize(_)
        )),
        "hovering must not queue a drag/resize command"
    );

    // Press: the drag region senses only drags, so this frame starts the drag —
    // and must send the one `StartDrag`.
    let out = gui.pump(Gui::press_at(pos));
    assert_eq!(
        count(&out),
        1,
        "the press frame queues exactly one StartDrag"
    );

    // Every following frame of the gesture: zero more (this is what the old code
    // broke — one per frame).
    for dx in [4.0, 8.0, 12.0] {
        let out = gui.pump(vec![Event::PointerMoved(Pos2::new(pos.x + dx, pos.y))]);
        assert_eq!(
            count(&out),
            0,
            "a frame while dragging must not re-queue StartDrag"
        );
    }

    // Release: the drag ends; nothing is queued.
    let out = gui.pump(Gui::release_at(pos));
    assert_eq!(count(&out), 0, "releasing queues no window command");
}

/// B-004 (ADR 017 v3): pure geometry of one client-side resize step — each
/// direction grows/shrinks exactly its own dimensions, north/west drags
/// re-anchor the outer top-left so the opposite edge/corner stays fixed, and
/// sizes are clamped to [`crate::appbar::MIN_WINDOW_SIZE`].
#[test]
fn resize_step_geometry_per_direction() {
    use egui::viewport::ResizeDirection::*;
    let grab = |dir: egui::viewport::ResizeDirection| crate::appbar::ResizeGrab {
        direction: dir,
        start_size: egui::vec2(1000.0, 700.0),
        start_outer: Some(Pos2::new(50.0, 40.0)),
        start_pointer: Pos2::new(3.0, 350.0),
    };
    let moved = grab(North).start_pointer + egui::vec2(40.0, 25.0);
    // (direction, expected new size, expected new outer top-left)
    let cases: [(egui::viewport::ResizeDirection, egui::Vec2, Option<Pos2>); 8] = [
        (East, egui::vec2(1040.0, 700.0), Some(Pos2::new(50.0, 40.0))),
        (West, egui::vec2(960.0, 700.0), Some(Pos2::new(90.0, 40.0))),
        (
            North,
            egui::vec2(1000.0, 675.0),
            Some(Pos2::new(50.0, 65.0)),
        ),
        (
            South,
            egui::vec2(1000.0, 725.0),
            Some(Pos2::new(50.0, 40.0)),
        ),
        (
            NorthEast,
            egui::vec2(1040.0, 675.0),
            Some(Pos2::new(50.0, 65.0)),
        ),
        (
            SouthEast,
            egui::vec2(1040.0, 725.0),
            Some(Pos2::new(50.0, 40.0)),
        ),
        (
            NorthWest,
            egui::vec2(960.0, 675.0),
            Some(Pos2::new(90.0, 65.0)),
        ),
        (
            SouthWest,
            egui::vec2(960.0, 725.0),
            Some(Pos2::new(90.0, 40.0)),
        ),
    ];
    for (dir, size, outer) in cases {
        let (s, o) = crate::appbar::resize_step(grab(dir), moved);
        assert_eq!(s, size, "{dir:?} size");
        assert_eq!(o, outer, "{dir:?} outer anchor");
    }
}

/// B-004 (ADR 017 v3): shrinking past the minimum is clamped — and the anchor
/// follows the *clamped* size, so the opposite edge still lands exactly where
/// the geometry says it should.
#[test]
fn resize_step_clamps_to_the_minimum_window_size() {
    use egui::viewport::ResizeDirection::*;
    let grab = crate::appbar::ResizeGrab {
        direction: West,
        start_size: egui::vec2(400.0, 700.0),
        start_outer: Some(Pos2::new(50.0, 40.0)),
        start_pointer: Pos2::new(3.0, 350.0),
    };
    // Drag the left edge 750 pt to the right → raw width would be -350.
    let (size, outer) =
        crate::appbar::resize_step(grab, grab.start_pointer + egui::vec2(750.0, 0.0));
    assert_eq!(
        size,
        egui::vec2(crate::appbar::MIN_WINDOW_SIZE.x, 700.0),
        "width clamped"
    );
    assert_eq!(
        outer,
        Some(Pos2::new(130.0, 40.0)),
        "anchor follows the clamp"
    );

    // Unknown start position (headless): no anchor is produced, size still computed.
    let grab = crate::appbar::ResizeGrab {
        direction: North,
        start_size: egui::vec2(1000.0, 700.0),
        start_outer: None,
        start_pointer: Pos2::new(500.0, 3.0),
    };
    let (size, outer) =
        crate::appbar::resize_step(grab, grab.start_pointer + egui::vec2(0.0, -30.0));
    assert_eq!(size, egui::vec2(1000.0, 730.0));
    assert_eq!(outer, None);
}

/// FR14 (ADR 017 v3): an undecorated window has no border to grab, so the app
/// owns resizing — invisible grips along all four screen edges and at the
/// corners. Each grip shows its resize cursor on hover.
#[test]
fn window_edge_grips_show_their_cursor_on_hover() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    // Two priming frames: a brand-new `Area` runs one invisible "sizing pass"
    // (its widgets are disabled and therefore not hit-testable), and egui
    // resolves hover from the PREVIOUS frame's widget rects — so the grips only
    // become hoverable from the third frame on.
    let _ = gui.frame();
    let _ = gui.frame();
    let (w, h) = (WIN_W, WIN_H);
    let g = 3.0; // well inside the 6-pt grip band

    let cases: [(Pos2, egui::CursorIcon); 8] = [
        (Pos2::new(g, g), egui::CursorIcon::ResizeNorthWest),
        (Pos2::new(w / 2.0, g), egui::CursorIcon::ResizeNorth),
        (Pos2::new(w - g, g), egui::CursorIcon::ResizeNorthEast),
        (Pos2::new(g, h / 2.0), egui::CursorIcon::ResizeWest),
        (Pos2::new(w - g, h / 2.0), egui::CursorIcon::ResizeEast),
        (Pos2::new(g, h - g), egui::CursorIcon::ResizeSouthWest),
        (Pos2::new(w / 2.0, h - g), egui::CursorIcon::ResizeSouth),
        (Pos2::new(w - g, h - g), egui::CursorIcon::ResizeSouthEast),
    ];

    for (i, (pos, icon)) in cases.iter().enumerate() {
        let out = gui.pump(vec![Event::PointerMoved(*pos)]);
        assert_eq!(
            out.platform_output.cursor_icon, *icon,
            "grip {i} ({pos:?}) hover cursor"
        );
    }
}

/// B-004 (ADR 017 v3): pressing and dragging an edge/corner grip resizes the
/// window — the app queues `InnerSize` with the pointer-driven size while the
/// button is held, and nothing after release. No `BeginResize` is ever queued
/// (the compositor-driven path did not work on KDE Plasma/KWin).
#[test]
fn edge_grip_press_and_drag_resizes_the_window() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.frame();
    let _ = gui.frame(); // priming (see the hover test)
    let (w, h) = (WIN_W, WIN_H);
    let g = 3.0;

    // (press pos, direction, expected size after dragging (+40, +25))
    let cases: [(Pos2, egui::viewport::ResizeDirection, egui::Vec2); 8] = [
        (
            Pos2::new(g, g),
            egui::viewport::ResizeDirection::NorthWest,
            egui::vec2(w - 40.0, h - 25.0),
        ),
        (
            Pos2::new(w / 2.0, g),
            egui::viewport::ResizeDirection::North,
            egui::vec2(w, h - 25.0),
        ),
        (
            Pos2::new(w - g, g),
            egui::viewport::ResizeDirection::NorthEast,
            egui::vec2(w + 40.0, h - 25.0),
        ),
        (
            Pos2::new(g, h / 2.0),
            egui::viewport::ResizeDirection::West,
            egui::vec2(w - 40.0, h),
        ),
        (
            Pos2::new(w - g, h / 2.0),
            egui::viewport::ResizeDirection::East,
            egui::vec2(w + 40.0, h),
        ),
        (
            Pos2::new(g, h - g),
            egui::viewport::ResizeDirection::SouthWest,
            egui::vec2(w - 40.0, h + 25.0),
        ),
        (
            Pos2::new(w / 2.0, h - g),
            egui::viewport::ResizeDirection::South,
            egui::vec2(w, h + 25.0),
        ),
        (
            Pos2::new(w - g, h - g),
            egui::viewport::ResizeDirection::SouthEast,
            egui::vec2(w + 40.0, h + 25.0),
        ),
    ];

    for (i, (pos, dir, expected)) in cases.iter().enumerate() {
        // Press: the gesture starts — and no `BeginResize` is queued (B-004).
        let out = gui.pump(Gui::press_at(*pos));
        assert!(
            window_commands(&out)
                .iter()
                .all(|c| !matches!(c, egui::ViewportCommand::BeginResize(_))),
            "grip {i} ({dir:?}) press must not queue BeginResize: {:?}",
            window_commands(&out)
        );

        // Drag outward by (40, 25): the app queues exactly the new inner size.
        let moved = *pos + egui::vec2(40.0, 25.0);
        let out = gui.pump(vec![Event::PointerMoved(moved)]);
        let sizes: Vec<egui::Vec2> = window_commands(&out)
            .iter()
            .filter_map(|c| match c {
                egui::ViewportCommand::InnerSize(s) => Some(*s),
                _ => None,
            })
            .collect();
        assert_eq!(
            sizes,
            vec![*expected],
            "grip {i} ({dir:?}) drag size: {:?}",
            window_commands(&out)
        );

        // Release: the gesture ends; the next frame queues nothing.
        let _ = gui.pump(Gui::release_at(moved));
        let out = gui.frame();
        assert!(
            window_commands(&out).is_empty(),
            "grip {i} ({dir:?}) after release queues nothing: {:?}",
            window_commands(&out)
        );
    }
}

/// B-004 (ADR 017 v3): dragging a west/north edge keeps the opposite edge
/// fixed — each step re-anchors the window's outer position (`OuterPosition`),
/// since `InnerSize` alone would grow/shrink from the top-left corner.
#[test]
fn west_and_north_grips_keep_the_opposite_edge_fixed() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let origin = Pos2::new(100.0, 60.0);
    let (w, h) = (WIN_W, WIN_H);
    let _ = gui.pump_viewport_at(origin, w, h, Vec::new());
    let _ = gui.pump_viewport_at(origin, w, h, Vec::new()); // priming

    // West edge, drag outward (left): the window gets wider and its left edge
    // follows the pointer — right edge stays at 100 + w.
    let pos = Pos2::new(3.0, h / 2.0);
    let _ = gui.pump_viewport_at(origin, w, h, Gui::press_at(pos));
    let out = gui.pump_viewport_at(
        origin,
        w,
        h,
        vec![Event::PointerMoved(pos + egui::vec2(-40.0, 0.0))],
    );
    let cmds = window_commands(&out);
    assert!(
        cmds.iter().any(
            |c| matches!(c, egui::ViewportCommand::InnerSize(s) if *s == egui::vec2(w + 40.0, h))
        ),
        "west drag widens the window: {cmds:?}"
    );
    assert!(
        cmds.iter().any(
            |c| matches!(c, egui::ViewportCommand::OuterPosition(p) if *p == Pos2::new(60.0, 60.0))
        ),
        "west drag re-anchors the left edge: {cmds:?}"
    );

    // Release, then north edge, drag outward (up): taller window, top edge
    // follows — bottom edge stays at 60 + h.
    let _ = gui.pump_viewport_at(origin, w, h, Gui::release_at(pos + egui::vec2(-40.0, 0.0)));
    let npos = Pos2::new(w / 2.0, 3.0);
    let _ = gui.pump_viewport_at(origin, w, h, Gui::press_at(npos));
    let out = gui.pump_viewport_at(
        origin,
        w,
        h,
        vec![Event::PointerMoved(npos + egui::vec2(0.0, -30.0))],
    );
    let cmds = window_commands(&out);
    assert!(
        cmds.iter().any(
            |c| matches!(c, egui::ViewportCommand::InnerSize(s) if *s == egui::vec2(w, h + 30.0))
        ),
        "north drag grows the height: {cmds:?}"
    );
    assert!(
        cmds.iter().any(
            |c| matches!(c, egui::ViewportCommand::OuterPosition(p) if *p == Pos2::new(100.0, 30.0))
        ),
        "north drag re-anchors the top edge: {cmds:?}"
    );

    // Release: nothing more.
    let _ = gui.pump_viewport_at(origin, w, h, Gui::release_at(npos + egui::vec2(0.0, -30.0)));
    let out = gui.frame();
    assert!(
        window_commands(&out).is_empty(),
        "after release: {:?}",
        window_commands(&out)
    );
}

/// A window narrower than two grip bands has no distinct edge zones: the grips are
/// not registered at all — no resize cursor on hover, no resize gesture (and thus
/// no `InnerSize`/`OuterPosition`) can start on press.
#[test]
fn resize_grips_disappear_on_a_degenerate_window() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let pos = Pos2::new(3.0, 10.0); // where the West grip would sit

    let out = gui.pump_screen(8.0, WIN_H, vec![Event::PointerMoved(pos)]);
    assert!(
        matches!(
            out.platform_output.cursor_icon,
            egui::CursorIcon::None | egui::CursorIcon::Default
        ),
        "no resize cursor where no grip exists: {:?}",
        out.platform_output.cursor_icon
    );
    let out = gui.pump_screen(8.0, WIN_H, Gui::press_at(pos));
    assert!(
        window_commands(&out).iter().all(|c| !matches!(
            c,
            egui::ViewportCommand::BeginResize(_)
                | egui::ViewportCommand::InnerSize(_)
                | egui::ViewportCommand::OuterPosition(_)
        )),
        "no resize commands on a degenerate window: {:?}",
        window_commands(&out)
    );
}

/// The minimize / fullscreen / close controls queue their viewport commands on click
/// (eframe applies them natively after the frame; headlessly they just land in the
/// output).
#[test]
fn window_controls_queue_viewport_commands() {
    // Minimize: “─”.
    {
        let mut gui = Gui::new(DrawingApp::new(Services::native()));
        let out = gui.frame();
        let btn = find_text_exact(&out, "─").expect("minimize control in the app bar");
        assert!(
            btn.max.y < crate::appbar::APP_BAR_HEIGHT,
            "the control sits in the app bar strip"
        );
        let _ = gui.pump(Gui::press_at(btn.center()));
        let out = gui.pump(Gui::release_at(btn.center())); // `clicked()` fires on release
        assert!(
            window_commands(&out)
                .iter()
                .any(|c| matches!(c, egui::ViewportCommand::Minimized(true))),
            "minimize click queues Minimized(true); got {:?}",
            window_commands(&out)
        );
    }

    // Fullscreen: “□” (headless viewport info is unknown → enters fullscreen).
    {
        let mut gui = Gui::new(DrawingApp::new(Services::native()));
        let out = gui.frame();
        let btn = find_text_exact(&out, "□").expect("fullscreen control in the app bar");
        let _ = gui.pump(Gui::press_at(btn.center()));
        let out = gui.pump(Gui::release_at(btn.center()));
        assert!(
            window_commands(&out)
                .iter()
                .any(|c| matches!(c, egui::ViewportCommand::Fullscreen(true))),
            "fullscreen click queues Fullscreen(true); got {:?}",
            window_commands(&out)
        );
    }

    // Close: “×” (eframe exits after this frame natively).
    {
        let mut gui = Gui::new(DrawingApp::new(Services::native()));
        let out = gui.frame();
        let btn = find_text_exact(&out, "×").expect("close control in the app bar");
        let _ = gui.pump(Gui::press_at(btn.center()));
        let out = gui.pump(Gui::release_at(btn.center()));
        assert!(
            window_commands(&out)
                .iter()
                .any(|c| matches!(c, egui::ViewportCommand::Close)),
            "close click queues Close; got {:?}",
            window_commands(&out)
        );
    }
}

/// AC2 — the app bar is painted with the active Material palette's surface color:
/// dark by default (Native resolves to dark headlessly), light after a switch.
#[test]
fn app_bar_paints_the_active_material_surface() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let out = gui.frame(); // default theme: Native → dark fallback

    let bar_rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(WIN_W, crate::appbar::APP_BAR_HEIGHT));
    let fill = |out: &egui::FullOutput| -> Option<Color32> {
        filled_rects(out)
            .into_iter()
            .find(|(r, _)| *r == bar_rect)
            .map(|(_, c)| c)
    };
    assert_eq!(
        fill(&out),
        Some(crate::theme::DARK.surface),
        "default (dark) app bar uses the dark Material surface"
    );

    // Switch to Light: the very next frame paints the light surface.
    let ctx = gui.ctx.clone();
    gui.app.set_theme(&ctx, Theme::Light);
    let out = gui.frame();
    assert_eq!(
        fill(&out),
        Some(crate::theme::LIGHT.surface),
        "light app bar uses the light Material surface"
    );
}

/// AC5 — the diagram content renders identically under both Material palettes: only
/// the chrome re-colors. Rasterize the same fixture under Dark and Light and compare
/// the canvas region pixel-for-pixel.
#[test]
fn canvas_is_pixel_identical_under_both_palettes() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/two-boxes-connector.ad");

    let raster = |theme: Theme| -> (Vec<[u8; 4]>, Rect) {
        let mut app = DrawingApp::with_config(
            Services::native(),
            crate::config::AppConfig {
                theme,
                ..Default::default()
            },
            None,
        );
        match app.open(&path) {
            crate::fileio::OpenOutcome::Loaded { .. } => {}
            other => panic!(
                "fixture failed to load: {}",
                crate::fileio::open_message(&other)
            ),
        }
        let mut gui = Gui::new(app);
        let out = gui.frame();
        // The canvas is the rect painted with the fixed (unthemed) CANVAS_BG.
        let canvas = filled_rects(&out)
            .into_iter()
            .find(|(_, c)| *c == crate::app::CANVAS_BG)
            .map(|(r, _)| r)
            .expect("the canvas background is drawn");
        (
            crate::screenshot::rasterize(&gui.ctx, &out, WIN_W as u32, WIN_H as u32),
            canvas,
        )
    };

    let (dark_buf, dark_canvas) = raster(Theme::Dark);
    let (light_buf, light_canvas) = raster(Theme::Light);
    assert_eq!(
        dark_canvas, light_canvas,
        "the canvas rect does not move between palettes"
    );

    for y in (dark_canvas.min.y as i32)..(dark_canvas.max.y as i32) {
        for x in (dark_canvas.min.x as i32)..(dark_canvas.max.x as i32) {
            let i = (y * WIN_W as i32 + x) as usize;
            assert_eq!(
                dark_buf[i], light_buf[i],
                "canvas pixel ({x},{y}) differs between palettes"
            );
        }
    }
}

/// AC6 — tier-2 golden-screenshot regression (the first committed golden set).
///
/// The software rasterizer is deterministic, so the restyled app must render
/// pixel-exact against `tests/goldens/material-{dark,light}.png`. To regenerate
/// after an intended visual change: `GOLDEN_UPDATE=1 cargo test -p gui-app material_theme_goldens`.
#[test]
fn material_theme_goldens_match_committed_pngs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let goldens = root.join("tests/goldens");
    let fixture = root.join("tests/fixtures/two-boxes-connector.ad");

    for (theme, name) in [
        (Theme::Dark, "material-dark.png"),
        (Theme::Light, "material-light.png"),
    ] {
        let mut app = DrawingApp::with_config(
            Services::native(),
            crate::config::AppConfig {
                theme,
                ..Default::default()
            },
            None,
        );
        match app.open(&fixture) {
            crate::fileio::OpenOutcome::Loaded { .. } => {}
            other => panic!(
                "fixture failed to load: {}",
                crate::fileio::open_message(&other)
            ),
        }
        let mut gui = Gui::new(app);
        let out = gui.frame();
        let fresh = crate::screenshot::rasterize(&gui.ctx, &out, WIN_W as u32, WIN_H as u32);

        let golden_path = goldens.join(name);
        if std::env::var("GOLDEN_UPDATE").is_ok() {
            crate::screenshot::capture_to_png(
                &gui.ctx,
                &out,
                WIN_W as u32,
                WIN_H as u32,
                &golden_path,
            )
            .unwrap_or_else(|e| panic!("write golden {name}: {e}"));
            eprintln!("regenerated {golden_path:?}");
            continue;
        }

        let file = std::fs::File::open(&golden_path)
            .unwrap_or_else(|e| panic!("missing golden {name} — run with GOLDEN_UPDATE=1: {e}"));
        let mut reader = png::Decoder::new(file)
            .read_info()
            .unwrap_or_else(|e| panic!("decode golden {name}: {e}"));
        let mut golden = vec![0u8; (WIN_W * WIN_H) as usize * 4];
        let info = reader
            .next_frame(&mut golden)
            .unwrap_or_else(|e| panic!("decode golden {name}: {e}"));
        assert_eq!(
            (info.width, info.height),
            (WIN_W as u32, WIN_H as u32),
            "golden {name} has an unexpected size"
        );

        let mut bad = 0;
        for (i, px) in fresh.iter().enumerate() {
            if [
                golden[i * 4],
                golden[i * 4 + 1],
                golden[i * 4 + 2],
                golden[i * 4 + 3],
            ] != *px
            {
                bad += 1;
            }
        }
        assert_eq!(
            bad, 0,
            "{name}: {bad} pixels differ from the committed golden — write the fresh render to /tmp/gui-dump and diff it against tests/goldens/{name}"
        );
    }
}

// ── SVG preview pane (S-026 / CR-004) ────────────────────────────────────────

/// AC3 — the pane is visible by default (CR assumption): an empty diagram shows the
/// placeholder inside the pane at the window's right edge.
#[test]
fn preview_pane_is_visible_by_default_with_a_placeholder() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let out = gui.frame();
    assert!(gui.app.preview_expanded(), "the pane is visible by default");
    let placeholder =
        find_text(&out, "Nothing to preview yet").expect("empty diagram shows the placeholder");
    assert!(
        placeholder.min.x > WIN_W - PREVIEW,
        "placeholder sits inside the right-side pane: {placeholder:?}"
    );
}

/// AC1 — once content exists, the export bytes are rasterized for the pane and the
/// placeholder is gone; the canvas model is untouched by the pane's presence.
#[test]
fn preview_shows_the_raster_once_content_exists() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 5, 3);
    let out = gui.frame();
    assert!(
        gui.app.preview_has_image(),
        "the export bytes were rasterized for the pane"
    );
    assert!(
        find_text(&out, "Nothing to preview yet").is_none(),
        "placeholder is replaced by the raster"
    );
}

/// AC2 — dragging the pane's left edge resizes it live: the canvas narrows (fewer grid
/// columns) and no grid line crosses into the pane.
#[test]
fn preview_pane_border_drag_resizes_the_canvas() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));

    /// Long vertical segments in canvas colors — i.e. the grid, not the pane's separator.
    fn grid_columns(out: &egui::FullOutput) -> Vec<f32> {
        out.shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                egui::epaint::Shape::LineSegment { points, stroke } => {
                    let (a, b) = (points[0], points[1]);
                    if (a.x - b.x).abs() < 0.5
                        && (a.y - b.y).abs() > 100.0
                        && stroke.color == crate::app::GRID_LINE_COLOR
                    {
                        Some(a.x)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect()
    }

    let before = grid_columns(&gui.frame()).len();

    // Widen the pane by 100 px: drag its left edge to the left.
    let edge = Pos2::new(WIN_W - PREVIEW, WIN_H / 2.0);
    gui.drag(edge, Pos2::new(WIN_W - PREVIEW - 100.0, WIN_H / 2.0));
    let out = gui.frame();

    assert!(gui.app.preview_expanded(), "resizing is not collapsing");
    let after = grid_columns(&out);
    assert!(
        after.len() < before.saturating_sub(5),
        "canvas narrowed: {before} → {} grid columns",
        after.len()
    );
    for x in &after {
        assert!(
            *x < WIN_W - PREVIEW - 100.0,
            "grid line at x={x} stays out of the widened pane"
        );
    }
}

/// AC3 — double-clicking the pane's edge collapses it; while collapsed a “Preview”
/// bookmark sits at the window's right edge and clicking it re-opens the pane.
#[test]
fn preview_pane_collapses_via_double_click_and_reopens_via_bookmark() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let _ = gui.frame(); // settle at the default width
    assert!(gui.app.preview_expanded());

    // Double-click the pane's left edge → collapse (the slide animation needs a few frames).
    let edge = Pos2::new(WIN_W - PREVIEW, WIN_H / 2.0);
    gui.double_click_at(edge);
    for _ in 0..300 {
        if !gui.app.preview_expanded() {
            break;
        }
        let _ = gui.frame();
    }
    assert!(
        !gui.app.preview_expanded(),
        "double-click collapsed the pane"
    );

    // The “Preview” bookmark appears once the slide-out animation has finished…
    let mut out = gui.frame();
    for _ in 0..300 {
        if find_text_exact(&out, "P\nr\ne\nv\ni\ne\nw").is_some() {
            break;
        }
        out = gui.frame();
    }
    let bookmark =
        find_text_exact(&out, "P\nr\ne\nv\ni\ne\nw").expect("bookmark drawn while collapsed");
    assert!(
        bookmark.center().x > WIN_W - 30.0,
        "bookmark hugs the window's right edge: {bookmark:?}"
    );

    // …and clicking it re-opens the pane.
    gui.click_at(bookmark.center());
    for _ in 0..300 {
        if gui.app.preview_expanded() {
            break;
        }
        let _ = gui.frame();
    }
    assert!(
        gui.app.preview_expanded(),
        "the bookmark re-opened the pane"
    );
}

/// AC4 — a rasterization problem is explained inside the pane (never a panic), and the
/// canvas keeps working beside it.
#[test]
fn preview_problem_message_shows_in_the_pane() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    // Pin the problem to the (empty) grid so the next refresh is a no-op, and frame.
    gui.app
        .set_preview_problem("", "rasterizer exploded".to_owned());
    let out = gui.frame();
    assert!(
        find_text(&out, "Preview unavailable").is_some(),
        "problem explained in the pane"
    );
    assert!(
        find_text(&out, "rasterizer exploded").is_some(),
        "the message itself is shown"
    );
    // The canvas beside the pane keeps working (still offering its empty-state hint).
    assert!(
        find_text(&out, "Pick a shape").is_some(),
        "canvas unaffected by the pane error"
    );
}

// ── Diagram colors dialog (FR15 / CR-009) ───────────────────────────────

impl Gui {
    /// Open the “Diagram colors” dialog via View ▾ → Diagram colors… and frame.
    fn open_color_dialog(&mut self) -> egui::FullOutput {
        let _ = self.click_menu_item("View ▾", "Diagram colors…");
        self.frame()
    }

    /// The rects of the three rows' `Set` buttons, top to bottom (row order =
    /// Background, Fill, Lines & text).
    fn color_set_buttons(out: &egui::FullOutput) -> Vec<Rect> {
        texts(out)
            .into_iter()
            .filter(|(t, _)| *t == "Set")
            .map(|(_, r)| r)
            .collect::<Vec<_>>()
    }

    /// Click the hex text field of dialog row `row` — it sits just left of that row's
    /// Set button (90 px wide) — so the following `type_text` lands in it.
    fn click_color_row_field(&mut self, out: &egui::FullOutput, row: usize) -> egui::FullOutput {
        let sets = Self::color_set_buttons(out);
        assert_eq!(sets.len(), 3, "the dialog shows one Set per role");
        let set = &sets[row];
        self.click_at(Pos2::new(set.min.x - 50.0, set.center().y))
    }

    /// Click row `row`'s Set (or Reset) button.
    fn click_color_row_action(
        &mut self,
        out: &egui::FullOutput,
        row: usize,
        action: &str,
    ) -> egui::FullOutput {
        let sets = texts(out)
            .into_iter()
            .filter(|(t, _)| *t == action)
            .map(|(_, r)| r)
            .collect::<Vec<_>>();
        assert_eq!(sets.len(), 3, "the dialog shows one {action} per role");
        self.click_at(sets[row].center())
    }

    /// The filled 18×18 swatches below `y` (the recent-colors strip), left to right.
    fn recent_strip_swatches(out: &egui::FullOutput, y: f32) -> Vec<(Rect, Color32)> {
        filled_rects(out)
            .into_iter()
            .filter(|(r, _)| r.min.y >= y && (15.0..=21.0).contains(&r.width()))
            .collect()
    }
}

/// AC5 — View ▾ → Diagram colors… opens the dialog with one row per role; all roles
/// unset shows each medium's "(default)" state.
#[test]
fn view_menu_opens_the_color_dialog_with_one_row_per_role() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let out = gui.open_color_dialog();
    assert!(
        find_text_exact(&out, "Diagram colors").is_some(),
        "window title"
    );
    for label in ["Background", "Shape fill", "Lines & text"] {
        assert!(
            find_text_exact(&out, label).is_some(),
            "row for {label:?} is shown; texts: {:?}",
            texts(&out).into_iter().map(|(t, _)| t).collect::<Vec<_>>()
        );
    }
    // No role set yet → every row advertises the medium default.
    let defaults = texts(&out)
        .iter()
        .filter(|(t, _)| *t == "(default)")
        .count();
    assert_eq!(defaults, 3, "all three rows show (default)");
}

/// AC5 — typing a hex into a row and pressing Set writes the model live (the canvas
/// and preview follow on their own hash gates), records the color in the recent
/// strip, and persists it to the options file.
#[test]
fn color_dialog_set_writes_the_model_and_persists_recent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let app = DrawingApp::with_config(
        Services::native(),
        crate::config::AppConfig::default(),
        Some(path.clone()),
    );
    let mut gui = Gui::new(app);
    let out = gui.open_color_dialog();

    // Fill row (index 1): focus its field, type the color, press its Set.
    gui.click_color_row_field(&out, 1);
    let _ = gui.type_text("#ff0000");
    let out = gui.frame();
    gui.click_color_row_action(&out, 1, "Set");

    use diagram_model::color::{ColorRole, Rgb};
    assert_eq!(
        gui.app.diagram.colors().get(ColorRole::Fill),
        Some(Rgb::new(0xff, 0x00, 0x00)),
        "the model carries the new fill color"
    );
    assert_eq!(
        gui.app.config().recent_colors,
        vec!["#ff0000".to_owned()],
        "the color joined the recent strip"
    );
    let json = std::fs::read_to_string(&path).expect("options file written");
    assert!(json.contains("\"recent_colors\""), "persisted: {json}");
    assert!(json.contains("#ff0000"), "persisted: {json}");

    // The row's swatch now paints the chosen color (WYSIWYG feedback in the dialog).
    let out = gui.frame();
    assert!(
        filled_rects(&out)
            .iter()
            .any(|(r, c)| *c == Color32::from_rgb(0xff, 0x00, 0x00) && r.width() > 15.0),
        "the Fill row shows a red swatch"
    );
}

/// AC5 — Reset removes the role from the model (medium default returns) and clears
/// the row's field.
#[test]
fn color_dialog_reset_clears_the_role_and_field() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let out = gui.open_color_dialog();
    gui.click_color_row_field(&out, 1);
    let _ = gui.type_text("#00ff00");
    let out = gui.frame();
    gui.click_color_row_action(&out, 1, "Set");

    use diagram_model::color::{ColorRole, Rgb};
    assert_eq!(
        gui.app.diagram.colors().get(ColorRole::Fill),
        Some(Rgb::new(0x00, 0xff, 0x00))
    );

    let out = gui.frame();
    gui.click_color_row_action(&out, 1, "Reset");
    assert_eq!(gui.app.diagram.colors().get(ColorRole::Fill), None);
    // The field is cleared — the typed hex no longer appears as row text.
    let out = gui.frame();
    assert!(
        !texts(&out).iter().any(|(t, _)| t.contains("00ff00")),
        "the reset row's field is empty"
    );
}

/// AC6 — the recent strip lists used colors newest-first; clicking a swatch applies
/// it to the active role (the last row touched).
#[test]
fn recent_strip_applies_a_color_to_the_active_role() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let out = gui.open_color_dialog();

    // Fill first, then Background — the strip must read newest-first afterwards.
    gui.click_color_row_field(&out, 1);
    let _ = gui.type_text("#123456");
    let out = gui.frame();
    gui.click_color_row_action(&out, 1, "Set");

    let out = gui.frame();
    gui.click_color_row_field(&out, 0);
    let _ = gui.type_text("#abcdef");
    let out = gui.frame();
    gui.click_color_row_action(&out, 0, "Set");

    use diagram_model::color::{ColorRole, Rgb};
    assert_eq!(
        gui.app.config().recent_colors,
        vec!["#abcdef".to_owned(), "#123456".to_owned()],
        "newest first"
    );

    // The strip: two swatches below the "Recent (…)" label — left = #abcdef,
    // right = #123456. Click the OLDER one; it applies to the active role
    // (Background — the last row touched).
    let out = gui.frame();
    let recent_y = find_text(&out, "Recent (applies to")
        .expect("the strip label is shown")
        .min
        .y;
    let swatches = Gui::recent_strip_swatches(&out, recent_y);
    assert_eq!(swatches.len(), 2, "two recent colors");
    assert_eq!(
        swatches[0].1,
        Color32::from_rgb(0xab, 0xcd, 0xef),
        "newest left"
    );
    assert_eq!(swatches[1].1, Color32::from_rgb(0x12, 0x34, 0x56));

    let _ = gui.click_at(swatches[1].0.center());
    assert_eq!(
        gui.app.diagram.colors().get(ColorRole::Background),
        Some(Rgb::new(0x12, 0x34, 0x56)),
        "the swatch applied to the active role"
    );
    // And it moved to the front of the strip.
    assert_eq!(
        gui.app.config().recent_colors,
        vec!["#123456".to_owned(), "#abcdef".to_owned()]
    );
}

/// AC6 — re-setting a color through the dialog moves it to the front of the recent
/// strip without duplicating it (the field still holds the value after Set).
#[test]
fn recent_colors_deduplicate_when_re_set() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let out = gui.open_color_dialog();

    // Two distinct colors: Fill, then Background.
    gui.click_color_row_field(&out, 1);
    let _ = gui.type_text("#123456");
    let out = gui.frame();
    gui.click_color_row_action(&out, 1, "Set");

    let out = gui.frame();
    gui.click_color_row_field(&out, 0);
    let _ = gui.type_text("#abcdef");
    let out = gui.frame();
    gui.click_color_row_action(&out, 0, "Set");
    assert_eq!(
        gui.app.config().recent_colors,
        vec!["#abcdef".to_owned(), "#123456".to_owned()]
    );

    // Press Fill's Set again — its field still holds #123456: it must move to the
    // front, not duplicate.
    let out = gui.frame();
    gui.click_color_row_action(&out, 1, "Set");
    assert_eq!(
        gui.app.config().recent_colors,
        vec!["#123456".to_owned(), "#abcdef".to_owned()],
        "moved to front, no duplicate"
    );
}

/// AC4/AC5 — an invalid hex never touches the model and is reported in the status bar.
#[test]
fn color_dialog_rejects_invalid_hex() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let out = gui.open_color_dialog();
    gui.click_color_row_field(&out, 2);
    let _ = gui.type_text("zz");
    let out = gui.frame();
    gui.click_color_row_action(&out, 2, "Set");

    use diagram_model::color::ColorRole;
    assert_eq!(gui.app.diagram.colors().get(ColorRole::LineText), None);
    assert!(
        find_text(&gui.frame(), "not a #rrggbb color").is_some(),
        "the parse failure is explained in the status bar"
    );
}

/// AC5 (FR19 / CR-016, S-037) — end-to-end: a region-bearing template is saved,
/// placed from the sidebar, double-clicked; the region-content window opens with one
/// field per declared region, and Apply writes the typed content to the instance.
#[test]
fn double_click_shape_opens_region_editor_and_apply_writes_model() {
    let tmp = tempfile::tempdir().unwrap();
    let (services, _warnings) = Services::native_with_collections(Some(tmp.path()));
    let mut gui = Gui::new(DrawingApp::new(services));

    // Register a 3×2 "tag" with one region covering it, via the editor save path —
    // this also exercises the NewTemplate seam carrying regions.
    let mut session = TemplateEditorSession::new_blank("testcol", "tag");
    assert!(session.draft.set_art("# #\n###"), "the 3×2 art parses");
    session.draft.toggle_anchor(1, 0); // drafts need ≥1 anchor
    assert_eq!(
        session.draft.add_region_drag((0, 0), (2, 1)).as_deref(),
        Some("r1")
    );
    gui.app.template_editor = Some(session);
    let msg = gui.app.save_template().expect("tag saves with its region");
    assert!(msg.contains("tag"));

    // Place it from the sidebar, then double-click its top-left cell. The group is
    // below the fold with the full built-in catalog: scroll to it first (egui
    // culls off-clip labels).
    let min = gui.canvas_min();
    let out = scroll_sidebar_until(&mut gui, "tag");
    let label = find_text_exact(&out, "tag").expect("sidebar lists the custom tag template");
    let _ = gui.click_at(label.center());
    let center = gui.cell_center(min, 3, 3);
    let _ = gui.click_at(center);
    assert_eq!(gui.object_count(), 1, "clicking the canvas placed one tag");

    // egui chains clicks closer than 0.3 s into multi-clicks: settle past that window
    // so the following pair reads as a fresh double-click (not a triple).
    for _ in 0..25 {
        gui.frame();
    }

    let _ = gui.double_click_at(center);
    assert!(
        gui.app.shape_text_edit.is_some(),
        "double-click opened the region editor"
    );
    // The window block runs before the canvas in `show`, so it appears one frame
    // after the double-click that opened it.
    let out = gui.frame();
    assert!(
        find_text(&out, "Text regions").is_some(),
        "the window title is drawn"
    );

    // Type into the label field and the r1 field, then Apply.
    let out = gui.frame();
    let label_rect = find_text_exact(&out, "Label:").expect("the label field is labelled");
    // Click well inside the 240 px-wide field (past the label + item spacing) and
    // type into it. The buffers live in the app state, so they can be asserted on.
    let _ = gui.click_at(Pos2::new(label_rect.max.x + 30.0, label_rect.center().y));
    let _ = gui.type_text("Main");

    let out = gui.frame();
    let region_rect = find_text_exact(&out, "r1:").expect("the region field is labelled");
    let _ = gui.click_at(Pos2::new(region_rect.max.x + 30.0, region_rect.center().y));
    let _ = gui.type_text("hello");

    let out = gui.frame();
    let apply = find_text_exact(&out, "Apply").expect("the Apply button is drawn");
    let _ = gui.click_at(apply.center());

    // The label and region content landed on the instance; the status confirms it.
    let shape = gui
        .app
        .diagram
        .objects()
        .iter()
        .find_map(|o| match o {
            diagram_model::objects::Object::Shape(s) => Some(s),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        shape.label, "Main",
        "the label field wrote the instance label"
    );
    assert_eq!(shape.region_content("r1"), Some("hello"));
    assert!(gui.app.status.contains("Updated text regions"));
}

/// AC4 (S-036, CR-015) — a placed *built-in* compartmented shape opens S-037's
/// region-content window with its `name` field on double-click (the S-037 test
/// covers the custom-template path; this pins the built-in one).
#[test]
fn double_click_builtin_component_opens_name_region_editor() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();

    // Place the built-in component from the sidebar (below the fold in the full
    // catalog — scroll to it first, egui culls off-clip labels).
    let out = scroll_sidebar_until(&mut gui, "component");
    let label = find_text_exact(&out, "component").expect("sidebar lists the component template");
    let _ = gui.click_at(label.center());
    let center = gui.cell_center(min, 8, 3);
    let _ = gui.click_at(center);
    assert_eq!(
        gui.object_count(),
        1,
        "clicking the canvas placed one component"
    );

    // egui chains clicks closer than 0.3 s into multi-clicks: settle past that window
    // so the following pair reads as a fresh double-click (not a triple).
    for _ in 0..25 {
        gui.frame();
    }

    let _ = gui.double_click_at(center);
    assert!(
        gui.app.shape_text_edit.is_some(),
        "double-click opened the region editor"
    );
    // The window block runs before the canvas in `show`, so it appears one frame
    // after the double-click that opened it.
    let out = gui.frame();
    assert!(
        find_text(&out, "Text regions").is_some(),
        "the window title is drawn"
    );
    assert!(
        find_text_exact(&out, "name:").is_some(),
        "the built-in's `name` region field is present"
    );
}
