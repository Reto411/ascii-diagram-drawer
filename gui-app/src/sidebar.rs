//! Sidebar view model (S-016 AC1, CR-011, CR-012, CR-013, CR-021): groups a flat
//! catalog listing into collapsible groups with subcategory nesting — built-ins,
//! the UML family in its own group with per-diagram subcategories (CR-021), and
//! one group per user collection — filters it by shape name, and renders each
//! shape as a whole-row grabable entry (name + pre-rendered WYSIWYG preview,
//! FR6/FR16) of the same pattern bytes the renderer stamps on placement, with a
//! separator line between adjacent entries (CR-021). The grouping/filtering/
//! sizing is pure data; the egui drawing in [`draw_shape_groups`] is driven by
//! plain inputs plus [`SidebarActions`] callbacks so it can be exercised
//! headlessly (tier 0b) without a full app — the app glue in `app.rs` stays thin.

use eframe::egui;
use std::collections::BTreeMap;
use ui_api::ShapeInfo;

/// The subcategory every shape without a more specific category lands in
/// (CR-011 assumption, confirmed at acceptance: user/imported templates and
/// unknown built-ins default here).
pub const OTHER_CATEGORY: &str = "Other";

/// The static default category set for non-UML built-in shapes (CR-011).
const BUILTIN_CATEGORIES: &[(&str, &str)] = &[("box", "Basic"), ("ellipse", "Basic")];

/// The top-level sidebar group for the UML shape family (CR-021): rendered after
/// [`BUILTIN_GROUP`] and before any user collection.
pub const UML_GROUP: &str = "UML";

/// The label of the built-in group (the non-UML built-ins: box, ellipse, …).
pub const BUILTIN_GROUP: &str = "Built-in";

/// The subcategory of each UML built-in inside the [`UML_GROUP`] group (CR-021):
/// Class / Component / Activity — one per supported diagram type. `None` keeps
/// the shape in the UML group's [`OTHER_CATEGORY`] subcategory (actor, activation:
/// UML shapes that belong to none of the three diagram families). Built-ins not
/// listed here are not UML and stay in the "Built-in" group.
const UML_SUBCATEGORIES: &[(&str, Option<&str>)] = &[
    ("class", Some("Class")),
    ("lollipop", Some("Component")),
    ("component", Some("Component")),
    ("port", Some("Component")),
    ("deployment", Some("Component")),
    ("artifact", Some("Component")),
    ("state", Some("Activity")),
    ("decision", Some("Activity")),
    ("initial", Some("Activity")),
    ("final", Some("Activity")),
    ("actor", None),
    ("activation", None),
];

/// The subcategory a UML built-in lands in inside the [`UML_GROUP`] group.
fn uml_subcategory(type_id: &str) -> &'static str {
    UML_SUBCATEGORIES
        .iter()
        .find(|(id, _)| *id == type_id)
        .map(|(_, sub)| sub.unwrap_or(OTHER_CATEGORY))
        .unwrap_or(OTHER_CATEGORY)
}

/// The category of a non-UML built-in (CR-011): the static set in
/// [`BUILTIN_CATEGORIES`], unknown built-ins fall back to [`OTHER_CATEGORY`].
fn builtin_category(type_id: &str) -> &'static str {
    BUILTIN_CATEGORIES
        .iter()
        .find(|(id, _)| *id == type_id)
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
    /// True for built-in families (the "Built-in" and "UML" groups) as opposed
    /// to user collections — built-ins render before collections and are not a
    /// writable template collection.
    pub builtins: bool,
    /// The group's subcategories: non-"Other" categories in first-appearance
    /// order, [`OTHER_CATEGORY`] always last.
    pub subgroups: Vec<SidebarSubgroup>,
}

/// Groups a flat catalog listing into sidebar groups with subcategories (S-016
/// AC1 + CR-011 AC2 + CR-021 AC1): the built-in group (label "Built-in") always
/// comes first, then the UML group ([`UML_GROUP`]) when the catalog holds any
/// UML shape, then one group per collection in the order the collections appear
/// in the listing; category and within-category template order follow the
/// listing.
#[must_use]
pub fn group_templates(infos: &[ShapeInfo]) -> Vec<SidebarGroup> {
    let mut builtins: Option<SidebarGroup> = None;
    let mut uml: Option<SidebarGroup> = None;
    let mut collections: Vec<SidebarGroup> = Vec::new();

    for info in infos {
        // (group, subcategory) — CR-021 splits the flat built-in set into the
        // UML family group and the remaining "Built-in" group.
        let (group, category): (&mut SidebarGroup, &'static str) = match &info.collection {
            Some(collection) => {
                let index = collections
                    .iter()
                    .position(|g| g.label == *collection)
                    .unwrap_or_else(|| {
                        collections.push(SidebarGroup {
                            label: collection.clone(),
                            builtins: false,
                            subgroups: Vec::new(),
                        });
                        collections.len() - 1
                    });
                (&mut collections[index], OTHER_CATEGORY)
            }
            None if UML_SUBCATEGORIES.iter().any(|(id, _)| *id == info.type_id) => (
                uml.get_or_insert_with(|| SidebarGroup {
                    label: UML_GROUP.to_owned(),
                    builtins: true,
                    subgroups: Vec::new(),
                }),
                uml_subcategory(&info.type_id),
            ),
            None => (
                builtins.get_or_insert_with(|| SidebarGroup {
                    label: BUILTIN_GROUP.to_owned(),
                    builtins: true,
                    subgroups: Vec::new(),
                }),
                builtin_category(&info.type_id),
            ),
        };
        push_subgroup(group, category, info);
    }

    let mut groups = Vec::with_capacity(
        collections.len() + usize::from(builtins.is_some()) + usize::from(uml.is_some()),
    );
    if let Some(group) = builtins {
        groups.push(group);
    }
    if let Some(group) = uml {
        groups.push(group);
    }
    groups.extend(collections);
    groups
}

