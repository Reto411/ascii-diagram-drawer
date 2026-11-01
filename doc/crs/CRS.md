---
title: change-request-board
description: persistent change-request board — open / closed; single source of truth for CR status (CRs live in doc/crs/CR-NNN-*.md)
scope: delivery
status: stable
updated: 2026-11-01
---

# Change Request Board

Filed via the **change-request skill** (`.agents/skills/change-request/SKILL.md`). One file per CR under `doc/crs/`; this board is the status source of truth. A CR stays Open while `proposed` (awaiting decision) and `accepted` (awaiting implementation); it closes as `implemented` or `rejected`.

## Open

| ID | Title | Priority | Status | File |
|----|-------|----------|--------|------|
| CR-019 | Window close button at the top right (rightmost of the app-bar controls) | low | accepted | [CR-019-close-button-at-top-right.md](CR-019-close-button-at-top-right.md) |
| CR-020 | Arrow object behavior and Visio-style line handles — move by drag, visible endpoint/bend handles, double-click bend insertion, in-canvas tip dropdown | high | accepted | [CR-020-arrow-object-behavior-and-line-handles.md](CR-020-arrow-object-behavior-and-line-handles.md) |
| CR-021 | UML category with per-diagram subcategories (Class/Component/Activity) and clearer template entries — whole-row grabable, separator lines | high | accepted | [CR-021-uml-category-and-template-row-clarity.md](CR-021-uml-category-and-template-row-clarity.md) |

## Closed

| ID | Title | Closed | Resolution |
|----|--------|--------|------------|
| CR-001 | Native file dialogs for Open, Save and Export | 2026-11-01 | implemented — S-039, commit `5f3edd1` (FR2/FR4/FR5; rfd + xdg-portal seam) |
| CR-015 | UML shape set in the built-in catalog (requires CR-003; compartmented shapes need CR-016) | 2026-11-01 | implemented — S-036, commit `6327c3d` (FR18) |
| CR-018 | Resize symbol on the shape's resize hook | 2026-11-01 | implemented — S-038, commit `0c05452` (FR21) |
| CR-016 | Shape-owned text boxes — definable text regions on templates (format change, ADR 004) | 2026-11-01 | implemented — S-037, commit `e387db4` (FR19; .adc/.ad v1.1 region clauses, ADR 004 stays v4) |
| CR-017 | Multi-select (Ctrl/Shift+click) and group move | 2026-11-01 | implemented — S-035, commit `d1bdc82` (FR20; selection set + group move) |
| CR-013 | Pre-rendered shape previews in the template sidebar (WYSIWYG picking) | 2026-11-01 | implemented — S-034, commit `0e5e610` (FR16) |
| CR-012 | Shape filter in the template sidebar | 2026-11-01 | implemented — S-033, commit `96e160e` (FR6) |
| CR-011 | Collapsible groupings with subcategories (menus + sidebar) | 2026-11-01 | implemented — S-032, commit `eff03c1` (FR6) |
| CR-009 | Diagram colors (background/shape fill/line & text) via color picker, svgbob Settings | 2026-11-01 | implemented — S-030, commit `0541141` (FR15; .ad v1.1, ADR 009 v8) |
| CR-010 | Standalone arrows with attachable/detachable ends | 2026-10-31 | implemented — S-031, commit `d9ad17c` (FR9, ADR 013 v6) |
| CR-008 | Selectable arrow tips on both ends + arrow-shaped anchor markers | 2026-10-31 | implemented — S-029, commit `adb1a3f` (FR9, ADR 013 v5) |
| CR-007 | User-modifiable arrow shapes (manual bends) | 2026-10-31 | implemented — S-028, commit `7010cc1` (FR9, ADR 013 v4) |
| CR-006 | Material Design GUI with custom title bar | 2026-10-31 | implemented — S-027, commit `5f78f74` (FR14) |
| CR-004 | Right-side SVG preview pane | 2026-10-31 | implemented — S-026, commit `fd9ce32` (FR6) |
| CR-002 | Grouped dropdown menus in the toolbar | 2026-10-31 | implemented — S-025, commit `16710cc` (FR6) |
| CR-003 | Template creation via text box + resize-rule toolset | 2026-10-31 | implemented — S-024, commit `d6d6f3e` (FR3) |
| CR-005 | Options menu with theme selection and config persistence | 2026-10-31 | implemented — S-023, commit `ea40669` (FR13) |
| CR-014 | Canvas panning for diagram navigation | 2026-10-31 | implemented — S-022, commit `973d2d7` (FR17) |
