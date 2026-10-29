//! The eframe application (ADR 008) — deliberately thin egui glue around the pure
//! helpers in [`crate::canvas`], [`crate::editor`] and [`crate::fileio`], which carry
//! all interaction/file-IO logic and are unit-tested headlessly.
//!
//! Per-frame flow: re-render the model through the ADR 009 seam (live WYSIWYG, AC2),
//! draw the exact text bytes on a cell-aligned canvas, map pointer edges to editor
//! events, run file-IO flows on menu actions, and report what happened in the status
//! bar.

use std::path::{Path, PathBuf};

use core_services::Services;
use diagram_model::{Diagram, objects::Object};
use eframe::egui::{self, Align2, Color32, FontFamily, FontId, Key, Pos2, RichText, Sense};

use crate::canvas::{HitTarget, Viewport, hit_target};
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
const ANCHOR_COLOR: Color32 = Color32::from_rgb(255, 196, 87);
const CONNECT_LINE_COLOR: Color32 = Color32::from_rgb(255, 196, 87);

/// The eframe app state. Everything user-visible is derived from `diagram` (the model)
/// plus transient view state in `editor` — ADR 009.
/// An inline path prompt (v1 has no native file dialog — works identically on
/// native and wasm; a future story may swap in `rfd` for native targets).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathPrompt {
    /// Collecting a path to open.
    Open,
    /// Collecting a path to save the current document as (sets the current file).
    SaveAs,
}

pub struct DrawingApp {
    services: Services,
    /// `pub` like `status`: this is a binary crate — the file-IO tests in
    /// [`crate::fileio`] read the model through it.
    pub diagram: Diagram,
    editor: EditorState,
    /// Last action summary for the status bar.
    pub status: String,
    /// Whether the primary button was held on the previous frame (edge detection).
    pointer_held: bool,
    /// The file the current model was loaded from — `Save` targets it (S-014).
    current_file: Option<PathBuf>,
    /// Unknown statements skipped when loading `current_file` (ADR 009 v4 / RS2):
    /// non-zero arms the AC4 save warning.
    skipped_unknowns: usize,
    /// The AC4 lossy-save confirmation dialog is open.
    confirming_save: bool,
    /// An inline Open/Save-As path prompt is showing (and its kind).
    path_prompt: Option<PathPrompt>,
}

