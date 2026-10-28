//! Application layer — the service seam, [ADR 009](../../../doc/adr/009-ui-api-service-layer.md).
//!
//! Trait definitions only (`DiagramStore`, `Renderer`, `Exporter`, `ShapeCatalog`):
//! the GUI depends on these abstractions; concrete backends are injected at the
//! composition root in `core-services`. Signatures pinned in S-010.
