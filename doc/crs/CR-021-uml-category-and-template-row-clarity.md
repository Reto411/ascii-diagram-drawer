---
title: UML category with per-diagram subcategories and clearer template entries
description: change request — UML shapes grouped under a UML category with Class/Component/Activity subcategories (each sufficient to draw that diagram type, incl. a new compartmented class shape), whole-row grabability in the template sidebar, and separator lines between entries; filed 2026-11-01
scope: change request — built-in catalog organization (FR18) + template sidebar usability (FR6)
status: implemented
filed: 2026-11-01
priority: high
format-compat: none
---

# CR-021: UML category with per-diagram subcategories and clearer template entries

## Requested change

The template menu (sidebar) shall be reorganized and made visually clearer:

1. **UML category.** The UML shapes from FR18 shall appear in the templates menu under a category **UML** — not scattered into "Other" as today.
2. **Per-diagram subcategories.** The UML category shall support at least **component, activity and class diagrams**, each diagram type getting its own subcategory: **Class**, **Component**, **Activity**. Each subcategory shall contain the shapes needed to draw that diagram type — at minimum:
   - **Class:** a class shape with name / attributes / methods text regions (FR19) — new, does not exist yet;
   - **Component:** component, port, artifact, interface lollipop, deployment node (all exist since S-036);
   - **Activity:** rounded-rectangle state (action), decision diamond, initial and final state nodes (all exist since S-036).

   UML shapes that belong to none of the three subcategories (today: actor, activation bar) stay within the UML category.
3. **Whole entry grabable.** As of now only the name text of a template entry is grabable. The *entire* entry — label, preview and padding — shall select/place its shape when pressed; the selected state shall be visible across the whole entry.
4. **Separator lines (very important).** A separator line shall divide adjacent template entries visually. Today it is hard to identify which text belongs to which template, because a tall multi-line preview blurs the boundary between consecutive entries.

## Why

Improve user experience. The requester draws UML daily: shapes must be findable by diagram type, not fished out of an "Other" bucket. And in the current sidebar the association between a label and its (often multi-row) preview is ambiguous — the requester explicitly calls the missing separators "very important".

## Current requirement wording (§14)

> **FR6** — "UI consists of a top menu bar with dropdown menus that group actions by category (e.g. File), a template sidebar with a filter input that narrows the shape list by name, and an editor canvas, plus an optional right-side preview pane showing the diagram's rendered SVG — resizable by dragging its border, hideable, and re-opened via a “Preview” bookmark at the right edge of the window (minimal); groups in the menu bar and in the sidebar are collapsible and can be organized into subcategories"

> **FR18** — "The predefined catalog covers the core UML shape set for the standard diagram types (class, use case, sequence, component, deployment, state machine, activity): at minimum interface lollipop, actor, activation bar, component, port, deployment node, artifact, rounded-rectangle state, decision diamond, initial and final state nodes — in addition to the existing box and ellipse. Every new shape is a built-in with standard anchors (FR11); shapes carry geometry only, UML semantics come from shape-owned text boxes (FR19) and free text (FR10). The exact glyph art and per-shape anchor sets are finalized in the implementing story"

## Proposed requirement wording (§14)

> **FR6** — "UI consists of a top menu bar with dropdown menus that group actions by category (e.g. File), a template sidebar with a filter input that narrows the shape list by name, and an editor canvas, plus an optional right-side preview pane showing the diagram's rendered SVG — resizable by dragging its border, hideable, and re-opened via a “Preview” bookmark at the right edge of the window (minimal); groups in the menu bar and in the sidebar are collapsible and can be organized into subcategories; each template entry reads as one unit — a separator line divides adjacent entries, and pressing anywhere on an entry (its label, preview or padding) selects/places that shape"

> **FR18** — "The predefined catalog covers the core UML shape set for the standard diagram types (class, use case, sequence, component, deployment, state machine, activity): at minimum interface lollipop, actor, activation bar, component, port, deployment node, artifact, rounded-rectangle state, decision diamond, initial and final state nodes — in addition to the existing box and ellipse. Every new shape is a built-in with standard anchors (FR11); shapes carry geometry only, UML semantics come from shape-owned text boxes (FR19) and free text (FR10). The exact glyph art and per-shape anchor sets are finalized in the implementing story. The UML shapes are organized in the template menu under a UML category with subcategories per diagram type — at minimum Class, Component and Activity, each containing the shapes needed to draw that diagram type (the Class subcategory includes a class shape with name/attributes/methods text regions, FR19); UML shapes not belonging to any of those subcategories stay within the UML category"

## Conflict check

- **No ADR conflicts.** ADR 007 (user-defined objects) and the FR19 region mechanism are used, not changed — the new class shape is a built-in declaring three named regions with the existing S-037 machinery. No version bumps needed.
- **S-032 / CR-011 (collapsible groups + subcategories, done) — extension, no contradiction.** The category table (`sidebar.rs::BUILTIN_CATEGORIES`) today maps only box/ellipse → "Basic"; all eleven S-036 UML shapes fall into the "Other" fallback. The change extends that mapping and introduces the UML grouping; the "Other" fallback for non-UML shapes stays. Structural note for the implementer: a UML category with three subcategories fits the existing group→subcategory mechanism if UML becomes its own top-level group — how to wire it is an implementation decision, not a requirement.
- **S-034 / FR16 (pre-rendered previews, done) — interaction layer only.** Previews keep rendering exactly as today (same bytes, same painter path); only the hit area and row affordances change. WYSIWYG untouched.
- **S-036 (UML set, done) — reorganization + one new shape.** No existing shape changes art/anchors; they move from "Other" into UML subcategories, and the catalog gains the compartmented class shape.

## Impact analysis

**Affected stories:** S-016 (sidebar foundation, done), S-032 (grouping/subcategories, done — category table rework), S-034 (previews, done — hit area/row affordance), S-036 (UML set, done — reorganization + new class shape), S-037 (text regions, done — reused for the class shape's three regions). All done → **rework/extension**; no in-progress story is touched.

**Format compatibility:** none — catalog organization plus one new built-in type id; `.ad` serialization of existing files is unaffected (new shapes are referenced by their new ids only when placed). ADR 004 / NFR2 not involved.

## Decision plan

On acceptance: update §14 FR6 and FR18 to the proposed wording (this CR keeps the verbatim copies); create a skeleton story on the board (**S-042** — S-040 is planned for CR-019, S-041 for CR-020) with this Why, depending on S-032/S-034/S-036/S-037 (all done); no ADR changes.

## Decision

- [x] **Accept** — proceed as proposed
- [ ] **Accept with adjustments** — <what changes>
- [ ] **Reject** — <reason>

Decided: accepted (2026-11-01) — blanket acceptance by the user: "Fix all bugs, implement all change requests and do extensive regression testing including End to end tests". Applied on acceptance: §14 FR6 + FR18 updated, skeleton story S-042 on the board; no ADR changes.

## Implementation record

Implemented by [S-042](../stories/S-042-uml-category-and-template-row-clarity.md) (done 2026-11-02): new compartmented `class` built-in (3 regions via S-037), UML top-level group with Class/Component/Activity subcategories in `group_templates` (two-level nesting; membership per item 2, display order = catalog order), full-width grabable template entries, separator lines between entries. Feature bar: `ui_decomposition_class_diagram`. Status → implemented.
