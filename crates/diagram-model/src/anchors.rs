//! Anchor references — the endpoint type for connectors.
//!
//! A connector end is either bound to a *named anchor of a shape* (`AnchorRef`) or free at an
//! explicit model-space cell ([`ConnectorEndpoint::Free`] — CR-010 / ADR 013 v6). Bound ends are
//! not raw coordinates: when the shape moves or resizes, the connector stays attached because its
//! endpoints are re-resolved from `(object id, anchor name)` at render time (FR9). Free ends never
//! re-resolve — they are fixed coordinates.

use crate::geometry::CellPos;
use crate::invariants::{ModelError, TokenField, validate_single_token};
use crate::objects::ObjectId;

/// One end of a connector (CR-010 / ADR 013 v6).
///
/// Either bound to a named anchor of a shape — following it when the shape moves or resizes —
/// or free at an explicit cell position in model space. A connector may have zero, one, or two
/// bound ends (standalone arrows need none).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConnectorEndpoint {
    /// Bound to a named anchor of a shape instance.
    Attached(AnchorRef),
    /// Free at an explicit cell position in model space — never re-resolved with any shape.
    Free(CellPos),
}

impl ConnectorEndpoint {
    /// Whether this end is bound to a shape anchor (as opposed to free).
    #[must_use]
    pub const fn is_attached(&self) -> bool {
        matches!(self, Self::Attached(_))
    }

    /// The referenced shape's id — `None` for a free end.
    #[must_use]
    pub fn object(&self) -> Option<&ObjectId> {
        match self {
            Self::Attached(reference) => Some(&reference.object),
            Self::Free(_) => None,
        }
    }

    /// The anchor name this end is bound to — `None` for a free end.
    #[must_use]
    pub fn anchor(&self) -> Option<&str> {
        match self {
            Self::Attached(reference) => Some(&reference.anchor),
            Self::Free(_) => None,
        }
    }
}

/// A reference to a named anchor of a shape instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnchorRef {
    /// The anchor's name within the shape type's anchor set (single token).
    pub anchor: String,
    /// The id of the shape object this anchor belongs to.
    pub object: ObjectId,
}

impl AnchorRef {
    /// The anchor's name within the shape type's anchor set.
    #[must_use]
    pub fn anchor(&self) -> &str {
        &self.anchor
    }

    /// Creates an anchor reference, validating the anchor name.
    ///
    /// # Errors
    /// [`ModelError::EmptyAnchorName`] or [`ModelError::AnchorNameContainsWhitespace`].
    pub fn new(object: ObjectId, anchor: &str) -> Result<Self, ModelError> {
        validate_single_token(anchor, TokenField::AnchorName)?;
        Ok(Self {
            anchor: anchor.to_owned(),
            object,
        })
    }

    /// The shape object the anchor belongs to.
    #[must_use]
    pub const fn object(&self) -> &ObjectId {
        &self.object
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "test code: panicking with the value on a failed construction is fine"
    )]

    use super::{AnchorRef, ConnectorEndpoint};
    use crate::CellPos;
    use crate::invariants::ModelError;
    use crate::objects::ObjectId;

    /// Builds an anchor reference on object `s1` for test assertions.
    fn make_anchor(name: &str) -> AnchorRef {
        AnchorRef::new(ObjectId::new("s1").expect("valid id"), name).expect("valid anchor")
    }

    /// Anchor names must be non-empty single tokens.
    #[test]
    fn anchor_name_validates_content() {
        assert_eq!(make_anchor("e").anchor(), "e");
        assert_eq!(
            AnchorRef::new(ObjectId::new("s1").expect("valid id"), ""),
            Err(ModelError::EmptyAnchorName)
        );
        assert_eq!(
            AnchorRef::new(ObjectId::new("s1").expect("valid id"), "n e"),
            Err(ModelError::AnchorNameContainsWhitespace("n e".to_owned()))
        );
    }

    /// `ConnectorEndpoint` distinguishes attached from free ends and exposes the
    /// reference parts only for attached ones (CR-010).
    #[test]
    fn endpoint_forms_expose_their_parts() {
        let attached = ConnectorEndpoint::Attached(make_anchor("e"));
        let free = ConnectorEndpoint::Free(CellPos::new(3, 4));

        assert!(attached.is_attached());
        assert!(!free.is_attached());
        assert_eq!(attached.object(), Some(&make_anchor("e").object));
        assert_eq!(attached.anchor(), Some("e"));
        assert_eq!(free.object(), None);
        assert_eq!(free.anchor(), None);
    }

    /// `AnchorRef` compares by value.
    #[test]
    fn anchor_refs_compare_by_value() {
        let first = make_anchor("e");
        let second =
            AnchorRef::new(ObjectId::new("s1").expect("valid id"), "e").expect("valid anchor");
        assert_eq!(first, second);
    }
}