/// Appends `info` to `category` inside `group`: non-"Other" subcategories keep
/// first-appearance order, [`OTHER_CATEGORY`] always stays last.
fn push_subgroup(group: &mut SidebarGroup, category: &'static str, info: &ShapeInfo) {
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

/// Narrows `groups` by shape name (CR-012 AC1): a case-insensitive substring
/// match on each shape's *name* only — group labels, subcategory labels, and
/// type ids are not matched in v1. Subcategories with no matching shape are
/// dropped; groups left with no subcategories disappear entirely (AC2). A
/// blank query returns the input unchanged (cloned).
#[must_use]
pub fn filter_groups(groups: &[SidebarGroup], query: &str) -> Vec<SidebarGroup> {
    let needle = query.trim();
    if needle.is_empty() {
        return groups.to_vec();
    }
    let needle = needle.to_lowercase();
    groups
        .iter()
        .map(|group| {
            let subgroups = group
                .subgroups
                .iter()
                .map(|sub| SidebarSubgroup {
                    label: sub.label.clone(),
                    shapes: sub
                        .shapes
                        .iter()
                        .filter(|info| info.name.to_lowercase().contains(&needle))
                        .cloned()
                        .collect(),
                })
                .filter(|sub| !sub.shapes.is_empty())
                .collect();
            SidebarGroup {
                label: group.label.clone(),
                builtins: group.builtins,
                subgroups,
            }
        })
        .filter(|group| !group.subgroups.is_empty())
        .collect()
}

/// The monospace font size for one shape preview (FR16 / CR-013 AC3): the base
/// (canvas) size when the pattern's natural width — `max_cols` × the monospace
/// advance at that size — fits within `max_width_px`; otherwise uniformly scaled
/// down so the block just fits. A uniform scale keeps the preview WYSIWYG: same
/// bytes, same relative glyph metrics as what placement stamps. Degenerate inputs
/// (no columns, no advance, non-finite or non-positive width) return the base size.
#[must_use]
pub fn preview_font_size(
    max_cols: u32,
    max_width_px: f32,
    base_size: f32,
    base_advance: f32,
) -> f32 {
    if max_cols == 0 || base_advance <= 0.0 || !max_width_px.is_finite() || max_width_px <= 0.0 {
        return base_size;
    }
    let natural = max_cols as f32 * base_advance;
    if natural <= max_width_px {
        return base_size;
    }
    (base_size * max_width_px / natural).max(MIN_PREVIEW_FONT_SIZE)
}

/// Degenerate-case guard for [`preview_font_size`]: below this size glyphs are
/// sub-pixel noise anyway — the fit still happens, we just avoid handing egui a
/// zero-size font.
const MIN_PREVIEW_FONT_SIZE: f32 = 2.0;

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
/// shapes flat (no redundant nesting level). Above the list sits the CR-012
/// filter input (`filter_query`, transient — owned by the app): while it holds a
/// non-blank query only matching groups are drawn and they render forced-open
/// (AC3), with toggle reporting suspended for the duration. Returns the `(key,
/// open)` pairs whose state changed this frame — keys are `"group"` and
/// `"group/subcategory"`; the caller persists them to the config file (CR-011 AC3).
pub fn draw_shape_groups(
    ui: &mut egui::Ui,
    groups: &[SidebarGroup],
    expanded: &BTreeMap<String, bool>,
    preview_color: egui::Color32,
    filter_query: &mut String,
    actions: &mut dyn SidebarActions,
) -> Vec<(String, bool)> {
    // CR-012 — single-line filter above the shape list; narrows live per keystroke.
    ui.add(egui::TextEdit::singleline(filter_query).hint_text("Filter shapes…"));
    let filtering = !filter_query.trim().is_empty();
    // Force matching groups open while filtering (AC3) so a match is visible
    // without first expanding; `None` otherwise keeps the persisted state.
    let force_open = if filtering { Some(true) } else { None };
    // Filtered draws use their own id slots: `.open(Some(true))` toggles egui's
    // remembered state, and that must not leak into the unfiltered slots —
    // clearing the filter has to re-apply the persisted open/closed states.
    let salt_suffix = if filtering { "filtered" } else { "" };
    let filtered = filter_groups(groups, filter_query);
    // FR16 — measure the monospace advance at the canvas font size once per frame
    // (the same 10-glyph sample the canvas uses) so every preview can be fitted to
    // the available width without re-measuring per row.
    let base_font = egui::FontId::new(crate::app::FONT_SIZE, egui::FontFamily::Monospace);
    let base_advance = (ui
        .painter()
        .layout(
            "MMMMMMMMMM".to_owned(),
            base_font.clone(),
            egui::Color32::TRANSPARENT,
            f32::INFINITY,
        )
        .size()
        .x
        / 10.0)
        .max(4.0);

    let mut toggles = Vec::new();
    for group in &filtered {
        let key = group.label.clone();
        // The state this frame started with — the map is kept in sync by the
        // caller after every reported toggle, so a change flips exactly this.
        let was_open = expanded.get(&key).copied().unwrap_or(true);
        let response =
            egui::CollapsingHeader::new(egui::RichText::new(group.label.clone()).strong())
                .default_open(expanded.get(&key).copied().unwrap_or(true))
                .open(force_open)
                .id_salt(("sidebar_group", group.label.as_str(), salt_suffix))
                .show(ui, |ui| {
                    if group.subgroups.len() >= 2 {
                        for sub in &group.subgroups {
                            let sub_key = format!("{}/{}", group.label, sub.label);
                            let sub_was_open = expanded.get(&sub_key).copied().unwrap_or(true);
                            let sub_response = egui::CollapsingHeader::new(sub.label.clone())
                                .default_open(expanded.get(&sub_key).copied().unwrap_or(true))
                                .open(force_open)
                                .id_salt((
                                    "sidebar_subgroup",
                                    group.label.as_str(),
                                    sub.label.as_str(),
                                    salt_suffix,
                                ))
                                .show(ui, |ui| {
                                    draw_shape_rows(
                                        ui,
                                        &sub.shapes,
                                        preview_color,
                                        base_advance,
                                        actions,
                                    );
                                });
                            // `body_response` follows the *animation* (still Some while
                            // the body animates shut), so report the logical state:
                            // a change flips what the frame started with. Suspended
                            // while filtering — the forced-open marks the header
                            // changed on its own, and that is not user intent.
                            if !filtering && sub_response.header_response.changed() {
                                toggles.push((sub_key, !sub_was_open));
                            }
                        }
                    } else {
                        for sub in &group.subgroups {
                            draw_shape_rows(ui, &sub.shapes, preview_color, base_advance, actions);
                        }
                    }
                });
        if !filtering && response.header_response.changed() {
            toggles.push((key, !was_open));
        }
    }
    toggles
}

/// One run of shape rows (CR-021 AC4): a separator line divides every pair of
/// adjacent entries — the first row of a run has no leading line because its
/// group/subcategory header already separates it from what is above.
fn draw_shape_rows(
    ui: &mut egui::Ui,
    shapes: &[ShapeInfo],
    preview_color: egui::Color32,
    base_advance: f32,
    actions: &mut dyn SidebarActions,
) {
    for (i, info) in shapes.iter().enumerate() {
        if i > 0 {
            ui.separator();
        }
        draw_shape_row(ui, info, preview_color, base_advance, actions);
    }
}

/// One shape entry (CR-021 AC3): the name label, the pre-rendered glyph preview
/// (FR16 — the same bytes and rendering as what the canvas stamps), and the
/// padding out to the sidebar edge form one grabable unit — pressing anywhere on
/// it starts placing the shape, and while a shape is being placed the selection
/// highlight spans the full width of the entry. Collection templates additionally
/// carry an edit button (topmost, so it wins clicks over the row).
fn draw_shape_row(
    ui: &mut egui::Ui,
    info: &ShapeInfo,
    preview_color: egui::Color32,
    base_advance: f32,
    actions: &mut dyn SidebarActions,
) {
    let selected = actions.is_placing(&info.type_id);
    let base_font = egui::FontId::new(crate::app::FONT_SIZE, egui::FontFamily::Monospace);

    // Entry metrics — everything is painted inside one full-width row rect.
    let label_size = ui
        .painter()
        .layout(
            info.name.clone(),
            base_font.clone(),
            egui::Color32::TRANSPARENT,
            f32::INFINITY,
        )
        .size();
    let has_edit_button = info.collection.is_some();
    // The preview gets whatever width is left after the name and (if present)
    // the edit button — the same fit rule as before the row rework.
    let preview_avail = ui.available_width()
        - label_size.x
        - ROW_GAP
        - f32::from(has_edit_button) * EDIT_BUTTON_WIDTH;
    let max_cols = info
        .preview
        .iter()
        .map(|row| row.chars().count())
        .max()
        .unwrap_or(0);
    // One fit computation for the whole entry: the reserved space and the
    // painting below both use these numbers (WYSIWYG — same bytes, same scale).
    let (preview_size, preview_row_h) = if info.preview.is_empty() {
        (crate::app::FONT_SIZE, 1.0)
    } else {
        let size = preview_font_size(
            max_cols as u32,
            preview_avail.max(0.0),
            crate::app::FONT_SIZE,
            base_advance,
        );
        (size, line_height(ui, size))
    };
    let preview_h = info.preview.len() as f32 * preview_row_h;
    let row_h = label_size.y.max(preview_h).max(1.0);
    let rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), row_h));

    // The whole entry is one clickable unit (AC3) — the rect spans to the sidebar
    // edge, so the padding after the preview is grabable too.
    let response = ui.allocate_rect(rect, egui::Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if selected {
        // Full-width selection highlight — the entry, not just its label.
        let visuals = ui.visuals();
        ui.painter().rect(
            rect,
            0.0,
            visuals.selection.bg_fill,
            visuals.selection.stroke,
            egui::StrokeKind::Middle,
        );
    }

    // Name label at the left edge, vertically centered in the row.
    let name_pos = egui::pos2(rect.min.x, rect.center().y - label_size.y / 2.0);
    ui.painter().text(
        name_pos,
        egui::Align2::LEFT_TOP,
        &info.name,
        base_font.clone(),
        ui.visuals().text_color(),
    );

    // The pre-rendered preview after the name, vertically centered.
    if !info.preview.is_empty() {
        let preview_origin = egui::pos2(
            rect.min.x + label_size.x + ROW_GAP,
            rect.center().y - preview_h / 2.0,
        );
        paint_preview(
            ui.painter(),
            preview_origin,
            &info.preview,
            preview_color,
            preview_size,
        );
    }

    // The edit button (collection templates only) sits at the right edge; added
    // after the row's interaction rect, so it wins clicks over the row.
    if has_edit_button {
        let button_rect = egui::Rect::from_center_size(
            egui::pos2(rect.max.x - EDIT_BUTTON_WIDTH / 2.0, rect.center().y),
            egui::vec2(EDIT_BUTTON_WIDTH, 18.0),
        );
        if ui
            .put(button_rect, egui::Button::new("✎"))
            .on_hover_text(format!("Edit {}", info.name))
            .clicked()
        {
            actions.edit_shape(info.type_id.clone());
        }
    }

    if response.clicked() {
        actions.select_shape(info.clone());
    }
}

