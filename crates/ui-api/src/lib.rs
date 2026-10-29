//! Application layer — the service seam, [ADR 009](../../../doc/adr/009-ui-api-service-layer.md).
//!
//! Four small, object-safe traits pin the whole surface `gui-app` may use: the GUI
//! depends on these abstractions (held as `Arc<dyn …>`), never on a context crate.
//! Concrete backends live in `core-services`; platform IO stays there (C3 rule).
//!
//! Deliberate non-abstraction (ADR 009): diagram *mutation* stays direct manipulation
//! of the pure [`Diagram`] value type inside the GUI — there is no "model service".

use std::path::Path;

use diagram_model::Diagram;
use render_pipeline::{RenderError, ResolvedGrid};

pub mod error;
pub mod types;

pub use error::{CatalogError, ExportError, StoreError};
pub use types::{AnchorInfo, LoadedDocument, NewTemplate, ShapeInfo};

/// Loads and saves `.ad` documents (FR4). The native backend uses the filesystem;
/// targets without one (wasm) get an in-memory store behind this same trait.
///
/// `load` reports forward-compat losses ([`LoadedDocument::skipped_unknowns`], ADR 009 v4)
/// so the GUI can warn instead of silently dropping newer-version statements (RS2).
/// Save serializes the current [`Diagram`] canonically (`ad-format`); unknown statements
/// skipped at load are gone by then, which is exactly what the warning protects against.
pub trait DiagramStore {
    /// Reads and parses the `.ad` document at `path`, including how many unknown
    /// statements (newer minor version) were skipped — RS2: nothing silently dropped.
    fn load(&self, path: &Path) -> Result<LoadedDocument, StoreError>;

    /// Serializes `diagram` canonically and writes it to `path`.
    fn save(&self, path: &Path, diagram: &Diagram) -> Result<(), StoreError>;
}

/// Renders a diagram into the resolved character grid (the single source of truth
/// for all outputs, C2). The shape catalog is bound at construction — the GUI never
/// passes registry types across the seam.
pub trait Renderer {
    /// Places and resolves `diagram` against the bound catalog.
    fn render(&self, diagram: &Diagram) -> Result<ResolvedGrid, RenderError>;

    /// Renders an as-yet-unregistered template pattern through the full pipeline —
    /// the live preview of the template editor (S-017 AC1). The pattern is placed at
    /// the origin of an otherwise empty diagram and resolved by the same place/resolve
    /// stages a saved template would use, so the preview equals the render output.
    fn preview_pattern(&self, rows: &[String]) -> Result<ResolvedGrid, RenderError>;
}

/// Emits the resolved grid as user-facing media (FR2/FR5). Text is infallible; SVG
/// and PNG may fail — and `to_png` reports [`ExportError::Unsupported`] when no
/// rasterizer backend was compiled in, which the GUI turns into an SVG-only offer.
pub trait Exporter {
    /// The grid as plain monospace text (LF-joined, trailing whitespace trimmed).
    fn to_text(&self, grid: &ResolvedGrid) -> String;

    /// The grid as an SVG document (svgbob interpretation of the exact text bytes).
    fn to_svg(&self, grid: &ResolvedGrid) -> Result<String, ExportError>;

    /// The grid as a PNG image (rasterization of the SVG output, ADR 016).
    fn to_png(&self, grid: &ResolvedGrid) -> Result<Vec<u8>, ExportError>;
}

/// Lists and looks up shape types (FR3) and registers user-defined templates
/// (S-017, FR11). Data crosses the seam as read models ([`ShapeInfo`],
/// [`AnchorInfo`]) — never registry types.
pub trait ShapeCatalog {
    /// Every available shape type: built-ins plus all loaded collections, in order.
    fn list(&self) -> Vec<ShapeInfo>;

    /// The shape with the given catalog key (`"box"`, `"mycol/arrow"`), if any.
    fn get(&self, type_id: &str) -> Option<ShapeInfo>;

    /// The named anchors of a shape type, in name order — `None` for unknown ids.
    fn anchors(&self, type_id: &str) -> Option<Vec<AnchorInfo>>;

    /// Registers (or replaces) `template` in `collection`, persisting the collection
    /// to its `.adc` file and updating every service that reads the catalog — the
    /// sidebar lists it immediately and the renderer resolves it (S-017 AC3).
    fn register(&self, collection: &str, template: NewTemplate) -> Result<(), CatalogError>;
}
