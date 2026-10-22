---
title: S-016-gui-collections-sidebar
description: sidebar shows built-ins + loaded collection templates; place fixed-size custom shapes; invalid collection files skipped with warning
scope: gui-app
status: backlog
updated: 2026-10-22
---

# S-016: gui-app — collections sidebar + custom shape placement

## Why

FR3 (user-defined objects usable from the UI) and FR6 (sidebar = template list); invalid collection files must be skipped with a warning, never crash startup (RS family). The catalog is loaded through `ShapeCatalog` (ADR 009 seam).

## Dependencies

- S-012 — sidebar + placement foundation
- S-005 — template/collection loading in the registry

## Acceptance criteria

1. **AC1 — Sidebar groups.** Template sidebar shows built-in shapes and each loaded collection as a group with its templates (name + small preview of the glyph pattern). Check: unit test of the sidebar view-model built from fake catalog data (grouping, ordering).
2. **AC2 — Load collections at startup.** Native: collections discovered from a configured directory via `ShapeCatalog`; web: user-supplied collection text loaded (mechanism recorded in Implementation notes). Invalid files are skipped and a warning lists file + reason; startup always succeeds. Check: unit tests with fake catalog (valid, invalid, empty dir) asserting warnings + available templates.
3. **AC3 — Place custom shapes.** Clicking a template then the canvas places it at its fixed size (no resize handles), stamped verbatim by the renderer (S-006 opaque-stamp rule); anchors remain live for connectors. Check: unit tests of placement with fakes + render assertion that the pattern appears in the grid.
4. **AC4 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p gui-app` ≥ 80%; wasm check still exits 0.

## Implementation notes

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
