//! Sidebar view model (S-016 AC1, CR-011): groups a flat catalog listing into
//! collapsible groups with subcategory nesting. The grouping is pure data; the
//! egui drawing in [`draw_shape_groups`] is driven by plain inputs plus
//! [`SidebarActions`] callbacks so it can be exercised headlessly (tier 0b)
//! without a full app — the app glue in `app.rs` stays thin.

use eframe::egui;
use std::collections::BTreeMap;
use ui_api::ShapeInfo;

/// The subcategory every shape without a more specific category lands in
/// (CR-011 assumption, confirmed at acceptance: user/imported templates and
/// unknown built-ins default here).
pub const OTHER_CATEGORY: &str = "Other";

/// The static default category set for built-in shapes (CR-011). Extending the
/// built-in catalog (e.g. the UML shape set, S-036) extends this table — one
/// entry per shape name.
const BUILTIN_CATEGORIES: &[(&str, &str)] = &[("box", "Basic"), ("ellipse", "Basic")];

/// The default category of one shape (CR-011): built-in shapes follow the static
/// set in [`BUILTIN_CATEGORIES`]; user/imported (collection) templates and
/// unknown built-ins land in [`OTHER_CATEGORY`].
#[must_use]
pub fn category_for(info: &ShapeInfo) -> &'static str {
    if info.collection.is_some() {
        return OTHER_CATEGORY;
    }
    BUILTIN_CATEGORIES
        .iter()
        .find(|(type_id, _)| *type_id == info.type_id)
        .map(|(_, category)| *category)
        .unwrap_or(OTHER_CATEGORY)
}

/// One subcategory inside a sidebar group (CR-011): a label + its shapes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarSubgroup {
    /// Subcategory name ("Basic", "Other", …).
    pub label: String,
    /// The subcategory's shapes, in catalog order.
    pub shapes: Vec<ShapeInfo>,
}

/// One sidebar group: a label + its subcategories (catalog order preserved).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarGroup {
    /// Group heading ("Built-in" or the collection name).
    pub label: String,
    /// True for the built-in group (rendered first, no collection identity).
    pub builtins: bool,
    /// The group's subcategories: non-"Other" categories in first-appearance
    /// order, [`OTHER_CATEGORY`] always last.
    pub subgroups: Vec<SidebarSubgroup>,
}

/// Groups a flat catalog listing into sidebar groups with subcategories (S-016
/// AC1 + CR-011 AC2): the built-in group (label "Built-in") always comes first,
/// then one group per collection in the order the collections appear in the
/// listing; category and within-category template order follow the listing.
#[must_use]
pub fn group_templates(infos: &[ShapeInfo]) -> Vec<SidebarGroup> {
    let mut builtins: Option<SidebarGroup> = None;
    let mut collections: Vec<SidebarGroup> = Vec::new();

    for info in infos {
        let (label, is_builtin) = match &info.collection {
            None => ("Built-in".to_owned(), true),
            Some(collection) => (collection.clone(), false),
        };
        let group = if is_builtin {
            builtins.get_or_insert_with(|| SidebarGroup {
                label: "Built-in".to_owned(),
                builtins: true,
                subgroups: Vec::new(),
            })
        } else {
            let index = collections
                .iter()
                .position(|g| g.label == label)
                .unwrap_or_else(|| {
                    collections.push(SidebarGroup {
                        label: label.clone(),
                        builtins: false,
                        subgroups: Vec::new(),
                    });
                    collections.len() - 1
                });
            &mut collections[index]
        };
        push_subgroup(group, info);
    }

    let mut groups = Vec::with_capacity(collections.len() + usize::from(builtins.is_some()));
    if let Some(group) = builtins {
        groups.push(group);
    }
    groups.extend(collections);
    groups
}

/// Appends `info` to its category inside `group`: non-"Other" subcategories keep
/// first-appearance order, [`OTHER_CATEGORY`] always stays last.
fn push_subgroup(group: &mut SidebarGroup, info: &ShapeInfo) {
    let category = category_for(info);
    let is_other = category == OTHER_CATEGORY;

    match group.subgroups.iter().position(|s| s.label == category) {
        Some(index) => group.subgroups[index].shapes.push(info.clone()),
        None => {
            // Insert before a trailing "Other" subgroup so it stays last.
            let insert_at = if is_other {
                group.subgroups.len()
            } else {
                group
                    .subgroups
                    .iter()
                    .rposition(|s| s.label == OTHER_CATEGORY)
                    .unwrap_or(group.subgroups.len())
            };
            group.subgroups.insert(
                insert_at,
                SidebarSubgroup {
                    label: category.to_owned(),
                    shapes: vec![info.clone()],
                },
            );
        }
    }
}

