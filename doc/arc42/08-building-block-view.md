---
title: "arc42-8-building-block-view"
description: static decomposition of the system, top-down — level 0 to 3 building blocks (design-level; no code yet)
scope: architecture
status: draft
updated: 2026-10-18
---

# 8. Building Block View

> *Static decomposition of the system, top-down. Level 0 = whole system; each level zooms into one building block from the previous level.*
>
> *Status note: this is a **design-level** decomposition derived from [ADR 001](../adr/001-glyph-resolution-alphabet.md), [ADR 004](../adr/004-diagram-file-format.md), [ADR 005](../adr/005-use-svgbob-as-vector-renderer.md), [ADR 007](../adr/007-user-defined-diagram-objects.md), [ADR 008](../adr/008-gui-framework-choice.md) and the concepts in [§12](12-concepts/index.md). Crate/module names are proposals until the workspace is scaffolded.*

## 8.1 Level 0: System Overview

```plantuml
@startuml
left to right direction
actor "User" as user
rectangle "ascii-diagram-drawer\n(egui/eframe app — native desktop + browser wasm)" as app
user --> app : gestures: place / move / resize /\nconnect / pan / export
app --> user : canvas view (WYSIWYG)\ntext · SVG · PNG output
file ".ad diagram files" as ad
folder "shape template &\ncollection files" as tpl
app <--> ad : load / save\n(ADR 004)
app --> tpl : load shapes,\nanchors, resize rules\n(ADR 007)
note bottom of app : fully offline — no external services (§5.2)
@enduml
```

## 8.2 Level 1: Major Components

Six building blocks — five platform-independent core crates (headless, GUI-free per NFR5/Q5) and one GUI application crate that compiles to native **and** wasm ([ADR 008](../adr/008-gui-framework-choice.md)):

| Building block | Purpose | Interfaces (in → out) | Depends on |
|----------------|---------|------------------------|------------|
| `diagram-model` | Pure domain model: objects, grid coordinates, anchors, invariants | construction/mutation API → `Diagram` value type | — (leaf) |
| `ad-format` | Open plain-text file format: parse/serialize, versioning ([ADR 004](../adr/004-diagram-file-format.md)) | `&str` → `Result<Diagram, ParseError>`; `Diagram` → `String` | diagram-model |
| `shape-registry` | Predefined shapes + user templates/collections; per-type anchor sets; resize rules ([ADR 007](../adr/007-user-defined-diagram-objects.md)) | template/collection files → `ShapeRegistry`; lookup by `type_id` | diagram-model (types) |
| `render-pipeline` | Pure renderer: place → resolve → **resolved character grid** ([concept C2](12-concepts/render-pipeline.md), [ADR 001](../adr/001-glyph-resolution-alphabet.md)) | `&Diagram, &ShapeRegistry` → `ResolvedGrid` | diagram-model, shape-registry |
| `exporters` | Text / SVG / PNG emission from the one grid ([ADR 005](../adr/005-use-svgbob-as-vector-renderer.md), FR5) | `&ResolvedGrid` → `String` (text/SVG), bytes (PNG) | render-pipeline (grid type), svgbob |
| `gui-app` | egui/eframe editor: template sidebar + WYSIWYG canvas, gestures → model mutations, file/export IO ([ADR 008](../adr/008-gui-framework-choice.md)) | user gestures → view + model mutations + exports | **all core crates**, egui/eframe |

```plantuml
@startuml
left to right direction
package "GUI — one codebase, native + wasm (ADR 008)" {
  [gui-app] as gui
}
package "Core — pure Rust, headless, no windowing deps" {
  [diagram-model] as model
  [ad-format] as fmt
  [shape-registry] as reg
  [render-pipeline] as pipe
  [exporters] as exp
}
gui --> model : mutate (place/move/resize/connect)
gui --> pipe : render(model) → grid (per frame)
gui --> exp : export(grid) → text/SVG/PNG
gui --> fmt : load / save .ad
gui --> reg : template sidebar, anchors
fmt --> model
pipe --> model
pipe --> reg : glyph patterns, anchor sets
exp ..> pipe : ResolvedGrid type only
note bottom of exp : text = grid as-is · SVG via svgbob (ADR 005) ·\nPNG = rasterized SVG (rasterizer TBD)
@enduml
```

**Dependency rules** (enforced by the Q5.2/T5 architecture test): strictly one-directional — GUI → core, never the reverse; within core: `ad-format → model`, `render-pipeline → {model, registry}`, `exporters → pipeline` (grid type only). No cycles, no windowing/event-loop/WASM-API dependency anywhere in core.

**Proposed workspace layout:**

```
ascii-diagram-drawer/
├── Cargo.toml                 # workspace
├── crates/
│   ├── diagram-model/         # leaf — pure data + invariants
│   ├── ad-format/             # .ad parse/serialize (ADR 004)
│   ├── shape-registry/        # shapes, templates, anchors (ADR 007)
│   ├── render-pipeline/       # place → resolve → grid (ADR 001)
│   └── exporters/             # text / svg / png (ADR 005)
└── gui-app/                   # eframe app — native + wasm32 target
```

## 8.3 Level 2: Component Internals

#### 8.3.1 `diagram-model`