/// Gap between the name label and the preview inside one entry row.
const ROW_GAP: f32 = 8.0;

/// Width of the collection-template edit button (its height is set by egui's
/// button layout within the fixed 18 pt slot).
const EDIT_BUTTON_WIDTH: f32 = 24.0;

/// The monospace line height at `size` (FR16 / CR-013): rows pack exactly like
/// the canvas does (row index × line height), so blank rows keep their slot when
/// skipped.
fn line_height(ui: &egui::Ui, size: f32) -> f32 {
    ui.painter()
        .layout(
            "M".to_owned(),
            egui::FontId::new(size, egui::FontFamily::Monospace),
            egui::Color32::TRANSPARENT,
            f32::INFINITY,
        )
        .size()
        .y
        .max(1.0)
}

/// Paints the pre-rendered preview block (FR16 / CR-013): the pattern rows drawn
/// exactly the way the canvas stamps them — monospace, one `painter.text` per
/// row at LEFT_TOP packed at line height, at the caller's fitted `size`. Pure
/// painting at an explicit origin; layout space is the caller's concern (the
/// entry row owns it).
fn paint_preview(
    painter: &egui::Painter,
    origin: egui::Pos2,
    preview: &[String],
    color: egui::Color32,
    size: f32,
) {
    if preview.is_empty() {
        return;
    }
    let font_id = egui::FontId::new(size, egui::FontFamily::Monospace);
    let row_h = painter
        .layout(
            "M".to_owned(),
            font_id.clone(),
            egui::Color32::TRANSPARENT,
            f32::INFINITY,
        )
        .size()
        .y
        .max(1.0);
    for (i, row) in preview.iter().enumerate() {
        if row.is_empty() {
            continue; // like the canvas — positions come from the row index
        }
        painter.text(
            origin + egui::vec2(0.0, i as f32 * row_h),
            egui::Align2::LEFT_TOP,
            row.as_str(),
            font_id.clone(),
            color,
        );
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
            regions: Vec::new(),
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

    /// AC2 (CR-011) — the static default category set: known non-UML built-ins get
    /// their table category; unknown built-ins and collection templates land in
    /// "Other".
    #[test]
    fn category_assignment_follows_the_static_set() {
        assert_eq!(builtin_category("box"), "Basic");
        assert_eq!(builtin_category("ellipse"), "Basic");
        assert_eq!(
            builtin_category("mystery"),
            OTHER_CATEGORY,
            "unknown built-ins fall back to Other"
        );
    }

    /// AC1 (CR-021) — UML subcategory assignment: one per supported diagram type;
    /// the UML shapes outside the three families stay in the group's "Other".
    #[test]
    fn uml_subcategory_assignment() {
        assert_eq!(uml_subcategory("class"), "Class");
        for id in ["lollipop", "component", "port", "deployment", "artifact"] {
            assert_eq!(uml_subcategory(id), "Component", "{id}");
        }
        for id in ["state", "decision", "initial", "final"] {
            assert_eq!(uml_subcategory(id), "Activity", "{id}");
        }
        assert_eq!(uml_subcategory("actor"), OTHER_CATEGORY);
        assert_eq!(uml_subcategory("activation"), OTHER_CATEGORY);
    }

    /// AC1 (CR-021) — the full built-in catalog organizes into a top-level UML
    /// group after "Built-in" and before any collection, with the Class /
    /// Component / Activity subcategories in that order and the remaining UML
    /// shapes last in "Other"; "Built-in" keeps Basic.
    #[test]
    fn group_templates_organizes_the_builtin_catalog_into_a_uml_category() {
        let mut all = core_services::Services::native().catalog.list();
        all.push(info("col/star", Some("col")));

        let groups = group_templates(&all);

        assert_eq!(groups.len(), 3);

        // Built-in keeps only the non-UML shapes, flat under Basic.
        assert_eq!(groups[0].label, BUILTIN_GROUP);
        assert_eq!(
            groups[0]
                .subgroups
                .iter()
                .map(|s| s.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Basic"]
        );
        assert_eq!(
            groups[0].subgroups[0]
                .shapes
                .iter()
                .map(|s| s.type_id.as_str())
                .collect::<Vec<_>>(),
            vec!["box", "ellipse"]
        );

        // The UML group: per-diagram subcategories in order, Other last.
        let uml = &groups[1];
        assert_eq!(uml.label, UML_GROUP);
        let subs: Vec<(&str, Vec<&str>)> = uml
            .subgroups
            .iter()
            .map(|s| {
                (
                    s.label.as_str(),
                    s.shapes.iter().map(|t| t.type_id.as_str()).collect(),
                )
            })
            .collect();
        // Subgroups in first-seen catalog order with "Other" pinned last
        // (`push_subgroup`); shapes within a subgroup keep catalog order.
        assert_eq!(
            subs,
            vec![
                (
                    "Component",
                    vec!["artifact", "component", "deployment", "lollipop", "port"]
                ),
                ("Class", vec!["class"]),
                ("Activity", vec!["decision", "final", "initial", "state"]),
                (OTHER_CATEGORY, vec!["activation", "actor"]),
            ]
        );

        // Collections still come after the UML group.
        assert_eq!(groups[2].label, "col");
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

    /// Records the placements/edits the drawing reported (CR-021 AC3 evidence).
    struct Recording {
        placed: Vec<String>,
        edited: Vec<String>,
    }

    impl Recording {
        fn new() -> Self {
            Self {
                placed: Vec::new(),
                edited: Vec::new(),
            }
        }
    }

    impl SidebarActions for Recording {
        fn is_placing(&self, _type_id: &str) -> bool {
            false
        }
        fn select_shape(&mut self, info: ShapeInfo) {
            self.placed.push(info.type_id);
        }
        fn edit_shape(&mut self, type_id: String) {
            self.edited.push(type_id);
        }
    }

    /// A bare headless context pumping one frame of the sidebar groups — the
    /// `gui_tests::Gui` pattern: `run_ui`, built-in fonts, texture deltas cleared.
    struct Pump {
        ctx: egui::Context,
        ts: f64,
        groups: Vec<SidebarGroup>,
        expanded: BTreeMap<String, bool>,
        filter: String,
        last_toggles: Vec<(String, bool)>,
        /// Type ids the drawing reported as placed/edited (via [`Recording`]).
        placed: Vec<String>,
        edited: Vec<String>,
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
                filter: String::new(),
                last_toggles: Vec::new(),
                placed: Vec::new(),
                edited: Vec::new(),
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
            let mut rec = Recording::new();
            let mut out = self.ctx.run_ui(raw, |ui| {
                ui.vertical(|ui| {
                    self.last_toggles = draw_shape_groups(
                        ui,
                        groups,
                        expanded,
                        egui::Color32::WHITE,
                        &mut self.filter,
                        &mut rec,
                    );
                });
            });
            self.placed.extend(rec.placed);
            self.edited.extend(rec.edited);
            // Headless: no GPU to upload glyph textures — clear the pending deltas.
            out.textures_delta.clear();
            out
        }

        /// Clicks (press + release) at `pos`, one event per frame.
        fn click(&mut self, pos: egui::Pos2) -> egui::FullOutput {
            let button = |pressed: bool| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            };
            self.pump(vec![egui::Event::PointerMoved(pos), button(true)]);
            self.pump(vec![button(false)])
        }

        /// The rect of the first drawn text shape whose content is exactly `text`.
        fn text_rect(&self, out: &egui::FullOutput, text: &str) -> egui::Rect {
            out.shapes
                .iter()
                .filter_map(|cs| match &cs.shape {
                    egui::epaint::Shape::Text(t) if t.galley.text() == text => {
                        Some(egui::Rect::from_min_size(t.pos, t.galley.size()))
                    }
                    _ => None,
                })
                .next()
                .unwrap_or_else(|| panic!("text {text:?} not drawn"))
        }

        /// Clicks the filter input (located via its placeholder galley) and types
        /// `s` into it, one `Event::Text` per frame — the CR-012 typing pattern.
        fn type_filter(&mut self, out: &egui::FullOutput, s: &str) -> egui::FullOutput {
            let rect = out
                .shapes
                .iter()
                .filter_map(|cs| match &cs.shape {
                    egui::epaint::Shape::Text(t) if t.galley.text() == "Filter shapes…" => {
                        Some(egui::Rect::from_min_size(t.pos, t.galley.size()))
                    }
                    _ => None,
                })
                .next()
                .expect("filter placeholder drawn")
                .expand(10.0);
            let center = rect.center();
            self.pump(vec![
                egui::Event::PointerMoved(center),
                egui::Event::PointerButton {
                    pos: center,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ]);
            let mut out = self.pump(vec![egui::Event::PointerButton {
                pos: center,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }]);
            for c in s.chars() {
                out = self.pump(vec![egui::Event::Text(c.to_string())]);
            }
            out
        }

        /// Deletes `n` trailing characters of the filter (press+release per key).
        fn backspace_filter(&mut self, n: usize) -> egui::FullOutput {
            let mut out = self.frame();
            for _ in 0..n {
                let key = |pressed: bool| egui::Event::Key {
                    key: egui::Key::Backspace,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: Default::default(),
                };
                let _ = self.pump(vec![key(true)]);
                out = self.pump(vec![key(false)]);
            }
            out
        }

        fn frame(&mut self) -> egui::FullOutput {
            self.pump(Vec::new())
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

    // ── CR-012: filter_groups (pure, tier 0) ──

    /// AC1 — case-insensitive substring match on the shape *name* only.
    #[test]
    fn filter_matches_case_insensitive_substring_of_name() {
        let groups = group_templates(&[info("box", None), info("ellipse", None)]);

        // "bo" matches "Box"; "BOX" (uppercase query) does too.
        let filtered = filter_groups(&groups, "bo");
        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered[0].subgroups[0]
                .shapes
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
            vec!["box"]
        );
        assert_eq!(filter_groups(&groups, "BOX").len(), 1);

        // No match at all → the group disappears entirely (AC2).
        assert!(filter_groups(&groups, "square").is_empty());
    }

    /// AC1 — matching is on the name, not the type id or labels: a shape whose
    /// type id contains the needle but whose name does not is excluded.
    #[test]
    fn filter_matches_name_not_type_id() {
        let mut star = info("col/star", Some("col"));
        star.name = "Compass".to_owned(); // name ≠ type id
        let groups = group_templates(&[star]);

        assert!(
            filter_groups(&groups, "star").is_empty(),
            "type id is not matched"
        );
        assert_eq!(filter_groups(&groups, "comp").len(), 1);
    }

    /// AC2 — empty subcategories are dropped, groups with none left disappear;
    /// a blank query returns the input unchanged.
    #[test]
    fn filter_drops_empty_groups_and_subgroups() {
        let groups = group_templates(&[
            info("box", None),
            info("ellipse", None),
            info("col/a", Some("col")),
        ]);

        // "b" matches only "box": the built-in group keeps one shape, the
        // collection group vanishes.
        let filtered = filter_groups(&groups, "b");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].label, "Built-in");
        assert_eq!(filtered[0].subgroups.len(), 1);

        // Blank queries (empty or whitespace-only) change nothing.
        assert_eq!(filter_groups(&groups, ""), groups);
        assert_eq!(filter_groups(&groups, "   "), groups);
    }

    /// AC2 — a multi-subcategory group keeps only the matching subcategories,
    /// in their original order.
    #[test]
    fn filter_keeps_matching_subcategories_in_order() {
        let mut alpha = info("mystery", None); // → "Other"
        alpha.name = "Zebra".to_owned();
        let groups = group_templates(&[info("box", None), alpha, info("ellipse", None)]);

        let filtered = filter_groups(&groups, "zebra");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].subgroups.len(), 1);
        assert_eq!(filtered[0].subgroups[0].label, OTHER_CATEGORY);
    }

    // ── CR-012: drawing with a filter (tier 0b) ──

    /// AC1 — typing into the filter input narrows the drawn rows live per
    /// keystroke; groups without matches are not drawn at all.
    #[test]
    fn typing_filter_narrows_visible_rows() {
        let groups = group_templates(&[
            info("box", None),
            info("ellipse", None),
            info("col/a", Some("col")),
        ]);
        let mut pump = Pump::new(groups, BTreeMap::new());
        let out = pump.pump(Vec::new());

        // "bo" → only the built-in group with its one matching shape remains.
        let out = pump.type_filter(&out, "bo");
        let texts = drawn_texts(&out);
        assert!(texts.contains(&"box".to_owned()));
        assert!(!texts.contains(&"ellipse".to_owned()), "non-match hidden");
        assert!(
            !texts.contains(&"col".to_owned()),
            "empty group header hidden"
        );

        // Backspacing to an empty query restores the full list (AC2).
        let out = pump.backspace_filter(2);
        let texts = drawn_texts(&out);
        assert!(texts.contains(&"box".to_owned()));
        assert!(texts.contains(&"ellipse".to_owned()));
        assert!(texts.contains(&"col".to_owned()));
    }

    /// AC3 — while the filter is active a persisted-collapsed group renders its
    /// matching rows (forced open) and reports no toggles, so the persisted
    /// state survives the filtering; clearing the filter re-applies it.
    #[test]
    fn filter_forces_open_without_persisting_toggles() {
        let groups = group_templates(&[info("box", None), info("ellipse", None)]);
        let mut expanded = BTreeMap::new();
        expanded.insert("Built-in".to_owned(), false); // persisted closed
        let mut pump = Pump::new(groups, expanded);

        // Collapsed: rows hidden, no toggles.
        let out = pump.pump(Vec::new());
        assert!(!drawn_texts(&out).contains(&"box".to_owned()));
        assert!(pump.last_toggles.is_empty());

        // Filter "bo": the match must be visible despite the persisted collapse,
        // and nothing may be reported for persistence.
        let out = pump.type_filter(&out, "bo");
        assert!(
            drawn_texts(&out).contains(&"box".to_owned()),
            "forced open shows match"
        );
        assert!(pump.last_toggles.is_empty(), "no toggles while filtering");

        // Clear the filter: the persisted closed state applies again.
        let mut out = pump.backspace_filter(2);
        for _ in 0..12 {
            out = pump.frame(); // let the closing animation settle
        }
        assert!(
            !drawn_texts(&out).contains(&"box".to_owned()),
            "persisted state re-applied"
        );
    }

    // ── CR-013: preview_font_size (pure, tier 0) ──

    /// AC3 — a pattern whose natural width fits the available width keeps the base
    /// (canvas) size; exactly-at-the-boundary still fits.
    #[test]
    fn preview_fits_at_base_size_when_it_fits() {
        // 3 cols × 8 px advance = 24 px ≤ 100 px → unchanged.
        assert_eq!(preview_font_size(3, 100.0, 14.0, 8.0), 14.0);
        // Exactly at the boundary: 5 × 8 = 40 px ≤ 40 px.
        assert_eq!(preview_font_size(5, 40.0, 14.0, 8.0), 14.0);
    }

    /// AC3 — overflow scales the font down uniformly so the block just fits.
    #[test]
    fn preview_scales_down_to_fit() {
        // 20 cols × 8 px = 160 px > 80 px → size = 14 × 80/160 = 7.0.
        let size = preview_font_size(20, 80.0, 14.0, 8.0);
        assert!((size - 7.0).abs() < f32::EPSILON);
    }

    /// AC3 — degenerate inputs are safe: no division by zero, no negative or zero
    /// sizes; extreme overflow floors at the minimum instead of collapsing.
    #[test]
    fn preview_size_degenerate_inputs_are_safe() {
        assert_eq!(preview_font_size(0, 100.0, 14.0, 8.0), 14.0);
        assert_eq!(preview_font_size(5, 100.0, 14.0, 0.0), 14.0);
        assert_eq!(preview_font_size(5, f32::NAN, 14.0, 8.0), 14.0);
        assert_eq!(preview_font_size(5, -1.0, 14.0, 8.0), 14.0);
        // 10 000 cols in 10 px would be ~0.002 pt — floored at the minimum.
        assert_eq!(
            preview_font_size(10_000, 10.0, 14.0, 8.0),
            MIN_PREVIEW_FONT_SIZE
        );
    }

    // ── CR-013: pre-rendered drawing (tier 0b) ──

    /// The font size of a drawn text shape's galley (single-section layouts).
    fn galley_font_size(galley: &egui::Galley) -> f32 {
        galley
            .job
            .sections
            .first()
            .map(|s| s.format.font_id.size)
            .unwrap_or(0.0)
    }

    /// AC1/AC2 — every preview row is drawn as its own text shape carrying exactly
    /// that row's bytes, and a small shape renders at the canvas font size
    /// (WYSIWYG: same bytes, same rendering as placement).
    #[test]
    fn preview_rows_are_prerendered_at_canvas_size() {
        let mut box_info = info("box", None);
        box_info.preview = vec!["###".into(), "# #".into(), "###".into()];
        let groups = group_templates(&[box_info]);
        let mut pump = Pump::new(groups, BTreeMap::new());
        let out = pump.pump(Vec::new());

        // The preview rows are the text shapes whose content is one of the pattern
        // rows (the name label "box" and headers are other texts).
        let preview_shapes: Vec<&egui::epaint::TextShape> = out
            .shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                egui::epaint::Shape::Text(t)
                    if t.galley.text() == "###" || t.galley.text() == "# #" =>
                {
                    Some(t)
                }
                _ => None,
            })
            .collect();

        // All three rows are drawn — including both identical "###" rows.
        assert_eq!(preview_shapes.len(), 3, "every row is its own text shape");
        for shape in &preview_shapes {
            assert_eq!(
                galley_font_size(&shape.galley),
                crate::app::FONT_SIZE,
                "a fitting shape renders at the canvas font size (WYSIWYG)"
            );
        }
    }

    /// AC3 — a wide template is scaled down so its preview fits the available
    /// width: drawn font size below base, galley width within the sidebar.
    #[test]
    fn wide_preview_is_scaled_down_to_fit() {
        let mut wide = info("col/wide", Some("col"));
        wide.name = "Wide".to_owned();
        // 60 columns: at base size (advance ≈ 8 px) that is ≈480 px — far beyond
        // the harness's ~384 px available width, so it must render scaled down.
        let row: String = "#".repeat(60);
        wide.preview = vec![row.clone(), row.clone()];
        let groups = group_templates(&[wide]);
        let mut pump = Pump::new(groups, BTreeMap::new());
        let out = pump.pump(Vec::new());

        let preview_shapes: Vec<&egui::epaint::TextShape> = out
            .shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == row => Some(t),
                _ => None,
            })
            .collect();
        assert_eq!(preview_shapes.len(), 2, "both rows drawn");

        // The harness screen is 400 px wide with the default 8 px margins — the
        // preview must fit within that (plus a sub-pixel rounding epsilon).
        let available = 400.0 - 16.0;
        for shape in &preview_shapes {
            assert!(
                galley_font_size(&shape.galley) < crate::app::FONT_SIZE,
                "wide template renders scaled down"
            );
            assert!(
                shape.galley.size().x <= available + 1.0,
                "galley width {} fits the sidebar",
                shape.galley.size().x
            );
        }
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

    // ── CR-021: whole-row grabable entries (AC3, tier 0b) ──

    /// AC3 — pressing anywhere on an entry places its shape: the name label, the
    /// preview area, and the padding out to the sidebar edge all start placement.
    #[test]
    fn clicking_the_preview_or_padding_of_an_entry_places_its_shape() {
        let mut box_info = info("box", None);
        box_info.preview = vec!["###".into(), "# #".into(), "###".into()];
        let groups = group_templates(&[box_info]);
        let mut pump = Pump::new(groups, BTreeMap::new());
        let out = pump.pump(Vec::new());

        // 1) The name label.
        pump.click(pump.text_rect(&out, "box").center());
        assert_eq!(pump.placed, vec!["box".to_owned()]);

        // 2) The preview area (its middle row is unique).
        let out = pump.frame();
        pump.click(pump.text_rect(&out, "# #").center());
        assert_eq!(pump.placed, vec!["box".to_owned(), "box".to_owned()]);

        // 3) The padding to the right of the entry: past the preview, inside the
        // row (the harness screen is 400 px wide with 8 px margins).
        let out = pump.frame();
        let preview_row = pump.text_rect(&out, "# #");
        pump.click(egui::pos2(380.0, preview_row.center().y));
        assert_eq!(pump.placed.len(), 3, "padding click also places");
    }

    // ── CR-021: separator lines between entries (AC4, tier 0b) ──

    /// AC4 — a full-width horizontal separator line divides every pair of adjacent
    /// entries within a run; the first row of a run has no leading line.
    #[test]
    fn separator_lines_divide_adjacent_entries() {
        let groups = group_templates(&[info("box", None), info("ellipse", None)]);
        let mut pump = Pump::new(groups, BTreeMap::new());
        let out = pump.pump(Vec::new());

        let box_row = pump.text_rect(&out, "box");
        let ellipse_row = pump.text_rect(&out, "ellipse");
        assert!(box_row.center().y < ellipse_row.center().y, "rows in order");

        // Full-width horizontal line shapes: (y, x_start, x_end).
        let lines: Vec<(f32, f32, f32)> = out
            .shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                egui::epaint::Shape::LineSegment { points, .. } => {
                    let (a, b) = (points[0], points[1]);
                    if a.y != b.y || (a.x - b.x).abs() <= 300.0 {
                        None // keep only full-width horizontal lines
                    } else {
                        Some((a.y, a.x.min(b.x), a.x.max(b.x)))
                    }
                }
                _ => None,
            })
            .collect();

        // Exactly one line at the boundary between the two rows…
        let between = lines
            .iter()
            .filter(|(y, _, _)| *y > box_row.center().y && *y < ellipse_row.center().y)
            .count();
        assert_eq!(between, 1, "one separator at the row boundary");
        // …and nothing below the last entry (the header underline above the run is
        // not a row separator and sits above both rows).
        assert!(
            lines.iter().all(|(y, _, _)| *y < ellipse_row.center().y),
            "no separators below the last row"
        );
    }
}
