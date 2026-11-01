//! Live SVG preview rasterization (CR-004 / S-026): the display seam for the GUI's
//! right-side preview pane. Re-exports the exporters' raw-RGBA rasterizer so
//! `gui-app` never names the raster crates directly — ADR 009 seam discipline: the
//! GUI asks the core services, and they own the pipeline. The pane shows exactly what
//! FR5 exports because this is the same `svg_from_text` → resvg path as `to_png`.

use diagram_model::color::DiagramColors;

pub use exporters::{RgbaImage, text_to_rgba};

/// Live-preview rasterization with the diagram's per-media colors (FR15 / CR-009):
/// the same pipeline as [`text_to_rgba`], but the SVG carries the color slots mapped
/// from `colors` — so a colored preview is exactly what a colored FR5 export of the
/// same bytes would produce.
pub fn text_to_rgba_with_colors(
    text: &str,
    colors: &DiagramColors,
) -> Result<RgbaImage, exporters::ExportError> {
    exporters::text_to_rgba_with_options(text, &crate::exporter::svg_options_for(colors))
}
