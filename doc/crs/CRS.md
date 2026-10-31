---
title: change-request-board
description: persistent change-request board — open / closed; single source of truth for CR status (CRs live in doc/crs/CR-NNN-*.md)
scope: delivery
status: stable
updated: 2026-10-31
---

# Change Request Board

Filed via the **change-request skill** (`.agents/skills/change-request/SKILL.md`). One file per CR under `doc/crs/`; this board is the status source of truth. A CR stays Open while `proposed` (awaiting decision) and `accepted` (awaiting implementation); it closes as `implemented` or `rejected`.

## Open

| ID | Title | Priority | Status | File |
|----|-------|----------|--------|------|
| CR-001 | Native file dialogs for Open, Save and Export | medium | accepted | [CR-001-native-file-dialogs.md](CR-001-native-file-dialogs.md) |
| CR-002 | Grouped dropdown menus in the toolbar | low | accepted | [CR-002-grouped-dropdown-menus.md](CR-002-grouped-dropdown-menus.md) |
| CR-003 | Template creation via text box + resize-rule toolset | high | accepted | [CR-003-template-textbox-resize-rules.md](CR-003-template-textbox-resize-rules.md) |
| CR-004 | Right-side SVG preview pane | medium | accepted | [CR-004-svg-preview-pane.md](CR-004-svg-preview-pane.md) |
| CR-005 | Options menu with theme selection and config persistence | low | accepted | [CR-005-options-menu-theme-config.md](CR-005-options-menu-theme-config.md) |
| CR-006 | Material Design GUI with custom title bar | medium | accepted | [CR-006-material-design-title-bar.md](CR-006-material-design-title-bar.md) |
| CR-007 | User-modifiable arrow shapes (manual bends) | medium | accepted | [CR-007-manual-arrow-bends.md](CR-007-manual-arrow-bends.md) |
| CR-008 | Selectable arrow tips on both ends + arrow-shaped anchor markers | medium | accepted | [CR-008-selectable-arrow-tips.md](CR-008-selectable-arrow-tips.md) |
| CR-009 | Diagram colors (background/shape fill/line & text) via color picker, svgbob Settings | medium | accepted | [CR-009-diagram-colors-picker.md](CR-009-diagram-colors-picker.md) |
| CR-010 | Standalone arrows with attachable/detachable ends | medium | accepted | [CR-010-standalone-arrows.md](CR-010-standalone-arrows.md) |
| CR-011 | Collapsible groupings with subcategories (menus + sidebar) | low | accepted | [CR-011-collapsible-groupings-subcategories.md](CR-011-collapsible-groupings-subcategories.md) |
| CR-012 | Shape filter in the template sidebar | low | accepted | [CR-012-shape-filter.md](CR-012-shape-filter.md) |
| CR-013 | Pre-rendered shape previews in the template sidebar (WYSIWYG picking) | medium | accepted | [CR-013-prerendered-shape-previews.md](CR-013-prerendered-shape-previews.md) |
| CR-015 | UML shape set in the built-in catalog (requires CR-003; compartmented shapes need CR-016) | high | accepted | [CR-015-uml-shape-set.md](CR-015-uml-shape-set.md) |
| CR-016 | Shape-owned text boxes — definable text regions on templates (format change, ADR 004) | high | accepted | [CR-016-shape-text-boxes.md](CR-016-shape-text-boxes.md) |
| CR-017 | Multi-select (Ctrl/Shift+click) and group move | medium | accepted | [CR-017-multi-select-group-move.md](CR-017-multi-select-group-move.md) |
| CR-018 | Resize symbol on the shape's resize hook | low | accepted | [CR-018-resize-handle-symbol.md](CR-018-resize-handle-symbol.md) |

## Closed

| ID | Title | Closed | Resolution |
|----|--------|--------|------------|
| CR-014 | Canvas panning for diagram navigation | 2026-10-31 | implemented — S-022, commit `973d2d7` (FR17) |
