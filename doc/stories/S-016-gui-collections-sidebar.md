---
title: S-016-gui-collections-sidebar
description: sidebar shows built-ins + loaded collection templates; place fixed-size custom shapes; invalid collection files skipped with warning
scope: gui-app
status: done
updated: 2026-10-29
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

- **Seam change (ADR 009 → v5):** `ShapeInfo` gains two read-model fields — `collection: Option<String>` (`None` = built-in) and `preview: Vec<String>` (the glyph pattern rows, so the sidebar can draw a preview without touching registry types). `RegistryCatalog` fills both from the bound `ShapeRegistry`; `FakeCatalog` carries them through for GUI tests.
- **Startup discovery lives in `core-services`, not `shape-registry`:** per the [collection concept](../arc42/12-concepts/collection.md), the search path is an *application* concern. New `crates/core-services/src/collections.rs::load_collections(&mut ShapeRegistry, Option<&Path>) -> Vec<CollectionLoadWarning>` — scans `*.adc` in sorted order (deterministic), parses each with the S-005 parser (identity = filename stem), and on error pushes `{file, reason}` (the parser's 1-based line/column message) instead of failing. A missing directory is a warning, not an error — startup always succeeds.
- **Composition root:** `Services::native_with_collections(dir: Option<&Path>) -> (Services, Vec<CollectionLoadWarning>)` wires built-ins + discovered collections into one registry behind the seam. The GUI never sees collection files or registry types.
- **CLI:** `gui-app --collections <dir> [fixture.ad]` — parsed by a pure, unit-tested `parse_startup_args` in `main.rs`; unknown flags and a dangling `--collections` are ignored (same not-fatal philosophy). Fixture pass-through for the visual tiers unchanged.
- **Sidebar view model:** `gui-app/src/sidebar.rs::group_templates(&[ShapeInfo]) -> Vec<SidebarGroup>` — pure, deterministic: built-in group (label "Built-in") always first, then one group per collection in listing order; within-group template order preserved. Rendered in `app.rs` as a scrollable left panel: group headings, selectable names, monospace 9px glyph previews (the same bytes the renderer stamps), and red weak warning lines under the "Shapes" heading for each skipped file.
- **Placement:** unchanged machinery — selecting a custom template (`collection/name`) places it at its fixed pattern size; the S-013 ADR 007 v3 gate already withholds the resize handle from qualified ids, and the renderer stamps the pattern verbatim with live anchors (S-006 opaque-stamp rule).
- **Web mechanism (recorded per AC2):** on wasm there is no filesystem; user-supplied collection text will be loaded through the same `parse_collection` path when the S-019 wasm entry point lands (a paste box + load button in the sidebar, or a downloaded-file flow — to be decided with S-019). Until then the wasm build is built-in-only; `native_with_collections` is native-gated.

## Verification

- [x] **AC1 — Sidebar groups.** `sidebar.rs::group_templates` unit-tested from fake catalog data (`FakeCatalog` via the seam, T7 pattern): built-ins form the first "Built-in" group, each collection its own group, interleaved listings regroup correctly, grouping is deterministic regardless of input order, empty listing → no groups. `app.rs` renders the groups (headings + selectable names + monospace glyph previews) inside a scrollable panel — visual confirmation deferred with the tiers below.
- [x] **AC2 — Load collections at startup.** `collections.rs`: `valid_files_load_in_sorted_order` (two `.adc` files → both templates available through the seam, group = filename stem), `invalid_file_is_skipped_with_a_warning_and_startup_succeeds` (row longer than declared width → warning with file + `line 3, column 1`, other collection still loads, built-ins intact), `empty_dir_is_silent`, `missing_dir_is_a_warning_not_an_error`. Composition level: `services.rs::native_with_collections_loads_valid_and_warns_on_invalid` (valid file reaches the catalog with `collection` + `preview` fields; broken file → one warning; built-ins survive) + `native_with_no_collections_dir_is_builtins_only`. GUI edge: `main.rs::parse_startup_args` tests (fixture, `--collections <dir>`, unknown flags ignored, dangling flag not fatal); `app.rs` carries the warnings into the sidebar (`create_app_loads_the_startup_fixture` asserts). Web mechanism recorded in Implementation notes (S-019).
- [x] **AC3 — Place custom shapes.** `editor.rs::custom_templates_place_at_their_fixed_size` — selecting a `stars/star` template then clicking places at the clicked cell with the template's fixed 3×3 size, single-shot mode, object selected (resize handle withheld by the S-013 qualified-id gate — `canvas.rs::hit_target` test pins it). Render assertion: `core-services renderer.rs::custom_templates_stamp_verbatim_and_keep_live_anchors` — a placed `col/star` instance renders with its exact pattern bytes in the grid, and `catalog.anchors("col/star")` returns the declared anchor set (connectors stay live).
- [x] **AC4 — Gates.** `cargo fmt --all --check` ✓; `cargo clippy --workspace --tests` 0 warnings/errors; `cargo test --workspace` all green (**61** gui-app tests, was 53; **31** core-services, was 27); `cargo check -p gui-app --target wasm32-unknown-unknown` exits 0. Coverage: canonical workspace gate `cargo tarpaulin --engine Llvm --workspace` = **87.95%** ≥ 80%; new modules — `sidebar.rs` 23/23 (100%), `collections.rs` 28/31 (uncovered = the IO-error arm of file reads, unreachable as root in this container), `main.rs` parse logic covered by unit tests (the uncovered remainder is the `main()`/`run_native` bootstrap). The literal `-p gui-app` tarpaulin total remains a measurement artifact (documented since S-013).
- [ ] **Visual regression (tier 1/2) — ⚠️ deferred to image rebuild** (same environment gap as S-012/S-013/S-014: this container predates the winit X11 client libs, `libXcursor.so.1` is missing; apt is blocked by ADR 014's egress allow-list, and the Dockerfile already installs them). After the next image rebuild: (1) `sh .agents/skills/gui-testing/scripts/smoke.sh launch --collections gui-app/tests/fixtures/collections` → expect exit-124 PASS; (2) goldens: `smoke.sh capture gui-app/tests/goldens/s016-one-box.png 3 gui-app/tests/fixtures/one-box.ad` (sidebar now shows previews — supersedes the S-014 golden once captured) and `smoke.sh capture gui-app/tests/goldens/s016-collection.png 3 gui-app/tests/fixtures/one-box.ad --collections gui-app/tests/fixtures/collections` → re-run with fresh shots, `smoke.sh diff` → expect AE=0. Visual check: sidebar has a "Built-in" group + a "stars" group (star preview), each shape shows its glyph pattern; the star template places a 3×3 `#.#`/`###`/`#.#`… pattern with no resize handle; a broken `.adc` in the directory shows a red warning line and every other shape still works.

**Manual smoke checklist for a human with a display** (`cargo run -p gui-app -- --collections <dir-with-*.adc>`):
1. Window opens; sidebar lists **Built-in** (box, ellipse) plus one group per `.adc` file in the directory, each shape with a small monospace preview of its pattern.
2. Drop a syntactically broken `.adc` into the directory, restart → a red warning line names the file + reason; all valid shapes still load and place; startup never fails.
3. Click a collection shape (e.g. `stars`) then click the canvas → it places at its fixed pattern size (no resize handle), is selected, and can be moved like any other object.
4. Draw a connector from a box anchor to the custom shape's declared anchor → it routes and stays attached after moving either object.
5. Save + reopen the document → the `collection/name` instance round-trips (the collection dir must be available at load time). 
