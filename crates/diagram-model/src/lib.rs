//! # diagram-model — core domain model of the Diagram context
//!
//! The [`Diagram`] aggregate and its cell-geometry value objects, per
//! [§8.4.1](../../../doc/arc42/08-building-block-view.md). This crate is a **leaf**: it has zero
//! workspace dependencies (checked by the T5 dependency audit) and no platform or GUI
//! dependencies — pure data in, deterministic behavior out (Q1).
//!
//! ## Model
//! - [`geometry::CellPos`] / [`geometry::CellSize`]: integer cell units on an unbounded grid with
//!   a top-left origin ([concept: cell grid](../../../doc/arc42/12-concepts/cell-grid.md)).
//!   Negative coordinates and zero-sized objects are unrepresentable by type construction.
//! - [`objects::Object`]: `Shape(ShapeInstance) | Text(TextInstance) | Connector(ConnectorInstance)`
//!   — shapes carry a catalog `type_id` (the only reference into the Shape Catalog context), a
//!   label and grid geometry; text is independent free content (FR10); connectors bind to
//!   [`anchors::AnchorRef`]s, not coordinates (FR9).
//! - [`Diagram`]: aggregate root holding objects in document order (= z-order) plus the
//!   whole-diagram scale (FR8). All mutations go through intent-revealing operations that
//!   enforce the boundary invariants; see [`invariants::ModelError`] for the failure surface.
//!
//! Invariants checked *here* (at the boundary): unique ids, single-token id/anchor names,
//! non-zero sizes, existing shape endpoints for connectors. Invariants needing the shape
//! catalog (an anchor name resolving to a cell inside its shape's bounds) are checked at render
//! time in `render-pipeline`, which alone sees both sides — see [`invariants`].

pub mod anchors;
pub mod geometry;
pub mod glyphs;
pub mod invariants;
pub mod objects;

use anchors::AnchorRef;
use geometry::{CellPos, CellSize};
use invariants::ModelError;
use objects::{
    ConnectorEnd, ConnectorInstance, Object, ObjectId, ShapeInstance, TextInstance, TipStyle,
};

/// The Diagram aggregate: an ordered collection of objects on the cell grid plus diagram-level
/// properties. Document order is z-order (later objects render on top).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagram {
    /// Objects in document order (= z-order).
    objects: Vec<Object>,
    /// Whole-diagram scale factor applied at render time (FR8); always ≥ 1.
    scale: u32,
}

impl Diagram {
    /// Adds a connector between two shape anchors. Both endpoints must reference existing
    /// shape objects (connectors cannot bind to text or other connectors).
    ///
    /// # Errors
    /// [`ModelError::DuplicateId`], [`ModelError::UnknownObject`] or [`ModelError::NotAShape`].
    pub fn connect(&mut self, instance: ConnectorInstance) -> Result<(), ModelError> {
        self.ensure_unique_id(&instance.id)?;
        self.validate_connector_end(&instance.from)?;
        self.validate_connector_end(&instance.to)?;
        self.objects.push(Object::Connector(instance));
        Ok(())
    }

    /// Inserts a manual bend (waypoint, model-space cell) into the connector at `index`
    /// (0..=bends.len()) — CR-007 / ADR 013 v4. Bends are plain coordinates: no
    /// referential integrity applies.
    ///
    /// # Errors
    /// [`ModelError::UnknownObject`], [`ModelError::NotAConnector`] or
    /// [`ModelError::BendIndexOutOfRange`].
    pub fn add_bend(
        &mut self,
        id: &ObjectId,
        index: usize,
        cell: CellPos,
    ) -> Result<(), ModelError> {
        let bends = self.connector_mut(id)?;
        if index > bends.len() {
            return Err(ModelError::BendIndexOutOfRange {
                object: id.clone(),
                index,
            });
        }
        bends.insert(index, cell);
        Ok(())
    }

