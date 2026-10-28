//! SVG emission (FR5): the exact text bytes of the grid passed unchanged to `svgbob`
//! (ADR 005) — no adapter layer, so text and SVG input are identical by construction.

use render_pipeline::ResolvedGrid;

use crate::{ExportError, to_text};

/// Exports a resolved grid as an SVG document.
///
/// The grid is first exported as text ([`to_text`]) and that exact byte string is what
/// `svgbob` consumes — one input domain for both media (ADR 005).
pub fn to_svg(grid: &ResolvedGrid) -> Result<String, ExportError> {
    svg_from_text(&to_text(grid))
}

/// Converts an exact svgbob input string to SVG. This is the seam `to_svg` funnels
/// through; svgbob 0.7.6's public API is infallible (out-of-domain input degrades to
/// literal text — ADR 005 consequence), so this returns `Ok` today and surfaces an
/// [`ExportError::Svgbob`] if a future upstream version becomes fallible.
pub(crate) fn svg_from_text(text: &str) -> Result<String, ExportError> {
    Ok(svgbob::to_svg(text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExportError;

    #[test]
    fn box_exports_to_an_svg_document() {
        let grid = ResolvedGrid::from_rows(&["+---+", "| A |", "+---+"]).expect("valid grid");
        let svg = to_svg(&grid).expect("renders");
        assert!(
            svg.starts_with("<svg"),
            "must be an SVG document: {svg:?}.."
        );
        assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    }

    #[test]
    fn svg_input_is_exactly_the_text_export() {
        // Same bytes in, same output out (ADR 005): the SVG for a grid equals the SVG
        // produced from its text export — no hidden transformation layer.
        let grid = ResolvedGrid::from_rows(&["+---+", "| A |", "+---+"]).expect("valid grid");
        assert_eq!(
            to_svg(&grid).expect("renders"),
            svg_from_text(&to_text(&grid)).expect("renders")
        );
    }

    /// AC4 — byte-stable: exporting the same grid twice yields identical bytes; no
    /// timestamps or randomness in the output.
    #[test]
    fn reexporting_the_same_grid_is_byte_identical() {
        let grid = ResolvedGrid::from_rows(&["+---->+---+", "| A |   B |", "+---->+---+"])
            .expect("valid grid");
        let a = to_svg(&grid).expect("renders");
        let b = to_svg(&grid).expect("renders");
        assert_eq!(a, b);
    }

    /// svgbob 0.7.6 degrades instead of failing: out-of-domain and degenerate inputs
    /// (empty, spaces-only, garbage) all export without error — pinned so a future
    /// upstream change that starts failing is caught here.
    #[test]
    fn out_of_domain_input_degrades_instead_of_failing() {
        for text in ["", "   ", "not a diagram at all", "a b c"] {
            assert!(svg_from_text(text).is_ok(), "input {text:?} must not fail");
        }
    }

    #[test]
    fn export_error_display_names_the_upstream_failure() {
        let err = ExportError::Svgbob {
            message: "boom".into(),
        };
        assert_eq!(err.to_string(), "svgbob conversion failed: boom");
    }
}
