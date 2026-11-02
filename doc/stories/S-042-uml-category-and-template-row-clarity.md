---
title: S-042 - uml-category-and-template-row-clarity
description: CR-021 — UML category with Class/Component/Activity subcategories (incl. new compartmented class shape), whole-row grabable template entries, separator lines between entries
scope: gui-app
status: in-progress
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

- **AC1 — UML category with per-diagram subcategories (CR items 1+2, FR18).** `group_templates` on the full built-in catalog produces a top-level **UML** group after "Built-in" and before any collection, whose subcategories appear in the order **Class** `[class]`, **Component** `[lollipop, component, port, deployment, artifact]`, **Activity** `[state, decision, initial, final]`, then **Other** `[actor, activation]` (last). The "Built-in" group keeps Basic `[box, ellipse]`; collection groups are unaffected. Evidence: sidebar unit test `group_templates_organizes_the_builtin_catalog_into_a_uml_category`.
- **AC2 — compartmented class shape (CR item 2, FR19).** New built-in `class`: a three-compartment box (13×10 glyph art pinned in tests), standard anchors `n=(6,0) s=(6,9) w=(0,4) e=(12,4)`, and regions `name (1,1,11,2)`, `attributes (1,4,11,2)`, `methods (1,7,11,2)`. Content of each region stamps into its compartment through the pipeline (new golden), and a class statement with all three `.ad` region clauses parses and round-trips. Evidence: `builtin.rs` catalog/art/anchor/region tests + render-pipeline golden `class-compartmented.grid` + ad-format round-trip test.
- **AC3 — whole entry grabable (CR item 3, FR6).** Pressing anywhere on a template entry — its name label, its preview area, or the padding to its right — starts placing that shape; while a shape is being placed, the highlight spans the full width of the entry (not just the label). Evidence: tier-0b test `clicking_the_preview_or_padding_of_an_entry_places_its_shape` + committed goldens showing the full-width selection.
- **AC4 — separator lines between entries (CR item 4, FR6).** A horizontal separator line across the sidebar width divides every pair of adjacent template entries. Evidence: tier-0b test asserting a full-width horizontal segment at each row boundary in the frame + committed goldens.
- **Feature bar:** `ui_decomposition_class_diagram` — a small class diagram drawn purely through the real UI: arm the UML ▸ Class entry by clicking its preview area, place two classes on the canvas, connect them with an arrow (anchor→anchor drag), then double-click one class and fill in name/attributes/methods region text; assert model state + rendered content.

## Implementation notes

> Filled in while working the story: approach, files touched, decisions made (link ADRs), deviations from plan.

- 

## Verification

> One line per AC with evidence (test name, command + result summary). All ACs must be ✅ before the story is marked done.

- [ ] AC1 — 
- [ ] AC2 —
