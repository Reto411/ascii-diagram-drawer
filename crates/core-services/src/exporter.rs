//! Exporter backend (ADR 009): wraps `exporters` and maps its crate-level errors
//! into the seam's vocabulary. The rasterizer's build-time availability (ADR 016)
//! degrades to [`ui_api::ExportError::Unsupported`] — the GUI offers SVG-only export
//! without any code change (ADR 009, risk R3).

use diagram_model::color::{ColorRole, DiagramColors};
use render_pipeline::ResolvedGrid;
use ui_api::{ExportError, Exporter};

/// Maps an `exporters` failure to the seam level: vector-seam failures stay vector,
/// rasterizer-internal failures become raster, and a missing backend is `Unsupported`.
fn map_error(err: exporters::ExportError) -> ExportError {
    match err {
        exporters::ExportError::Svgbob { message } => ExportError::Vector { message },
        exporters::ExportError::SvgParse { message }
        | exporters::ExportError::PngEncode { message } => ExportError::Raster { message },
        exporters::ExportError::Unsupported => ExportError::Unsupported,
    }
}

/// Maps the diagram's per-media colors (FR15) onto the exporter's color slots. The
/// roles map 1:1 — background→backdrop, fill→shape fill, line & text→stroke; unset
/// roles stay unset so svgbob keeps its default for that slot (byte-stable exports).
pub fn svg_options_for(colors: &DiagramColors) -> exporters::SvgOptions {
    let hex = |role: ColorRole| colors.get(role).map(|c| c.to_hex());
    exporters::SvgOptions {
        background: hex(ColorRole::Background),
        fill: hex(ColorRole::Fill),
        stroke: hex(ColorRole::LineText),
    }
}

/// Emits the resolved grid as text/SVG/PNG via the `exporters` crate. Stateless.
#[derive(Debug, Default, Clone, Copy)]
pub struct GridExporter;

impl Exporter for GridExporter {
    fn to_text(&self, grid: &ResolvedGrid) -> String {
        exporters::to_text(grid)
    }

    fn to_svg(&self, grid: &ResolvedGrid, colors: &DiagramColors) -> Result<String, ExportError> {
        exporters::to_svg_with_options(grid, &svg_options_for(colors)).map_err(map_error)
    }

    fn to_png(&self, grid: &ResolvedGrid, colors: &DiagramColors) -> Result<Vec<u8>, ExportError> {
        exporters::to_png_with_options(grid, &svg_options_for(colors)).map_err(map_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use render_pipeline::ResolvedGrid;

    fn small_grid() -> ResolvedGrid {
        ResolvedGrid::from_rows(&["+---+", "| A |", "+---+"]).expect("valid grid")
    }

    /// AC2 — export of a fixture model through the seam: text is the grid as-is, SVG
    /// is a real svgbob document, PNG decodes (raster feature on in this build).
    #[test]
    fn exports_all_three_media() {
        let exporter = GridExporter;
        let grid = small_grid();

        assert_eq!(exporter.to_text(&grid), "+---+\n| A |\n+---+");

        let svg = exporter
            .to_svg(&grid, &DiagramColors::default())
            .expect("svg");
        assert!(svg.starts_with("<svg"), "real svgbob document: {svg:?}");

        let png = exporter
            .to_png(&grid, &DiagramColors::default())
            .expect("png (raster feature on)");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    /// FR15 (CR-009) — the color roles map 1:1 onto the exporter's slots, and unset
    /// roles stay unset (svgbob defaults keep the export byte-stable).
    #[test]
    fn color_roles_map_onto_the_exporter_slots() {
        use diagram_model::color::Rgb;

        let mut colors = DiagramColors::default();
        assert_eq!(svg_options_for(&colors), exporters::SvgOptions::default());

        colors.set(
            ColorRole::Background,
            Some(Rgb::from_hex("#202124").unwrap()),
        );
        colors.set(ColorRole::Fill, Some(Rgb::from_hex("#8ab4f8").unwrap()));
        colors.set(ColorRole::LineText, Some(Rgb::from_hex("#e8eaed").unwrap()));
        assert_eq!(
            svg_options_for(&colors),
            exporters::SvgOptions {
                background: Some("#202124".into()),
                fill: Some("#8ab4f8".into()),
                stroke: Some("#e8eaed".into()),
            }
        );

        // Resetting one role unsets exactly that slot.
        colors.set(ColorRole::Fill, None);
        let options = svg_options_for(&colors);
        assert_eq!(options.fill, None);
        assert_eq!(options.background.as_deref(), Some("#202124"));
    }

    /// FR15 (CR-009) — a set color reaches the SVG through the seam.
    #[test]
    fn colored_export_through_the_seam() {
        use diagram_model::color::Rgb;
        let exporter = GridExporter;
        let grid = small_grid();
        let mut colors = DiagramColors::default();
        colors.set(ColorRole::LineText, Some(Rgb::from_hex("#ff0000").unwrap()));

        let svg = exporter.to_svg(&grid, &colors).expect("svg");
        assert!(
            svg.contains("#ff0000"),
            "line & text role lands in the SVG: {svg}"
        );
    }

    /// The seam never leaks crate-level errors: every `exporters` variant maps to a
    /// seam-level one with its message preserved.
    #[test]
    fn every_upstream_variant_maps_to_the_seam() {
        let upstream = [
            (
                exporters::ExportError::Svgbob {
                    message: "m".into(),
                },
                ExportError::Vector {
                    message: "m".into(),
                },
            ),
            (
                exporters::ExportError::SvgParse {
                    message: "m".into(),
                },
                ExportError::Raster {
                    message: "m".into(),
                },
            ),
            (
                exporters::ExportError::PngEncode {
                    message: "m".into(),
                },
                ExportError::Raster {
                    message: "m".into(),
                },
            ),
            (
                exporters::ExportError::Unsupported,
                ExportError::Unsupported,
            ),
        ];
        for (from, to) in upstream {
            assert_eq!(map_error(from), to);
        }
    }
}