    /// Moves the connector's bend at `index` to `cell` (CR-007 / ADR 013 v4).
    ///
    /// # Errors
    /// [`ModelError::UnknownObject`], [`ModelError::NotAConnector`] or
    /// [`ModelError::BendIndexOutOfRange`].
    pub fn move_bend(
        &mut self,
        id: &ObjectId,
        index: usize,
        cell: CellPos,
    ) -> Result<(), ModelError> {
        let bends = self.connector_mut(id)?;
        *bends
            .get_mut(index)
            .ok_or_else(|| ModelError::BendIndexOutOfRange {
                object: id.clone(),
                index,
            })? = cell;
        Ok(())
    }

    /// Removes the connector's bend at `index` (CR-007 / ADR 013 v4).
    ///
    /// # Errors
    /// [`ModelError::UnknownObject`], [`ModelError::NotAConnector`] or
    /// [`ModelError::BendIndexOutOfRange`].
    pub fn remove_bend(&mut self, id: &ObjectId, index: usize) -> Result<(), ModelError> {
        let bends = self.connector_mut(id)?;
        if index >= bends.len() {
            return Err(ModelError::BendIndexOutOfRange {
                object: id.clone(),
                index,
            });
        }
        bends.remove(index);
        Ok(())
    }

    /// Sets the tip style of one end of a connector (CR-008 / ADR 013 v5).
    ///
    /// # Errors
    /// [`ModelError::UnknownObject`] when the id does not exist, or
    /// [`ModelError::NotAConnector`] when it exists but is another object kind.
    pub fn set_tip(
        &mut self,
        id: &ObjectId,
        end: ConnectorEnd,
        style: TipStyle,
    ) -> Result<(), ModelError> {
        let connector = match self.get_mut(id) {
            Some(Object::Connector(connector)) => connector,
            None => return Err(ModelError::UnknownObject(id.clone())),
            Some(_) => return Err(ModelError::NotAConnector(id.clone())),
        };
        match end {
            ConnectorEnd::Source => connector.from_tip = style,
            ConnectorEnd::Target => connector.to_tip = style,
        }
        Ok(())
    }

    /// Mutable access to a connector's bend list (aggregate-internal).
    ///
    /// # Errors
    /// [`ModelError::UnknownObject`] when the id does not exist, or
    /// [`ModelError::NotAConnector`] when it exists but is another object kind.
    fn connector_mut(&mut self, id: &ObjectId) -> Result<&mut Vec<CellPos>, ModelError> {
        match self.get_mut(id) {
            Some(Object::Connector(connector)) => Ok(&mut connector.bends),
            None => Err(ModelError::UnknownObject(id.clone())),
            Some(_) => Err(ModelError::NotAConnector(id.clone())),
        }
    }

    /// Rejects an id that is already taken.
    ///
    /// # Errors
    /// [`ModelError::DuplicateId`].
    fn ensure_unique_id(&self, id: &ObjectId) -> Result<(), ModelError> {
        if self.objects.iter().any(|object| object.id() == id) {
            return Err(ModelError::DuplicateId(id.clone()));
        }
        Ok(())
    }

    /// Looks up an object by id.
    #[must_use]
    pub fn get(&self, id: &ObjectId) -> Option<&Object> {
        self.objects.iter().find(|object| object.id() == id)
    }

    /// Mutable lookup (aggregate-internal; mutations must go through the intent-revealing operations).
    fn get_mut(&mut self, id: &ObjectId) -> Option<&mut Object> {
        self.objects.iter_mut().find(|object| object.id() == id)
    }

    /// Whether the diagram has no objects.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Number of objects (shapes + text + connectors).
    #[must_use]
    pub const fn len(&self) -> usize {
        self.objects.len()
    }

    /// Moves a shape or text element to a new top-left position. Connectors have no position of
    /// their own — they follow their bound shapes (FR9).
    ///
    /// # Errors
    /// [`ModelError::UnknownObject`] or [`ModelError::ConnectorNotMovable`].
    pub fn move_object(&mut self, id: &ObjectId, to: CellPos) -> Result<(), ModelError> {
        let object = self
            .get_mut(id)
            .ok_or_else(|| ModelError::UnknownObject(id.clone()))?;
        match object {
            Object::Shape(shape) => shape.pos = to,
            Object::Text(text) => text.pos = to,
            Object::Connector(connector) => {
                return Err(ModelError::ConnectorNotMovable(connector.id.clone()));
            }
        }
        Ok(())
    }

