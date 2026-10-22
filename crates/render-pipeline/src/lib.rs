//! Rendering & Export bounded context (1/2) — [ADR 001](../../../doc/adr/001-glyph-resolution-alphabet.md).
//!
//! The render pipeline: `render(&Diagram, &ShapeRegistry) -> ResolvedGrid` —
//! place objects on the cell grid, resolve one svgbob-alphabet glyph per occupied
//! cell. Pure functions; the grid is a derived intermediate, never stored (ADR 004).
