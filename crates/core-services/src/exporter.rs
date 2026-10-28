//! Exporter backend (ADR 009): wraps `exporters` and maps its crate-level errors
//! into the seam's vocabulary. The rasterizer's build-time availability (ADR 016)
//! degrades to [`ui_api::ExportError::Unsupported`] — the GUI offers SVG-only export
//! without any code change (ADR 009, risk R3).

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

/// Emits the resolved grid as text/SVG/PNG via the `exporters` crate. Stateless.
#[derive(Debug, Default, Clone, Copy)]
pub struct GridExporter;

impl Exporter for GridExporter {
    fn to_text(&self, grid: &ResolvedGrid) -> String {
        exporters::to_text(grid)
    }

    fn to_svg(&self, grid: &ResolvedGrid) -> Result<String, ExportError> {
        exporters::to_svg(grid).map_err(map_error)
    }

    fn to_png(&self, grid: &ResolvedGrid) -> Result<Vec<u8>, ExportError> {
        exporters::to_png(grid).map_err(map_error)
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

        let svg = exporter.to_svg(&grid).expect("svg");
        assert!(svg.starts_with("<svg"), "real svgbob document: {svg:?}");

        let png = exporter.to_png(&grid).expect("png (raster feature on)");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
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
