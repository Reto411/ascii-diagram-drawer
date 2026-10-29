//! The eframe application (ADR 008) — deliberately thin egui glue around the pure
//! helpers in [`crate::canvas`] and [`crate::editor`], which carry all interaction
//! logic and are unit-tested headlessly.
//!
//! Per-frame flow: re-render the model through the ADR 009 seam (live WYSIWYG, AC2),
//! draw the exact text bytes on a cell-aligned canvas, map pointer edges to editor
//! events, and report what happened in the status bar.

use std::path::Path;

use core_services::Services;
use diagram_model::{Diagram, objects::Object};
use eframe::egui::{self, Align2, Color32, FontFamily, FontId, Pos2, RichText, Sense};

use crate::canvas::Viewport;
use crate::editor::{EditOutcome, EditorState, PointerEvent};

/// Monospace font size for the canvas; the actual cell size is measured from it at
/// runtime so glyphs land exactly on cells regardless of font metrics.
const FONT_SIZE: f32 = 14.0;
/// Margin around the grid inside the canvas area.
const MARGIN_PX: f32 = 8.0;
/// Fixed width of the template sidebar.
const SIDEBAR_PX: f32 = 180.0;

const TEXT_COLOR: Color32 = Color32::from_rgb(222, 226, 232);
const GRID_LINE_COLOR: Color32 = Color32::from_rgb(44, 47, 54);
const SELECTION_COLOR: Color32 = Color32::from_rgb(90, 160, 255);
const GHOST_COLOR: Color32 = Color32::from_rgb(140, 200, 140);
const ERROR_COLOR: Color32 = Color32::from_rgb(255, 96, 96);

/// The eframe app state. Everything user-visible is derived from `diagram` (the model)
/// plus transient view state in `editor` — ADR 009.
pub struct DrawingApp {
    services: Services,
    diagram: Diagram,
    editor: EditorState,
    /// Last action summary for the status bar.
    pub status: String,
    /// Whether the primary button was held on the previous frame (edge detection).
    pointer_held: bool,
}

impl DrawingApp {
    /// Creates an app with an empty diagram.
    #[must_use]
    pub fn new(services: Services) -> Self {
        Self {
            services,
            diagram: Diagram::new(),
            editor: EditorState::default(),
            status: "Ready".to_owned(),
            pointer_held: false,
        }
    }

    /// Loads a `.ad` document from the store, replacing the current diagram.
    pub fn load(&mut self, path: &Path) -> Result<(), String> {
        self.diagram = self.services.store.load(path).map_err(|e| e.to_string())?;
        self.status = format!("Loaded {}", path.display());
        Ok(())
    }

    /// The WYSIWYG canvas frame: the exact text bytes of the resolved grid (AC2) plus
    /// its extent in cells. Fails with a human-readable message when the model cannot
    /// be rendered.
    pub fn canvas_frame(&self) -> Result<(String, u32, u32), String> {
        let grid = self
            .services
            .renderer
            .render(&self.diagram)
            .map_err(|e| e.to_string())?;
        Ok((
            self.services.exporter.to_text(&grid),
            grid.width(),
            grid.height(),
        ))
    }

    /// The top-left model cell of the diagram's content bounding box — where the
    /// trimmed resolved grid starts. Empty diagram → the origin.
    #[must_use]
    pub fn content_origin(&self) -> (u32, u32) {
        let mut min_x = None;
        let mut min_y = None;
        for object in self.diagram.objects() {
            match object {
                Object::Shape(shape) => {
                    min_x = Some(min_x.map_or(shape.pos.x, |m: u32| m.min(shape.pos.x)));
                    min_y = Some(min_y.map_or(shape.pos.y, |m: u32| m.min(shape.pos.y)));
                }
                Object::Text(text) => {
                    min_x = Some(min_x.map_or(text.pos.x, |m: u32| m.min(text.pos.x)));
                    min_y = Some(min_y.map_or(text.pos.y, |m: u32| m.min(text.pos.y)));
                }
                // Connectors have no position of their own (FR9).
                Object::Connector(_) => {}
            }
        }
        (min_x.unwrap_or(0), min_y.unwrap_or(0))
    }

