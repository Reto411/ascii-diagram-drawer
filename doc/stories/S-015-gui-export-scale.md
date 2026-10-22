---
title: S-015-gui-export-scale
description: export menu — SVG save/download, PNG via worker with "exporting" state (RS4) + scale control (FR8)
scope: gui-app
status: backlog
updated: 2026-10-22
---

# S-015: gui-app — SVG/PNG export + scale control

## Why

FR5 (SVG/PNG export from the same resolved grid) and FR8 (scale control scales the *rendering output*, not display zoom); RS4 (export may take time — show "exporting", never freeze the UI) is satisfied via the S-011 worker API.

## Dependencies

- S-012 — app shell + canvas
- S-011 — PNG worker API with pollable state
- S-008, S-009 — text/SVG/PNG exporters behind `Services`

## Acceptance criteria

1. **AC1 — SVG export.** "Export SVG" produces the exact `Exporter::to_svg` bytes and saves them (native: file; web: download); a failure notice shows on error. Check: unit test of the export flow with fake services asserting the sink receives the exporter's exact bytes.
2. **AC2 — PNG export, non-blocking.** "Export PNG" submits to the worker API; while pending the UI shows an "exporting…" state and stays responsive (no blocking call on the UI thread); on completion the PNG is saved/downloaded or an error notice appears. Check: unit test with a fake slow exporter asserting Pending → Done transitions drive the UI state machine; code review confirms no rasterization call on the frame path.
3. **AC3 — Scale control.** A scale control (top-right area per FR8) changes the model-level scale via the aggregate, re-renders the canvas, and affects all exports (text/SVG/PNG) identically; default 1. Check: unit test asserting scale change → `render`/`expand` output differs and export bytes follow.
4. **AC4 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p gui-app` ≥ 80%; wasm check still exits 0.

## Implementation notes

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
