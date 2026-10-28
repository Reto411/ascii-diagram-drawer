//! Rendering & Export bounded context (2/2) — [ADR 005](../../../doc/adr/005-use-svgbob-as-vector-renderer.md).
//!
//! Presents the one `ResolvedGrid` in three v1 media:
//! - **Text** (`to_text`): the grid as-is, LF-joined, per-line trailing whitespace trimmed.
//! - **SVG** (`to_svg`): the *exact* text bytes passed unchanged to `svgbob::to_svg` — no
//!   adapter layer; text and SVG input are the same bytes (ADR 005).
//! - **PNG** (`to_png`): rasterization of that exact SVG via `resvg`
//!   ([ADR 016](../../../doc/adr/016-png-rasterizer.md)), behind the default `raster`
//!   feature — builds without it get [`ExportError::Unsupported`], not a broken build.

mod png;
mod svg;
mod text;

pub use png::to_png;
pub use svg::to_svg;
pub use text::to_text;

/// Errors an export can produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    /// Upstream grid→vector conversion failed (svgbob). Unreachable on svgbob 0.7.6 —
    /// its public API is infallible and out-of-domain input degrades to literal text
    /// rather than failing (ADR 005 consequence) — but kept as the seam for future
    /// fallible upstream versions (the ADR records a deliberate coupling to svgbob 0.x).
    Svgbob {
        /// The upstream error message.
        message: String,
    },
    /// The rasterizer's SVG frontend (usvg) could not parse the document produced by
    /// [`to_svg`]. Cannot occur with today's svgbob output (always well-formed); kept
    /// because `usvg::Tree::from_data` is fallible and this seam must never panic.
    SvgParse {
        /// The upstream error message.
        message: String,
    },
    /// PNG encoding of the rendered pixmap failed (or the document had zero size).
    PngEncode {
        /// The upstream error message.
        message: String,
    },
    /// No rasterizer backend is available in this build — `to_png` is called without
    /// the default `raster` feature (ADR 016). Text and SVG exports are unaffected.
    Unsupported,
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Svgbob { message } => write!(f, "svgbob conversion failed: {message}"),
            Self::SvgParse { message } => write!(f, "SVG parse failed: {message}"),
            Self::PngEncode { message } => write!(f, "PNG encoding failed: {message}"),
            Self::Unsupported => write!(
                f,
                "no rasterizer backend available in this build (enable the `raster` feature)"
            ),
        }
    }
}

impl std::error::Error for ExportError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant formats without panicking and names its failure mode — the
    /// Display arms of `SvgParse`/`PngEncode` are otherwise only exercised through
    /// the rasterizer seam.
    #[test]
    fn every_error_variant_has_a_display_message() {
        let variants = [
            ExportError::Svgbob {
                message: "m".into(),
            },
            ExportError::SvgParse {
                message: "m".into(),
            },
            ExportError::PngEncode {
                message: "m".into(),
            },
            ExportError::Unsupported,
        ];
        for variant in variants {
            let message = variant.to_string();
            assert!(!message.is_empty(), "{variant:?} must format");
        }
    }
}
