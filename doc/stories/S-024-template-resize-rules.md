---
title: S-024 Template resize rules (.adc 1.1 + re-rasterization)
description: story for CR-003 — per-edge insert/remove resize rules with parity variants in the .adc format, pure re-rasterization at instance size, resize gesture enabled for rule-bearing custom shapes, template editor text box + resize toolset
scope: doc/crs/CR-003-template-textbox-resize-rules.md / shape-registry / render-pipeline / ui-api / core-services / gui-app
status: done
updated: 2026-10-31
---

# S-024 Template resize rules (.adc 1.1 + re-rasterization)

**CR:** [CR-003](../crs/CR-003-template-textbox-resize-rules.md) — user-defined objects are fixed-size; users want to declare per-edge insert/remove characters (+ parity variants) so their templates resize like the built-ins
**Dependencies:** S-017 (template editor + .adc serializer), B-001 (ellipse re-rasterization precedent) — both done.

## Acceptance criteria

### AC1: `ResizeRules` data + pure re-rasterization in shape-registry

- `shape_registry::types::EdgeRule { insert, remove, even: Option<ParityChars>, odd: Option<ParityChars> }` and `ResizeRules { top, bottom, left, right }` (all four edges always present — a rule set is complete by construction); `ShapeType.resize_rules: Option<ResizeRules>` stays.
- New module `shape_registry::resize`:
  - `rasterize(grid, rules, width, height) -> Result<TemplateGrid, ResizeError>` — **pure function of (art, rules, target size)**, no IO (NFR3).
  - Algorithm (finalized here, normative for C10):
    - Target == natural size → the template grid itself (identity).
    - **Width w→W** first: each row keeps its first and last cells; its interior run gains/loses `|Δ|` cells at the right end of the run (just before the last cell — "before the right corner", C10). Row characters: top row → `top` rule, bottom row → `bottom` rule (when h > 1; a 1-row template uses `top`), interior rows → **auto-derived fill** = the cell just inside the right border (`grid[y][w−2]`), used for both insert and remove.
    - **Height h→H** second (on the width-adjusted grid): growth inserts Δ new rows just above the bottom row, each built as `left.insert + fill × (W−2) + right.insert` with `fill` sampled from the bottom interior row's inside-right cell (`grid[h−2][w−2]`, or space when no interior exists); shrink removes |Δ| rows from just above the bottom row, each valid only if its first cell == `left.remove`, last cell == `right.remove`, and its interior is uniform (any art in a removed row → error).
    - **Parity:** for an axis, the selected characters are the edge's `even`/`odd` override when the *target* extent has that parity and the override exists, else the base pair.
    - **Minimum extent:** width and height can never drop below 2 (corner cells must stay distinct) — `ResizeError::BelowMinimum`; a shrink step whose removed cell differs from the declared/auto-derived remove character is `ResizeError::RuleMismatch`. Shrinking stops there; corners are never destroyed.
  - `min_size(grid, rules) -> CellSize` — pure reachability: simulate single-step shrinks per axis from natural size until each fails (result ≥ 2×2); exposed to the GUI for gesture clamping.

### AC2: `.adc` minor version 1.1 — `resize` statements

