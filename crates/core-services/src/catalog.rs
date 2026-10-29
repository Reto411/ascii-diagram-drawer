//! Catalog backend (ADR 009): wraps `shape-registry` and presents its data as the
//! seam's read models — registry types never cross into the GUI. Registration
//! (S-017) mutates the shared state that the renderer reads from the same handle.

use shape_registry::{ShapeRegistry, ShapeType};
use ui_api::{AnchorInfo, CatalogError, NewTemplate, ShapeCatalog, ShapeInfo};

use crate::registry_state::SharedRegistry;

/// The collection a shape belongs to (via its qualified `collection/name` id), if any.
/// Bare ids are built-ins and never belong to a collection — even when a user
/// collection happens to be named like one.
fn collection_of(registry: &ShapeRegistry, shape: &ShapeType) -> Option<String> {
    let (prefix, _) = shape.type_id.split_once('/')?;
    registry.collection(prefix).map(|c| c.name.clone())
}

/// Converts a registry shape type to its seam-level read model (geometry crosses
/// from the registry's local value objects into `diagram-model`'s shared kernel).
fn to_info(shape: &shape_registry::ShapeType, collection: Option<&str>) -> ShapeInfo {
    let pattern = &shape.glyph_pattern;
    ShapeInfo {
        type_id: shape.type_id.clone(),
        name: shape.name.clone(),
        size: diagram_model::geometry::CellSize {
            width: pattern.width(),
            height: pattern.height(),
        },
        collection: collection.map(str::to_owned),
        preview: (0..pattern.height())
            .filter_map(|y| pattern.row(y))
            .collect(),
    }
}

/// Presents the shared registry — built-ins plus every loaded collection, in order.
#[derive(Debug)]
pub struct RegistryCatalog {
    state: SharedRegistry,
}

impl RegistryCatalog {
    /// Binds `registry` as this catalog's source (standalone state — no sharing with
    /// a renderer; for tests and simple compositions).
    #[must_use]
    pub fn new(registry: ShapeRegistry) -> Self {
        Self::with_shared(SharedRegistry::standalone(registry))
    }

    /// Binds the shared state composed with a [`PipelineRenderer`](crate::PipelineRenderer)
    /// — registration is visible to both services at once (S-017).
    #[must_use]
    pub fn with_shared(state: SharedRegistry) -> Self {
        Self { state }
    }
}

impl ShapeCatalog for RegistryCatalog {
    fn list(&self) -> Vec<ShapeInfo> {
        let registry = self.state.snapshot();
        registry
            .builtins()
            .map(|s| to_info(s, None))
            .chain(registry.collections().flat_map(|c| {
                let label = c.name.clone();
                c.types
                    .iter()
                    .map(move |s| to_info(s, Some(label.as_str())))
            }))
            .collect()
    }

    fn get(&self, type_id: &str) -> Option<ShapeInfo> {
        let registry = self.state.snapshot();
        registry
            .lookup(type_id)
            .ok()
            .map(|s| to_info(s, collection_of(&registry, s).as_deref()))
    }

    fn anchors(&self, type_id: &str) -> Option<Vec<AnchorInfo>> {
        let registry = self.state.snapshot();
        Some(
            registry
                .lookup(type_id)
                .ok()?
                .anchor_set
                .iter()
                .map(|(name, pos)| AnchorInfo {
                    name: name.clone(),
                    position: diagram_model::geometry::CellPos { x: pos.x, y: pos.y },
                })
                .collect(),
        )
    }

    fn register(&self, collection: &str, template: NewTemplate) -> Result<(), CatalogError> {
        self.state.register(collection, template)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shape_registry::{AnchorSet, TemplateGrid};

    /// AC2 — the built-in catalog surfaces through the seam with its standard anchor
    /// set (recorded in concept C7): box is 9×5 with n/s/w/e edge-midpoint anchors.
    #[test]
    fn builtin_shapes_cross_the_seam_as_read_models() {
        let catalog = RegistryCatalog::new(ShapeRegistry::builtin());

        let list = catalog.list();
        assert!(list.iter().any(|s| s.type_id == "box"));
        assert!(list.iter().any(|s| s.type_id == "ellipse"));

        let box_info = catalog.get("box").expect("built-in");
        assert_eq!((box_info.size.width, box_info.size.height), (9, 5));

        let anchors = catalog.anchors("box").expect("has anchors");
        let names: Vec<&str> = anchors.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["e", "n", "s", "w"]); // BTreeMap order
    }

    /// Collection shapes are listed under their qualified ids and resolve via `get`.
    #[test]
    fn collection_shapes_use_qualified_ids() {
        let mut registry = ShapeRegistry::builtin();
        let mut anchors = AnchorSet::new();
        anchors
            .add("n", shape_registry::CellPos { x: 2, y: 0 })
            .expect("valid anchor");
        let grid = TemplateGrid::from_rows(&["###", "#.#", "###"]).expect("valid");
        // `lookup` resolves the part after `/` against the template *name* (C8).
        registry.add_collection(shape_registry::Collection {
            name: "mycol".into(),
            types: vec![shape_registry::ShapeType {
                type_id: "mycol/arrow".into(),
                name: "arrow".into(),
                glyph_pattern: grid,
                anchor_set: anchors,
                geometry: shape_registry::GeometryKind::Pattern,
                resize_rules: None,
            }],
        });

        let catalog = RegistryCatalog::new(registry);
        let arrow = catalog.get("mycol/arrow").expect("qualified lookup");
        assert_eq!((arrow.size.width, arrow.size.height), (3, 3));
        assert!(catalog.list().iter().any(|s| s.type_id == "mycol/arrow"));

        let anchors = catalog.anchors("mycol/arrow").expect("has anchors");
        assert_eq!(anchors.len(), 1);
        assert_eq!((anchors[0].position.x, anchors[0].position.y), (2, 0));
    }

    /// A user collection named like a built-in id must not re-label the built-in
    /// (bare ids never belong to a collection, even by name collision).
    #[test]
    fn builtin_is_not_relabelled_by_a_colliding_collection_name() {
        use shape_registry::{AnchorSet, Collection, ShapeType, TemplateGrid};

        let mut registry = ShapeRegistry::builtin();
        registry.add_collection(Collection {
            name: "box".into(),
            types: vec![ShapeType {
                type_id: "box/star".into(),
                name: "star".into(),
                glyph_pattern: TemplateGrid::from_rows(&["#.#", "###", "#.#"]).unwrap(),
                anchor_set: AnchorSet::new(),
                geometry: shape_registry::GeometryKind::Pattern,
                resize_rules: None,
            }],
        });

        let catalog = RegistryCatalog::new(registry);
        assert_eq!(catalog.get("box").expect("builtin").collection, None);
        assert_eq!(
            catalog
                .get("box/star")
                .expect("qualified")
                .collection
                .as_deref(),
            Some("box")
        );
    }

    /// Unknown ids are `None`, not panics — the GUI filters on this.
    #[test]
    fn unknown_ids_are_none() {
        let catalog = RegistryCatalog::new(ShapeRegistry::builtin());
        assert!(catalog.get("nope").is_none());
        assert!(catalog.anchors("nope").is_none());
    }
}
