//! The object model of the Diagram aggregate: identity ([`ObjectId`]), the three
//! entity kinds ([`Object`] — shape, text, connector) and their instances.
//!
//! Instances carry geometry as value objects (`CellPos`/`CellSize`, see [`crate::geometry`])
//! and are validated at construction; the aggregate ([`crate::Diagram`]) validates
//! cross-object invariants on every mutation.

use core::fmt;

use crate::anchors::AnchorRef;
use crate::geometry::{CellPos, CellSize};
use crate::invariants::{ModelError, TokenField, validate_single_token};

/// Stable identity of an object within a diagram.
///
/// Ids are single tokens (non-empty, no whitespace) because they are referenced by name in the
/// line-based `.ad` format (connectors reference shape ids) and must survive round-trips.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectId(String);

impl ObjectId {
    /// The id as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Builds the deterministic generated id `obj-{n}` (never invalid, so constructed directly).
    ///
    /// This is the single source of generated ids: [`crate::Diagram::new_object_id`] uses it so
    /// that the same operation sequence always yields the same ids (Q1).
    #[must_use]
    pub fn generated(n: u32) -> Self {
        let mut id = String::from("obj-");
        id.push_str(&n.to_string());
        Self(id)
    }

    /// Creates an id, rejecting empty or whitespace-containing values.
    ///
    /// # Errors
    /// [`ModelError::EmptyId`] or [`ModelError::IdContainsWhitespace`].
    pub fn new(id: &str) -> Result<Self, ModelError> {
        validate_single_token(id, TokenField::ObjectId)?;
        Ok(Self(id.to_owned()))
    }
}

impl fmt::Display for ObjectId {
    /// Renders the id exactly as stored.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Visual style of a connector line (FR9). Arrowheads are an open spec point of the
/// `.ad` format ([ADR 004](../../../doc/adr/004-diagram-file-format.md)); the enum grows there first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ConnectorStyle {
    /// A dashed line.
    Dashed,
    /// A solid line (`-` / `|` segments).
    #[default]
    Solid,
}

/// A placed instance of a shape type from the Shape Catalog context.
///
/// `type_id` is the *only* reference into the catalog (Customer/Supplier mapping): this crate
/// never resolves it; rendering does, with the catalog in hand. `label` is the text drawn
/// inside the shape (may be empty); all coordinates are cell units (FR7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeInstance {
    /// Stable identity of this instance.
    pub id: ObjectId,
    /// Label rendered inside the shape; empty for an unlabeled shape.
    pub label: String,
    /// Top-left corner on the cell grid.
    pub pos: CellPos,
    /// Extent in cells.
    pub size: CellSize,
    /// Reference to the shape type in the catalog (single token, unresolved here).
    pub type_id: String,
}

impl ShapeInstance {
    /// Creates a shape instance, validating the type id. Geometry is pre-validated by its value objects.
    ///
    /// # Errors
    /// [`ModelError::EmptyShapeTypeId`] or [`ModelError::ShapeTypeIdContainsWhitespace`].
    pub fn new(
        id: ObjectId,
        type_id: &str,
        label: &str,
        pos: CellPos,
        size: CellSize,
    ) -> Result<Self, ModelError> {
        validate_single_token(type_id, TokenField::ShapeTypeId)?;
        Ok(Self {
            id,
            label: label.to_owned(),
            pos,
            size,
            type_id: type_id.to_owned(),
        })
    }
}

/// Free text placed on the canvas independently of shapes (FR10).
///
/// Stamped at `pos` starting in the top-left cell; multi-line content is allowed and renders
/// one line per grid row. Its extent is derived from the content at render time — it has no
/// stored size, so it cannot be resized (`Diagram::resize` rejects it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInstance {
    /// Stable identity of this instance.
    pub id: ObjectId,
    /// Top-left corner on the cell grid.
    pub pos: CellPos,
    /// The text content (non-empty; may contain newlines).
    pub text: String,
}

impl TextInstance {
    /// Creates a text instance, rejecting empty content.
    ///
    /// # Errors
    /// [`ModelError::EmptyText`].
    pub fn new(id: ObjectId, pos: CellPos, text: &str) -> Result<Self, ModelError> {
        if text.is_empty() {
            return Err(ModelError::EmptyText);
        }
        Ok(Self {
            id,
            pos,
            text: text.to_owned(),
        })
    }
}

/// A line/arrow between two shape anchors (FR9).
///
/// Bound to anchors, not raw coordinates: when a bound shape moves or resizes, the connector
/// stays attached because its endpoints are re-resolved from `(object, anchor name)` at render time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorInstance {
    /// Starting endpoint (an anchor on a shape).
    pub from: AnchorRef,
    /// Stable identity of this connector.
    pub id: ObjectId,
    /// Line style.
    pub style: ConnectorStyle,
    /// Ending endpoint (an anchor on a shape).
    pub to: AnchorRef,
}

impl ConnectorInstance {
    /// Creates a connector. Endpoints are pre-validated [`AnchorRef`]s; cross-object checks
    /// (both ends exist and are shapes) happen in [`crate::Diagram::connect`].
    #[must_use]
    pub const fn new(id: ObjectId, from: AnchorRef, to: AnchorRef, style: ConnectorStyle) -> Self {
        Self {
            from,
            id,
            style,
            to,
        }
    }
}

