---
title: S-037-shape-text-regions
description: CR-016 — shape-owned text regions (FR19) (skeleton; ACs at implementation)
scope: diagram-model/ad-format/shape-registry/render-pipeline/gui-app
status: done
updated: 2026-11-01
---

# S-037: model + ad-format + shape-registry + render-pipeline + gui-app — shape-owned text regions (FR19)

## Why

FR19: templates declare one or more named text regions (text boxes) with position + extent; instances carry per-region content, and regions move/resize with the shape. Defining is textual (`region` statements in `.adc`, ADR 004 v4 — minor version 1.1 batch) and graphical in the template editor (mirroring anchors, FR11). Generalizes the single implicit label, which stays supported. Prerequisite for compartmented UML shapes (class name/attributes/methods).

## Dependencies

- none beyond done stories (S-005/S-006); unblocks compartmented UML shapes in S-036's usage

## Acceptance criteria

From CR-016 "Requested change" + FR19 (§14). Exact syntax settled by this story (C6/C7/C8 normative update in the same commit).

**AC1 — Textual region definition (`.adc` 1.1).** A template block may declare `region <name> at <x> <y> w <w> h <h>` statements after its grid rows (anchor phase), from minor version 1.1 up: the rect must fit inside the declared grid (`x + w ≤ width`, `y + h ≤ height`, extents ≥ 1); names are single tokens, unique per template. Violations and out-of-block statements are typed parse errors with line/column; a `region` statement in an `adc 1.0` file is rejected as unknown. Round trip: parse → serialize is byte-identical (canonical order: anchors, then regions by name, then resize rules).

**AC2 — Per-region instance content (`.ad`).** The `shape` statement gains zero or more `region <name> "<content>"` clauses after the size; content is a quoted string with the standard escapes and may be empty. Duplicate region names in one statement are a parse error. Instances do not validate region names against the catalog (same rule as `type_id`) — unknown names round-trip but render nothing. Canonical serialization emits clauses sorted by region name; round trip `serialize(parse(serialize(d))) == serialize(d)` holds (proptest-enforced), and files without regions serialize byte-identically to before.

**AC3 — Regions move and resize with the shape.** Rendering maps each declared region's template rect proportionally onto the instance's rendered extent (identity at natural size; same mapping as anchors, ADR 013) and stamps the instance's content for that region with the label rules: centered in the mapped rect, lines clipped to it, only space cells of drawn art may receive content (Pattern/Ellipse), svgbob quoting applied. The existing single label renders exactly as before; region content is stamped after the label so explicit content wins on overlap.

**AC4 — Graphical definition in the template editor.** Mirroring anchors (FR11): the template editor gains a region tool — dragging on the grid draws a rect that becomes an auto-named region (`r1`, `r2`, …) on release; clicking inside an existing region removes it. Regions are listed with their extent and removed individually; they persist through save (`.adc` `region` statements) and are restored when the template is reopened.

**AC5 — Editing per-region content in the GUI.** Double-clicking a placed shape whose type declares at least one region opens an edit window with a field for the label and one field per declared region; committing updates the model (empty fields clear that region's content), cancelling changes nothing. Shapes without declared regions keep their current double-click behavior.

**AC6 — Backward compatibility.** `adc 1.0` / region-free files parse and serialize exactly as before (golden files unchanged); a reader of this build preserves unknown statements from higher minors verbatim, including inside template blocks that mix known and unknown statements.

## Implementation notes

Landed in dependency order, one bounded context at a time (ADR 011):

- **diagram-model** — `ShapeInstance.regions: BTreeMap<String,String>` (sorted by name; iteration order is canonical); `region_content(name)` accessor; `Diagram::set_shape_region` (empty content clears the entry) and `Diagram::set_shape_label` (the single implicit label stays first-class, FR19). Region names are validated as single tokens at mutation time.
- **shape-registry (`.adc`)** — `region <name> at <x> <y> w <w> h <h>` statements in the template block's anchor phase, from minor 1.1 up (AC1/AC6: `region` in a 1.0 file is a typed parse error). Bounds + name-uniqueness checked at parse with line/column; canonical order anchors → regions (by name) → resize rules; a collection serializes at 1.1 iff any template carries regions or resize rules.
- **ad-format (`.ad`)** — `shape` statements gain zero or more `region <name> "<content>"` clauses after the size (AC2): duplicates are a parse error, names are not catalog-validated (same rule as `type_id`), canonical output sorted by name, region-free files byte-identical to before. Proptest round-trip extended.
- **render-pipeline** — each declared region's template rect maps proportionally onto the instance's rendered extent (identity at natural size; same mapping as anchors) and the instance content stamps into it with the label rules: centered, clipped to the mapped rect, space cells of drawn art only, svgbob quoting. Stamped after the label so explicit content wins on overlap (AC3).
- **core-services / ui-api** — `RegionInfo` in the seam types; catalog `ShapeInfo.regions`, `NewTemplate.regions`; fakes carry regions.
- **gui-app** — template editor region tool: drag draws a rect → auto-named region (`r1`, `r2`, …, skipping in-use names), click inside removes, list with extent + individual removal, overlap drags replace, art shrinkage drops out-of-extent regions, restored when the template reopens (AC4). Double-click on a shape body reports `EditOutcome::DoubleClickShape`; for types with ≥1 region this opens the content window — label field + one field per declared region in app state (`ShapeTextEditState`, egui's `TextEdit` only edits bound strings), Apply writes label + all regions, Cancel/Escape/X close without applying; region-less shapes get a status hint (AC5).

## Verification

- **AC1** — `shape-registry`: `parses_region_statements`, `region_statement_errors_are_typed_and_located` (bounds, duplicate names, outside-block, 1.0 rejection), `region_round_trip_is_stable`.
- **AC2** — `ad-format`: `shape_region_clauses_parse`, `shape_region_clauses_round_trip`, `shape_duplicate_region_clause_is_an_error`; proptest round-trip in `tests/round_trip.rs`; golden files unchanged (region-free byte identity).
- **AC3** — `render-pipeline`: `region_content_stamps_in_mapped_rect` (proportional mapping incl. non-natural sizes), `region_content_never_overwrites_art`, `region_without_content_is_inert`, `rect_region_clamps_to_interior`; label rendering unchanged (existing golden tests).
- **AC4** — `gui-app::template_editor`: `toggle_region_adds_and_removes_with_auto_names`, `add_region_drag_normalizes_and_replaces_overlaps`, `set_art_drops_regions_outside_the_new_extent`, `region_validation_blocks_saving`, `to_new_template_carries_regions`, `from_existing_restores_declared_regions`.
- **AC5** — `gui-app`: tier-0b `double_click_shape_opens_region_editor_and_apply_writes_model` (save a region-bearing template → place from sidebar → double-click → window opens with Label + r1 fields → typed label "Main" and content "hello" land on the instance, status confirms); unit `region_template_round_trips_and_double_click_reports_shape`; region-less shapes keep their behavior (`open_shape_text_edit` hint path).
- **AC6** — golden tests unchanged (`.ad` 1.0 + region-free `.adc` byte-identical); skip-and-preserve for unknown statements covered by existing parser tests; `region` in a 1.0 file rejected.
- **Gates** — `cargo fmt` clean; `cargo clippy --workspace --all-targets` zero errors (default group denied); full suite green (650 tests); `cargo tarpaulin --engine Llvm --workspace` = 92.77% (≥ 80% target). 
