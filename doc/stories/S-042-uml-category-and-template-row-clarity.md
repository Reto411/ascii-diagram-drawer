---
title: S-042 - uml-category-and-template-row-clarity
description: CR-021 — UML category with Class/Component/Activity subcategories (incl. new compartmented class shape), whole-row grabable template entries, separator lines between entries
scope: gui-app
status: done
updated: 2026-11-02
---

# S-042: UML category with per-diagram subcategories and clearer template entries

## Why

[CR-021](../crs/CR-021-uml-category-and-template-row-clarity.md) (accepted 2026-11-01, FR6 + FR18): the templates menu has no UML category — all eleven S-036 UML shapes dump into "Other"; only the entry's text label is grabable (the preview and padding are dead space); and there is no visual separation between entries, so it is hard to tell which text belongs to which template. The sidebar shall offer a UML category with Class / Component / Activity subcategories (each sufficient to draw that diagram type — Class adds the new compartmented class shape), whole entries must be grabable, and a separator line must divide adjacent entries.

## Dependencies

- S-032 (done) — collapsible groups + subcategories: the group→subcategory mechanism UML builds on (`sidebar.rs::BUILTIN_CATEGORIES`)
- S-034 (done) — pre-rendered previews: hit-area/row-affordance rework only, preview rendering unchanged
- S-036 (done) — the eleven UML shapes being re-homed; catalog gains the new class shape
- S-037 (done) — shape-owned text regions: reused for the class shape's name/attributes/methods compartments

## Acceptance criteria

Written from CR-021's requested change (items 1–4), 2026-11-02.

- **AC1 — UML category with per-diagram subcategories (CR items 1+2, FR18).** `group_templates` on the full built-in catalog produces a top-level **UML** group after "Built-in" and before any collection, containing the three per-diagram subcategories with their required members: **Class** = `[class]`, **Component** = component/port/artifact/lollipop/deployment-node (all five), **Activity** = state/decision/initial/final (all four); UML shapes belonging to none of the three (`actor`, `activation`) stay in the UML category's **Other** subcategory, which is pinned last. The CR specifies membership, not display order — subgroups and shapes follow catalog order (deterministic; pinned by the test). The "Built-in" group keeps Basic `[box, ellipse]`; collection groups are unaffected. Evidence: sidebar unit test `group_templates_organizes_the_builtin_catalog_into_a_uml_category`.
- **AC2 — compartmented class shape (CR item 2, FR19).** New built-in `class`: a three-compartment box (13×10 glyph art pinned in tests), standard anchors `n=(6,0) s=(6,9) w=(0,4) e=(12,4)`, and regions `name (1,1,11,2)`, `attributes (1,4,11,2)`, `methods (1,7,11,2)`. Content of each region stamps into its compartment through the pipeline (new golden), and a class statement with all three `.ad` region clauses parses and round-trips. Evidence: `builtin.rs` catalog/art/anchor/region tests + render-pipeline golden `class-compartmented.grid` + ad-format round-trip test.
- **AC3 — whole entry grabable (CR item 3, FR6).** Pressing anywhere on a template entry — its name label, its preview area, or the padding to its right — starts placing that shape; while a shape is being placed, the highlight spans the full width of the entry (not just the label). Evidence: tier-0b test `clicking_the_preview_or_padding_of_an_entry_places_its_shape` + committed goldens showing the full-width selection.
- **AC4 — separator lines between entries (CR item 4, FR6).** A horizontal separator line across the sidebar width divides every pair of adjacent template entries. Evidence: tier-0b test asserting a full-width horizontal segment at each row boundary in the frame + committed goldens.
- **Feature bar:** `ui_decomposition_class_diagram` — a small class diagram drawn purely through the real UI: arm the UML ▸ Class entry by clicking its preview area, place two classes on the canvas, connect them with an arrow (anchor→anchor drag), then double-click one class and fill in name/attributes/methods region text; assert model state + rendered content.

## Implementation notes

> Filled in while working the story: approach, files touched, decisions made (link ADRs), deviations from plan.

- **AC1/AC2 (shape-registry):** `builtin.rs` gains the compartmented `class` built-in — 13×10 three-compartment glyph art (`CLASS_ART`, pinned by a byte-exact test), standard anchors `n/s/w/e`, and three named regions `name`/`attributes`/`methods` via the existing S-037 region machinery (no model/pipeline changes needed — FR19 already supported multiple regions). `sidebar.rs::BUILTIN_CATEGORIES` reworked: a `UML_GROUP` top-level group; UML shapes map to per-diagram subcategories (`class` → Class; artifact/component/deployment/lollipop/port → Component; decision/final/initial/state → Activity); actor + activation fall through to the UML group's Other. `group_templates` now supports **two levels of nesting** (group → subgroup → rows) with "Other" pinned last per level; display order within a level follows catalog order — the CR specifies membership, not order (see AC1).
- **AC3/AC4 (gui-app sidebar):** `draw_shape_row` reworked from label-only interaction to a full-width entry: one `ui.allocate_rect(row_rect, Sense::click())` covering name + preview + right padding; press anywhere → `select_shape`. The placing highlight now spans the full row width. Between rows (and after the last row of each subgroup), a horizontal separator line is painted across the sidebar content width (`SIDEBAR_SEP_COLOR`, palette-aware). Previews are painted raw into the allocated rect as before (WYSIWYG at canvas font size; tall entries like the class simply get a taller row).
- **Feature bar:** `ui_decomposition_class_diagram` (gui_tests) — arms the class entry by clicking its *preview* area (after `scroll_sidebar_until`, the UML group sits below the fold), places two classes, connects east→west with an anchor drag, then double-clicks one and fills name/attributes/methods through the S-037 region editor; asserts model state (positions, connection endpoints, region content) + rendered canvas text.
- **Goldens regenerated** (`GOLDEN_UPDATE=1`): the sidebar now shows the UML group with subcategory headers, per-row separators, and the tall class preview — an intentional visual change. Canvas area byte-identical (WYSIWYG preserved).
- No ADR changes; no new dependencies.

## Verification

> One line per AC with evidence (test name, command + result summary). All ACs must be ✅ before the story is marked done.

- [x] AC1 — `group_templates_organizes_the_builtin_catalog_into_a_uml_category` (sidebar unit test): UML group after Built-in / before collections; Class=`[class]`, Component=all five, Activity=all four, Other=`[activation, actor]` pinned last; Built-in keeps `[box, ellipse]`. ✅
- [x] AC2 — `builtin.rs`: catalog entry + `CLASS_ART` byte-exact + anchor set + region rects tests; render-pipeline golden `class-compartmented.grid` (region content stamped into compartments); ad-format round-trip with all three region clauses. ✅
- [x] AC3 — tier-0b `clicking_the_preview_or_padding_of_an_entry_places_its_shape`: clicks on label, preview, and right padding each start placing; full-width highlight asserted in-frame; goldens show it. ✅
- [x] AC4 — tier-0b separator test: full-width horizontal segment at every row boundary in the frame; goldens show the lines. ✅
- [x] Feature bar — `ui_decomposition_class_diagram` passes (real UI only: preview-click arm → canvas placement ×2 → anchor drag connect → double-click region text entry). ✅

**Gates (2026-11-02):** `cargo fmt` clean; `cargo clippy --workspace --all-targets` 0 warnings; `cargo test --workspace` all green (707 passed); `cargo tarpaulin --engine Llvm --workspace` **93.52%** ≥ 80%; `mkdocs build --strict` exit 0.