    /// An empty diagram with the default scale of 1.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            objects: Vec::new(),
            scale: 1,
        }
    }

    /// The next deterministic id for a new object: `obj-{n}` with the smallest `n` not yet used.
    ///
    /// Deterministic by construction (no hash iteration, no randomness), so the same sequence of
    /// operations always yields the same ids — Q1.
    #[must_use]
    pub fn new_object_id(&self) -> ObjectId {
        let mut highest: u32 = 0;
        for object in &self.objects {
            if let Some(digits) = object.id().as_str().strip_prefix("obj-")
                && let Ok(n) = digits.parse::<u32>()
            {
                highest = highest.max(n);
            }
        }
        ObjectId::generated(highest.saturating_add(1))
    }

    /// All objects in document order (= z-order).
    #[must_use]
    pub fn objects(&self) -> &[Object] {
        &self.objects
    }

    /// Places a shape instance; it is appended (top of z-order).
    ///
    /// # Errors
    /// [`ModelError::DuplicateId`] if the id is already taken.
    pub fn place_shape(&mut self, instance: ShapeInstance) -> Result<(), ModelError> {
        self.ensure_unique_id(&instance.id)?;
        self.objects.push(Object::Shape(instance));
        Ok(())
    }

    /// Replaces the content of a free text object (FR10). The rendered extent follows the
    /// content at render time; empty content is rejected.
    ///
    /// # Errors
    /// [`ModelError::EmptyText`], [`ModelError::UnknownObject`] or [`ModelError::NotAText`].
    pub fn set_text(&mut self, id: &ObjectId, text: &str) -> Result<(), ModelError> {
        if text.is_empty() {
            return Err(ModelError::EmptyText);
        }
        let object = self
            .get_mut(id)
            .ok_or_else(|| ModelError::UnknownObject(id.clone()))?;
        match object {
            Object::Text(instance) => instance.text = text.to_owned(),
            _ => return Err(ModelError::NotAText(id.clone())),
        }
        Ok(())
    }

    /// Places a free-text element; it is appended (top of z-order).
    ///
    /// # Errors
    /// [`ModelError::DuplicateId`] if the id is already taken.
    pub fn place_text(&mut self, instance: TextInstance) -> Result<(), ModelError> {
        self.ensure_unique_id(&instance.id)?;
        self.objects.push(Object::Text(instance));
        Ok(())
    }

    /// Removes an object and, for shapes, every connector that references it (a connector's
    /// endpoints must never dangle). Returns how many objects were removed in total.
    ///
    /// # Errors
    /// [`ModelError::UnknownObject`].
    pub fn remove(&mut self, id: &ObjectId) -> Result<usize, ModelError> {
        if self.get(id).is_none() {
            return Err(ModelError::UnknownObject(id.clone()));
        }
        let before = self.len();
        self.objects.retain(|object| {
            object.id() != id
                && !matches!(
                    object,
                    Object::Connector(connector)
                        if *connector.from.object() == *id || *connector.to.object() == *id
                )
        });
        Ok(before.saturating_sub(self.len()))
    }

    /// Resizes a shape instance. Text extent is derived from its content and connectors have no
    /// size, so both are rejected.
    ///
    /// # Errors
    /// [`ModelError::UnknownObject`] or [`ModelError::NotAShape`].
    pub fn resize(&mut self, id: &ObjectId, size: CellSize) -> Result<(), ModelError> {
        let object = self
            .get_mut(id)
            .ok_or_else(|| ModelError::UnknownObject(id.clone()))?;
        if let Object::Shape(shape) = object {
            shape.size = size;
            Ok(())
        } else {
            Err(ModelError::NotAShape(object.id().clone()))
        }
    }

    /// The current whole-diagram scale (≥ 1).
    #[must_use]
    pub const fn scale(&self) -> u32 {
        self.scale
    }

    /// Sets the whole-diagram scale (FR8).
    ///
    /// # Errors
    /// [`ModelError::ScaleTooSmall`] if `scale` is zero.
    pub const fn set_scale(&mut self, scale: u32) -> Result<(), ModelError> {
        if scale < 1 {
            return Err(ModelError::ScaleTooSmall { scale });
        }
        self.scale = scale;
        Ok(())
    }

    /// A connector endpoint must reference an existing shape.
    ///
    /// # Errors
    /// [`ModelError::UnknownObject`] or [`ModelError::NotAShape`].
    fn validate_connector_end(&self, end: &AnchorRef) -> Result<(), ModelError> {
        match self.get(end.object()) {
            Some(&Object::Shape(_)) => Ok(()),
            Some(_) => Err(ModelError::NotAShape(end.object().clone())),
            None => Err(ModelError::UnknownObject(end.object().clone())),
        }
    }

    /// A diagram with an explicit initial scale.
    ///
    /// # Errors
    /// [`ModelError::ScaleTooSmall`] if `scale` is zero.
    pub fn with_scale(scale: u32) -> Result<Self, ModelError> {
        let mut diagram = Self::new();
        diagram.set_scale(scale)?;
        Ok(diagram)
    }
}