/// Interaction callbacks between [`draw_shape_groups`] and the app (CR-011 AC2):
/// the drawing stays decoupled from `DrawingApp` so it is testable headlessly.
pub trait SidebarActions {
    /// True while this shape is the one being placed (its row is highlighted).
    fn is_placing(&self, type_id: &str) -> bool;
    /// The user clicked a shape row — start placing it.
    fn select_shape(&mut self, info: ShapeInfo);
    /// The user clicked the ✎ button of a collection template (built-ins are code,
    /// not files — ADR 007: they get no edit button).
    fn edit_shape(&mut self, type_id: String);
}

/// Draws the shape groups of `groups` (CR-011): one collapsing header per group,
/// expanded by default or per `expanded`; a group holding ≥ 2 subcategories nests
/// one collapsing header per subcategory, a single-subgroup group renders its
/// shapes flat (no redundant nesting level). Returns the `(key, open)` pairs
/// whose state changed this frame — keys are `"group"` and `"group/subcategory"`;
/// the caller persists them to the config file (AC3).
pub fn draw_shape_groups(
    ui: &mut egui::Ui,
    groups: &[SidebarGroup],
    expanded: &BTreeMap<String, bool>,
    preview_color: egui::Color32,
    actions: &mut dyn SidebarActions,
) -> Vec<(String, bool)> {
    let mut toggles = Vec::new();
    for group in groups {
        let key = group.label.clone();
        // The state this frame started with — the map is kept in sync by the
        // caller after every reported toggle, so a change flips exactly this.
        let was_open = expanded.get(&key).copied().unwrap_or(true);
        let response =
            egui::CollapsingHeader::new(egui::RichText::new(group.label.clone()).strong())
                .default_open(expanded.get(&key).copied().unwrap_or(true))
                .id_salt(("sidebar_group", group.label.as_str()))
                .show(ui, |ui| {
                    if group.subgroups.len() >= 2 {
                        for sub in &group.subgroups {
                            let sub_key = format!("{}/{}", group.label, sub.label);
                            let sub_was_open = expanded.get(&sub_key).copied().unwrap_or(true);
                            let sub_response = egui::CollapsingHeader::new(sub.label.clone())
                                .default_open(expanded.get(&sub_key).copied().unwrap_or(true))
                                .id_salt((
                                    "sidebar_subgroup",
                                    group.label.as_str(),
                                    sub.label.as_str(),
                                ))
                                .show(ui, |ui| {
                                    for info in &sub.shapes {
                                        draw_shape_row(ui, info, preview_color, actions);
                                    }
                                });
                            // `body_response` follows the *animation* (still Some while
                            // the body animates shut), so report the logical state:
                            // a change flips what the frame started with.
                            if sub_response.header_response.changed() {
                                toggles.push((sub_key, !sub_was_open));
                            }
                        }
                    } else {
                        for sub in &group.subgroups {
                            for info in &sub.shapes {
                                draw_shape_row(ui, info, preview_color, actions);
                            }
                        }
                    }
                });
        if response.header_response.changed() {
            toggles.push((key, !was_open));
        }
    }
    toggles
}

