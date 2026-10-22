//! Diagram bounded context (core domain) — [ADR 011](../../../doc/adr/011-domain-driven-design.md).
//!
//! Owns the `Diagram` aggregate: identity-bearing objects (shapes, text, connectors),
//! value-object cell geometry, and invariants enforced at the boundary.
//! Pure data + logic; no IO, no windowing dependencies. See concept docs
//! `doc/arc42/12-concepts/diagram-aggregate.md` and `cell-grid.md`.