/// Builds the app from services and an optional startup fixture (the visual test tiers
/// pass a `.ad` path as the first CLI argument). Load failures are reported, not fatal —
/// the user can still open a file later.
#[must_use]
pub fn create_app(services: Services, fixture: Option<&Path>) -> DrawingApp {
    let mut app = DrawingApp::new(services);
    if let Some(path) = fixture {
        let outcome = app.open(path);
        if !matches!(outcome, crate::fileio::OpenOutcome::Loaded { .. }) {
            eprintln!(
                "could not load {}: {}",
                path.display(),
                crate::fileio::open_message(&outcome)
            );
        }
    }
    app
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
            current_file: None,
            skipped_unknowns: 0,
            confirming_save: false,
            path_prompt: None,
        }
    }

    // ── File IO (S-014) — pure flows in [`crate::fileio`], this is the glue ───────

    /// The file the current model was loaded from (`Save` targets it).
    #[must_use]
    pub fn current_file(&self) -> Option<&PathBuf> {
        self.current_file.as_ref()
    }

    /// AC4 state: is the lossy-save confirmation dialog open?
    #[must_use]
    pub fn confirming_save(&self) -> bool {
        self.confirming_save
    }

    /// AC1 — open a `.ad` file through the store seam. On success the model is
    /// replaced and the file remembered; on failure the previous model is kept and a
    /// notice with line/column (parse) or cause (IO) is shown (RS2/RS5).
    #[must_use]
    pub fn open(&mut self, path: &Path) -> crate::fileio::OpenOutcome {
        let outcome = crate::fileio::open(self.services.store.as_ref(), path);
        if let crate::fileio::OpenOutcome::Loaded {
            diagram,
            skipped_unknowns,
            ..
        } = &outcome
        {
            self.diagram = diagram.clone();
            self.current_file = Some(path.to_path_buf());
            self.skipped_unknowns = *skipped_unknowns;
            // A new document invalidates any in-flight interaction.
            self.editor.reset();
        }
        self.status = crate::fileio::open_message(&outcome);
        outcome
    }

    /// AC2/AC4 — `Save` (or Ctrl+S): writes immediately for clean documents; for lossy
    /// loads it opens the confirmation dialog instead of writing. Without a current
    /// file there is nothing to save (use Save As).
    #[must_use]
    pub fn save_requested(&mut self) -> crate::fileio::SaveOutcome {
        let Some(path) = self.current_file.clone() else {
            self.status = "Nothing to save — use Save As…".to_owned();
            return crate::fileio::SaveOutcome::IoError {
                path: PathBuf::new(),
                message: "no document open".to_owned(),
            };
        };
        self.write_to(&path)
    }

    /// AC4 — the user confirmed the lossy save; it writes now.
    #[must_use]
    pub fn confirm_save(&mut self) -> crate::fileio::SaveOutcome {
        let path = self.current_file.clone().unwrap_or_default();
        let outcome = crate::fileio::save(
            self.services.store.as_ref(),
            &path,
            &self.diagram,
            self.skipped_unknowns,
            true,
        );
        self.confirming_save = false;
        self.status = crate::fileio::save_message(&outcome);
        outcome
    }

    /// AC4 — the user cancelled the lossy save; nothing is written.
    pub fn cancel_confirmation(&mut self) {
        self.confirming_save = false;
        self.status = "Save cancelled — document unchanged".to_owned();
    }

    /// AC2/AC4 — `Save As…`: remembers the new current file, then runs the same save
    /// flow (a lossy document still warns before overwriting).
    #[must_use]
    pub fn save_as(&mut self, path: &Path) -> crate::fileio::SaveOutcome {
        self.current_file = Some(path.to_path_buf());
        self.write_to(path)
    }

    /// The shared write path behind Save / Save As (AC2 + AC4).
    fn write_to(&mut self, path: &Path) -> crate::fileio::SaveOutcome {
        let outcome = crate::fileio::save(
            self.services.store.as_ref(),
            path,
            &self.diagram,
            self.skipped_unknowns,
            false,
        );
        if matches!(
            outcome,
            crate::fileio::SaveOutcome::ConfirmationNeeded { .. }
        ) {
            self.confirming_save = true;
        }
        self.status = crate::fileio::save_message(&outcome);
        outcome
    }

    /// AC3 — the exact text output of the current model (same bytes as the canvas).
    pub fn text_output(&self) -> Result<String, String> {
        crate::fileio::text_output(&self.services, &self.diagram)
    }

    /// AC3 — write the text output next to the document (`<name>.txt`).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_text(&mut self) -> Result<PathBuf, String> {
        let Some(path) = self.current_file.clone() else {
            return Err("no document open — nothing to export".to_owned());
        };
        let text = self.text_output()?;
        let txt_path = crate::fileio::text_output_path(&path);
        crate::fileio::write_text_file(&txt_path, &text)?;
        self.status = format!("Text output saved: {}", txt_path.display());
        Ok(txt_path)
    }

    /// AC3 — web target: no filesystem; the browser download lands in S-019.
    #[cfg(target_arch = "wasm32")]
    pub fn save_text(&mut self) -> Result<PathBuf, String> {
        Err("text download arrives with the wasm milestone (S-019)".to_owned())
    }

    /// Starts an inline Open/Save-As path prompt (v1: no native file dialog).
    pub fn begin_path_prompt(&mut self, kind: PathPrompt) {
        self.path_prompt = Some(kind);
    }

    /// The active path prompt, if any (the top bar renders it when `Some`).
    #[must_use]
    pub fn path_prompt(&self) -> Option<PathPrompt> {
        self.path_prompt
    }

    /// Submits the inline path prompt: opens or saves-as depending on its kind.
    pub fn submit_path_prompt(&mut self, path_str: &str) {
        let Some(kind) = self.path_prompt.take() else {
            return;
        };
        let path = Path::new(path_str);
        match kind {
            PathPrompt::Open => {
                let _ = self.open(path);
            }
            PathPrompt::SaveAs => {
                let _ = self.save_as(path);
            }
        }
    }

    /// Cancels the inline path prompt.
    pub fn cancel_path_prompt(&mut self) {
        self.path_prompt = None;
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
    /// pure gesture machine, unit-testable without a window. `hit` is the canvas cell
    /// plus the interactive target under it (see [`crate::canvas::hit_target`]).
    pub fn handle_pointer_frame(
        &mut self,
        held: bool,
        hit: Option<((u32, u32), HitTarget)>,
    ) -> EditOutcome {
        let outcome = match (held, self.pointer_held) {
            (true, false) => match hit {
                Some(((col, row), target)) => self.editor.on_pointer(
                    PointerEvent::Down {
                        cell: (col, row),
                        target,
                    },
                    &mut self.diagram,
                ),
                None => EditOutcome::Nothing,
            },
            (true, true) => match hit {
                Some(((col, row), target)) => self.editor.on_pointer(
                    PointerEvent::Move {
                        cell: (col, row),
                        target,
                    },
                    &mut self.diagram,
                ),
                None => EditOutcome::Nothing,
            },
            (false, true) => match hit {
                Some((_, target)) => self
                    .editor
                    .on_pointer(PointerEvent::Up { target }, &mut self.diagram),
                // Released off-canvas: drop on nothing cancels any gesture.
                None => self.editor.on_pointer(
                    PointerEvent::Up {
                        target: HitTarget::Empty,
                    },
                    &mut self.diagram,
                ),
            },
            (false, false) => EditOutcome::Nothing,
        };
        self.pointer_held = held;
        outcome
    }

    /// Keyboard edge for one frame: Delete/Backspace removes the selection, Escape
    /// cancels placement/gestures/selection. Returns a status message on change.
    /// The booleans are parsed from egui input in `ui()` so this seam stays testable
    /// without a window.
    pub fn handle_key_frame(&mut self, delete: bool, escape: bool) -> Option<String> {
        if delete {
            return status_for(self.editor.delete_selection(&mut self.diagram));
        }
        if escape {
            return status_for(self.editor.on_escape());
        }
        None
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

        // Hover: the model cell under the pointer plus the interactive target there
        // (anchors / resize handle of the selection, or connect targets while dragging).
        let hover_cell = response
            .hover_pos()
            .and_then(|p| vp.screen_to_model_cell(p.x - rect.min.x, p.y - rect.min.y));
        let hit: Option<((u32, u32), HitTarget)> = hover_cell.map(|(col, row)| {
            (
                (col, row),
                hit_target(
                    &self.diagram,
                    self.services.catalog.as_ref(),
                    self.editor.selection.as_ref(),
                    self.editor.connecting_source(),
                    (col, row),
                ),
            )
        });

        // Placement ghost at the hovered cell (model space — unbounded).
        if let (Some(info), Some((col, row))) = (&self.editor.placing, hover_cell) {
            let (x, y, w, h) = vp.cell_block(col, row, info.size.width, info.size.height);
            painter.rect_stroke(
                egui::Rect::from_min_max(Pos2::new(x, y), Pos2::new(x + w, y + h)),
                0.0,
                egui::epaint::Stroke::new(1.0, GHOST_COLOR),
                egui::epaint::StrokeKind::Middle,
            );
        }

        // Anchor dots, resize handle and the connect drag line (S-013).
        draw_overlays(painter, self, &vp, cell_px, response.hover_pos());

        // Pointer edges → editor events (AC3/AC4); report the outcome in the status bar.
        // Right-click cancels placement/gestures/selection.
        if ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Secondary)) {
            let _ = self.editor.on_escape();
            self.status = "Action cancelled".to_owned();
        }

        let held = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        if let Some(message) = status_for(self.handle_pointer_frame(held, hit)) {
            self.status = message;
        }
    }
}