- Grammar (normative; joins C7's file-syntax EBNF):
  ```ebnf
  resize-stmt = "resize", edge, [parity], "insert", char, "remove", char
  edge        = "top" / "bottom" / "left" / "right"
  parity      = "even" / "odd"          ; absent = the base rule for that edge
  char        = one printable NON-whitespace ASCII character
  ```
- Rules: `resize` statements appear inside a template block after the grid rows (any order among themselves/anchors in the file; canonical re-save emits them after anchors in fixed order top, bottom, left, right — base, then even, then odd). A complete set requires all four edges' **base** rules; at most one `even` and one `odd` per edge. Space is not declarable (whitespace token) — space fills are auto-derived from the art instead.
- Versioning: `resize` statements require minor ≥ 1 — a `adc 1.0` file containing one is a parse error; the reader's understood minor becomes 1 (unknown-statement tolerance for hypothetical 1.2+ files unchanged, C8). The serializer emits `adc 1.1` iff any template carries rules, else `adc 1.0` (existing files stay byte-identical).
- Round-trip: `parse(serialize(c)) == c` and canonical byte-stability hold with rules present (golden-pinned).

### AC3: render pipeline stamps rule-bearing shapes at instance size

- `place_pattern`: a `Pattern` shape with `resize_rules` is rasterized at the instance size (clamped per axis to `min_size` — hand-edited `.ad` files with out-of-range sizes render at the minimum instead of failing the whole render) and stamped; anchors map proportionally into the rasterized extent (same mapping as the B-001 ellipse path); the label renders over the rasterized extent. Rule-less templates keep today's fixed-size behavior byte-for-byte.

### AC4: seam + catalog expose resizability

- `ui_api::types`: `ShapeInfo` gains `resizable: bool` and `min_size: CellSize` (built-ins: resizable, 1×1; rule-bearing custom: computed; plain custom: not resizable). `NewTemplate` gains `resize_rules: Option<ResizeRuleSpec>` — plain seam data mirroring the registry types (no registry type crosses the seam, C3).
- `Renderer` gains `preview_resized(rows, rules: Option<&ResizeRuleSpec>, width, height)` — the template editor's multi-size live preview through the same place/resolve stages as a saved shape; `core_services::renderer` implements it, fakes follow.

### AC5: resize gesture works on rule-bearing custom shapes

- The bottom-right resize handle is offered for a selected shape iff its catalog entry says `resizable` (built-ins OR rule-bearing custom) — plain custom shapes still reject resizing (FR12).
- The editor's resize gesture clamps the new size to the shape's `min_size`; the model stores the instance size as today (no format change — `.ad` already carries per-instance sizes).

### AC6: template editor — text box primary + resize toolset

- **Art text box (top):** multi-line monospace edit of the rows — typing/pasting replaces cell-by-cell painting as the primary art input (CR motivation). Editing re-parses into rows: printable ASCII only, ≥1 row, at least one non-space cell; invalid input is rejected with a hint and the previous art kept. The existing cell grid stays as the live preview + graphical anchor surface (FR11 unchanged); `TemplateDraft` keeps its pure validation API.
- **Resize toolset:** per-edge insert/remove character fields for top/bottom/left/right, an enable toggle, and per-edge parity toggles revealing even/odd pairs; "all four edges required" + non-whitespace-character validation with hints; a live preview at chosen sizes through `Renderer::preview_resized`.
- Saving a rule-bearing draft registers the template with rules (AC4 path) — the `.adc` on disk carries the `resize` statements (AC2), the sidebar/renderer pick it up immediately, and a placed instance resizes via AC5.

## Verification

All gates green on 2026-10-31 (devcontainer): `cargo fmt --check`, `cargo clippy --workspace --all-targets` (0 warnings), `cargo test --workspace` all green, `cargo tarpaulin --engine Llvm --workspace` = **94.07%** (target 80%).

**AC1 — shape-registry (58 tests):** new `resize` module with 14 unit tests pinning the algorithm: identity at natural size; width growth stretches runs (`box_width_growth_stretches_runs`); height growth adds interior rows built from left/fill/right (`box_height_growth_adds_interior_rows`); combined growth; one-step-per-axis shrink; `BelowMinimum` rejection (corners never destroyed); `RuleMismatch` when a removed cell differs from the declared/auto-derived remove char; art-row protection on height shrink (`height_shrink_protects_art_rows`); parity variants follow the *target* extent (`parity_variants_follow_target_extent`); `min_size` reachability (2×2 for the box, stops at art boundaries); solid templates use their own fill; degenerate 1-row/1-column templates.

**AC2 — `.adc` 1.1:** parser tests `parses_resize_rules_with_parity_overrides`, `resize_statement_errors_are_typed_and_located` (typed + located errors for missing edges, whitespace chars, duplicate parity, and a `resize` in an `adc 1.0` file), `v1_1_file_without_resize_statements_stays_fixed_size`; serializer tests `resize_rules_serialize_in_edge_order_and_bump_the_header` (top/bottom/left/right, base→even→odd; header bumps to 1.1 iff rules present) and `resize_round_trip_is_stable` (parse∘serialize identity + canonical byte-stability with rules).

**AC3 — render-pipeline (71+45 tests):** new place-stage tests `rule_bearing_pattern_re_rasterizes_at_instance_size` (a 5×3 rule box placed at 7×4 stamps exactly the rasterized 7×4 extent) and `rule_bearing_pattern_clamps_to_the_minimum_extent` (hand-edited 1×1 renders at the 2×2 minimum, no render failure). Existing `resized_rule_bearing_shape_re_rasterizes` + `anchors_map_proportionally_on_resized_shapes` cover rule-free byte-for-byte behavior and proportional anchor mapping.

**AC4 — seam/catalog (core-services):** `preview_resized_re_rasterizes_rule_bearing_patterns` and `preview_resized_clamps_to_the_minimum_extent` (the editor's multi-size preview runs the same place/resolve stages); catalog builds `resizable`/`min_size` read models per shape kind (built-ins 1×1, rule-bearing customs computed, plain customs not resizable).

**AC5 — resize gesture:** `editor::tests::resize_clamps_to_the_shape_minimum_extent` (gesture clamped to the catalog min); `canvas::tests::resize_handle_follows_the_catalog_resizable_flag` (handle offered iff `resizable`; plain customs still reject, FR12).

**AC6 — template editor:** unit tests for the art text box (`set_art_reparses_into_padded_rows`, `set_art_rejects_blank_and_empty`, `set_art_rejects_non_alphabet_and_keeps_previous`, `set_art_drops_out_of_extent_anchors`, `apply_art_reverts_the_buffer_on_rejection`, paint/clear resync the buffer) and the resize toolset (`resize_toolset_complete_lands_in_seam_payload`, `resize_toolset_disabled_is_inert`, `resize_parity_pairs_validated_when_enabled`, `resize_spec_is_none_until_complete`, `from_existing_restores_declared_resize_rules`). Tier-0b end-to-end `gui_tests::template_editor_art_box_and_resize_rules_save_a_resizable_template`: type box art into the text box (Enter-key newlines) → the grid + live preview mirror the exact 5×3 art → enable all four edges → complete rules render a resized preview without error → save → the catalog marks it `resizable` with the computed 2×2 minimum → place an instance from the sidebar → drag the resize handle +2×1 → the canvas renders exactly the re-rasterized 7×4 box (a fixed-size stamp would still show the original 5×3 art).

**Totals:** workspace-wide 469 tests green — 174 gui-app (incl. the tier-0b end-to-end), 58 shape-registry, 117 render-pipeline (lib + integration), rest across ad-format / ui-api / core-services / diagram-model. Coverage per new/changed file: `resize.rs` 89/97 (91.8%), `place.rs` 215/218, `template_editor.rs` 211/216, `catalog.rs`/`renderer.rs`/`types.rs` 100%.
