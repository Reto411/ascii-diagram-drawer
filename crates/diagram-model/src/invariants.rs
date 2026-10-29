//! Model invariants and their errors.
//!
//! The model is the single source of truth for what a valid diagram looks like (Q1). This
//! module owns the invariant checks that value objects run at construction time and the
//! [`ModelError`] type every fallible model operation returns. Cross-object invariants
//! (uniqueness, referential integrity, movability) are enforced by the aggregate
//! ([`crate::Diagram`]) on mutation.

use core::error::Error;
use core::fmt;

use crate::objects::ObjectId;

/// Which single-token field is being validated (for error messages).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenField {
    /// An anchor name.
    AnchorName,
    /// An object id.
    ObjectId,
    /// A shape type id.
    ShapeTypeId,
}

impl fmt::Display for TokenField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AnchorName => f.write_str("anchor name"),
            Self::ObjectId => f.write_str("object id"),
            Self::ShapeTypeId => f.write_str("shape type id"),
        }
    }
}

/// All model-level invariant violations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    /// Anchor names must be single tokens (no whitespace).
    AnchorNameContainsWhitespace(String),
    /// Connectors cannot be moved or resized (only their bound shapes can).
    ConnectorNotMovable(ObjectId),
    /// Object ids must be unique within a diagram.
    DuplicateId(ObjectId),
    /// Anchor names must not be empty.
    EmptyAnchorName,
    /// Object ids must not be empty.
    EmptyId,
    /// Shape type ids must not be empty.
    EmptyShapeTypeId,
    /// Text content must not be empty.
    EmptyText,
    /// Object ids must be single tokens (no whitespace).
    IdContainsWhitespace(String),
    /// An operation requires a shape, but the object is another kind.
    NotAShape(ObjectId),
    /// An operation requires a free text object, but the object is another kind.
    NotAText(ObjectId),
    /// The diagram scale factor must be at least one.
    ScaleTooSmall {
        /// The rejected scale value.
        scale: u32,
    },
    /// Shape type ids must be single tokens (no whitespace).
    ShapeTypeIdContainsWhitespace(String),
    /// A referenced object id does not exist in the diagram.
    UnknownObject(ObjectId),
    /// Object sizes must be at least one cell in each dimension.
    ZeroSize {
        /// Rejected height.
        height: u32,
        /// Rejected width.
        width: u32,
    },
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AnchorNameContainsWhitespace(name) => write!(
                f,
                "anchor name must be a single token (no whitespace): '{name}'"
            ),
            Self::ConnectorNotMovable(id) => {
                write!(f, "connectors cannot be moved or resized: {id}")
            }
            Self::DuplicateId(id) => write!(f, "object id already exists in diagram: {id}"),
            Self::EmptyAnchorName => f.write_str("anchor name must not be empty"),
            Self::EmptyId => f.write_str("object id must not be empty"),
            Self::EmptyShapeTypeId => f.write_str("shape type id must not be empty"),
            Self::EmptyText => f.write_str("text content must not be empty"),
            Self::IdContainsWhitespace(id) => write!(
                f,
                "object id must be a single token (no whitespace): '{id}'"
            ),
            Self::NotAShape(id) => {
                write!(f, "expected a shape but found another object kind: {id}")
            }
            Self::NotAText(id) => {
                write!(f, "expected free text but found another object kind: {id}")
            }
            Self::ScaleTooSmall { scale } => {
                write!(f, "scale factor must be at least 1 (got {scale})")
            }
            Self::ShapeTypeIdContainsWhitespace(type_id) => write!(
                f,
                "shape type id must be a single token (no whitespace): '{type_id}'"
            ),
            Self::UnknownObject(id) => write!(f, "unknown object id: {id}"),
            Self::ZeroSize { height, width } => {
                write!(
                    f,
                    "object size must be at least 1x1 cell (got {width}x{height})"
                )
            }
        }
    }
}

impl Error for ModelError {}