/// Draws the interactive overlays (S-013): anchor dots on the selected shape — and on
/// every other shape while a connection is in flight — the resize handle on the
/// selection's bottom-right corner, and the connect drag line from the source anchor.
fn draw_overlays(
    painter: &egui::Painter,
    app: &DrawingApp,
    vp: &Viewport,
    cell_px: f32,
    cursor: Option<Pos2>,
) {
    let center_of = |col: u32, row: u32| -> Pos2 {
        let (x, y) = vp.cell_origin(col, row);
        Pos2::new(x + cell_px / 2.0, y + cell_px / 2.0)
    };

    // Anchor dots + resize handle: the *which cells* decisions live in `canvas.rs`
    // (unit-tested); this is pixels only.
    for overlay in crate::canvas::overlays(
        &app.diagram,
        app.services.catalog.as_ref(),
        app.editor.selection.as_ref(),
        app.editor.connecting_source(),
    ) {
        match overlay {
            crate::canvas::Overlay::AnchorDot((col, row)) => {
                painter.circle_filled(center_of(col, row), 3.0, ANCHOR_COLOR);
            }
            crate::canvas::Overlay::ResizeHandle((col, row)) => {
                let (x, y) = vp.cell_origin(col, row);
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        Pos2::new(x - 3.0, y - 3.0),
                        Pos2::new(x + 3.0, y + 3.0),
                    ),
                    0.0,
                    ANCHOR_COLOR,
                );
            }
        }
    }

    // Connect drag: line from the source anchor to the cursor (the source dot is not in
    // `overlays()` while connecting — it belongs to this line).
    if let Some(source) = app.editor.connecting_source()
        && let Some(Object::Shape(shape)) = app.diagram.get(source.object())
    {
        let anchor_info = app
            .services
            .catalog
            .anchors(&shape.type_id)
            .and_then(|anchors| anchors.into_iter().find(|a| a.name == source.anchor()));
        let source_cell = anchor_info.as_ref().and_then(|info| {
            crate::canvas::anchor_cell(shape, app.services.catalog.as_ref(), info)
        });
        if let (Some((col, row)), Some(cursor)) = (source_cell, cursor) {
            painter.line_segment(
                [center_of(col, row), cursor],
                egui::epaint::Stroke::new(1.5, CONNECT_LINE_COLOR),
            );
            painter.circle_filled(center_of(col, row), 3.0, ANCHOR_COLOR);
        }
    }
}