/// An object on the canvas. Document order in the aggregate is z-order (later = on top).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Object {
    /// A connector between two shape anchors.
    Connector(ConnectorInstance),
    /// A placed shape instance.
    Shape(ShapeInstance),
    /// Free text.
    Text(TextInstance),
}

impl Object {
    /// The object's stable identity, regardless of kind.
    #[must_use]
    pub const fn id(&self) -> &ObjectId {
        match self {
            Self::Connector(connector) => &connector.id,
            Self::Shape(shape) => &shape.id,
            Self::Text(text) => &text.id,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::panic,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::arithmetic_side_effects,
        clippy::indexing_slicing,
        clippy::missing_docs_in_private_items,
        reason = "test code: panicking on assertion failure is the correct behavior; literal arithmetic and indexing are idiomatic in tests"
    )]

    use super::{ConnectorInstance, ConnectorStyle, Object, ObjectId, ShapeInstance, TextInstance};
    use crate::anchors::AnchorRef;
    use crate::geometry::{CellPos, CellSize};
    use crate::invariants::ModelError;

    /// Builds an object id from a test string.
    fn id(name: &str) -> ObjectId {
        ObjectId::new(name).expect("valid test id")
    }

    /// Ids must be non-empty single tokens.
    #[test]
    fn object_id_validates_content() {
        assert_eq!(id("box-1").as_str(), "box-1");
        assert_eq!(ObjectId::new(""), Err(ModelError::EmptyId));
        assert_eq!(
            ObjectId::new("a b"),
            Err(ModelError::IdContainsWhitespace("a b".to_owned()))
        );
    }

    /// `Display` renders the id verbatim; `generated` builds `obj-{n}`.
    #[test]
    fn object_id_display_and_generated() {
        assert_eq!(id("x-9").to_string(), "x-9");
        let generated = ObjectId::generated(7);
        assert_eq!(generated.as_str(), "obj-7");
        assert_eq!(ObjectId::generated(42).as_str(), "obj-42");
    }

    /// The default connector style is solid.
    #[test]
    fn connector_style_defaults_to_solid() {
        assert_eq!(ConnectorStyle::default(), ConnectorStyle::Solid);
        assert_ne!(ConnectorStyle::Solid, ConnectorStyle::Dashed);
    }

    /// Shape instances validate their type id and store label/geometry.
    #[test]
    fn shape_instance_new_validates_type_id() {
        let instance = ShapeInstance::new(
            id("s1"),
            "box",
            "User",
            CellPos::new(1, 2),
            CellSize::new(4, 2).expect("valid"),
        )
        .expect("valid type id");
        assert_eq!(instance.type_id, "box");
        assert_eq!(instance.label, "User");
        assert_eq!(instance.pos, CellPos::new(1, 2));

        assert_eq!(
            ShapeInstance::new(id("s2"), "", "", CellPos::origin(), CellSize::unit()),
            Err(ModelError::EmptyShapeTypeId)
        );
        assert_eq!(
            ShapeInstance::new(id("s3"), "box x", "", CellPos::origin(), CellSize::unit()),
            Err(ModelError::ShapeTypeIdContainsWhitespace(
                "box x".to_owned()
            ))
        );
    }

    /// An empty label is legal (an unlabeled shape).
    #[test]
    fn shape_instance_allows_empty_label() {
        let instance =
            ShapeInstance::new(id("s4"), "circle", "", CellPos::origin(), CellSize::unit())
                .expect("valid");
        assert_eq!(instance.label, "");
    }

    /// Text instances reject empty content; multi-line content is stored verbatim.
    #[test]
    fn text_instance_rejects_empty_content() {
        assert_eq!(
            TextInstance::new(id("t1"), CellPos::origin(), ""),
            Err(ModelError::EmptyText)
        );
        let multi =
            TextInstance::new(id("t2"), CellPos::new(0, 3), "line one\nline two").expect("valid");
        assert_eq!(multi.text, "line one\nline two");
    }

    /// Connectors hold their id, endpoints and style.
    #[test]
    fn connector_instance_holds_its_fields() {
        let from = AnchorRef::new(id("s1"), "e").expect("valid anchor");
        let to = AnchorRef::new(id("s2"), "w").expect("valid anchor");
        let connector =
            ConnectorInstance::new(id("c1"), from.clone(), to.clone(), ConnectorStyle::Dashed);
        assert_eq!(connector.id, id("c1"));
        assert_eq!(connector.from, from);
        assert_eq!(connector.to, to);
        assert_eq!(connector.style, ConnectorStyle::Dashed);
    }

    /// `Object::id` works for all three kinds.
    #[test]
    fn object_id_accessor_covers_all_kinds() {
        let shape = Object::Shape(
            ShapeInstance::new(id("s1"), "box", "", CellPos::origin(), CellSize::unit())
                .expect("valid"),
        );
        let text =
            Object::Text(TextInstance::new(id("t1"), CellPos::origin(), "hi").expect("valid"));
        let connector = Object::Connector(ConnectorInstance::new(
            id("c1"),
            AnchorRef::new(id("s1"), "e").expect("valid anchor"),
            AnchorRef::new(id("s2"), "w").expect("valid anchor"),
            ConnectorStyle::Solid,
        ));
        assert_eq!(shape.id(), &id("s1"));
        assert_eq!(text.id(), &id("t1"));
        assert_eq!(connector.id(), &id("c1"));
    }
}
