//! Shape Catalog bounded context — [ADR 007](../../../doc/adr/007-user-defined-diagram-objects.md).
//!
//! Owns `ShapeType` (glyph patterns, anchor sets, resize rules) and `Collection`.
//! The diagram carries only opaque `type_id`s; meaning is resolved through this
//! catalog at render time (Customer/Supplier context mapping, §8.2.3).