- **Sub-modules:** `objects`, `anchors`, `invariants`.
- **Key data structures:**
  - `Diagram { objects: Vec<Object>, /* stable document order = z-order */ }`
  - `Object::Shape(ShapeInstance) | Text(TextInstance) | Connector(ConnectorInstance)`
  - `ShapeInstance { id, type_id, pos: CellPos, size: CellSize }` — all coordinates in **cell units** (grid alignment, FR7)
  - `AnchorSet` — named anchor cells *relative to the shape origin*, defined **per type** and shared by all instances (FR11, [ADR 007](../adr/007-user-defined-diagram-objects.md))
  - `ConnectorInstance { id, from: AnchorRef, to: AnchorRef, style }` — bound to anchors, not raw coordinates (FR9)
- **Key invariants** (enforced at construction — invalid states unrepresentable): integer cell coordinates; anchor cells inside their shape's bounds; connectors reference existing anchor refs.

#### 8.3.2 `ad-format`

- **Sub-modules:** `parser`, `serializer`, `versioning`, `errors`.
- **Key data structures:** textual `.ad` document per [ADR 004](../adr/004-diagram-file-format.md) with a version header; parse errors carry line/column.
- **Key algorithms:** deterministic serialization (object order = document order → stable diffs, Q3); lossless round-trip `load(serialize(m)) ≡ m` as a property test (QS7); unknown-field tolerance within one major version (QS8).

#### 8.3.3 `shape-registry`

- **Sub-modules:** `builtin` (predefined catalog + standard default anchors incl. UML connection points, FR11), `templates` (user template/collection files, [ADR 007](../adr/007-user-defined-diagram-objects.md)), `resize-rules` (**v1+** — v1 shapes are fixed-size), `registration` (**v1+**, FR12).
- **Key data structures:** `ShapeType { id, glyph_pattern: TemplateGrid, anchor_set: AnchorSet, resize_rules: Option<…> }`; `Collection { name, types }`.
- **Key algorithms:** template matching for ASCII import (v1+) — diagram regions matched against known `glyph_pattern`s; unmatched regions → warning + fixed-size reduced form until registered (FR12).

#### 8.3.4 `render-pipeline`

- **Sub-modules:** `place`, `resolve` ([glyph resolution, concept C1](12-concepts/glyph-resolution.md)), `grid`.
- **Key data structures:** `ResolvedGrid { width, height, cells: Vec<Cell> }` — one canonical-alphabet glyph per occupied cell; the grid is a **derived intermediate, never stored** ([ADR 004](../adr/004-diagram-file-format.md)).
- **Key algorithms:** stamp objects in document order (z-order) → per-cell glyph selection from N/E/S/W connections + content (§8.4); strictly row-major traversal and fixed priority orders — no hash-iteration order may reach the output (Q1, T1).

#### 8.3.5 `exporters`

- **Sub-modules:** `text`, `svg`, `png`.
- **Key algorithms:**
  - *Text*: dump the resolved grid as UTF-8 with LF — the grid **is** the output.
  - *SVG*: pass the resolved grid **unchanged** to `svgbob::to_svg` — no adapter layer ([ADR 005](../adr/005-use-svgbob-as-vector-renderer.md)); fixed attribute set, no timestamps → byte-identical re-export (QS1/QS2).
  - *PNG*: rasterize the produced SVG. **Rasterizer TBD** — must work on native *and* wasm32 (risk R3; v1 may degrade to SVG-only if needed).

#### 8.3.6 `gui-app`

- **Sub-modules:** `canvas` (view), `interaction` (gestures), `sidebar` (templates, FR6), `commands` (model-mutation boundary), `io` (file open/save, clipboard, export — per-target native APIs, [ADR 008](../adr/008-gui-framework-choice.md) consequences).
- **Key design decision — WYSIWYG canvas:** the canvas draws the **resolved character grid in a monospace font** — i.e. the exact bytes of the text output — with selection/anchor handles as an overlay on top. What the user sees *is* what gets exported; no separate vector preview that could diverge from the text. (Proposed here; promote to an ADR if contested.)
- **Key algorithms:** per-frame immediate-mode re-render of grid + overlays (bounded by P2, ≤16 ms/frame); gesture → grid-snapped mutation (display zoom is view-only; the scale control changes rendered spacing at model level, FR8); connectors drawn anchor-to-anchor with live re-binding on move/resize (FR9).
- **Targets:** one codebase → native (winit + wgpu/glow) and browser wasm (WebGL) per [ADR 008](../adr/008-gui-framework-choice.md).

## 8.4 Level 3: Module Internals (as needed)

Only one module's complexity demands a third level of zoom: the **resolve stage** of `render-pipeline`.

- **Input:** per occupied cell — the set of connections passing through it (N/E/S/W) plus content kind (shape border/fill, connector line, text glyph).
- **Output:** exactly one glyph from the canonical svgbob alphabet ([ADR 001](../adr/001-glyph-resolution-alphabet.md)).
- **Algorithm sketch:** classify the junction — cross > T-junction > corner > straight (deterministic priority table; ties broken by fixed order, never by iteration order); text cells pass through unchanged.
- **Open spec point:** the full rule table (connection set × content → glyph) is finalized during implementation and pinned immediately by golden tests + the alphabet-purity property test (T1/T3).

**Other open items carried from earlier chapters:** PNG rasterizer choice (§8.3.5, risk R3); template/collection file naming & extension ([ADR 007](../adr/007-user-defined-diagram-objects.md) follow-up); exact default anchor set per predefined shape (FR11).
