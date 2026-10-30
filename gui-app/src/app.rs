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
use diagram_model::geometry::CellSize;
use diagram_model::{Diagram, objects::Object};
use eframe::egui::{self, Align2, Color32, FontFamily, FontId, Key, Pos2, RichText, Sense};

use crate::canvas::{HitTarget, Viewport, hit_target};
use crate::editor::{EditOutcome, EditorState, Placing, PointerEvent};
use crate::exportio::{self, PngExport};
use crate::template_editor::TemplateEditorSession;

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
    /// Collecting a path for the SVG export (S-015 FR5).
    ExportSvg,
    /// Collecting a path for the PNG export (S-015 FR5/RS4).
    ExportPng,
}

/// The inline path prompt's state (v1: no native file dialog).
///
/// The typed path lives in **app state**, not in a per-frame local: egui's `TextEdit`
/// does not store text — it only edits the string bound to it. Re-creating the string
/// every frame discarded every typed character (the "nothing happens when I type" bug).
#[derive(Debug, Clone)]
pub(crate) struct PathPromptState {
    /// What the prompt submits to.
    pub kind: PathPrompt,
    /// The path being typed — prefilled for exports, empty otherwise.
    pub path: String,
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
    /// An inline Open/Save-As path prompt is showing — its kind and the typed path.
    path_prompt: Option<PathPromptState>,
    /// Collection files skipped at startup (S-016 AC2) — shown in the sidebar, never
    /// fatal: startup always succeeds with at least the built-ins.
    collection_warnings: Vec<String>,
    /// The open template editor session (S-017); `None` when the window is closed.
    template_editor: Option<TemplateEditorSession>,
    /// In-flight PNG export (S-015 RS4): ticked once per frame while the worker
    /// rasterizes; the UI stays responsive meanwhile (ADR 012).
    png_export: Option<PngExport>,
    /// Sink for the in-flight PNG job — where the bytes are written on completion.
    pending_png_path: Option<PathBuf>,
}