    /// Maps one frame of raw pointer state to editor events (press/release edge
    /// detection) and returns what changed — the seam between egui input and the
    /// pure gesture machine, unit-testable without a window.
    pub fn handle_pointer_frame(&mut self, held: bool, cell: Option<(u32, u32)>) -> EditOutcome {
        let outcome = match (held, self.pointer_held) {
            (true, false) => match cell {
                Some((col, row)) => self
                    .editor
                    .on_pointer(PointerEvent::Down { col, row }, &mut self.diagram),
                None => EditOutcome::Nothing,
            },
            (true, true) => match cell {
                Some((col, row)) => self
                    .editor
                    .on_pointer(PointerEvent::Move { col, row }, &mut self.diagram),
                None => EditOutcome::Nothing,
            },
            (false, true) => self.editor.on_pointer(PointerEvent::Up, &mut self.diagram),
            (false, false) => EditOutcome::Nothing,
        };
        self.pointer_held = held;
        outcome
    }

    /// Draws the cell-aligned canvas: grid lines, the exact text bytes, selection and
    /// placement overlays — then feeds this frame's pointer state to the editor.
    fn draw_canvas(&mut self, ui: &mut egui::Ui, text: &str, width: u32, height: u32) {
        let ctx = ui.ctx();
        let font_id = FontId::new(FONT_SIZE, FontFamily::Monospace);
        // Measure the real monospace advance (10 glyphs) so every glyph lands exactly
        // in its cell, whatever font metrics egui loaded.
        let sample = ui.painter().layout(
            "MMMMMMMMMM".to_owned(),
            font_id.clone(),
            Color32::TRANSPARENT,
            f32::INFINITY,
        );
        let cell_px = (sample.size().x / 10.0).max(4.0);
        // The viewport maps MODEL space to pixels; the trimmed grid text starts at
        // the content origin within it, so rendered cells and model cells coincide.
        let vp = Viewport::new(MARGIN_PX, cell_px);
        let (origin_x, origin_y) = self.content_origin();

        let rect = ui.max_rect();
        let response = ui.interact(rect, egui::Id::unique("canvas"), Sense::click());
        let painter = ui.painter();

        // Subtle background grid lines over the content extent.
        for col in origin_x..=origin_x + width {
            let x = vp.origin_x + col as f32 * cell_px;
            painter.line_segment(
                [
                    Pos2::new(x, vp.origin_y + origin_y as f32 * cell_px),
                    Pos2::new(x, vp.origin_y + (origin_y + height) as f32 * cell_px),
                ],
                egui::epaint::Stroke::new(1.0, GRID_LINE_COLOR),
            );
        }
        for row in origin_y..=origin_y + height {
            let y = vp.origin_y + row as f32 * cell_px;
            painter.line_segment(
                [
                    Pos2::new(vp.origin_x + origin_x as f32 * cell_px, y),
                    Pos2::new(vp.origin_x + (origin_x + width) as f32 * cell_px, y),
                ],
                egui::epaint::Stroke::new(1.0, GRID_LINE_COLOR),
            );
        }

        // The exact text bytes of the resolved grid — one line per row (AC2),
        // offset by the content origin.
        for (row, line) in text.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            let pos = Pos2::new(
                vp.origin_x,
                vp.origin_y + (origin_y + row as u32) as f32 * cell_px,
            );
            painter.text(pos, Align2::LEFT_TOP, line, font_id.clone(), TEXT_COLOR);
        }

        // Selection overlay around the selected shape's bounds.
        let selected_shape =
            self.editor
                .selection
                .as_ref()
                .and_then(|id| match self.diagram.get(id) {
                    Some(Object::Shape(shape)) => Some(shape),
                    _ => None,
                });
        if let Some(shape) = selected_shape {
            let (x, y, w, h) = vp.cell_block(
                shape.pos.x,
                shape.pos.y,
                shape.size.width,
                shape.size.height,
            );
            painter.rect_stroke(
                egui::Rect::from_min_max(Pos2::new(x, y), Pos2::new(x + w, y + h)),
                0.0,
                egui::epaint::Stroke::new(1.5, SELECTION_COLOR),
                egui::epaint::StrokeKind::Middle,
            );
        }

