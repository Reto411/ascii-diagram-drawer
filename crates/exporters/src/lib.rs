//! Rendering & Export bounded context (2/2) — [ADR 005](../../../doc/adr/005-use-svgbob-as-vector-renderer.md).
//!
//! Presents the one `ResolvedGrid` in two v1 media:
//! - **Text** (`to_text`): the grid as-is, LF-joined, per-line trailing whitespace trimmed.
//! - **SVG** (`to_svg`): the *exact* text bytes passed unchanged to `svgbob::to_svg` — no
//!   adapter layer; text and SVG input are the same bytes (ADR 005).
//!
//! PNG emission is a follow-up story (S-009, rasterizer TBD per ADR 005).

mod svg;
mod text;

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
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Svgbob { message } => write!(f, "svgbob conversion failed: {message}"),
        }
    }
}

impl std::error::Error for ExportError {}