/// Builds the app from services, an optional startup fixture (the visual test tiers
/// pass a `.ad` path as the first CLI argument) and the collection-load warnings from
/// startup (S-016 AC2). Load failures are reported, not fatal — the user can still
/// open a file later.
#[must_use]
pub fn create_app(
    services: Services,
    fixture: Option<&Path>,
    collection_warnings: Vec<String>,
) -> DrawingApp {
    let mut app = DrawingApp::new(services);
    app.collection_warnings = collection_warnings;
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
            collection_warnings: Vec::new(),
            template_editor: None,
            png_export: None,
            pending_png_path: None,
        }
    }

    // ── File IO (S-014) — pure flows in [`crate::fileio`], this is the glue ───────

    /// The file the current model was loaded from (`Save` targets it).
    #[must_use]
    pub fn current_file(&self) -> Option<&PathBuf> {
        self.current_file.as_ref()
    }

    /// The service seam (catalog, renderer, exporter, stores). Read-only view for the
    /// GUI tests that assert on registered templates.
    #[cfg(test)]
    #[must_use]
    pub fn services(&self) -> &Services {
        &self.services
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
        let path = match kind {
            // Exports default to the document's sibling path (S-015); open/save start empty.
            PathPrompt::ExportSvg => self.default_export_path("svg"),
            PathPrompt::ExportPng => self.default_export_path("png"),
            PathPrompt::Open | PathPrompt::SaveAs => String::new(),
        };
        self.path_prompt = Some(PathPromptState { kind, path });
    }

    /// The active path prompt's kind, if any (test support; the top bar reads the
    /// field directly).
    #[cfg(test)]
    #[must_use]
    pub fn path_prompt(&self) -> Option<PathPrompt> {
        self.path_prompt.as_ref().map(|p| p.kind)
    }

    /// The active path prompt's typed value (test support).
    #[cfg(test)]
    #[must_use]
    pub fn path_prompt_value(&self) -> Option<String> {
        self.path_prompt.as_ref().map(|p| p.path.clone())
    }

    /// The open template editor's draft name (test support).
    #[cfg(test)]
    #[must_use]
    pub fn template_editor_name(&self) -> Option<String> {
        self.template_editor.as_ref().map(|s| s.draft.name.clone())
    }

    /// True while a text object is being edited inline (S-018; test support).
    #[cfg(test)]
    pub fn is_editing(&self) -> bool {
        self.editor.editing.is_some()
    }

    /// Submits the inline path prompt: opens or saves-as depending on its kind.
    pub fn submit_path_prompt(&mut self, path_str: &str) {
        let Some(prompt) = self.path_prompt.take() else {
            return;
        };
        let kind = prompt.kind;
        let path = Path::new(path_str);
        match kind {
            PathPrompt::Open => {
                let _ = self.open(path);
            }
            PathPrompt::SaveAs => {
                let _ = self.save_as(path);
            }
            // AC1/AC2 — a failed export shows a notice, never fails silently.
            PathPrompt::ExportSvg => {
                if let Err(e) = self.export_svg_to(path) {
                    self.status = format!("SVG export failed: {e}");
                }
            }
            PathPrompt::ExportPng => {
                if let Err(e) = self.begin_png_export(path) {
                    self.status = format!("PNG export failed: {e}");
                }
            }
        }
    }

    // ── Export (S-015) — pure flows in [`crate::exportio`], this is the glue ─────

    /// AC1 — synchronous SVG export: the exact exporter bytes go to `path`. The
    /// bytes are produced by the same render + `to_svg` call that feeds the canvas
    /// (WYSIWYG). Native only; wasm has no filesystem (S-019).
    pub fn export_svg_to(&mut self, path: &Path) -> Result<(), String> {
        let bytes = exportio::svg_bytes(&self.services, &self.diagram)?;
        exportio::write_bytes_file(path, bytes.as_bytes())?;
        self.status = format!("SVG saved to {}", path.display());
        Ok(())
    }

    /// AC2/RS4 — starts the non-blocking PNG export: submits the grid to the
    /// [`PngWorker`] (ADR 012) and remembers the sink. The UI shows an
    /// "exporting" state; completion arrives via [`Self::tick_png_export`].
    pub fn begin_png_export(&mut self, path: &Path) -> Result<(), String> {
        let export = PngExport::start(&self.services, &self.diagram)?;
        self.png_export = Some(export);
        self.pending_png_path = Some(path.to_path_buf());
        self.status = "Exporting PNG…".to_owned();
        Ok(())
    }

    /// AC2/RS4 — polls the in-flight PNG job once (non-blocking). Called from
    /// [`Self::ui`] every frame; on completion it writes the exact bytes to the
    /// sink and clears the "exporting" state, on failure it reports the message.
    pub fn tick_png_export(&mut self) {
        let Some(export) = self.png_export.as_mut() else {
            return;
        };
        let Some(tick) = export.tick() else {
            return; // still exporting
        };
        self.png_export = None;
        let Some(path) = self.pending_png_path.take() else {
            return;
        };
        match tick {
            exportio::PngTick::Done(bytes) => match exportio::write_bytes_file(&path, &bytes) {
                Ok(()) => self.status = format!("PNG saved to {}", path.display()),
                Err(msg) => self.status = msg,
            },
            exportio::PngTick::Failed(msg) => self.status = format!("PNG export failed: {msg}"),
        }
    }

    /// The prefilled default for an export prompt: the document's sibling path with
    /// the given extension (S-015); empty when there is no current file.
    fn default_export_path(&self, ext: &str) -> String {
        self.current_file()
            .map(|p| exportio::export_output_path(p, ext).display().to_string())
            .unwrap_or_default()
    }

    /// AC3 — the whole-diagram scale control (FR8): clamped to [1, MAX_GUI_SCALE];
    /// the model enforces ≥ 1 anyway. The canvas and every export follow it.
    pub fn change_scale(&mut self, delta: i32) {
        let current = self.diagram.scale();
        let next = (current as i32 + delta).clamp(1, exportio::MAX_GUI_SCALE as i32) as u32;
        if next != current {
            // Clamped to ≥ 1 above, so the model invariant cannot fire.
            self.diagram.set_scale(next).expect("scale clamped to ≥ 1");
            self.status = format!("Scale ×{next}");
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

    /// Keyboard edges for one frame, with dialog priority: while an inline text edit
    /// is open only Escape applies (it cancels the edit); while a path prompt is open
    /// Delete/Backspace belong to the field and Escape closes it; while the template
    /// editor window is open Escape closes it and a focused text field swallows the
    /// rest. Only with no dialog open do Delete/Backspace remove the selection and
    /// Escape cancel canvas gestures. Returns a status message on change.
    pub fn handle_global_keys(
        &mut self,
        delete: bool,
        escape: bool,
        text_focused: bool,
    ) -> Option<String> {
        // Escape closes the innermost open thing first: a prompt opened after an
        // inline edit started yields to Escape before the edit does (cancelling the
        // edit would discard its draft while the user is still looking at the prompt).
        if self.path_prompt.is_some() {
            return if escape {
                self.cancel_path_prompt();
                Some("Cancelled".to_owned())
            } else {
                // Delete/Backspace edit the path, not the diagram.
                None
            };
        }
        if self.editor.editing.is_some() {
            // The draft field owns every other key (typing, backspacing the text).
            return if escape {
                status_for(self.editor.on_escape())
            } else {
                None
            };
        }
        if self.template_editor.is_some() && escape {
            self.close_template_editor();
            return Some("Template editor closed".to_owned());
        }
        if text_focused {
            // A dialog's text field is focused (template editor name/collection/cell
            // input): its keys are its own.
            return None;
        }
        self.handle_key_frame(delete, escape)
    }

    /// Keyboard edge for one frame: Delete/Backspace removes the selection, Escape
    /// cancels placement/gestures/selection. Returns a status message on change.
    /// The booleans are parsed from egui input in `ui()` so this seam stays testable
    /// without a window.
    pub fn handle_key_frame(&mut self, delete: bool, escape: bool) -> Option<String> {
        // While an inline text edit is open, Delete/Backspace belong to the text field
        // (editing the draft), not to object deletion (S-018). Escape still cancels the
        // edit — `on_escape` checks it before anything else.
        if self.editor.editing.is_some() {
            if escape {
                // `on_escape` checks the inline edit first and cancels it.
                return status_for(self.editor.on_escape());
            }
            return None; // Delete/Backspace belong to the text field, not object deletion
        }
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
        // The cell must be as wide as the glyph advance AND as tall as the font's
        // *line height* — not just the advance. Square (advance-sized) cells make
        // adjacent text rows overlap by ~half their height and the canvas text is
        // unreadable (the "invisible text" bug).
        let cell_w = (sample.size().x / 10.0).max(4.0);
        let cell_h = sample.size().y.max(cell_w);
        // The viewport maps MODEL space to canvas-RELATIVE pixels; the trimmed grid
        // text starts at the content origin within it, so rendered cells and model
        // cells coincide (the same mapping hit-testing uses).
        let vp = Viewport::new(MARGIN_PX, cell_w, cell_h);
        let (origin_x, origin_y) = self.content_origin();

        let rect = ui.max_rect();
        let response = ui.interact(rect, egui::Id::unique("canvas"), Sense::click());
        // ALL canvas painting goes through a painter clipped to the canvas rect.
        // egui 0.37 painters draw in SCREEN coordinates — `painter_at` only clips,
        // it does NOT offset — so every canvas-relative point below is shifted by
        // the canvas origin (`off`) before being handed to the painter. Drawing
        // raw local coordinates would put content near the model origin outside
        // the clip and make it invisible (the "invisible text" bug).
        let painter = ui.painter_at(rect);
        let off = |p: Pos2| p + rect.min.to_vec2();
        let off_rect = |r: egui::Rect| r.translate(rect.min.to_vec2());

        // Subtle background grid lines over the content extent.
        for col in origin_x..=origin_x + width {
            let x = vp.cell_origin(col, 0).0;
            painter.line_segment(
                [
                    off(Pos2::new(x, vp.cell_origin(0, origin_y).1)),
                    off(Pos2::new(x, vp.cell_origin(0, origin_y + height).1)),
                ],
                egui::epaint::Stroke::new(1.0, GRID_LINE_COLOR),
            );
        }
        for row in origin_y..=origin_y + height {
            let y = vp.cell_origin(0, row).1;
            painter.line_segment(
                [
                    off(Pos2::new(vp.cell_origin(origin_x, 0).0, y)),
                    off(Pos2::new(vp.cell_origin(origin_x + width, 0).0, y)),
                ],
                egui::epaint::Stroke::new(1.0, GRID_LINE_COLOR),
            );
        }

        // The exact text bytes of the resolved grid — one line per row (AC2), offset by
        // the content origin in BOTH axes: the trimmed text starts at the content origin,
        // so rendered cells coincide with model cells (the same mapping hit-testing uses).
        for (row, line) in text.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            let pos = off(Pos2::new(
                vp.cell_origin(origin_x, 0).0,
                vp.cell_origin(0, origin_y + row as u32).1,
            ));
            painter.text(pos, Align2::LEFT_TOP, line, font_id.clone(), TEXT_COLOR);
        }

        // Selection overlay around the selected object's bounds — shapes and free text
        // (S-018; text uses its hit extent).
        let selected_extent =
            self.editor
                .selection
                .as_ref()
                .and_then(|id| match self.diagram.get(id) {
                    Some(Object::Shape(shape)) => Some((shape.pos, shape.size)),
                    Some(Object::Text(text)) => {
                        let (w, h) = crate::canvas::text_hit_extent(text);
                        Some((
                            text.pos,
                            CellSize::new(w, h).expect("hit extent is a valid size"),
                        ))
                    }
                    _ => None,
                });
        if let Some((pos, size)) = selected_extent {
            let (x, y, w, h) = vp.cell_block(pos.x, pos.y, size.width, size.height);
            painter.rect_stroke(
                off_rect(egui::Rect::from_min_max(
                    Pos2::new(x, y),
                    Pos2::new(x + w, y + h),
                )),
                0.0,
                egui::epaint::Stroke::new(1.5, SELECTION_COLOR),
                egui::epaint::StrokeKind::Middle,
            );
        }

        // Pointer over the canvas: canvas-relative position → model cell plus the
        // interactive target there (anchors / resize handle of the selection, or
        // connect targets while dragging).
        //
        // Primary source is `response.hover_pos()`, but egui suppresses hover on move
        // frames while a button is held — which would freeze body-drag / resize /
        // connect gestures mid-drag (the "handle drawn where it does not work" bug).
        // While we are already in a gesture (`pointer_held`) we therefore fall back to
        // the raw pointer position, gated to the canvas rect so covering widgets
        // (inline editor, dialogs) cannot feed the canvas outside of a running gesture.
        let held = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        let cursor = response
            .hover_pos()
            .or_else(|| {
                if held && self.pointer_held {
                    ctx.input(|i| i.pointer.interact_pos())
                        .filter(|&p| rect.contains(p))
                } else {
                    None
                }
            })
            .map(|p| (p - rect.min).to_pos2());
        let hover_cell = cursor.and_then(|p| vp.screen_to_model_cell(p.x, p.y));
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

        // Placement ghost at the hovered cell (model space — unbounded). The text tool
        // ghosts a 1×1 anchor marker — the placed object grows with its content.
        if let (Some(tool), Some((col, row))) = (&self.editor.placing, hover_cell) {
            let extent = match tool {
                Placing::Shape(info) => info.size,
                Placing::Text => CellSize::new(1, 1).expect("1×1 is a valid size"),
            };
            let (x, y, w, h) = vp.cell_block(col, row, extent.width, extent.height);
            painter.rect_stroke(
                off_rect(egui::Rect::from_min_max(
                    Pos2::new(x, y),
                    Pos2::new(x + w, y + h),
                )),
                0.0,
                egui::epaint::Stroke::new(1.0, GHOST_COLOR),
                egui::epaint::StrokeKind::Middle,
            );
        }

        // Anchor dots, resize handle and the connect drag line (S-013).
        draw_overlays(&painter, self, &vp, cursor, rect.min);

        // Pointer edges → editor events (AC3/AC4); report the outcome in the status bar.
        // Right-click cancels placement/gestures/selection.
        if ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Secondary)) {
            let _ = self.editor.on_escape();
            self.status = "Action cancelled".to_owned();
        }

        if let Some(message) = status_for(self.handle_pointer_frame(held, hit.clone())) {
            self.status = message;
        }

        // Double click (S-018 AC1): open inline editing of a text object under the pointer.
        if ctx.input(|i| {
            i.pointer
                .button_double_clicked(egui::PointerButton::Primary)
        }) && let Some(message) = status_for(self.handle_pointer_double(hit))
        {
            self.status = message;
        }

        // Inline text editing (S-018 AC1): a TextEdit bound to the editor's draft,
        // floating over the text object. Plain Enter or blur commits, Escape cancels
        // (routed through `handle_key_frame` → `EditorState::on_escape`).
        if let Some(id) = self.editor.editing.clone()
            && let Some(Object::Text(text)) = self.diagram.get(&id)
        {
            let (enter, shift) =
                ctx.input(|i| (i.key_pressed(egui::Key::Enter), i.modifiers.shift));
            // Check Enter BEFORE showing the field: egui's multiline TextEdit would
            // otherwise insert a newline this frame and commit it. Shift+Enter is
            // reserved for inserting line breaks inside the text.
            if enter && !shift {
                if let Some(message) = status_for(self.editor.commit_edit(&mut self.diagram)) {
                    self.status = message;
                }
            } else {
                let (w, h) = crate::canvas::text_hit_extent(text);
                // A little breathing room so typing does not clip.
                let (x, y, cw, ch) = vp.cell_block(text.pos.x, text.pos.y, w.max(8), h.max(1));
                // Canvas-relative → screen coordinates for the floating area.
                let pos = Pos2::new(x + rect.min.x, y + rect.min.y);
                egui::Area::new(egui::Id::unique("inline-text-edit"))
                    .fixed_pos(pos)
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        let response = ui.add_sized(
                            [cw, ch.max(FONT_SIZE * 1.6)],
                            egui::TextEdit::multiline(&mut self.editor.edit_draft),
                        );
                        // The field must own the keyboard while editing — it appears
                        // without a click (right after placement or a double click).
                        // Request focus every frame, not just once: egui attributes
                        // the opening click to the canvas (installed before this area
                        // exists) and surrenders the field's focus on the release
                        // frame — without re-requesting, the editor would commit the
                        // placeholder and close immediately after placement.
                        // Two exceptions:
                        // * Yield when another text field holds focus: `request_focus`
                        //   writes immediately and this area draws last in the frame,
                        //   so an unconditional re-request would steal the keyboard
                        //   back every frame — typing in the path prompt or a
                        //   template-editor field while an edit is open landed in the
                        //   canvas draft. egui's dead-man's switch drops focus on
                        //   fields that stop being drawn, so yielding cannot stick.
                        // * Skip the re-request on the frame the user clicked away:
                        //   `interact` (inside the TextEdit call above) already ran
                        //   egui's surrender-on-outside-click, and re-requesting right
                        //   after would undo it — a click on the path prompt or a
                        //   dialog field would never take the keyboard.
                        let other_text_focused = ctx
                            .memory(|m| m.focused())
                            .is_some_and(|id| id != response.id)
                            && ctx.text_edit_focused();
                        let clicked_away =
                            ctx.input(|i| i.pointer.any_click()) && !response.hovered();
                        if !other_text_focused && !clicked_away {
                            response.request_focus();
                        }
                        // Blur without pointer activity (e.g. tabbing away): commit.
                        // Guarded against pointer frames: during the opening click's
                        // release egui also reports `lost_focus` spuriously. Also skip
                        // while another text edit holds the keyboard — that is a yield,
                        // not a blur; the field that took focus commits this edit via
                        // its own gained-focus hook (below and in the template editor).
                        if response.lost_focus()
                            && self.editor.editing.is_some()
                            && !other_text_focused
                            && !ctx.input(|i| i.pointer.any_down() || i.pointer.any_click())
                        {
                            let _ = self.editor.commit_edit(&mut self.diagram);
                        }
                    });
            }
        }
    }

    /// Double-click edge (S-018 AC1): opens inline editing of the text object under the
    /// pointer; shapes and empty space are not editable in v1.
    pub fn handle_pointer_double(&mut self, hit: Option<((u32, u32), HitTarget)>) -> EditOutcome {
        match hit {
            Some(((col, row), target)) => self.editor.on_pointer(
                PointerEvent::DoubleClick {
                    cell: (col, row),
                    target,
                },
                &mut self.diagram,
            ),
            None => EditOutcome::Nothing,
        }
    }

    // ── Template editor (S-017) — state in [`crate::template_editor`], glue here ──

    /// AC3 entry point: opens the editor for a new template. The collection defaults
    /// to the first loaded one (or `myshapes` when none is loaded) and can be changed
    /// in the window.
    pub fn open_new_template(&mut self) {
        let groups = crate::sidebar::group_templates(&self.services.catalog.list());
        let collection = groups
            .iter()
            .find(|g| !g.builtins)
            .map(|g| g.label.clone())
            .unwrap_or_else(|| "myshapes".to_owned());
        self.template_editor = Some(TemplateEditorSession::new_blank(collection, "new"));
    }

    /// AC3 entry point: opens the editor for an existing collection template (editing
    /// reuses the same flow as creating). Built-ins are not editable (ADR 007 — the
    /// built-in registry is code, not a file).
    pub fn open_template_editor(&mut self, type_id: &str) {
        let Some(info) = self.services.catalog.get(type_id) else {
            return;
        };
        if info.collection.is_none() {
            return; // built-in — nothing to write back
        }
        let anchors = self.services.catalog.anchors(type_id).unwrap_or_default();
        self.template_editor = Some(TemplateEditorSession::from_existing(&info, anchors));
    }

    /// AC2/AC3 — validates the draft and registers it through the seam. On success
    /// the window closes and the template is immediately placeable (the sidebar
    /// re-reads the catalog every frame). On validation failure the inline hint is
    /// set and nothing is registered.
    pub fn save_template(&mut self) -> Result<String, String> {
        let Some(session) = self.template_editor.as_mut() else {
            return Err("no template editor open".to_owned());
        };
        let errors = session.draft.validation_errors();
        if !errors.is_empty() {
            session.draft.rejection = Some(errors.join("; "));
            return Err(errors.join("; "));
        }
        let collection = session.draft.collection.clone();
        let template = session.draft.to_new_template();
        let name = template.name.clone();
        self.services
            .catalog
            .register(&collection, template)
            .map_err(|e| e.to_string())?;
        self.template_editor = None;
        Ok(format!("Template {name} saved to collection {collection}"))
    }

    /// Closes the editor without saving (Cancel / window X).
    pub fn close_template_editor(&mut self) {
        self.template_editor = None;
    }

    /// Draws the template editor window (S-017 AC1): name/collection fields, the cell
    /// grid (click = select a cell, or toggle an anchor in anchor mode), live commit
    /// of the single-character input, the anchor list, and a live preview — the exact
    /// bytes [`Renderer::preview_pattern`] resolves for the current grid.
    fn draw_template_editor(&mut self, ui: &mut egui::Ui) {
        let Some(session) = self.template_editor.as_mut() else {
            return;
        };
        let draft = &mut session.draft;

        let mut field_gained_focus = false;
        ui.horizontal(|ui| {
            ui.label("Collection:");
            if ui
                .text_edit_singleline(&mut draft.collection)
                .gained_focus()
            {
                field_gained_focus = true;
            }
            ui.label("Name:");
            if ui.text_edit_singleline(&mut draft.name).gained_focus() {
                field_gained_focus = true;
            }
        });
        // A field taking the keyboard blurs an open inline edit → commit it (S-018 AC1).
        // Gated on Escape so a fast Escape that closes this window does not commit as
        // a side effect — that frame's Escape is meant for the window.
        if field_gained_focus
            && self.editor.editing.is_some()
            && !ui.input(|i| i.key_pressed(Key::Escape))
        {
            let _ = self.editor.commit_edit(&mut self.diagram);
        }
        if let Some(rejection) = &draft.rejection {
            // AC1/AC2 — inline hint for rejected input / blocked save.
            ui.label(RichText::new(rejection.clone()).color(ERROR_COLOR));
        }

        ui.horizontal(|ui| {
            ui.checkbox(
                &mut session.anchor_mode,
                "Anchor mode (click cells to add/remove anchors)",
            );
            if ui.button("Clear cell").clicked()
                && let Some((x, y)) = session.selected_cell
            {
                draft.clear_cell(x, y);
                session.cell_input.clear();
            }
        });

        // The grid: one row of cells per template row. Empty cells show a faint dot;
        // anchored cells are tinted; the selected cell is highlighted.
        egui::ScrollArea::both().max_height(200.0).show(ui, |ui| {
            for y in 0..draft.height() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{y:>2}")).monospace().weak());
                    for x in 0..draft.width() {
                        let ch = draft
                            .rows
                            .get(y as usize)
                            .and_then(|row| row.chars().nth(x as usize))
                            .unwrap_or(' ');
                        let anchored = draft
                            .anchors
                            .iter()
                            .any(|a| a.position.x == x && a.position.y == y);
                        let selected = session.selected_cell == Some((x, y));
                        let fill = if anchored {
                            ANCHOR_COLOR.gamma_multiply(0.45)
                        } else if selected {
                            SELECTION_COLOR.gamma_multiply(0.35)
                        } else {
                            Color32::TRANSPARENT
                        };
                        let glyph = if ch == ' ' {
                            RichText::new("·")
                                .monospace()
                                .color(Color32::from_gray(100))
                        } else {
                            RichText::new(ch.to_string()).monospace().color(TEXT_COLOR)
                        };
                        if ui
                            .add_sized([16.0, 18.0], egui::Button::new(glyph).fill(fill))
                            .clicked()
                        {
                            if session.anchor_mode {
                                draft.toggle_anchor(x, y);
                            } else {
                                session.selected_cell = Some((x, y));
                                session.cell_input = if ch == ' ' {
                                    String::new()
                                } else {
                                    ch.to_string()
                                };
                            }
                        }
                    }
                });
            }
        });

        // Character input for the selected cell — live commit on every change.
        ui.horizontal(|ui| {
            let hint = session
                .selected_cell
                .map_or("select a cell to edit".to_owned(), |(x, y)| {
                    format!("cell ({x}, {y})")
                });
            ui.label(RichText::new(hint).weak());
            if ui
                .add_sized(
                    [24.0, 24.0],
                    egui::TextEdit::singleline(&mut session.cell_input),
                )
                .changed()
                && let Some((x, y)) = session.selected_cell
            {
                match session.cell_input.chars().next() {
                    Some(c) => {
                        // `false` = rejected input; the inline hint is shown.
                        let _ = draft.set_cell(x, y, c);
                    }
                    None => draft.clear_cell(x, y),
                }
            }
        });

        // The anchor list (names with positions).
        let anchors = if draft.anchors.is_empty() {
            "none".to_owned()
        } else {
            draft
                .anchors
                .iter()
                .map(|a| format!("{}@({},{})", a.name, a.position.x, a.position.y))
                .collect::<Vec<_>>()
                .join("  ")
        };
        ui.label(RichText::new(format!("Anchors: {anchors}")).weak());

        // AC1 — live preview: the exact bytes the renderer would stamp.
        match services_preview(&self.services, &draft.rows) {
            Ok(text) => ui.label(
                RichText::new(if text.is_empty() {
                    "(blank)".to_owned()
                } else {
                    text
                })
                .monospace()
                .color(TEXT_COLOR),
            ),
            Err(e) => ui.label(RichText::new(format!("preview error: {e}")).color(ERROR_COLOR)),
        };

        // The last use of `session` is above, so the borrow of
        // `self.template_editor` has ended (NLL) and the buttons may use `self`.
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                match self.save_template() {
                    Ok(message) => self.status = message,
                    Err(e) => self.status = e, // inline hint is set in the window
                }
            }
            if ui.button("Cancel").clicked() {
                self.close_template_editor();
            }
        });
    }
}

