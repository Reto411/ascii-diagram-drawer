---
title: S-032-collapsible-groupings-subcategories
description: CR-011 — collapsible menu/sidebar groups + subcategories (FR6)
scope: gui-app
status: done
updated: 2026-10-31
---

# S-032: gui-app — collapsible menu/sidebar groups + subcategories (FR6)

## Why

FR6 extended by CR-011: every UI grouping (menu-bar dropdowns, template sidebar) is collapsible/expandable and supports nested subcategories, so long lists of actions and shapes stay scannable as the catalog grows.

## Dependencies

- S-025 — the menu bar must exist first

## Acceptance criteria

From [CR-011](../crs/CR-011-collapsible-groupings-subcategories.md) "Requested change" + §14 FR6 wording ("groups in the menu bar and in the sidebar are collapsible and can be organized into subcategories"). Confirmed assumptions (bulk acceptance): predefined shapes get a static default category set; user/imported templates land in the default ("Other") category; collapse state defaults to expanded for first use and persists with the CR-005 config file.

- **AC1 — Collapsible sidebar groups.** Every template-sidebar group (Built-in, one per loaded collection) renders as an `egui::CollapsingHeader`: expanded by default on first use, clicking the header toggles it, and a collapsed group hides its shape rows. *Headless:* tier-0b test — after clicking the "Built-in" header the box/ellipse labels vanish from the frame; a second click brings them back.
- **AC2 — Sidebar subcategories.** Shapes within a group are organized into subcategories by a pure view-model function: built-in shapes get a static default category set (`box`, `ellipse` → “Basic”; an unknown built-in → “Other”), and user/imported (collection) templates land in “Other”. A group with ≥ 2 subcategories renders each as a nested collapsing header; a group with exactly one renders its shapes flat (no redundant nesting level). *Headless:* unit tests on the view model against fake catalog data (assignment, order, “Other” last) + a tier-0b test driving the group-drawing function with a two-subgroup group — nested headers appear and collapse.
- **AC3 — Collapse state persists.** Per-group/subcategory open-closed state is written to the app config file on toggle (CR-005 store), read at startup, missing key = expanded; a pre-feature file (no field) loads with everything expanded. *Headless:* config round-trip unit test + tier-0b test via `DrawingApp::with_config` on a temp dir — collapse a group, the file records it closed, and a second app instance with the same config starts collapsed.
- **AC4 — Menu-bar subcategories.** File ▾ nests related actions one level down: “Output ▸” (Copy, Text .txt) and “Export ▸” (Export SVG…, Export PNG…); Open/Save/Save As… stay top-level; item order within a group is unchanged and every item stays reachable and functional (S-025 behavior preserved). *Headless:* tier-0b — open File ▾, assert both submenu entries exist and the moved items are no longer top-level; open “Export ▸”, assert both export items appear; click “Export SVG…” → the path prompt opens.
- **AC5 — Pure GUI structure.** No changes to `diagram-model`, `ad-format`, `render-pipeline`, `exporters`, or the `ui-api`/`core-services` seams (CR-011 conflict check: no model/format/pipeline change). *Verification:* `git diff --stat` shows only `gui-app` + docs.

## Implementation notes

- **Sidebar (AC1–AC3):** `gui-app/src/sidebar.rs` — the group drawing moved out of `app.rs` into a dedicated module. `group_templates(&[ShapeInfo]) -> Vec<SidebarGroup>` is a pure view-model function (unit-tested against fake catalogs): built-ins get the static default subcategory set (`box`/`ellipse` → "Basic", anything else → "Other"), collection templates land in "Other"; groups with exactly one subcategory render flat, ≥ 2 render each as a nested `egui::CollapsingHeader`. `draw_sidebar` renders each group as a `CollapsingHeader` wired to the app's open/closed state; toggles are reported back through an out-param so `app.rs` persists them.
- **Persistence (AC3):** `gui-app/src/config.rs` — `AppConfig.sidebar_expanded: BTreeMap<String, bool>` (group/subcategory path → open); `default()` = absent (everything expanded), so pre-feature config files load fully expanded. `app.rs` writes on every toggle (same immediate-save pattern as CR-005's theme) and seeds the state at startup.
- **Menu bar (AC4):** `gui-app/src/app.rs` — File ▾ now nests `ui.menu_button("Output", …)` (Copy, Text .txt) and `ui.menu_button("Export", …)` (Export SVG…, Export PNG…); inside a menu egui renders those as hover-opened submenus with their own arrow glyph, so the labels stay plain. Open/Save/Save As… stay top-level; item order within each group is unchanged.
- **Tests:** 9 new sidebar tests in `sidebar.rs` (pure view-model: assignment/order/"Other"-last/flat-vs-nested; config round-trip + pre-feature file; egui tier-0b: header toggle hides/shows shape labels, nested subcategory headers collapse, toggle persists to the config file and a second instance starts collapsed). `gui_tests.rs` gained `click_submenu_item` (hover-opens a submenu headlessly) and `open_prompt` routes the export items through it; S-025's grouped-actions test was rewritten for the nested structure.

## Verification

- AC1/AC2: `sidebar::tests` — view-model unit tests + `builtin_header_toggles_its_shape_rows`, `nested_subcategory_headers_render_and_collapse` (tier-0b, real egui frames).
- AC3: `sidebar::config_round_trip_records_and_restores_state`, `pre_feature_config_loads_everything_expanded`; tier-0b `toggle_persists_to_the_config_file_and_survives_a_restart` (temp dir, two instances via `DrawingApp::with_config`).
- AC4: `gui_tests::file_menu_lists_the_grouped_actions_with_submenus` (top-level = Open…/Save/Save As…/Output/Export in order; moved items absent top-level; hovering Output reveals Copy + Text .txt in order; hovering Export reveals both export items in order); item reachability/functionality via the existing S-014/S-015 tests, which now drive the submenus (`copy_button_puts_the_exact_canvas_bytes_on_the_clipboard`, `open_prompt("Export SVG…")`/`"Export PNG…"` call sites) — all pass.
- AC5: `git diff --stat` for this story touches only `gui-app/src/{app,config,sidebar,gui_tests}.rs` + docs; no core-crate or seam files.
- Gates: `cargo fmt` clean; `cargo clippy --workspace --all-targets` 0 warnings; `cargo test --workspace` 603 passed / 0 failed; `cargo tarpaulin --engine Llvm --workspace` 93.60% (≥ 80%). 