impl Default for Diagram {
    /// An empty diagram with scale 1.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::missing_asserts_for_indexing,
        reason = "test code: panicking with the value on a failed construction is fine; indexing known test fixtures by position is idiomatic"
    )]

    use super::{Diagram, ModelError};
    use crate::anchors::AnchorRef;
    use crate::geometry::{CellPos, CellSize};
    use crate::objects::{
        ConnectorEnd, ConnectorInstance, ConnectorStyle, Object, ObjectId, ShapeInstance,
        TextInstance, TipStyle,
    };

    /// Builds a solid connector between two shapes' `e`/`w` anchors.
    fn connector(id: &str, from: &str, to: &str) -> ConnectorInstance {
        ConnectorInstance::new(
            ObjectId::new(id).expect("valid id"),
            AnchorRef::new(ObjectId::new(from).expect("valid id"), "e").expect("valid anchor"),
            AnchorRef::new(ObjectId::new(to).expect("valid id"), "w").expect("valid anchor"),
            ConnectorStyle::Solid,
        )
    }

    /// `Default` matches `new`.
    #[test]
    fn default_matches_new() {
        assert_eq!(Diagram::default(), Diagram::new());
    }

    /// Move applies to shapes and text only; connectors follow their shapes.
    #[test]
    fn move_object_moves_shapes_and_text_only() {
        let mut diagram = Diagram::new();
        diagram.place_shape(shape("s1", "box")).expect("places");
        diagram.place_text(text("t1")).expect("places");
        diagram
            .connect(connector("c1", "s1", "s1"))
            .expect("connects");

        assert_eq!(diagram.move_object(&oid("s1"), CellPos::new(9, 4)), Ok(()));
        let moved = diagram.get(&oid("s1")).expect("present").clone();
        let Object::Shape(shape) = moved else {
            panic!("expected shape")
        };
        assert_eq!(shape.pos, CellPos::new(9, 4));

        assert_eq!(diagram.move_object(&oid("t1"), CellPos::new(0, 7)), Ok(()));
        assert_eq!(
            diagram.move_object(&oid("c1"), CellPos::origin()),
            Err(ModelError::ConnectorNotMovable(oid("c1")))
        );
        assert_eq!(
            diagram.move_object(&oid("ghost"), CellPos::origin()),
            Err(ModelError::UnknownObject(oid("ghost")))
        );
    }

    /// `set_text` replaces the content of a text object and rejects empty content,
    /// unknown ids, and non-text targets (FR10).
    #[test]
    fn set_text_edits_content_and_validates() {
        let mut diagram = Diagram::new();
        diagram.place_shape(shape("s1", "box")).expect("places");
        diagram.place_text(text("t1")).expect("places");

        assert_eq!(diagram.set_text(&oid("t1"), "hello\nworld"), Ok(()));
        let Object::Text(instance) = diagram.get(&oid("t1")).expect("present") else {
            panic!("expected text")
        };
        assert_eq!(instance.text, "hello\nworld");

        assert_eq!(diagram.set_text(&oid("t1"), ""), Err(ModelError::EmptyText));
        assert_eq!(
            diagram.set_text(&oid("ghost"), "x"),
            Err(ModelError::UnknownObject(oid("ghost")))
        );
        assert_eq!(
            diagram.set_text(&oid("s1"), "x"),
            Err(ModelError::NotAText(oid("s1")))
        );
    }

    /// A fresh diagram is empty with scale 1.
    #[test]
    fn new_diagram_is_empty_with_scale_one() {
        let diagram = Diagram::new();
        assert_eq!(diagram.len(), 0);
        assert!(diagram.is_empty());
        assert_eq!(diagram.scale(), 1);
        assert_eq!(diagram.objects().len(), 0);
    }

    /// Generated ids are deterministic and skip already-used numbers.
    #[test]
    fn new_object_id_is_deterministic_and_skips_used_numbers() {
        let mut diagram = Diagram::new();
        assert_eq!(diagram.new_object_id(), oid("obj-1"));

        // Custom ids that are not obj-N do not affect the counter.
        diagram.place_shape(shape("custom", "box")).expect("places");
        assert_eq!(diagram.new_object_id(), oid("obj-1"));

        diagram.place_shape(shape("obj-3", "box")).expect("places");
        diagram
            .place_shape(shape("obj-2", "circle"))
            .expect("places");
        assert_eq!(diagram.new_object_id(), oid("obj-4"));
    }

    /// Builds an object id from a test string.
    fn oid(name: &str) -> ObjectId {
        ObjectId::new(name).expect("valid id")
    }

    /// Duplicate ids are rejected on placement.
    #[test]
    fn place_rejects_duplicate_ids() {
        let mut diagram = Diagram::new();
        diagram.place_shape(shape("s1", "box")).expect("places");
        assert_eq!(
            diagram.place_shape(shape("s1", "circle")),
            Err(ModelError::DuplicateId(oid("s1")))
        );
        assert_eq!(
            diagram.place_text(text("s1")),
            Err(ModelError::DuplicateId(oid("s1")))
        );
    }

    /// Placed objects keep document order (= z-order) and are findable by id.
    #[test]
    fn place_shape_and_text_in_document_order() {
        let mut diagram = Diagram::new();
        diagram.place_shape(shape("s1", "box")).expect("places");
        diagram.place_text(text("t1")).expect("places");
        diagram.place_shape(shape("s2", "circle")).expect("places");

        assert_eq!(diagram.len(), 3);
        let objects = diagram.objects();
        assert!(matches!(objects[0], Object::Shape(_)));
        assert!(matches!(objects[1], Object::Text(_)));
        assert!(matches!(objects[2], Object::Shape(_)));

        let found = diagram.get(&oid("t1")).expect("present");
        assert!(matches!(found, Object::Text(_)));
        assert!(diagram.get(&oid("nope")).is_none());
    }

    /// Removing a shape cascades to every connector referencing it.
    #[test]
    fn remove_cascades_to_referencing_connectors() {
        let mut diagram = Diagram::new();
        diagram.place_shape(shape("s1", "box")).expect("places");
        diagram.place_shape(shape("s2", "circle")).expect("places");
        diagram
            .connect(connector("c1", "s1", "s2"))
            .expect("connects");
        diagram
            .connect(connector("c2", "s2", "s1"))
            .expect("connects");
        diagram
            .connect(connector("c3", "s1", "s1"))
            .expect("connects");

        // Removing s1 takes c1, c2 and c3 with it; s2 survives.
        assert_eq!(diagram.remove(&oid("s1")), Ok(4));
        assert_eq!(diagram.len(), 1);
        assert!(diagram.get(&oid("s2")).is_some());
        assert!(diagram.get(&oid("c3")).is_none());

        // Removing a bare connector removes exactly itself.
        diagram
            .connect(connector("c4", "s2", "s2"))
            .expect("connects");
        assert_eq!(diagram.remove(&oid("c4")), Ok(1));

        assert_eq!(
            diagram.remove(&oid("ghost")),
            Err(ModelError::UnknownObject(oid("ghost")))
        );
    }

    /// A diagram with two boxes and one solid connector between them.
    fn diagram_with_connector() -> Diagram {
        let mut diagram = Diagram::new();
        diagram.place_shape(shape("s1", "box")).expect("places");
        diagram.place_shape(shape("s2", "circle")).expect("places");
        diagram
            .connect(connector("c1", "s1", "s2"))
            .expect("connects");
        diagram
    }

    /// Connectors start without manual bends (automatic routing — CR-007).
    #[test]
    fn connectors_start_bend_free() {
        let diagram = diagram_with_connector();
        let Object::Connector(connector) = diagram.get(&oid("c1")).expect("present") else {
            panic!("expected connector")
        };
        assert!(connector.bends.is_empty());
    }

    /// add/move/remove bend round-trip through the aggregate (CR-007 / ADR 013 v4).
    #[test]
    fn bend_operations_round_trip() {
        let mut diagram = diagram_with_connector();
        let id = oid("c1");

        // Insert at both ends and in the middle — index is a position, not a key.
        diagram
            .add_bend(&id, 0, CellPos::new(3, 2))
            .expect("inserts first");
        diagram
            .add_bend(&id, 1, CellPos::new(6, 4))
            .expect("inserts second");
        let Object::Connector(connector) = diagram.get(&id).expect("present") else {
            panic!("expected connector")
        };
        assert_eq!(
            connector.bends,
            vec![CellPos::new(3, 2), CellPos::new(6, 4)]
        );

        diagram
            .move_bend(&id, 0, CellPos::new(3, 9))
            .expect("moves");
        diagram.remove_bend(&id, 1).expect("removes");
        let Object::Connector(connector) = diagram.get(&id).expect("present") else {
            panic!("expected connector")
        };
        assert_eq!(connector.bends, vec![CellPos::new(3, 9)]);
    }

    /// Bend ops fail with typed errors for unknown ids, wrong kinds and bad indices.
    #[test]
    fn bend_operations_report_typed_errors() {
        let mut diagram = diagram_with_connector();
        let shape_id = oid("s1");

        assert_eq!(
            diagram.add_bend(&oid("ghost"), 0, CellPos::origin()),
            Err(ModelError::UnknownObject(oid("ghost")))
        );
        assert_eq!(
            diagram.move_bend(&shape_id, 0, CellPos::origin()),
            Err(ModelError::NotAConnector(shape_id.clone()))
        );
        let id = oid("c1");
        assert_eq!(
            diagram.add_bend(&id, 1, CellPos::origin()),
            Err(ModelError::BendIndexOutOfRange {
                object: id.clone(),
                index: 1
            })
        );
        assert_eq!(
            diagram.move_bend(&id, 0, CellPos::origin()),
            Err(ModelError::BendIndexOutOfRange {
                object: id.clone(),
                index: 0
            })
        );
        assert_eq!(
            diagram.remove_bend(&id, 0),
            Err(ModelError::BendIndexOutOfRange {
                object: id,
                index: 0
            })
        );
    }

    /// New connectors carry the CR-008 defaults: source bare, target hollow arrow — so
    /// every pre-existing diagram renders identically.
    #[test]
    fn connectors_start_with_default_tips() {
        let diagram = diagram_with_connector();
        let connector = diagram.objects().iter().find_map(|object| match object {
            Object::Connector(connector) => Some(connector),
            _ => None,
        });
        let Some(connector) = connector else {
            panic!("test fixture lost its connector")
        };
        assert_eq!(connector.from_tip, TipStyle::Bare);
        assert_eq!(connector.to_tip, TipStyle::HollowArrow);
    }

    /// `set_tip` updates exactly the addressed end; typed errors mirror the bend ops.
    #[test]
    fn set_tip_updates_the_addressed_end_and_reports_typed_errors() {
        let mut diagram = diagram_with_connector();
        let id = oid("c1");
        let shape_id = oid("s1");

        diagram
            .set_tip(&id, ConnectorEnd::Target, TipStyle::FilledRhombus)
            .expect("sets the target tip");
        diagram
            .set_tip(&id, ConnectorEnd::Source, TipStyle::EmptyRhombus)
            .expect("sets the source tip");
        let Object::Connector(connector) = diagram.get(&id).expect("still present") else {
            panic!("test fixture lost its connector");
        };
        assert_eq!(connector.from_tip, TipStyle::EmptyRhombus);
        assert_eq!(connector.to_tip, TipStyle::FilledRhombus);

        assert_eq!(
            diagram.set_tip(&oid("ghost"), ConnectorEnd::Source, TipStyle::Bare),
            Err(ModelError::UnknownObject(oid("ghost")))
        );
        assert_eq!(
            diagram.set_tip(&shape_id, ConnectorEnd::Target, TipStyle::Bare),
            Err(ModelError::NotAConnector(shape_id))
        );
    }

    /// Resize applies to shapes only.
    #[test]
    fn resize_only_applies_to_shapes() {
        let mut diagram = Diagram::new();
        diagram.place_shape(shape("s1", "box")).expect("places");
        diagram.place_text(text("t1")).expect("places");

        assert_eq!(
            diagram.resize(&oid("s1"), CellSize::new(10, 3).expect("valid size")),
            Ok(())
        );
        let resized = diagram.get(&oid("s1")).expect("present").clone();
        let Object::Shape(shape) = resized else {
            panic!("expected shape")
        };
        assert_eq!(shape.size, CellSize::new(10, 3).expect("valid size"));

        assert_eq!(
            diagram.resize(&oid("t1"), CellSize::unit()),
            Err(ModelError::NotAShape(oid("t1")))
        );
        assert_eq!(
            diagram.resize(&oid("ghost"), CellSize::unit()),
            Err(ModelError::UnknownObject(oid("ghost")))
        );
    }

    /// Builds a test shape at (1,1) with size 4x2.
    fn shape(id: &str, type_id: &str) -> ShapeInstance {
        ShapeInstance::new(
            ObjectId::new(id).expect("valid id"),
            type_id,
            "label",
            CellPos::new(1, 1),
            CellSize::new(4, 2).expect("valid size"),
        )
        .expect("valid shape")
    }

    /// Builds a test text element at the origin.
    fn text(id: &str) -> TextInstance {
        TextInstance::new(
            ObjectId::new(id).expect("valid id"),
            CellPos::origin(),
            "hello",
        )
        .expect("valid text")
    }

    /// Connectors require existing shape endpoints; self-connections are allowed.
    #[test]
    fn connect_requires_existing_shape_endpoints() {
        let mut diagram = Diagram::new();
        diagram.place_shape(shape("s1", "box")).expect("places");
        diagram.place_text(text("t1")).expect("places");

        assert_eq!(diagram.connect(connector("c1", "s1", "s1")), Ok(()));
        assert_eq!(
            diagram.connect(connector("c2", "s1", "ghost")),
            Err(ModelError::UnknownObject(oid("ghost")))
        );
        assert_eq!(
            diagram.connect(connector("c3", "t1", "s1")),
            Err(ModelError::NotAShape(oid("t1")))
        );
        assert_eq!(
            diagram.connect(connector("c4", "s1", "t1")),
            Err(ModelError::NotAShape(oid("t1")))
        );
        assert_eq!(
            diagram.connect(connector("c1", "s1", "s1")),
            Err(ModelError::DuplicateId(oid("c1")))
        );

        let connector = diagram.get(&oid("c1")).expect("present");
        assert!(matches!(connector, Object::Connector(_)));
    }

    /// Scale can be set at construction and later; zero is rejected.
    #[test]
    fn with_scale_and_set_scale() {
        let mut diagram = Diagram::with_scale(2).expect("valid scale");
        assert_eq!(diagram.scale(), 2);
        assert_eq!(
            Diagram::with_scale(0),
            Err(ModelError::ScaleTooSmall { scale: 0 })
        );
        assert_eq!(diagram.set_scale(3), Ok(()));
        assert_eq!(
            diagram.set_scale(0),
            Err(ModelError::ScaleTooSmall { scale: 0 })
        );
        assert_eq!(diagram.scale(), 3);
    }

    /// The diagram is a value type: clones are independent.
    #[test]
    fn diagram_is_a_value_type() {
        let mut base = Diagram::new();
        base.place_shape(shape("s1", "box")).expect("places");
        let clone = base.clone();
        base.place_text(text("t1")).expect("places");
        assert_eq!(clone.len(), 1);
        assert_eq!(base.len(), 2);
        assert_ne!(base, clone);
    }
}
