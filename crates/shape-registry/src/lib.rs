//! Shape Catalog bounded context — [ADR 007](../../../doc/adr/007-user-defined-diagram-objects.md).
//!
//! Owns `ShapeType` (glyph patterns, anchor sets, resize rules) and `Collection`.
//! The diagram carries only opaque `type_id`s; meaning is resolved through this
//! catalog at render time (Customer/Supplier context mapping, §8.2.3).
//!
//! This crate is a leaf in the §8.3 dependency DAG — no workspace dependencies:
//! anchor geometry is owned here as registry-local value types ([`geometry`]).
//! Pure data in → catalog out; no IO, no windowing.
//!
//! - [`builtin`]: the predefined shape catalog with standard default anchors (FR11).
//! - [`templates`]: `.adc` collection file parsing — grammar normative in
//!   [shape-template](../../../doc/arc42/12-concepts/shape-template.md) and
//!   [collection](../../../doc/arc42/12-concepts/collection.md).
//! - [`registry`]: type-id resolution — bare token → built-ins only, `collection/name`
//!   → user collections only; disjoint namespaces, no silent fallback (C8).

pub mod builtin;
pub mod error;
pub mod geometry;
pub mod registry;
pub mod templates;
pub mod types;

pub use error::{AnchorSetError, GridError, LookupError, ParseError};
pub use geometry::{CellPos, CellSize};
pub use registry::ShapeRegistry;
pub use templates::{CollectionFile, parse_collection};
pub use types::{AnchorSet, Collection, GeometryKind, ResizeRules, ShapeType, TemplateGrid};
