//! Persistence bounded context — [ADR 004](../../../doc/adr/004-diagram-file-format.md).
//!
//! `.ad` is a *serialization* of the Diagram aggregate, not a second model:
//! load = parse → aggregate, save = serialize → text. No IO here — pure text ↔ `Diagram`.
//!
//! The grammar (v1) is normative in
//! [diagram-file-format.md](../../../doc/arc42/12-concepts/diagram-file-format.md):
//! - [`parse`] / [`parse_document`]: `.ad` text → [`Diagram`] aggregate, with line/column
//!   positions on every error; unknown statements from higher minor versions are preserved.
//! - [`serialize`] / [`serialize_document`]: aggregate → canonical `.ad` text (deterministic,
//!   byte-stable re-saves — QS7).

pub mod error;
pub mod parser;
pub mod serializer;

pub use error::ParseError;
pub use parser::{Document, parse, parse_document};
pub use serializer::{serialize, serialize_document};
