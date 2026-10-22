---
title: S-017-gui-template-editor
description: shape template editor — text-grid art editing + graphical anchor definition (click cell to toggle), save to collection file; ADR 007 last piece
scope: gui-app
status: backlog
updated: 2026-10-22
---

# S-017: gui-app — template editor (text art + graphical anchors)

## Why

FR3 includes *defining* user objects fully in plain text; ADR 007 v2 puts the **graphical anchor definition mode** in v1 scope, and FR11 requires per-shape anchor sets. This is the last unimplemented piece of [ADR 007](../adr/007-user-defined-diagram-objects.md).

## Dependencies

- S-016 — collections sidebar (editor entry point + saved templates appear there)
- S-005 — template file grammar + anchor markers to write out

## Acceptance criteria

1. **AC1 — Text-art editing.** Editor shows the template's character grid; the user types svgbob-alphabet characters into cells (non-alphabet input rejected with inline hint); edits update a live preview rendered through the real pipeline. Check: unit tests of the editor state (edit, reject invalid char, preview grid equality with `render` output).
2. **AC2 — Graphical anchors.** Clicking a cell in "anchor mode" toggles an anchor there (visual marker distinct from art); anchors are stored per the S-003 marker rule; at least one anchor is required to save (validated, not silently dropped). Check: unit tests of toggle/requirement logic + round-trip through the template file format.
3. **AC3 — Save to collection.** Saving writes the template into a collection file via `ShapeCatalog`/store services; it immediately appears in the sidebar (S-016) and can be placed on the canvas with working anchors (connectable, S-007). Check: end-to-end unit test with fakes: edit → save → catalog contains type → render shows pattern + live anchor cells.
4. **AC4 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p gui-app` ≥ 80%; wasm check still exits 0.
5. **AC5 — ADR 007 flipped to accepted** (all v1-scope follow-ups implemented: templates, collections, per-type anchors, graphical anchor mode); `shape-template.md`/`collection.md` Implementation sections finalized; overview tracker updated.

## Implementation notes

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 — 
