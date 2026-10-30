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
use core_services::Services;
use diagram_model::objects::Object;
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
/// Must match `ANCHOR_COLOR` / `SELECTION_COLOR` in `app.rs`.
const ANCHOR: Color32 = Color32::from_rgb(255, 196, 87);
const SELECTION: Color32 = Color32::from_rgb(90, 160, 255);

// ─────────────────────────── test harness ───────────────────────────

/// One headless egui context driving a [`DrawingApp`], frame by frame.
struct Gui {
    ctx: egui::Context,
    app: DrawingApp,
    ts: f64,
}

impl Gui {
    fn new(app: DrawingApp) -> Self {
        let ctx = egui::Context::default();
        // Keep the built-in fonts: eframe (without `system_fonts`) lays out with exactly
        // these defaults — Hack monospace + Ubuntu proportional — so every metric the
        // app computes here matches the native app's.
        ctx.set_fonts(egui::FontDefinitions::default());
        Self { ctx, app, ts: 1.0 }
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
            out = self.pump(vec![Event::Text(c.to_string())]);
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

    /// Open a path prompt (Save As / Open / Export …) and return its kind after it appears.
    fn open_prompt(&mut self, button: &str) -> PathPrompt {
        let _ = self.click_topbar_button(button);
        let prompt = self.app.path_prompt();
        assert!(prompt.is_some(), "clicking {button:?} opens a path prompt");
        prompt.expect("checked above")
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

/// All filled circles as `(center, radius, fill)`.
fn filled_circles(out: &egui::FullOutput) -> Vec<(Pos2, f32, Color32)> {
    out.shapes
        .iter()
        .filter_map(|cs| match &cs.shape {
            egui::epaint::Shape::Circle(c) if c.fill != Color32::TRANSPARENT => {
                Some((c.center, c.radius, c.fill))
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
        assert!(
            a.x >= min.x - 0.5 && a.y >= min.y - 0.5 && b.x >= min.x - 0.5 && b.y >= min.y - 0.5,
            "line ({a:?}..{b:?}) stays inside the canvas area"
        );
    }
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

#[test]
fn resize_handle_marker_sits_on_the_inner_bottom_right_cell() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 3, 2);
    gui.select_first_box(min);
    let out = gui.frame();

    let (x, y, w, h) = gui.first_box();
    // The marker is a small filled square in ANCHOR color.
    let markers: Vec<Rect> = filled_rects(&out)
        .into_iter()
        .filter(|(r, c)| {
            *c == ANCHOR && (2.0..=8.0).contains(&r.width()) && (2.0..=8.0).contains(&r.height())
        })
        .map(|(r, _)| r)
        .collect();
    let marker = markers
        .into_iter()
        .next()
        .expect("resize handle marker is drawn");
    let expected_center = gui.cell_center(min, x + w - 1, y + h - 1);
    assert!(
        close(marker.center().x, expected_center.x, 2.0)
            && close(marker.center().y, expected_center.y, 2.0),
        "marker at {:?} is on the inner bottom-right cell {expected_center:?} — where \
         resizing actually works",
        marker.center()
    );
}

#[test]
fn anchor_dots_sit_at_the_edge_midpoints_of_the_selection() {
    let mut gui = Gui::new(DrawingApp::new(Services::native()));
    let min = gui.canvas_min();
    gui.place_box_via_ui(min, 0, 0);
    gui.select_first_box(min);
    let out = gui.frame();

    let (x, y, w, h) = gui.first_box();
    // Default box: 9 wide, 5 tall → anchors n(4,0) s(4,4) w(0,2) e(8,2).
    let expected = [
        gui.cell_center(min, x + w / 2, y),
        gui.cell_center(min, x + w / 2, y + h - 1),
        gui.cell_center(min, x, y + h / 2),
        gui.cell_center(min, x + w - 1, y + h / 2),
    ];
    let dots: Vec<Pos2> = filled_circles(&out)
        .into_iter()
        .filter(|(_, r, c)| *c == ANCHOR && close(*r, 3.0, 0.5))
        .map(|(p, _, _)| p)
        .collect();
    assert_eq!(dots.len(), 4, "four anchor dots are drawn");
    for e in expected {
        assert!(
            dots.iter()
                .any(|d| close(d.x, e.x, 2.0) && close(d.y, e.y, 2.0)),
            "anchor dot at {e:?} (edge midpoint cell), dots: {dots:?}"
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
    assert!(content.starts_with("ad 1.0"), "valid .ad header");
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

    // Click Open…, but press Escape before the settle frame: both are open now.
    let out = gui.frame();
    let rect = find_text_exact(&out, "Open…").expect("top bar has Open…");
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
    // instead of going through `click_at`'s settle frame.
    let out = gui.frame();
    let rect = find_text_exact(&out, "Copy").expect("top bar has Copy");
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

    // Save via the window's own Save button (the lower of the two "Save" labels).
    let out = gui.frame();
    let saves: Vec<Rect> = texts(&out)
        .into_iter()
        .filter(|(t, _)| t == "Save")
        .map(|(_, r)| r)
        .collect();
    assert!(
        saves.len() >= 2,
        "top-bar Save and window Save are both visible"
    );
    let window_save = saves
        .iter()
        .max_by(|a, b| a.min.y.total_cmp(&b.min.y))
        .unwrap();
    let out = gui.click_at(window_save.center());

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
    // header plus the template's own name.
    let out = gui.frame();
    assert!(
        find_text(&out, "myshapes").is_some(),
        "sidebar has the collection group"
    );
    assert!(
        find_text_exact(&out, "new").is_some(),
        "sidebar lists the new template by name"
    );
}
