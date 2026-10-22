//! Rendering & Export bounded context (2/2) — [ADR 005](../../../doc/adr/005-use-svgbob-as-vector-renderer.md).
//!
//! Presents the one `ResolvedGrid` in three media: text (the grid as-is),
//! SVG (svgbob on the exact grid bytes), PNG (rasterization, optional backend).
