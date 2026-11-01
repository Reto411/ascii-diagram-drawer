---
title: S-033-shape-filter
description: CR-012 — name filter in the template sidebar (FR6)
scope: gui-app
status: done
updated: 2026-11-01
---

# S-033: gui-app — name filter in the template sidebar (FR6)

## Why

FR6 extended by CR-012: a text input in the template sidebar narrows the visible shape list by name while typing — any shape findable in one keystroke regardless of catalog size.

## Dependencies

- none beyond done stories (S-016, S-032)

## Acceptance criteria

Written from [CR-012](../crs/CR-012-shape-filter.md)'s "Requested change" + the §14 FR6 wording ("a template sidebar **with a filter input that narrows the shape list by name**"), per its confirmed assumptions (case-insensitive substring on the shape name; transient — not persisted; empty groups hidden while filtering; no category/tag matching in v1):

- **AC1** — The template sidebar shows a single-line text input above the shape groups. Typing into it narrows the visible list live, per keystroke: only shapes whose *name* contains the query as a case-insensitive substring are shown (e.g. "bo" matches "Box"; "BOX" does not). Matching is on the shape name only — group labels, subcategory labels, and type ids are not matched.
- **AC2** — While the filter is active (non-blank query), groups and subcategories with no matching shape are hidden entirely (their headers are not drawn); when the last character is deleted, the full list returns exactly as before. The filter state is transient: it is not written to the options file and a restart starts with an empty filter.
- **AC3** — While the filter is active, matching groups/subcategories render expanded regardless of their persisted collapse state (a match must be visible without first expanding), and header clicks during filtering do not alter the persisted open/closed states; when the filter clears, the previously persisted states apply again.

## Implementation notes

- `sidebar.rs` gains a pure `filter_groups(groups, query)` (tier 0, no egui) used by the drawing path; the input is a single-line `egui::TextEdit` drawn inside `draw_shape_groups`, with the query state owned by `DrawingApp` (transient field, not in `GuiConfig`).
- Filter-active expansion uses egui's `CollapsingHeader::open(Some(true))`; because that force marks the header changed every frame, toggle reporting/persistence is skipped while the query is non-blank.

## Verification

- AC1: `sidebar::tests::filter_matches_case_insensitive_substring_of_name` ("bo"/"BOX" match "Box", no-match query empties the group), `filter_matches_name_not_type_id` (type id containing the needle is *not* matched — name only); tier-0b `typing_filter_narrows_visible_rows` (real egui frames: click the filter input, type "bo" → only the matching row remains, non-matching shape and empty group header absent; backspacing to blank restores the full list).
- AC2: `filter_drops_empty_groups_and_subgroups` (partial match keeps one subgroup, empty collection group disappears; blank/whitespace queries return the input unchanged); tier-0b restore covered by `typing_filter_narrows_visible_rows`; transience by construction — `DrawingApp.filter_query` is a plain field, not part of `AppConfig` (no config round-trip test applies).
- AC3: tier-0b `filter_forces_open_without_persisting_toggles` — a persisted-collapsed group shows its matching row while filtering (forced open via `CollapsingHeader::open(Some(true))` on separate `id_salt` slots so the forced state cannot leak into the unfiltered slots), reports zero toggles, and re-applies the persisted closed state after the filter clears.
- AC5-style scope: `git diff --stat` touches only `gui-app/src/{app,sidebar}.rs`, `gui-app/tests/goldens/*.png` + docs; no model/format/pipeline/seam change (CR-012 conflict check held — pure GUI).
- Material goldens regenerated for the new filter row (pixel diff confined to the sidebar band, x 8–170).
- Gates: `cargo fmt` clean; `cargo clippy --workspace --all-targets` 0 warnings; `cargo test --workspace` all green (gui-app 253 passed); `cargo tarpaulin --engine Llvm --workspace` **93.59%** (≥ 80%).
