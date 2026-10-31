---
title: shape-template
description: a user-defined diagram object — named fixed-size grid of svgbob-alphabet characters with declared anchor cells; read before implementing template parsing or the template editor
scope: format / domain model
status: stable
updated: 2026-10-29
---

# Concept: Shape Template

## Definition

A shape template is a named, fixed-size grid of svgbob-alphabet characters plus declared **anchor points** (named cells where connectors may bind) — the unit of user-defined diagram content ([ADR 007](../../adr/007-user-defined-diagram-objects.md)). Templates are stored inside [collection](collection.md) files and referenced from diagrams as `collection/name`.

## Why it exists

FR3 (High): users define their own diagram objects entirely themselves, beyond the predefined shape set. Stamped ASCII art keeps authoring plain-text ("type it like an ASCII diagram by hand"), while per-type anchors keep connectors joining cleanly (FR9) and rendering deterministic (NFR3).

## How it works / behaves

### Data

- **Grid:** exactly `width × height` cells; each cell holds one printable ASCII character (U+0020–U+007E, space = empty cell). The alphabet is svgbob's input set (ADR 001) — drawing characters are interpreted by the renderer, everything else stamps as literal text.
- **Anchors:** named cells; connectors bind to them and stay attached when the instance moves (FR9). Anchor cells resolve against connecting edges like built-in shape borders at render time; all other cells stamp verbatim (opaque stamp — ADR 007 point 3).

### File syntax (v1, inside a `.adc` collection file)

```ebnf
template-block = "template", name, width, "x", height, grid-row* , anchor-stmt*
grid-row       = line of ≤ width printable-ASCII chars ; right-padded with spaces to width on parse
anchor-stmt    = "anchor", name, "at", int, int
```

- Exactly `height` physical lines follow the declaration — they are consumed as grid rows whatever they contain (a blank line is a fully empty row). Trailing spaces on rows are trimmed on parse and never emitted (canonical form); rows longer than `width` are a parse error.
- **Anchor placement rules:** `0 ≤ x < width`, `0 ≤ y < height`; anchor names are single tokens, unique within the template; several anchors may share one cell. No other v1 constraint (e.g. "on the border" is a guideline for clean joins, not an invariant).
- **No in-grid marker characters.** ADR 007's open "anchor marker character" point is settled by *coordinate-based* declarations: anchors never appear in the art itself, so there is nothing to collide with the svgbob alphabet or the double-quote escape. The graphical anchor mode (FR11) produces exactly this data — same underlying definition, file remains the source of truth.
- **Resize rules:** a template may declare **resize rules** — per-edge insert/remove characters + optional even/odd parity variants — making its instances resizable; the mechanism is normative in [resize-rules](resize-rules.md) (C10, ADR 007 v6, CR-003 accepted 2026-10-31). `resize` statements inside a template block carry them (`.adc` minor version 1.1; exact syntax settled by S-024); unknown-statement tolerance (see [diagram file format](diagram-file-format.md)) applies inside `.adc` files identically. Without rules a template stays fixed-size (ADR 007 point 4). Built-ins do not need rules: their resizability comes from the `geometry` kind (`Rect` derives borders at instance size, `Ellipse` re-rasterizes its outline at instance size — B-001).
- **Text regions (FR19, CR-016 accepted 2026-10-31):** a template may declare one or more named **text regions** with a position and extent inside the shape; instances carry per-region content that moves/resizes with the shape. `region` statements join the template block (`.adc` 1.1, ADR 004 v4); definable textually and graphically in the template editor, mirroring anchors (FR11). The existing single shape label remains supported alongside regions. Exact syntax settles with S-037.

### Identity and use

- Template `name`: single token, unique within its collection.
- Referenced from diagrams as `<collection>/<name>` — a bare type-id always means a built-in shape (resolution rule: [collection](collection.md)).
- An instance of a template is still a `ShapeInstance` in the model (`type_id = "col/name"`, position, size = the template's fixed size, label rendered inside per ADR 004); resizing a custom object without declared rules is rejected (FR12).

## Relations

- **Collection** — templates live in collections; one file per collection (link: `doc/arc42/12-concepts/collection.md`)
- **Diagram file format** — `collection/name` type-ids and the shared versioning scheme (link: `doc/arc42/12-concepts/diagram-file-format.md`)
- **Glyph resolution** — template cells are opaque stamps except anchor cells, which join like borders (link: `doc/arc42/12-concepts/glyph-resolution.md`)
- **Cell grid** — the instance occupies a `width × height` rectangle of cells (link: `doc/arc42/12-concepts/cell-grid.md`)

## Implementation

`crates/shape-registry` (leaf crate — Shape Catalog context, [§8.4.2](../08-building-block-view.md); landed with S-005):

- `types.rs` — `TemplateGrid` (fixed `width × height`, printable-ASCII cells), `AnchorSet` (named cell positions, duplicate names rejected), `ShapeType { type_id, name, glyph_pattern, anchor_set, geometry, resize_rules }` with `GeometryKind::{Rect, Ellipse, Pattern}` (how the pipeline draws the type — [render pipeline](render-pipeline.md)), `Collection { name, types }`.
- `builtin.rs` — the built-in catalog as data: `box` (9×5 rectangle, `Rect`) and `ellipse` (9×5 svgbob-style oval, `Ellipse` — re-rasterized at instance size by the pipeline, B-001). All v1 shapes carry `resize_rules = None`; custom templates are `Pattern`. S-024 populates `resize_rules` for rule-bearing templates and extends the catalog with the UML shape set (S-036).
- **Standard default anchor set (FR11):** every built-in declares the four edge midpoints — `n`, `s`, `w`, `e`. For both built-ins (both 9×5): `n` at (4,0), `s` at (4,4), `w` at (0,2), `e` at (8,2). Anchor cells sit on the shape border.
- `templates.rs` — `.adc` parser per the file syntax above (`parse_collection`): 1-based line/column errors, versioning + unknown-statement tolerance identical to [`.ad`](diagram-file-format.md), short grid rows right-padded, statement order kept for canonical re-save.
- `registry.rs` — `ShapeRegistry::lookup`: bare type-id → built-ins only, `collection/name` → user collections only (disjoint namespaces, typed `LookupError`).
- `serializer.rs` (S-017) — `serialize_collection(&Collection) -> String`, the inverse of the parser for files this reader understands: canonical bytes (templates in declaration order, trailing spaces trimmed from rows, anchors in name order), golden-pinned; `parse(serialize(c)) == c` for every well-formed collection.

Rendering of stamps + anchor joins landed with S-006/S-007 (`render-pipeline place.rs`: custom templates stamp verbatim as `GeometryKind::Pattern` with live anchor cells). The graphical template editor (FR11, same underlying data) landed with S-017: `gui-app/src/template_editor.rs` (pure draft state — cell edits validated against the svgbob alphabet, click-to-toggle anchors with standard edge names on odd grids, at-least-one-anchor validation) over the seam's `ShapeCatalog::register` + `Renderer::preview_pattern` ([ADR 009](../../adr/009-ui-api-service-layer.md) v6); saves persist through `core-services/src/registry_state.rs` into `<collection>.adc` via `serialize_collection`. Both authoring modes write the same canonical bytes — the file remains the source of truth.
