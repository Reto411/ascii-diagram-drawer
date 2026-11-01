//! SVG emission (FR5): the exact text bytes of the grid passed unchanged to `svgbob`
//! (ADR 005) — no adapter layer, so text and SVG input are identical by construction.

use render_pipeline::ResolvedGrid;

use crate::{ExportError, to_text};

/// Per-media color overrides for SVG export (FR15 / CR-009). Each set slot maps 1:1
/// onto a `svgbob` [`Settings`](svgbob::Settings) field; an unset slot keeps svgbob's
/// own default, so all-unset options reproduce the plain [`to_svg`] output byte for
/// byte (NFR2 pin).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SvgOptions {
    /// Backdrop color as `#rrggbb` (svgbob `background`).
    pub background: Option<String>,
    /// Fill for filled shapes as `#rrggbb` (svgbob `fill_color`).
    pub fill: Option<String>,
    /// Stroke for lines and shapes — and the text ink, which svgbob paints with the
    /// stroke color — as `#rrggbb` (svgbob `stroke_color`).
    pub stroke: Option<String>,
}

/// Exports a resolved grid as an SVG document.
///
/// The grid is first exported as text ([`to_text`]) and that exact byte string is what
/// `svgbob` consumes — one input domain for both media (ADR 005). Colors are svgbob's
/// defaults; use [`to_svg_with_options`] to override them (FR15).
pub fn to_svg(grid: &ResolvedGrid) -> Result<String, ExportError> {
    to_svg_with_options(grid, &SvgOptions::default())
}

/// Exports a resolved grid as an SVG document with per-media color overrides (FR15):
/// the same translation as [`to_svg`], but each set slot of `options` replaces
/// svgbob's default color for that media in the emitted document.
pub fn to_svg_with_options(
    grid: &ResolvedGrid,
    options: &SvgOptions,
) -> Result<String, ExportError> {
    svg_from_text_with_options(&to_text(grid), options)
}

/// The grid→vector step with per-media color overrides (FR15): starts from `svgbob`
/// defaults, applies each set slot of `options`, and renders the same bytes. With all
/// slots unset this is exactly svgbob's default rendering (byte-stable — pinned in
/// tests).
pub fn svg_from_text_with_options(text: &str, options: &SvgOptions) -> Result<String, ExportError> {
    let mut settings = svgbob::Settings::default();
    if let Some(bg) = &options.background {
        settings.background = bg.clone();
    }
    if let Some(fill) = &options.fill {
        settings.fill_color = fill.clone();
    }
    if let Some(stroke) = &options.stroke {
        settings.stroke_color = stroke.clone();
    }
    Ok(svgbob::to_svg_with_settings(text, &settings))
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
            svg_from_text_with_options(&to_text(&grid), &SvgOptions::default()).expect("renders")
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
            assert!(
                svg_from_text_with_options(text, &SvgOptions::default()).is_ok(),
                "input {text:?} must not fail"
            );
        }
    }

    #[test]
    fn export_error_display_names_the_upstream_failure() {
        let err = ExportError::Svgbob {
            message: "boom".into(),
        };
        assert_eq!(err.to_string(), "svgbob conversion failed: boom");
    }

    /// FR15 (CR-009) — NFR2 pin: all-unset options reproduce the plain `to_svg` output
    /// byte for byte, so an all-default diagram exports exactly as before this story.
    #[test]
    fn unset_options_are_byte_identical_to_the_plain_export() {
        let grid = ResolvedGrid::from_rows(&[
            "+-------+     +-------+",
            "|   A   +---->+   B   |",
            "+-------+     +-------+",
        ])
        .expect("valid grid");
        assert_eq!(
            to_svg_with_options(&grid, &SvgOptions::default()).expect("renders"),
            to_svg(&grid).expect("renders")
        );
    }

    /// FR15 (CR-009) — each set slot lands in the emitted document: background on the
    /// backdrop style, fill on the `.filled` class, stroke on the ink (lines *and*
    /// text). Unset slots keep svgbob's defaults.
    #[test]
    fn set_options_land_in_the_svg_and_unset_slots_keep_defaults() {
        let grid = ResolvedGrid::from_rows(&["+---+", "| A |", "+---+"]).expect("valid grid");

        // All three roles set.
        let all = SvgOptions {
            background: Some("#202124".into()),
            fill: Some("#8ab4f8".into()),
            stroke: Some("#e8eaed".into()),
        };
        let svg = to_svg_with_options(&grid, &all).expect("renders");
        assert!(svg.contains("#202124"), "background slot: {svg}");
        assert!(svg.contains("#8ab4f8"), "fill slot: {svg}");
        assert!(svg.contains("#e8eaed"), "stroke slot: {svg}");

        // Only the fill role set — stroke and background stay at svgbob's defaults
        // (`stroke: black`, `fill: white` in its emitted CSS).
        let only_fill = SvgOptions {
            fill: Some("#ff0000".into()),
            ..SvgOptions::default()
        };
        let svg = to_svg_with_options(&grid, &only_fill).expect("renders");
        assert!(svg.contains("#ff0000"));
        assert!(svg.contains("stroke: black"), "default stroke kept: {svg}");
    }

    /// FR15 (CR-009) — deterministic: same options, same bytes.
    #[test]
    fn colored_export_is_byte_identical_across_calls() {
        let grid = ResolvedGrid::from_rows(&["+---+"]).expect("valid grid");
        let options = SvgOptions {
            background: Some("#101010".into()),
            fill: Some("#203040".into()),
            stroke: Some("#f0e0d0".into()),
        };
        assert_eq!(
            to_svg_with_options(&grid, &options).expect("renders"),
            to_svg_with_options(&grid, &options).expect("renders")
        );
    }
}