        // Placement ghost at the hovered cell (model space — unbounded).
        let hover_cell = response
            .hover_pos()
            .and_then(|p| vp.screen_to_model_cell(p.x - rect.min.x, p.y - rect.min.y));
        if let (Some(info), Some((col, row))) = (&self.editor.placing, hover_cell) {
            let (x, y, w, h) = vp.cell_block(col, row, info.size.width, info.size.height);
            painter.rect_stroke(
                egui::Rect::from_min_max(Pos2::new(x, y), Pos2::new(x + w, y + h)),
                0.0,
                egui::epaint::Stroke::new(1.0, GHOST_COLOR),
                egui::epaint::StrokeKind::Middle,
            );
        }

        // Pointer edges → editor events (AC3/AC4); report the outcome in the status bar.
        // Right-click cancels placement mode and returns to selection/drag.
        if ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Secondary)) {
            self.editor.cancel_placing();
            self.status = "Placement cancelled".to_owned();
        }

        let held = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        if let Some(message) = status_for(self.handle_pointer_frame(held, hover_cell)) {
            self.status = message;
        }
    }
}

impl eframe::App for DrawingApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Status bar.
        egui::Panel::top("statusbar").show(ui, |ui| {
            ui.label(RichText::new(format!("ascii-diagram-drawer — {}", self.status)).weak());
        });

        // Shape template sidebar (FR3 read model from the catalog).
        let templates = self.services.catalog.list();
        let placing_id = self.editor.placing.as_ref().map(|p| p.type_id.clone());
        egui::Panel::left("templates")
            .size_range(SIDEBAR_PX..=SIDEBAR_PX)
            .show(ui, |ui| {
                ui.heading("Shapes");
                for info in &templates {
                    let selected = Some(info.type_id.as_str()) == placing_id.as_deref();
                    if ui.selectable_label(selected, &info.name).clicked() {
                        self.editor.select_template(info.clone());
                        self.status = format!("Placing {} — click the canvas", info.name);
                    }
                }
            });

        // WYSIWYG canvas: re-render the model every frame (live updates during gestures).
        egui::CentralPanel::default_margins().show(ui, |ui| match self.canvas_frame() {
            Err(e) => {
                ui.label(RichText::new(format!("Render error: {e}")).color(ERROR_COLOR));
            }
            Ok((text, width, height)) => self.draw_canvas(ui, &text, width, height),
        });
    }
}