/// Draws the interactive overlays (S-013): anchor dots on the selected shape — and on
/// every other shape while a connection is in flight — the resize handle on the
/// selection's bottom-right cell, and the connect drag line from the source anchor.
///
/// `painter` is already offset by the canvas rect (see [`DrawingApp::draw_canvas`]),
/// so all positions here — including `cursor` — are canvas-relative.
fn draw_overlays(
    painter: &egui::Painter,
    app: &DrawingApp,
    vp: &Viewport,
    cursor: Option<Pos2>,
    origin: Pos2,
) {
    // Canvas-relative → screen coordinates (the painter draws in screen space).
    let off = |p: Pos2| p + origin.to_vec2();
    let center_of = |col: u32, row: u32| -> Pos2 {
        let (x, y) = vp.center_of(col, row);
        off(Pos2::new(x, y))
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
                // The marker sits on the handle cell's CENTER — exactly the cell
                // `hit_target` answers with a resize gesture for.
                let c = center_of(col, row); // already in screen coordinates
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        Pos2::new(c.x - 3.0, c.y - 3.0),
                        Pos2::new(c.x + 3.0, c.y + 3.0),
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
                [center_of(col, row), off(cursor)],
                egui::epaint::Stroke::new(1.5, CONNECT_LINE_COLOR),
            );
            painter.circle_filled(center_of(col, row), 3.0, ANCHOR_COLOR);
        }
    }
}

