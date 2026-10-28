---
title: S-014-gui-file-io-text-output
description: open/save .ad through DiagramStore with parse-error notices (RS2/RS5) + copy/save plain text output (FR2)
scope: gui-app
status: backlog
updated: 2026-10-22
---

# S-014: gui-app — file open/save + text output UX

## Why

FR4 (save/load `.ad`) and FR2 (plain monospace text out) reach the user through the GUI; RS2 (parse errors shown with line/column, nothing silently dropped) and RS5 (save failures surface clearly) are runtime safeguards that live in this layer. IO goes through `DiagramStore` (ADR 009) — never direct file access in views.

## Dependencies

- S-012 — app shell + canvas
- S-010 — `DiagramStore` service (fs backend native, in-memory web)

## Acceptance criteria

1. **AC1 — Open.** "Open" loads a `.ad` via `DiagramStore::load` and renders it; on parse error the notice shows line/column and the previous model is kept (RS2). Check: unit tests of the open flow with fake store (success, parse error, IO error) asserting model + notice state.
2. **AC2 — Save.** "Save" / "Save as" writes via `DiagramStore::save`; failures surface a clear notice (RS5); native uses the filesystem backend, web target offers download of the `.ad` text (decision recorded in Implementation notes). Check: unit tests with fake store; native path exercised by a tempdir integration test.
3. **AC3 — Text output.** "Copy text" puts `Exporter::to_text(render(model))` on the clipboard and "Save as .txt" writes it (web: download); the copied bytes equal the canvas content exactly (WYSIWYG). Check: unit test asserting the exact string passed to the clipboard/file sink for a fixture model.
4. **AC4 — No silent data loss.** Opening a file with unknown v1 statements keeps them per the ADR 004 tolerance rule only if re-saving would preserve them; otherwise the user is warned before overwriting (rule recorded in Implementation notes). Check: unit test of the warning path.
5. **AC5 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p gui-app` ≥ 80%; wasm check still exits 0.

## Implementation notes

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 — 