impl eframe::App for DrawingApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Toolbar + status bar (S-014): file actions on the left, last-action text on
        // the right. Ctrl+S saves without touching the mouse (read before the panel —
        // `ui.ctx()` is a borrow that must not live across `Panel::show`).
        let ctrl_s = ui
            .ctx()
            .input(|i| i.key_pressed(Key::S) && i.modifiers.ctrl);
        egui::Panel::top("statusbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open…").clicked() {
                    self.begin_path_prompt(PathPrompt::Open);
                }
                if ui.button("Save").clicked() || ctrl_s {
                    let _ = self.save_requested();
                }
                if ui.button("Save As…").clicked() {
                    self.begin_path_prompt(PathPrompt::SaveAs);
                }
                // AC3 — copy the exact canvas bytes to the clipboard (WYSIWYG).
                if ui.button("Copy").clicked() {
                    match self.text_output() {
                        Ok(text) => {
                            let chars = text.chars().count();
                            ui.ctx().copy_text(text);
                            self.status = format!("Copied {chars} character(s) to clipboard");
                        }
                        Err(e) => self.status = e,
                    }
                }
                if ui.button("Text .txt").clicked() {
                    match self.save_text() {
                        Ok(_) => {}
                        Err(e) => self.status = e,
                    }
                }
                ui.separator();
                let file = self
                    .current_file()
                    .map(|p| format!("[{}] ", p.display()))
                    .unwrap_or_default();
                ui.label(RichText::new(format!("{file}{}", self.status)).weak());
            });

            // Inline path prompt (v1: no native file dialog).
            if let Some(kind) = self.path_prompt() {
                ui.horizontal(|ui| {
                    let label = match kind {
                        PathPrompt::Open => "Open file:",
                        PathPrompt::SaveAs => "Save as:",
                    };
                    ui.label(label);
                    let mut path_str = String::new();
                    if ui.text_edit_singleline(&mut path_str).lost_focus() && !path_str.is_empty() {
                        self.submit_path_prompt(&path_str);
                    }
                    if ui.button("OK").clicked() && !path_str.is_empty() {
                        self.submit_path_prompt(&path_str);
                    }
                    if ui.button("Cancel").clicked() {
                        self.cancel_path_prompt();
                    }
                });
            }
        });

        // AC4 — the lossy-save confirmation dialog: explicit Confirm/Cancel, no silent
        // data loss (ADR 009 v4).
        if self.confirming_save() {
            let count = self.skipped_unknowns;
            egui::Window::new("Confirm save")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label(RichText::new(format!(
                        "This document was loaded with {count} newer-version statement(s) skipped. Saving will drop them permanently."
                    )));
                    ui.horizontal(|ui| {
                        if ui.button("Confirm save").clicked() {
                            let _ = self.confirm_save();
                        }
                        if ui.button("Cancel").clicked() {
                            self.cancel_confirmation();
                        }
                    });
                });
        }

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

        // Keyboard edges (S-013 AC3): Delete/Backspace removes the selection, Escape
        // cancels placement/gestures/selection.
        let ctx = ui.ctx();
        let delete = ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace));
        let escape = ctx.input(|i| i.key_pressed(Key::Escape));
        if let Some(message) = self.handle_key_frame(delete, escape) {
            self.status = message;
        }

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
        EditOutcome::Resized { id, to } => {
            Some(format!("{id} resized to {}×{}", to.width, to.height))
        }
        EditOutcome::Connecting { from } => Some(format!(
            "Connecting from {} anchor of {} — drop on another anchor (Esc cancels)",
            from.anchor(),
            from.object()
        )),
        EditOutcome::Connected { id } => Some(format!("Connector added as {id}")),
        EditOutcome::Deleted { id, removed } => {
            let connectors = removed.saturating_sub(1);
            Some(if connectors == 0 {
                format!("Deleted {id}")
            } else {
                format!("Deleted {id} and {connectors} connector(s)")
            })
        }
        EditOutcome::Cancelled => Some("Action cancelled".to_owned()),
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
            app.handle_pointer_frame(true, Some(((3, 4), HitTarget::Empty))),
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
            app.handle_pointer_frame(true, Some(((3, 4), HitTarget::Empty))),
            EditOutcome::Placed { .. }
        ));
        assert_eq!(app.handle_pointer_frame(false, None), EditOutcome::Nothing);

        // Press inside the placed box (cell (5,5) is within its 9×5 at (3,4)) and drag.
        assert_eq!(
            app.handle_pointer_frame(
                true,
                Some(((5, 5), HitTarget::Shape(ObjectId::generated(1))))
            ),
            EditOutcome::Selected(ObjectId::generated(1))
        );
        assert!(matches!(
            app.handle_pointer_frame(true, Some(((12, 8), HitTarget::Empty))),
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

    /// AC4: the canvas frame is re-rendered from the mutated model in the same frame —
    /// a move changes the exact bytes the canvas draws.
    #[test]
    fn a_move_changes_the_rendered_grid_in_the_same_frame() {
        let mut app = app();
        // Two boxes so moving one actually changes the trimmed grid.
        app.editor.select_template(box_info());
        app.handle_pointer_frame(true, Some(((0, 0), HitTarget::Empty)));
        app.handle_pointer_frame(false, None);
        app.editor.select_template(box_info());
        app.handle_pointer_frame(true, Some(((20, 0), HitTarget::Empty)));
        app.handle_pointer_frame(false, None);

        let before = app.canvas_frame().expect("renders").0;

        // Drag the first box (press at its interior cell (2,1), drag right).
        app.handle_pointer_frame(
            true,
            Some(((2, 1), HitTarget::Shape(ObjectId::generated(1)))),
        );
        let moved = app.handle_pointer_frame(true, Some(((30, 1), HitTarget::Empty)));
        assert!(
            matches!(moved, EditOutcome::Moved { .. }),
            "the drag moved it"
        );

        // Same frame: the canvas bytes are re-rendered from the mutated model.
        let (after, _, _) = app.canvas_frame().expect("renders");
        assert_ne!(before, after, "moving a shape changes the rendered grid");
    }

    /// The startup fixture path (visual test tiers) loads through the store seam.
    #[test]
    fn create_app_loads_the_startup_fixture() {
        let services = Services::native();
        let fixture =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/one-box.ad");
        let app = create_app(services, Some(&fixture));
        assert_eq!(app.diagram.len(), 1);

        // A missing fixture is reported (eprintln), not fatal — the app still starts empty.
        let services = Services::native();
        let app = create_app(
            services,
            Some(std::path::Path::new("/nonexistent/missing.ad")),
        );
        assert_eq!(app.diagram.len(), 0);
    }

    /// Keyboard seam: Delete removes the selection, Escape cancels (S-013 AC3).
    #[test]
    fn key_frames_delete_and_cancel() {
        let mut app = app();
        app.editor.select_template(box_info());
        app.handle_pointer_frame(true, Some(((0, 0), HitTarget::Empty)));
        app.handle_pointer_frame(false, None);

        // Delete removes the placed (and selected) box.
        assert!(
            app.handle_key_frame(true, false)
                .unwrap()
                .contains("Deleted")
        );
        assert_eq!(app.diagram.len(), 0);

        // Escape with nothing to cancel is a no-op.
        assert_eq!(app.handle_key_frame(false, true), None);
    }

    #[test]
    fn open_replaces_the_diagram_and_remembers_the_file() {
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
        assert!(matches!(
            app.open(std::path::Path::new("t.ad")),
            crate::fileio::OpenOutcome::Loaded {
                skipped_unknowns: 0,
                ..
            }
        ));
        assert_eq!(app.diagram.len(), 1);
        assert_eq!(
            app.current_file().map(|p| p.as_path()),
            Some(Path::new("t.ad"))
        );

        // A missing file is reported in the status bar; the model is kept.
        let outcome = app.open(std::path::Path::new("missing.ad"));
        assert!(matches!(
            outcome,
            crate::fileio::OpenOutcome::IoError { .. }
        ));
        assert_eq!(app.diagram.len(), 1);
    }

    #[test]
    fn status_for_covers_every_outcome() {
        use diagram_model::anchors::AnchorRef;

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
            status_for(EditOutcome::Resized {
                id: ObjectId::generated(0),
                to: CellSize::new(3, 2).expect("valid")
            })
            .unwrap()
            .contains("3×2")
        );
        assert!(
            status_for(EditOutcome::Connecting {
                from: AnchorRef::new(ObjectId::generated(0), "e").expect("valid")
            })
            .unwrap()
            .contains("e anchor")
        );
        assert!(
            status_for(EditOutcome::Connected {
                id: ObjectId::generated(9)
            })
            .unwrap()
            .contains("obj-9")
        );
        assert_eq!(
            status_for(EditOutcome::Deleted {
                id: ObjectId::generated(0),
                removed: 1
            })
            .as_deref(),
            Some("Deleted obj-0")
        );
        assert!(
            status_for(EditOutcome::Deleted {
                id: ObjectId::generated(0),
                removed: 3
            })
            .unwrap()
            .contains("2 connector")
        );
        assert_eq!(
            status_for(EditOutcome::Cancelled).as_deref(),
            Some("Action cancelled")
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
