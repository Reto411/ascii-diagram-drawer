//! Sidebar view model (S-016 AC1): groups the flat catalog listing into a built-in
//! group plus one group per loaded collection, preserving catalog order. Pure data —
//! the egui drawing in `app.rs` stays thin and untested-by-pixels.

use ui_api::ShapeInfo;

/// One sidebar group: a label + the shapes it contains (catalog order preserved).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarGroup {
    /// Group heading ("Built-in" or the collection name).
    pub label: String,
    /// True for the built-in group (rendered first, no collection identity).
    pub builtins: bool,
    /// The group's shapes, in catalog order.
    pub shapes: Vec<ShapeInfo>,
}

/// Groups a flat catalog listing into sidebar groups (S-016 AC1): the built-in group
/// (label "Built-in") always comes first, then one group per collection in the order
/// the collections appear in the listing; template order within a group is preserved.
#[must_use]
pub fn group_templates(infos: &[ShapeInfo]) -> Vec<SidebarGroup> {
    let mut builtins: Option<SidebarGroup> = None;
    let mut collections: Vec<SidebarGroup> = Vec::new();

    for info in infos {
        if info.collection.is_none() {
            match &mut builtins {
                Some(group) => group.shapes.push(info.clone()),
                None => {
                    builtins = Some(SidebarGroup {
                        label: "Built-in".to_owned(),
                        builtins: true,
                        shapes: vec![info.clone()],
                    })
                }
            }
        } else {
            let label = info.collection.clone().unwrap_or_default();
            match collections.iter_mut().find(|g| g.label == label) {
                Some(group) => group.shapes.push(info.clone()),
                None => collections.push(SidebarGroup {
                    label,
                    builtins: false,
                    shapes: vec![info.clone()],
                }),
            }
        }
    }

    let mut groups = Vec::with_capacity(collections.len() + usize::from(builtins.is_some()));
    if let Some(group) = builtins {
        groups.push(group);
    }
    groups.extend(collections);
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_model::geometry::CellSize;

    fn info(type_id: &str, collection: Option<&str>) -> ShapeInfo {
        ShapeInfo {
            type_id: type_id.to_owned(),
            name: type_id.rsplit('/').next().unwrap_or(type_id).to_owned(),
            size: CellSize::new(3, 3).expect("valid size"),
            collection: collection.map(str::to_owned),
            preview: vec!["###".to_owned()],
        }
    }

    /// AC1 — built-ins form the first group; each collection its own group.
    #[test]
    fn groups_builtins_and_collections() {
        let infos = [
            info("box", None),
            info("ellipse", None),
            info("col-a/star", Some("col-a")),
            info("col-b/arrow", Some("col-b")),
        ];

        let groups = group_templates(&infos);

        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].label, "Built-in");
        assert!(groups[0].builtins);
        assert_eq!(
            groups[0]
                .shapes
                .iter()
                .map(|s| s.type_id.as_str())
                .collect::<Vec<_>>(),
            vec!["box", "ellipse"]
        );
        assert_eq!(groups[1].label, "col-a");
        assert!(!groups[1].builtins);
        assert_eq!(groups[2].label, "col-b");
    }

    /// AC1 — catalog order is preserved (group order and within-group template order).
    #[test]
    fn catalog_order_is_preserved() {
        // Interleaved listing: the same collection appears in one group, in listing order.
        let infos = [
            info("z/zeta", Some("z")),
            info("box", None),
            info("a/alpha", Some("a")),
            info("z/epsilon", Some("z")),
            info("ellipse", None),
            info("a/beta", Some("a")),
        ];

        let groups = group_templates(&infos);

        assert_eq!(groups[0].label, "Built-in");
        assert_eq!(groups[1].label, "z");
        assert_eq!(groups[2].label, "a");
        assert_eq!(
            groups[1]
                .shapes
                .iter()
                .map(|s| s.type_id.as_str())
                .collect::<Vec<_>>(),
            vec!["z/zeta", "z/epsilon"]
        );
        assert_eq!(
            groups[2]
                .shapes
                .iter()
                .map(|s| s.type_id.as_str())
                .collect::<Vec<_>>(),
            vec!["a/alpha", "a/beta"]
        );
    }

    /// An empty catalog yields no groups (the sidebar shows its heading only).
    #[test]
    fn empty_listing_is_empty() {
        assert!(group_templates(&[]).is_empty());
    }

    /// AC1 — the view model built from *fake catalog data* (the T7 pattern): what the
    /// seam serves is exactly what the sidebar groups.
    #[test]
    fn grouping_works_on_fake_catalog_data() {
        use core_services::fakes::FakeCatalog;
        use ui_api::ShapeCatalog;

        let catalog = FakeCatalog::default()
            .add(info("box", None), Vec::new())
            .add(info("col/star", Some("col")), Vec::new());

        let groups = group_templates(&catalog.list());

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].label, "Built-in");
        assert_eq!(groups[1].label, "col");
        assert_eq!(groups[1].shapes[0].preview, vec!["###".to_owned()]);
    }
}