/// Validates that `value` is a non-empty single token (no whitespace).
pub(crate) fn validate_single_token(value: &str, field: TokenField) -> Result<(), ModelError> {
    if value.is_empty() {
        return Err(match field {
            TokenField::AnchorName => ModelError::EmptyAnchorName,
            TokenField::ObjectId => ModelError::EmptyId,
            TokenField::ShapeTypeId => ModelError::EmptyShapeTypeId,
        });
    }
    if value.chars().any(char::is_whitespace) {
        return Err(match field {
            TokenField::AnchorName => ModelError::AnchorNameContainsWhitespace(value.to_owned()),
            TokenField::ObjectId => ModelError::IdContainsWhitespace(value.to_owned()),
            TokenField::ShapeTypeId => ModelError::ShapeTypeIdContainsWhitespace(value.to_owned()),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::arithmetic_side_effects,
        reason = "test code: arithmetic is on fixed literals"
    )]
    #![allow(
        clippy::expect_used,
        reason = "test code: panicking with the value on a failed construction is fine"
    )]

    use core::error::Error;

    use super::{ModelError, TokenField, validate_single_token};
    use crate::objects::ObjectId;

    /// Single-token validation rejects empty and whitespace values per field.
    #[test]
    fn token_validation_rejects_empty_and_whitespace() {
        assert_eq!(
            validate_single_token("", TokenField::ObjectId),
            Err(ModelError::EmptyId)
        );
        assert_eq!(
            validate_single_token("a b", TokenField::ObjectId),
            Err(ModelError::IdContainsWhitespace("a b".to_owned()))
        );
        assert_eq!(
            validate_single_token("", TokenField::ShapeTypeId),
            Err(ModelError::EmptyShapeTypeId)
        );
        assert_eq!(
            validate_single_token("a\tb", TokenField::ShapeTypeId),
            Err(ModelError::ShapeTypeIdContainsWhitespace("a\tb".to_owned()))
        );
        assert_eq!(
            validate_single_token("", TokenField::AnchorName),
            Err(ModelError::EmptyAnchorName)
        );
        assert_eq!(
            validate_single_token("n e", TokenField::AnchorName),
            Err(ModelError::AnchorNameContainsWhitespace("n e".to_owned()))
        );
        assert_eq!(validate_single_token("ok", TokenField::ObjectId), Ok(()));
    }

    /// `ModelError` is a plain-data error type: equality by value, stable messages.
    #[test]
    fn model_error_variants_carry_context() {
        let id = ObjectId::new("x").expect("valid id");
        let errors = [
            ModelError::DuplicateId(id.clone()),
            ModelError::UnknownObject(id.clone()),
            ModelError::ConnectorNotMovable(id.clone()),
            // Last use of `id`: move, no clone needed.
            ModelError::NotAShape(id),
            ModelError::ScaleTooSmall { scale: 0 },
        ];
        for expected in &errors {
            assert_eq!(*expected, expected.clone());
        }

        assert_eq!(
            ModelError::IdContainsWhitespace("a b".to_owned()).to_string(),
            "object id must be a single token (no whitespace): 'a b'"
        );
        assert_eq!(
            ModelError::ShapeTypeIdContainsWhitespace("s t".to_owned()).to_string(),
            "shape type id must be a single token (no whitespace): 's t'"
        );
        assert_eq!(
            ModelError::AnchorNameContainsWhitespace("n e".to_owned()).to_string(),
            "anchor name must be a single token (no whitespace): 'n e'"
        );
    }

    /// `ModelError` implements `std::error::Error` (via `core::error::Error`).
    #[test]
    fn model_error_implements_the_error_trait() {
        let error = ModelError::EmptyText;
        let as_dyn: &dyn Error = &error;
        assert_eq!(as_dyn.to_string(), "text content must not be empty");
    }

    /// Every remaining error variant renders a message naming the offending value.
    #[test]
    fn model_error_display_covers_remaining_variants() {
        let id = ObjectId::new("box-1").expect("valid id");
        assert_eq!(
            ModelError::ConnectorNotMovable(id.clone()).to_string(),
            "connectors cannot be moved or resized: box-1"
        );
        assert_eq!(
            ModelError::DuplicateId(id.clone()).to_string(),
            "object id already exists in diagram: box-1"
        );
        assert_eq!(
            ModelError::EmptyAnchorName.to_string(),
            "anchor name must not be empty"
        );
        assert_eq!(
            ModelError::EmptyId.to_string(),
            "object id must not be empty"
        );
        assert_eq!(
            ModelError::EmptyShapeTypeId.to_string(),
            "shape type id must not be empty"
        );
        assert_eq!(
            ModelError::NotAShape(id.clone()).to_string(),
            "expected a shape but found another object kind: box-1"
        );
        assert_eq!(
            ModelError::ScaleTooSmall { scale: 0 }.to_string(),
            "scale factor must be at least 1 (got 0)"
        );
        // Last use of `id`: move, no clone needed.
        assert_eq!(
            ModelError::UnknownObject(id).to_string(),
            "unknown object id: box-1"
        );
        assert_eq!(
            ModelError::ZeroSize {
                height: 2,
                width: 0
            }
            .to_string(),
            "object size must be at least 1x1 cell (got 0x2)"
        );
    }

    /// `TokenField` renders the field name used in validation messages.
    #[test]
    fn token_field_display() {
        assert_eq!(TokenField::AnchorName.to_string(), "anchor name");
        assert_eq!(TokenField::ObjectId.to_string(), "object id");
        assert_eq!(TokenField::ShapeTypeId.to_string(), "shape type id");
    }
}
