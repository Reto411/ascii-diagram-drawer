//! Anchor references — the endpoint type for connectors.
//!
//! A connector is bound to a *named anchor of a shape* (`AnchorRef`), not to raw coordinates:
//! when the shape moves or resizes, the connector stays attached because its endpoints are
//! re-resolved from `(object id, anchor name)` at render time (FR9).

use crate::invariants::{ModelError, TokenField, validate_single_token};
use crate::objects::ObjectId;

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

    use super::AnchorRef;
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

    /// `AnchorRef` compares by value.
    #[test]
    fn anchor_refs_compare_by_value() {
        let first = make_anchor("e");
        let second =
            AnchorRef::new(ObjectId::new("s1").expect("valid id"), "e").expect("valid anchor");
        assert_eq!(first, second);
    }
}
