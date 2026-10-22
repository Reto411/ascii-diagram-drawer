//! Persistence bounded context — [ADR 004](../../../doc/adr/004-diagram-file-format.md).
//!
//! `.ad` is a *serialization* of the Diagram aggregate, not a second model:
//! load = parse → aggregate, save = serialize → text. No IO here — pure text ↔ `Diagram`.