/// The editor's live preview (S-017 AC1): the resolved pattern of the draft grid as
/// text — `Err` when the grid cannot be resolved.
fn services_preview(services: &Services, rows: &[String]) -> Result<String, String> {
    let pattern = services
        .renderer
        .preview_pattern(rows)
        .map_err(|e| e.to_string())?;
    Ok(services.exporter.to_text(&pattern))
}

impl eframe::App for DrawingApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

impl DrawingApp {
    /// The full UI frame without the eframe wrapper — the same code path as
    /// [`eframe::App::ui`], exposed so the GUI can be driven headlessly in tests
    /// (`tests/gui_tests.rs`) without a window or GPU.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        // AC2/RS4 — poll the in-flight PNG job once per frame (non-blocking); on
        // completion the exact bytes are written and the "exporting" state clears.
        self.tick_png_export();

        // Toolbar + status bar (S-014): file actions on the left, last-action text on
        // the right. Ctrl+S saves without touching the mouse (read before the panel —
        // `ui.ctx()` is a borrow that must not live across `Panel::show`).
        // Global shortcuts are suppressed while a text input owns the keyboard (an
        // inline edit or an open path prompt), so Ctrl+S never surprises a user who
        // is typing.
        let ctrl_s = ui
            .ctx()
            .input(|i| i.key_pressed(Key::S) && i.modifiers.ctrl)
            && self.path_prompt.is_none()
            && self.editor.editing.is_none();
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
                // AC1 — SVG export (synchronous): the exact exporter bytes to file.
                if ui.button("Export SVG…").clicked() {
                    self.begin_path_prompt(PathPrompt::ExportSvg);
                }
                // AC2/RS4 — PNG export starts non-blocking (ADR 012); the status
                // bar shows "exporting" until the worker's job completes.
                let exporting = self.png_export.is_some();
                if ui
                    .add_enabled(!exporting, egui::Button::new("Export PNG…"))
                    .clicked()
                {
                    self.begin_path_prompt(PathPrompt::ExportPng);
                }
                ui.separator();
                let file = self
                    .current_file()
                    .map(|p| format!("[{}] ", p.display()))
                    .unwrap_or_default();
                ui.label(RichText::new(format!("{file}{}", self.status)).weak());

