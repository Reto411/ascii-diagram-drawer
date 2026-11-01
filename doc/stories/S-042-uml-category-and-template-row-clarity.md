---
title: S-042 - uml-category-and-template-row-clarity
description: CR-021 — UML category with Class/Component/Activity subcategories (incl. new compartmented class shape), whole-row grabable template entries, separator lines between entries
scope: gui-app
status: backlog
updated: 2026-11-01
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

> Written by the cr-implementation skill when it starts, from CR-021's requested change (items 1–4). Feature bar: at least one `ui_decomposition_*` test drawing a representative diagram through the real UI — e.g. a small class diagram placed via the new UML ▸ Class entry.

## Implementation notes

> Filled in while working the story: approach, files touched, decisions made (link ADRs), deviations from plan.

- 

## Verification

> One line per AC with evidence (test name, command + result summary). All ACs must be ✅ before the story is marked done.

- [ ] AC1 — 
- [ ] AC2 —