/// A human-readable status-bar message for an editor outcome (`None` = no change).
fn status_for(outcome: EditOutcome) -> Option<String> {
    match outcome {
        EditOutcome::Nothing => None,
        EditOutcome::Deselected => Some("Selection cleared".to_owned()),
        EditOutcome::Selected(id) => Some(format!("Selected {id}")),
        EditOutcome::Placed { id, type_id } => Some(format!("Placed {type_id} as {id}")),
        EditOutcome::Moved { id, to } => Some(format!("{id} moved to ({}, {})", to.x, to.y)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_services::fakes::{FakeRenderer, FakeStore};
    use diagram_model::geometry::{CellPos, CellSize};
    use diagram_model::objects::{Object as Obj, ObjectId, ShapeInstance};
    use std::sync::Arc;
    use ui_api::ShapeInfo;

    /// App wired to the real (headless-safe) render/export/catalog services.
    fn app() -> DrawingApp {
        DrawingApp::new(Services::native())
    }

    /// The built-in 9×5 `box` template, as the catalog serves it.
    fn box_info() -> ShapeInfo {
        Services::native()
            .catalog
            .get("box")
            .expect("built-in box exists")
    }

    #[test]
    fn canvas_frame_of_an_empty_diagram_is_empty() {
        let (text, width, height) = app().canvas_frame().expect("renders");
        assert_eq!(text, "");
        assert_eq!((width, height), (0, 0));
    }

    #[test]
    fn canvas_frame_is_the_exact_rendered_bytes_after_placement() {
        let mut app = app();
        app.editor.select_template(box_info());
        assert!(matches!(
            app.handle_pointer_frame(true, Some((3, 4))),
            EditOutcome::Placed { .. }
        ));
        app.handle_pointer_frame(false, None);

        // The pipeline trims to the content bounding box: a lone 9×5 box renders as
        // 9×5 regardless of where it sits in model space.
        let (text, width, height) = app.canvas_frame().expect("renders");
        assert_eq!((width, height), (9, 5));
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 5, "box is 5 rows tall:\n{text}");
        assert!(
            lines[0].contains("+-------+"),
            "top border on row 0: {text}"
        );
        assert!(
            lines[4].contains("+-------+"),
            "bottom border on row 4: {text}"
        );
        assert_eq!(
            lines[0].chars().next(),
            Some('+'),
            "border starts at column 0"
        );

        // The canvas draws this text offset by the content origin in model space.
        assert_eq!(app.content_origin(), (3, 4));
    }

    #[test]
    fn canvas_text_reports_render_failures_readably() {
        let services = Services::native();
        let mut app = DrawingApp::new(Services {
            renderer: Arc::new(FakeRenderer::failing("mystery")),
            ..services
        });
        // Put a shape into the model to exercise the error path.
        let id = app.diagram.new_object_id();
        app.diagram
            .place_shape(
                ShapeInstance::new(
                    id,
                    "box",
                    "",
                    CellPos::new(0, 0),
                    CellSize::new(9, 5).unwrap(),
                )
                .expect("valid"),
            )
            .expect("placement");
        let err = app.canvas_frame().expect_err("renderer is failing");
        assert!(err.contains("mystery"), "{err:?}");
    }

    #[test]
    fn pointer_frames_drive_place_select_and_drag() {
        let mut app = app();
        // Place a box at (3,4) with two frames (press + release).
        app.editor.select_template(box_info());
        assert!(matches!(
            app.handle_pointer_frame(true, Some((3, 4))),
            EditOutcome::Placed { .. }
        ));
        assert_eq!(app.handle_pointer_frame(false, None), EditOutcome::Nothing);

        // Press inside the placed box (cell (5,5) is within its 9×5 at (3,4)) and drag.
        assert_eq!(
            app.handle_pointer_frame(true, Some((5, 5))),
            EditOutcome::Selected(ObjectId::generated(1))
        );
        assert!(matches!(
            app.handle_pointer_frame(true, Some((12, 8))),
            EditOutcome::Moved { .. }
        ));
        // Grab offset was (2,1): top-left now at (10,7).
        let shape = app
            .diagram
            .objects()
            .iter()
            .find_map(|o| match o {
                Obj::Shape(s) => Some(s),
                _ => None,
            })
            .expect("the box exists");
        assert_eq!((shape.pos.x, shape.pos.y), (10, 7));
    }

    #[test]
    fn load_replaces_the_diagram_from_the_store() {
        let mut diagram = Diagram::new();
        let id = diagram.new_object_id();
        diagram
            .place_shape(
                ShapeInstance::new(
                    id,
                    "box",
                    "",
                    CellPos::new(0, 0),
                    CellSize::new(9, 5).unwrap(),
                )
                .expect("valid"),
            )
            .expect("placement");

        let services = Services::native();
        let mut app = DrawingApp::new(Services {
            store: Arc::new(FakeStore::default().with_document("t.ad", diagram)),
            ..services
        });
        app.load(std::path::Path::new("t.ad")).expect("loads");
        assert_eq!(app.diagram.len(), 1);

        let err = app.load(std::path::Path::new("missing.ad"));
        assert!(err.is_err());
    }

    #[test]
    fn status_for_covers_every_outcome() {
        assert_eq!(status_for(EditOutcome::Nothing), None);
        assert_eq!(
            status_for(EditOutcome::Deselected).as_deref(),
            Some("Selection cleared")
        );
        assert!(
            status_for(EditOutcome::Selected(ObjectId::generated(0)))
                .unwrap()
                .contains("obj-0")
        );
        assert!(
            status_for(EditOutcome::Placed {
                id: ObjectId::generated(1),
                type_id: "box".into()
            })
            .unwrap()
            .contains("box")
        );
        assert!(
            status_for(EditOutcome::Moved {
                id: ObjectId::generated(2),
                to: CellPos::new(1, 2)
            })
            .unwrap()
            .contains("(1, 2)")
        );
    }
}