                // AC3/FR8 — whole-diagram scale control, top-right: the canvas and
                // every export follow it; the model enforces ≥ 1.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Scale ×{}", self.diagram.scale()));
                    if ui.button("−").clicked() {
                        self.change_scale(-1);
                    }
                    if ui.button("+").clicked() {
                        self.change_scale(1);
                    }
                });
            });

            // Inline path prompt (v1: no native file dialog). The typed path lives in
            // `self.path_prompt` — egui's TextEdit does not store text, it only edits
            // the string bound to it, so a per-frame local would discard every
            // keystroke. Committing happens on Enter/OK (blur deliberately does NOT
            // commit: clicking away keeps the half-typed path for another look).
            if self.path_prompt.is_some() {
                let (submit, cancel, path) = ui
                    .horizontal(|ui| {
                        let prompt = self.path_prompt.as_mut().expect("checked above");
                        let label = match prompt.kind {
                            PathPrompt::Open => "Open file:",
                            PathPrompt::SaveAs => "Save as:",
                            PathPrompt::ExportSvg => "Export SVG as:",
                            PathPrompt::ExportPng => "Export PNG as:",
                        };
                        ui.label(label);
                        // Per-kind widget id: egui persists text-edit state (cursor,
                        // selection) by id, so without this a cancelled prompt would leak
                        // them into the next prompt of a different kind (S-015).
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut prompt.path)
                                .id_source(egui::Id::unique(("path_prompt", prompt.kind as u8)))
                                .desired_width(240.0),
                        );
                        // The field owns the keyboard as soon as it appears — no click
                        // needed before typing. Yield when another text field holds
                        // focus (the user clicked one deliberately): re-requesting
                        // whenever we lack focus would fight it back on every frame,
                        // because a later-drawn field's request overwrites ours.
                        let other_text_focused = ui
                            .ctx()
                            .memory(|m| m.focused())
                            .is_some_and(|id| id != response.id)
                            && ui.ctx().text_edit_focused();
                        if !response.has_focus() && !other_text_focused {
                            response.request_focus();
                            // Taking the keyboard blurs an open inline edit → commit it
                            // (S-018 AC1: Enter/blur commits). Gated on Escape so a fast
                            // Escape that closes this prompt does not commit as a side
                            // effect — that frame's Escape is meant for the prompt.
                            if self.editor.editing.is_some()
                                && !ui.input(|i| i.key_pressed(Key::Escape))
                            {
                                let _ = self.editor.commit_edit(&mut self.diagram);
                            }
                        }
                        let enter = ui.input(|i| i.key_pressed(Key::Enter)) && response.has_focus();
                        let ok = ui.button("OK").clicked();
                        let cancel = ui.button("Cancel").clicked();
                        (enter || ok, cancel, prompt.path.clone())
                    })
                    .inner;
                if submit && !path.is_empty() {
                    self.submit_path_prompt(&path);
                } else if cancel {
                    self.cancel_path_prompt();
                }
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

        // Shape template sidebar (FR3 read model from the catalog, S-016 AC1):
        // built-in group + one group per loaded collection, each shape with a
        // small preview of its glyph pattern.
        let groups = crate::sidebar::group_templates(&self.services.catalog.list());
        let (placing_id, placing_text) = match self.editor.placing.as_ref() {
            Some(Placing::Shape(info)) => (Some(info.type_id.clone()), false),
            Some(Placing::Text) => (None, true),
            None => (None, false),
        };
        egui::Panel::left("templates")
            .size_range(SIDEBAR_PX..=SIDEBAR_PX)
            .show(ui, |ui| {
                // Scrollable: a few loaded collections easily exceed one screen.
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // S-018 — the free-text tool: one click places a text object and
                    // opens inline editing of it immediately.
                    if ui.selectable_label(placing_text, "T  Text").clicked() {
                        self.editor.select_text_tool();
                        self.status = "Placing text — click the canvas".to_owned();
                    }
                    ui.separator();
                    ui.heading("Shapes");
                    // S-017 AC3 — entry point: create a new template.
                    if ui.button("＋ New template…").clicked() {
                        self.open_new_template();
                    }
                    for warning in &self.collection_warnings {
                        ui.label(RichText::new(warning.clone()).color(ERROR_COLOR).weak());
                    }
                    for group in &groups {
                        ui.add_space(6.0);
                        ui.strong(&group.label);
                        for info in &group.shapes {
                            let selected = Some(info.type_id.as_str()) == placing_id.as_deref();
                            if ui.selectable_label(selected, &info.name).clicked() {
                                self.editor.select_template(info.clone());
                                self.status = format!("Placing {} — click the canvas", info.name);
                            }
                            // Small preview of the glyph pattern (S-016 AC1) — the same
                            // bytes the renderer stamps on placement.
                            for row in &info.preview {
                                ui.label(
                                    RichText::new(row.clone())
                                        .monospace()
                                        .size(9.0)
                                        .color(TEXT_COLOR),
                                );
                            }
                            // S-017 AC3 — entry point: edit this collection template
                            // (built-ins are code, not files — ADR 007).
                            if !group.builtins
                                && ui
                                    .add_sized([24.0, 18.0], egui::Button::new("✎"))
                                    .on_hover_text(format!("Edit {}", info.name))
                                    .clicked()
                            {
                                self.open_template_editor(&info.type_id);
                            }
                        }
                    }
                });
            });

        // Template editor window (S-017 AC1) — open while a session is active; the X
        // button or Cancel closes it without saving.
        if self.template_editor.is_some() {
            let mut open = true;
            egui::Window::new("Template editor")
                .open(&mut open)
                .collapsible(false)
                .default_size([480.0, 430.0])
                .show(ui.ctx(), |ui| self.draw_template_editor(ui));
            if !open {
                self.close_template_editor();
            }
        }

        // Keyboard edges: open dialogs own the keyboard first (a text field must be
        // able to receive Delete/Backspace; Escape closes the innermost dialog), only
        // then do they reach the canvas gestures (S-013 AC3).
        let ctx = ui.ctx();
        let delete = ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace));
        let escape = ctx.input(|i| i.key_pressed(Key::Escape));
        // A text field with focus (path prompt, template editor, inline edit) — its
        // keys must not reach the canvas. Read after the dialogs were drawn this frame.
        let text_focused = ctx.text_edit_focused();
        if let Some(message) = self.handle_global_keys(delete, escape, text_focused) {
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
        EditOutcome::PlacedText { id } => Some(format!(
            "Text placed as {id} — type its content (Enter commits, Esc cancels)"
        )),
        EditOutcome::Editing { id } => Some(format!("Editing {id} — Enter commits, Esc cancels")),
        EditOutcome::Edited { id } => Some(format!("Text updated [{id}]")),
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
                Some(((5, 5), HitTarget::Body(ObjectId::generated(1))))
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
            Some(((2, 1), HitTarget::Body(ObjectId::generated(1)))),
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
        let app = create_app(services, Some(&fixture), Vec::new());
        assert_eq!(app.diagram.len(), 1);

        // A missing fixture is reported (eprintln), not fatal — the app still starts empty.
        let services = Services::native();
        let app = create_app(
            services,
            Some(std::path::Path::new("/nonexistent/missing.ad")),
            vec!["broken.adc: line 2, column 1: boom".to_owned()],
        );
        assert_eq!(app.diagram.len(), 0);
        // Startup warnings are carried into the sidebar (S-016 AC2).
        assert_eq!(
            app.collection_warnings,
            vec!["broken.adc: line 2, column 1: boom"]
        );
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

    /// The S-018 startup fixture loads through the real open flow (shape + single-line
    /// and multi-line text) — it feeds the post-rebuild golden capture.
    #[test]
    fn the_text_sample_fixture_loads() {
        let mut app = app();
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text-sample.ad");
        let outcome = app.open(&path);
        match outcome {
            crate::fileio::OpenOutcome::Loaded { .. } => {}
            other => panic!(
                "fixture failed to load: {}",
                crate::fileio::open_message(&other)
            ),
        }
        let (text, width, height) = app.canvas_frame().expect("renders");
        assert_eq!(height, 10); // box rows 4..8 + text rows 12..13 → trimmed extent
        assert!(text.contains("hello world"));
        assert!(text.contains("line one"));
        assert!(text.contains("line two"));
        assert!(width >= 10);
    }

    /// AC3 — render parity: committed content appears in the canvas frame byte-exactly,
    /// including the C1 F4 quote fallback (lines with svgbob-alphabet glyphs get wrapped
    /// in double quotes) and long lines.
    #[test]
    fn canvas_frame_reflects_edited_text_bytes_exactly() {
        let mut app = app();
        // Place at (0,0) and commit an edit with plain, reserved-glyph and long lines.
        app.editor.select_text_tool();
        app.handle_pointer_frame(true, Some(((0, 0), HitTarget::Empty)));
        app.handle_pointer_frame(false, None);
        assert!(app.editor.editing.is_some());
        // The user types (the egui TextEdit binds to the draft) and commits.
        app.editor.edit_draft =
            "hello\na | b\nthis is a rather long plain line for width".to_owned();
        app.editor.commit_edit(&mut app.diagram);

        let (canvas_text, width, height) = app.canvas_frame().expect("renders");
        // F4: the `|` line is wrapped in quotes; plain lines stay verbatim. The exporter
        // trims row ends, so rows are unpadded.
        let rows: Vec<&str> = canvas_text.lines().collect();
        assert_eq!(height, 3);
        assert_eq!(width, 42); // the long line sets the extent
        assert_eq!(rows[0], "hello");
        assert_eq!(rows[1], "\"a | b\"");
        assert_eq!(rows[2], "this is a rather long plain line for width");
    }

    /// Keyboard seam (S-018): while an inline edit is open, Delete/Backspace belong to
    /// the text field (they must not delete the object); Escape cancels the edit.
    #[test]
    fn key_frames_during_inline_editing_protect_the_text_object() {
        let mut app = app();
        // Place a text object at (2,1) — placement opens inline editing at once.
        app.editor.select_text_tool();
        app.handle_pointer_frame(true, Some(((2, 1), HitTarget::Empty)));
        app.handle_pointer_frame(false, None);
        assert!(app.editor.editing.is_some());

        // Backspace while typing does NOT delete the object.
        assert_eq!(app.handle_key_frame(true, false), None);
        assert_eq!(app.diagram.len(), 1);

        // Escape cancels the edit; the object stays (selected).
        assert!(
            app.handle_key_frame(false, true)
                .unwrap()
                .contains("cancelled")
        );
        assert!(app.editor.editing.is_none());
        assert_eq!(app.diagram.len(), 1);
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

    /// An app whose catalog can persist templates into a throwaway directory (S-017
    /// tests must never touch the real user state).
    fn editor_app(dir: &std::path::Path) -> DrawingApp {
        let (services, _warnings) = Services::native_with_collections(Some(dir));
        DrawingApp::new(services)
    }

    /// S-017 AC3 — saving a valid draft registers it through the seam; the new
    /// template is immediately visible in the catalog (the sidebar re-reads `list()`
    /// every frame, so it becomes placeable without any reload).
    #[test]
    fn save_template_registers_through_the_seam() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut app = editor_app(dir.path());
        app.open_new_template();
        let Some(session) = app.template_editor.as_mut() else {
            panic!("editor is open")
        };
        session.draft.collection = "testcol".to_owned();
        session.draft.name = "star".to_owned();
        // A 3×3 star in the top-left corner.
        for (x, y, c) in [
            (0u32, 0u32, '#'),
            (1, 0, '.'),
            (2, 0, '#'),
            (0, 1, '#'),
            (1, 1, 'X'),
            (2, 1, '#'),
            (0, 2, '#'),
            (1, 2, '.'),
            (2, 2, '#'),
        ] {
            assert!(session.draft.set_cell(x, y, c));
        }
        session.draft.toggle_anchor(1, 0); // edge midpoint → "n"

        let message = app.save_template().expect("saves");
        assert!(message.contains("testcol"), "{message}");
        assert!(app.template_editor.is_none(), "window closes on success");

        // Immediately listed under its qualified id (C8) and resolvable.
        let listed = app.services.catalog.list();
        assert!(listed.iter().any(|i| i.type_id == "testcol/star"));
        assert!(app.services.catalog.get("testcol/star").is_some());
    }

    /// S-017 AC3 — end-to-end through the seam: edit → save → place on the canvas →
    /// the rendered grid carries the template's pattern bytes at the placement cell.
    #[test]
    fn saved_template_places_and_renders_its_pattern() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut app = editor_app(dir.path());
        app.open_new_template();
        {
            let session = app.template_editor.as_mut().expect("editor is open");
            session.draft.collection = "stars".to_owned();
            session.draft.name = "star".to_owned();
            for (x, y, c) in [
                (0u32, 0u32, '#'),
                (1, 0, '.'),
                (2, 0, '#'),
                (0, 1, '#'),
                (1, 1, 'X'),
                (2, 1, '#'),
                (0, 2, '#'),
                (1, 2, '.'),
                (2, 2, '#'),
            ] {
                assert!(session.draft.set_cell(x, y, c));
            }
            session.draft.toggle_anchor(1, 0);
        }
        app.save_template().expect("saves");

        // Place the freshly saved template at (5, 7) through the same gesture path
        // the canvas uses.
        let info = app
            .services
            .catalog
            .get("stars/star")
            .expect("listed after save");
        app.editor.select_template(info);
        assert!(matches!(
            app.handle_pointer_frame(true, Some(((5, 7), HitTarget::Empty))),
            EditOutcome::Placed { .. }
        ));
        app.handle_pointer_frame(false, None);

        // The rendered grid carries the pattern bytes verbatim (trimmed to the star).
        let (text, width, height) = app.canvas_frame().expect("renders");
        assert_eq!((width, height), (3, 3), "star is 3×3:\n{text}");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "#.#", "{text}");
        assert_eq!(lines[1], "#X#", "{text}");
        assert_eq!(lines[2], "#.#", "{text}");
    }

    /// S-017 AC2 — saving is blocked without anchors: the error is reported, the
    /// inline hint is set, the editor stays open, and nothing is registered.
    #[test]
    fn save_template_is_blocked_without_anchors() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut app = editor_app(dir.path());
        app.open_new_template();
        {
            let session = app.template_editor.as_mut().expect("editor is open");
            session.draft.set_cell(0, 0, '#'); // a grid, but no anchor
        }

        let err = app.save_template().expect_err("no anchors — must not save");
        assert!(err.contains("anchor"), "{err}");
        // The inline hint is set and the editor stays open.
        let session = app.template_editor.as_mut().expect("editor stays open");
        assert!(
            session
                .draft
                .rejection
                .as_deref()
                .unwrap_or("")
                .contains("anchor")
        );
        // Nothing was registered.
        assert!(
            app.services
                .catalog
                .list()
                .iter()
                .all(|i| i.collection.is_none())
        );
    }

    /// S-017 AC3 — the edit entry point loads an existing template's grid and anchors
    /// into a draft; built-ins have no entry point (they are code, not files).
    #[test]
    fn open_template_editor_loads_an_existing_template() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("stars.adc"),
            "adc 1.0\ntemplate star 3 x 3\n###\n#.#\n###\nanchor n at 1 0\n",
        )
        .expect("seed file");
        let mut app = editor_app(dir.path());

        // Built-ins are not editable: the entry point is a no-op for them.
        app.open_template_editor("box");
        assert!(app.template_editor.is_none());

        app.open_template_editor("stars/star");
        let session = app.template_editor.as_mut().expect("editor is open");
        assert_eq!(session.draft.collection, "stars");
        assert_eq!(session.draft.name, "star");
        assert_eq!(session.draft.rows, vec!["###", "#.#", "###"]);
        assert_eq!(session.draft.anchors.len(), 1);
        assert_eq!(session.draft.anchors[0].name, "n");
    }

    // ── S-015: SVG/PNG export + scale ────────────────────────────────────────────

    /// Places the built-in box on the canvas and releases the pointer.
    fn place_box(app: &mut DrawingApp) {
        app.editor.select_template(box_info());
        assert!(matches!(
            app.handle_pointer_frame(true, Some(((3, 4), HitTarget::Empty))),
            EditOutcome::Placed { .. }
        ));
        app.handle_pointer_frame(false, None);
    }

    /// AC3 — scale control: default ×1; “+” doubles the rendered size and both the
    /// canvas text and the SVG bytes follow it; clamped to [1, MAX_GUI_SCALE].
    #[test]
    fn scale_control_changes_rendered_output_and_clamps() {
        let mut app = app();
        assert_eq!(app.diagram.scale(), 1);
        place_box(&mut app);

        let (text1, w1, h1) = app.canvas_frame().expect("renders");
        assert_eq!((w1, h1), (9, 5));

        app.change_scale(1); // ×2 — every cell becomes a 2×2 block
        assert_eq!(app.diagram.scale(), 2);
        let (text2, w2, h2) = app.canvas_frame().expect("renders");
        assert_eq!((w2, h2), (18, 10));
        assert_ne!(text1, text2);

        // The SVG bytes follow the scale too.
        let svg_at_2 = exportio::svg_bytes(&app.services, &app.diagram).expect("svg");
        app.change_scale(1); // ×4
        let svg_at_4 = exportio::svg_bytes(&app.services, &app.diagram).expect("svg");
        assert_ne!(svg_at_2, svg_at_4);

        // Lower bound: never below 1 (the model enforces it anyway).
        for _ in 0..5 {
            app.change_scale(-1);
        }
        assert_eq!(app.diagram.scale(), 1);

        // Upper bound: clamped to MAX_GUI_SCALE.
        for _ in 0..20 {
            app.change_scale(1);
        }
        assert_eq!(app.diagram.scale(), exportio::MAX_GUI_SCALE);
    }

    /// AC1 — the SVG sink receives exactly what `to_svg` produced (fakes pin the
    /// bytes, so any re-serialization or mutation would be visible).
    #[test]
    fn svg_export_sink_gets_the_exporters_exact_bytes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("out.svg");
        let services = Services {
            renderer: Arc::new(FakeRenderer::with_grid(
                ui_api::ResolvedGrid::from_rows(&["+---+"]).expect("grid"),
            )),
            exporter: Arc::new(core_services::fakes::FakeExporter),
            ..Services::native()
        };
        let mut app = DrawingApp::new(services);

        app.export_svg_to(&path).expect("writes");
        assert_eq!(
            std::fs::read_to_string(&path).expect("reads"),
            "<svg xmlns=\"http://www.w3.org/2000/svg\">fake</svg>"
        );
        assert!(app.status.contains("SVG saved"));
    }

    /// AC1 — through the real pipeline: the file equals `to_svg` of the same grid
    /// the canvas shows (WYSIWYG).
    #[test]
    fn svg_export_matches_the_canvas_grid() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("out.svg");
        let mut app = app();
        place_box(&mut app);

        app.export_svg_to(&path).expect("writes");
        let on_disk = std::fs::read_to_string(&path).expect("reads");
        let grid = app.services.renderer.render(&app.diagram).expect("renders");
        assert_eq!(on_disk, app.services.exporter.to_svg(&grid).expect("svg"));
    }

    /// AC2/RS4 — PNG export through the app: start → “Exporting PNG…” state; per-frame
    /// ticks complete without blocking; the exact PNG bytes land at the sink.
    #[test]
    fn png_export_completes_through_per_frame_ticks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("out.png");
        let mut app = app();
        place_box(&mut app);

        app.begin_png_export(&path).expect("starts");
        assert_eq!(app.status, "Exporting PNG…");
        assert!(app.png_export.is_some());

        // Drive the per-frame polling exactly like `ui()` does — spaced like frames
        // (~2 ms), so we don't outrun the worker thread's startup.
        for _ in 0..500 {
            app.tick_png_export();
            if app.png_export.is_none() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(app.png_export.is_none(), "job finished");
        let bytes = std::fs::read(&path).expect("png on disk");
        assert_eq!(&bytes[0..4], &b"\x89PNG"[..4], "PNG signature");
        assert!(app.status.contains("PNG saved"));
    }

    /// AC2 — a failed worker job surfaces as a user-facing status, not a panic.
    #[test]
    fn png_export_failure_is_reported_in_the_status() {
        let mut app = app();
        let (tx, rx) = std::sync::mpsc::channel::<Result<Vec<u8>, ui_api::ExportError>>();
        app.png_export = Some(exportio::PngExport::from_job(
            core_services::PngJob::from_reply(rx),
        ));
        app.pending_png_path = Some(std::path::PathBuf::from("/tmp/never.png"));

        tx.send(Err(ui_api::ExportError::Unsupported))
            .expect("sends");
        app.tick_png_export();
        assert!(
            app.status.starts_with("PNG export failed:"),
            "got: {}",
            app.status
        );
        assert!(app.png_export.is_none());
    }

    /// AC1 — a failed SVG export (bad sink path) shows a failure notice instead of
    /// failing silently.
    #[test]
    fn svg_export_failure_shows_a_notice() {
        let mut app = app();
        app.begin_path_prompt(PathPrompt::ExportSvg);
        app.submit_path_prompt("/nonexistent-dir-xyz-1234/out.svg");
        assert!(
            app.status.starts_with("SVG export failed:"),
            "got: {}",
            app.status
        );
    }

    /// S-015 — export prompts prefill with the document's sibling path.
    #[test]
    fn export_prompt_defaults_to_document_sibling() {
        let mut app = app();
        assert_eq!(
            app.default_export_path("svg"),
            "",
            "no current file → empty"
        );
        app.current_file = Some(std::path::PathBuf::from("/tmp/diagram.ad"));
        assert_eq!(app.default_export_path("svg"), "/tmp/diagram.svg");
        assert_eq!(app.default_export_path("png"), "/tmp/diagram.png");
    }
}
