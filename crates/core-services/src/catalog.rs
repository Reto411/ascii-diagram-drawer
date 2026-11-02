//! Catalog backend (ADR 009): wraps `shape-registry` and presents its data as the
//! seam's read models — registry types never cross into the GUI. Registration
//! (S-017) mutates the shared state that the renderer reads from the same handle.

use shape_registry::{ShapeRegistry, ShapeType, min_size};
use ui_api::{AnchorInfo, CatalogError, NewTemplate, RegionInfo, ShapeCatalog, ShapeInfo};

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
    // S-024 AC4: resizability crosses the seam as data — built-ins (Rect/Ellipse
    // geometry) are always resizable down to 1×1; rule-bearing customs down to their
    // computed minimum; plain customs not at all (FR12).
    let (resizable, min_size_) = match &shape.resize_rules {
        Some(rules) => (true, min_size(pattern, rules)),
        None if shape.geometry != shape_registry::GeometryKind::Pattern => {
            (true, shape_registry::CellSize::new(1, 1))
        }
        None => (
            false,
            shape_registry::CellSize::new(pattern.width(), pattern.height()),
        ),
    };
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
        resizable,
        min_size: diagram_model::geometry::CellSize {
            width: min_size_.width,
            height: min_size_.height,
        },
        // S-024: the editor needs the declared rules to restore them when editing
        // an existing rule-bearing template (saving must not drop them).
        resize_rules: shape.resize_rules.as_ref().map(rules_to_spec),
        // FR19 (CR-016): declared text regions, sorted by name — the editor restores
        // them on reopen and the canvas offers per-region content editing.
        regions: shape
            .regions
            .iter()
            .map(|(name, rect)| RegionInfo {
                name: name.clone(),
                pos: diagram_model::geometry::CellPos {
                    x: rect.x,
                    y: rect.y,
                },
                size: diagram_model::geometry::CellSize {
                    width: rect.width,
                    height: rect.height,
                },
            })
            .collect(),
    }
}

/// Maps the registry's rule set to its seam-level spec (the inverse of
/// `registry_state::rules_from_spec`) — no registry type crosses the seam (C3).
fn rules_to_spec(rules: &shape_registry::ResizeRules) -> ui_api::ResizeRuleSpec {
    let edge = |e: &shape_registry::EdgeRule| ui_api::EdgeRuleSpec {
        insert: e.insert,
        remove: e.remove,
        even: e.even.map(|p| ui_api::ParityCharsSpec {
            insert: p.insert,
            remove: p.remove,
        }),
        odd: e.odd.map(|p| ui_api::ParityCharsSpec {
            insert: p.insert,
            remove: p.remove,
        }),
    };
    ui_api::ResizeRuleSpec {
        top: edge(&rules.top),
        bottom: edge(&rules.bottom),
        left: edge(&rules.left),
        right: edge(&rules.right),
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

    fn import_collection(&self, name: &str, text: &str) -> Result<(), CatalogError> {
        self.state.import_collection(name, text)
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
                regions: shape_registry::RegionSet::new(),
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
                regions: shape_registry::RegionSet::new(),
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

    /// AC5 (S-036, CR-015) — every built-in of the UML set crosses the seam with
    /// correct metadata: natural size, verbatim preview, declared anchors, and the
    /// S-024 resizability rule (Rect/Ellipse geometry resizable to 1×1; no-rule
    /// Pattern shapes fixed at their natural size).
    #[test]
    fn uml_set_crosses_the_seam_with_correct_metadata() {
        use diagram_model::geometry::CellSize;

        let registry = ShapeRegistry::builtin();
        let catalog = RegistryCatalog::new(registry.clone());
        let ids: Vec<&str> = registry.builtins().map(|s| s.type_id.as_str()).collect();
        assert_eq!(ids.len(), 14);

        // Deterministic order across the seam: exactly the built-ins, in catalog order.
        let listed: Vec<String> = catalog.list().iter().map(|s| s.type_id.clone()).collect();
        assert_eq!(
            listed,
            ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>()
        );

        for id in &ids {
            let source = registry.lookup(id).expect("built-in");
            let info = catalog.get(id).expect("listed through the seam");
            assert_eq!(info.collection, None, "{id} is a built-in");
            assert_eq!(
                info.size,
                CellSize::new(source.glyph_pattern.width(), source.glyph_pattern.height())
                    .expect("non-zero"),
                "{id} size"
            );
            let rows: Vec<String> = (0..source.glyph_pattern.height())
                .filter_map(|y| source.glyph_pattern.row(y))
                .collect();
            assert_eq!(info.preview, rows, "{id} preview verbatim");

            let anchors = catalog.anchors(id).expect("anchor read model");
            let expected: Vec<(&str, (u32, u32))> = source
                .anchor_set
                .iter()
                .map(|(n, p)| (n.as_str(), (p.x, p.y)))
                .collect();
            let got: Vec<(&str, (u32, u32))> = anchors
                .iter()
                .map(|a| (a.name.as_str(), (a.position.x, a.position.y)))
                .collect();
            assert_eq!(got, expected, "{id} anchors");

            // Declared regions cross the seam as RegionInfo (FR19 read model).
            let expected_regions: Vec<&str> = source
                .regions
                .iter()
                .map(|(name, _)| name.as_str())
                .collect();
            assert_eq!(
                info.regions
                    .iter()
                    .map(|r| r.name.as_str())
                    .collect::<Vec<_>>(),
                expected_regions,
                "{id} regions"
            );

            match source.geometry {
                shape_registry::GeometryKind::Pattern => {
                    // No-rule Pattern built-in: stamped at natural size, not resizable.
                    assert!(!info.resizable, "{id}");
                    assert_eq!(info.min_size, info.size, "{id} min = natural");
                }
                _ => {
                    // Rect/Ellipse built-ins (box, ellipse): resizable down to 1×1.
                    assert!(info.resizable, "{id}");
                    assert_eq!(
                        info.min_size,
                        CellSize::new(1, 1).expect("non-zero"),
                        "{id} min 1x1"
                    );
                }
            }
        }
    }

    /// Unknown ids are `None`, not panics — the GUI filters on this.
    #[test]
    fn unknown_ids_are_none() {
        let catalog = RegistryCatalog::new(ShapeRegistry::builtin());
        assert!(catalog.get("nope").is_none());
        assert!(catalog.anchors("nope").is_none());
    }
}