/// One shape row: selectable name, the monospace glyph preview (the same bytes
/// the renderer stamps on placement), and — for collection templates only — the
/// edit button.
fn draw_shape_row(
    ui: &mut egui::Ui,
    info: &ShapeInfo,
    preview_color: egui::Color32,
    actions: &mut dyn SidebarActions,
) {
    let selected = actions.is_placing(&info.type_id);
    if ui.selectable_label(selected, &info.name).clicked() {
        actions.select_shape(info.clone());
    }
    // Small preview of the glyph pattern (S-016 AC1).
    for row in &info.preview {
        ui.label(
            egui::RichText::new(row.clone())
                .monospace()
                .size(9.0)
                .color(preview_color),
        );
    }
    if info.collection.is_some()
        && ui
            .add_sized([24.0, 18.0], egui::Button::new("✎"))
            .on_hover_text(format!("Edit {}", info.name))
            .clicked()
    {
        actions.edit_shape(info.type_id.clone());
    }
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
            resizable: true,
            min_size: CellSize::new(1, 1).expect("valid size"),
            resize_rules: None,
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
                .subgroups
                .iter()
                .flat_map(|s| s.shapes.iter())
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
                .subgroups
                .iter()
                .flat_map(|s| s.shapes.iter())
                .map(|s| s.type_id.as_str())
                .collect::<Vec<_>>(),
            vec!["z/zeta", "z/epsilon"]
        );
        assert_eq!(
            groups[2]
                .subgroups
                .iter()
                .flat_map(|s| s.shapes.iter())
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

    /// AC2 — the view model built from *fake catalog data* (the T7 pattern): what
    /// the seam serves is exactly what the sidebar groups.
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
        assert_eq!(
            groups[1]
                .subgroups
                .iter()
                .flat_map(|s| s.shapes.iter())
                .next()
                .expect("one shape")
                .preview,
            vec!["###".to_owned()]
        );
    }

    /// AC2 — the static default category set: known built-ins get their table
    /// category; everything else (collection templates, unknown built-ins) lands
    /// in "Other".
    #[test]
    fn category_assignment_follows_the_static_set() {
        assert_eq!(category_for(&info("box", None)), "Basic");
        assert_eq!(category_for(&info("ellipse", None)), "Basic");
        assert_eq!(
            category_for(&info("col/star", Some("col"))),
            OTHER_CATEGORY,
            "collection templates land in Other"
        );
        assert_eq!(
            category_for(&info("mystery", None)),
            OTHER_CATEGORY,
            "unknown built-ins fall back to Other"
        );
    }

    /// AC2 — subcategories keep first-appearance order and "Other" stays last,
    /// even when the listing interleaves them.
    #[test]
    fn subgroup_order_first_appearance_other_last() {
        // A fake built-in not in the static table → "Other" appears *first* in
        // the listing but must sort last.
        let infos = [
            info("mystery", None),
            info("box", None),
            info("ellipse", None),
        ];

        let groups = group_templates(&infos);
        let builtins = &groups[0];

        assert_eq!(
            builtins
                .subgroups
                .iter()
                .map(|s| s.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Basic", OTHER_CATEGORY]
        );
        assert_eq!(
            builtins.subgroups[0]
                .shapes
                .iter()
                .map(|s| s.type_id.as_str())
                .collect::<Vec<_>>(),
            vec!["box", "ellipse"]
        );
        assert_eq!(
            builtins.subgroups[1]
                .shapes
                .iter()
                .map(|s| s.type_id.as_str())
                .collect::<Vec<_>>(),
            vec!["mystery"]
        );
    }

    /// AC2 — a group whose shapes all share one category gets exactly that one
    /// subgroup (the renderer then draws it flat, no nesting level).
    #[test]
    fn single_category_group_gets_one_subgroup() {
        let infos = [
            info("box", None),
            info("col/a", Some("col")),
            info("col/b", Some("col")),
        ];
        let groups = group_templates(&infos);

        assert_eq!(groups[0].subgroups.len(), 1);
        assert_eq!(groups[0].subgroups[0].label, "Basic");
        assert_eq!(groups[1].subgroups.len(), 1);
        assert_eq!(groups[1].subgroups[0].label, OTHER_CATEGORY);
    }

    // ── draw_shape_groups: headless egui (tier 0b pattern, cf. gui_tests::Gui) ──

    struct Noop;
    impl SidebarActions for Noop {
        fn is_placing(&self, _type_id: &str) -> bool {
            false
        }
        fn select_shape(&mut self, _info: ShapeInfo) {}
        fn edit_shape(&mut self, _type_id: String) {}
    }

    /// A bare headless context pumping one frame of the sidebar groups — the
    /// `gui_tests::Gui` pattern: `run_ui`, built-in fonts, texture deltas cleared.
    struct Pump {
        ctx: egui::Context,
        ts: f64,
        groups: Vec<SidebarGroup>,
        expanded: BTreeMap<String, bool>,
        last_toggles: Vec<(String, bool)>,
    }

    impl Pump {
        fn new(groups: Vec<SidebarGroup>, expanded: BTreeMap<String, bool>) -> Self {
            let ctx = egui::Context::default();
            ctx.set_fonts(egui::FontDefinitions::default());
            Self {
                ctx,
                ts: 1.0,
                groups,
                expanded,
                last_toggles: Vec::new(),
            }
        }

        fn pump(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
            let raw = egui::RawInput {
                time: Some(self.ts),
                screen_rect: Some(egui::Rect::from_min_max(
                    egui::Pos2::ZERO,
                    egui::pos2(400.0, 800.0),
                )),
                events,
                ..Default::default()
            };
            self.ts += 1.0 / 60.0;
            let groups = &self.groups;
            let expanded = &self.expanded;
            let mut out = self.ctx.run_ui(raw, |ui| {
                ui.vertical(|ui| {
                    self.last_toggles =
                        draw_shape_groups(ui, groups, expanded, egui::Color32::WHITE, &mut Noop);
                });
            });
            // Headless: no GPU to upload glyph textures — clear the pending deltas.
            out.textures_delta.clear();
            out
        }
    }

    fn drawn_texts(out: &egui::FullOutput) -> Vec<String> {
        out.shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                egui::epaint::Shape::Text(t) => Some(t.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    }

    /// AC2 — a group with ≥ 2 subcategories nests one header per subcategory;
    /// the single-subgroup group ("col") renders flat (no "Other" header).
    #[test]
    fn nested_headers_appear_only_for_multi_subgroup_groups() {
        let groups = group_templates(&[
            info("mystery", None),
            info("box", None),
            info("col/a", Some("col")),
        ]);
        let mut pump = Pump::new(groups, BTreeMap::new());
        let out = pump.pump(Vec::new());
        let texts = drawn_texts(&out);

        // Group headers.
        assert!(texts.contains(&"Built-in".to_owned()));
        assert!(texts.contains(&"col".to_owned()));
        // The two-subgroup built-in group shows both subcategory headers…
        assert!(texts.contains(&"Basic".to_owned()));
        assert!(texts.contains(&"Other".to_owned()));
        // …but the single-subgroup "col" group does not repeat its "Other".
        assert_eq!(texts.iter().filter(|t| *t == "Other").count(), 1);
        // Shape rows are drawn for all groups.
        assert!(texts.contains(&"mystery".to_owned()));
        assert!(texts.contains(&"box".to_owned()));
        assert!(texts.contains(&"a".to_owned()));
    }

    /// AC1/AC3 — a collapsed group (persisted `expanded` state) hides its rows,
    /// and clicking an open header toggles it closed for the next frame.
    #[test]
    fn persisted_collapse_hides_rows_and_click_toggles() {
        let groups = group_templates(&[info("box", None), info("col/a", Some("col"))]);
        let mut expanded = BTreeMap::new();
        expanded.insert("Built-in".to_owned(), false);
        let mut pump = Pump::new(groups, expanded);

        // "Built-in" persisted closed: its rows are absent; "col" (no entry →
        // expanded) still shows its shape.
        let out = pump.pump(Vec::new());
        let texts = drawn_texts(&out);
        assert!(texts.contains(&"Built-in".to_owned()), "header still drawn");
        assert!(
            !texts.contains(&"box".to_owned()),
            "collapsed group hides rows"
        );
        assert!(texts.contains(&"a".to_owned()), "other group still open");

        // Click the open "col" header (press frame + release frame).
        let col_rect = out
            .shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == "col" => {
                    Some(egui::Rect::from_min_size(t.pos, t.galley.size()))
                }
                _ => None,
            })
            .next()
            .expect("col header drawn");
        let center = col_rect.center();
        pump.pump(vec![
            egui::Event::PointerMoved(center),
            egui::Event::PointerButton {
                pos: center,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
        ]);
        pump.pump(vec![egui::Event::PointerButton {
            pos: center,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }]);

        // AC3 — the release frame reports the *logical* state for persistence:
        // "col" was open, so a toggle reports `false` (not the animation's
        // still-open body).
        assert_eq!(pump.last_toggles, vec![("col".to_owned(), false)]);

        // The click toggled "col" closed; egui animates the body shut over a few
        // frames, so let the animation finish before asserting.
        for _ in 0..12 {
            pump.pump(Vec::new());
        }
        let out = pump.pump(Vec::new());
        assert!(
            !drawn_texts(&out).contains(&"a".to_owned()),
            "col collapsed after click"
        );
    }
}
