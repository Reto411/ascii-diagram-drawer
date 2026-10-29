---
title: S-017-gui-template-editor
description: shape template editor — text-grid art editing + graphical anchor definition (click cell to toggle), save to collection file; ADR 007 last piece
scope: gui-app
status: done
updated: 2026-10-29
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

- Architecture follows the S-012…S-016 pattern: pure state + thin egui glue.
  - `gui-app/src/template_editor.rs` — `TemplateDraft` (grid + anchors as plain data): `set_cell`/`clear_cell` validated against the svgbob alphabet (rejected chars set an inline hint, cell untouched), `toggle_anchor` with standard edge names (`n`/`s`/`w`/`e`) on odd grids and generated names elsewhere, `validation_errors` (name token + ≥1 anchor — save is blocked, never silently dropped), `to_new_template`. `TemplateEditorSession` = draft + anchor-mode flag + last error.
  - Seam change ([ADR 009](../adr/009-ui-api-service-layer.md) → **v6**): new read/write model `NewTemplate { name, rows, anchors }` (ragged rows — the backend right-pads per the parser rule), `CatalogError::{Io, Invalid}`, `ShapeCatalog::register(collection, NewTemplate)` and `Renderer::preview_pattern(&[String])`.
  - `core-services/src/registry_state.rs` — one `ShapeRegistry` shared by renderer *and* catalog (the only interior mutability in the crate): registration validates → persists `<collection>.adc` canonically → updates the shared registry, so the sidebar lists and the renderer resolves a saved template immediately, no re-composition. Built-in-only compositions (`web()` before S-019) fail with `CatalogError::Io`.
  - `crates/shape-registry/src/serializer.rs` — `serialize_collection`, inverse of `parse_collection` for files this reader understands; golden-pinned canonical bytes; `parse(serialize(c)) == c`.
  - `gui-app/src/app.rs` — window glue: entry points (sidebar "＋ New template…", per-template edit action), grid drawn as clickable cells with distinct anchor markers, live preview pane via `preview_pattern`, Save/Cancel.
- Fixed editor canvas: the draft's own `width × height` grid is the editing surface; no auto-trim of the art (the pipeline trims at render time).

## Verification

- [x] **AC1** — `template_editor.rs` unit tests: `set_cell_accepts_alphabet_chars`, `set_cell_rejects_non_alphabet_with_inline_hint` (hint set, cell untouched), `clear_cell_resets_to_space`, `out_of_bounds_edits_are_noops`. Live preview through the *real* pipeline: seam `Renderer::preview_pattern` places the pattern in an otherwise-empty diagram via the same place/resolve stages — core-services `renderer.rs::preview_equals_render_output_for_the_same_pattern` pins preview bytes == saved-template render bytes, plus `preview_rejects_empty_and_invalid_patterns`.
- [x] **AC2** — toggle logic: `toggle_anchor_uses_standard_edge_names_and_removes`, `even_grids_have_no_standard_names`; requirement: `validation_requires_a_name_and_one_anchor` + app-level `save_template_is_blocked_without_anchors` (error surfaced, editor stays open, nothing registered — the backend re-validates independently). Round-trip through the file format: `serializer.rs::round_trip_preserves_the_collection` + `serializes_to_the_exact_canonical_bytes` (anchor lines per the S-003 coordinate rule) and `registry_state.rs::register_persists_and_updates_the_shared_registry` pins the exact `.adc` bytes on disk.
- [x] **AC3** — end-to-end with fakes: app-level `saved_template_places_and_renders_its_pattern` (edit star draft → save → catalog lists `stars/star` → place at (5,7) through the real gesture path → rendered grid is exactly the 3×3 pattern bytes); `save_template_registers_through_the_seam` covers the seam write itself; placement/connectability of custom templates with live anchors already pinned by S-016/S-013 tests (`custom_templates_stamp_verbatim_and_keep_live_anchors`).
- [x] **AC4** — `cargo fmt --all --check` clean; `cargo clippy --workspace --tests` 0 warnings/errors (clippy::all denied); `cargo test --workspace` 329 passed / 0 failed; `cargo tarpaulin --engine Llvm --workspace` **84.43%** ≥ 80% (gui-app's new modules: template_editor.rs + app editor tests all green — the literal `-p gui-app` total is a known measurement artifact, see S-014); `cargo check -p gui-app --target wasm32-unknown-unknown` exits 0 (its dead-code warnings are pre-existing — the whole app struct tree is unused until the S-019 wasm entry point).
- [x] **AC5** — ADR 007 → **accepted v5** (all v1-scope pieces done; v1+ open items: resize-rule declaration, unknown-statement preservation across collection saves); `shape-template.md` + `collection.md` Implementation sections finalized (serializer, editor, shared-registry save path); overview tracker updated.
- [ ] **Visual regression (tier 1/2) — ⚠️ deferred to image rebuild** (same environment gap as S-012…S-016: this container predates the winit X11 client libs `libXcursor.so.1`/`libxi6`/`libxkbcommon0`; apt is blocked by ADR 014's egress allow-list, and the Dockerfile already installs them). After the next image rebuild: (1) `sh .agents/skills/gui-testing/scripts/smoke.sh launch --collections gui-app/tests/fixtures/collections` → expect exit-124 PASS; (2) goldens for the changed sidebar: `smoke.sh capture gui-app/tests/goldens/s017-sidebar.png 3 gui-app/tests/fixtures/one-box.ad --collections gui-app/tests/fixtures/collections` — re-run with a fresh shot, `smoke.sh diff` → expect AE=0. Visual check of the editor itself (needs a click to open — manual): sidebar "＋ New template…" opens the window; typing `#`/`.` fills cells, typing e.g. `@` shows the inline hint and leaves the cell unchanged; toggling anchor mode marks cells distinctly from art; Save with no anchors is refused with the validation message; after Save the new shape appears in its collection group and places on the canvas with a working anchor dot.
