---
title: connector-routing
description: how anchor-bound connectors are resolved to cells and routed in the render pipeline — straight or single-corner L, structural-edge joins, arrowhead at the target end; read when touching connector rendering
scope: rendering
status: stable
updated: 2026-10-28
---

# Concept: Connector Routing

## Definition

The stage of the render pipeline (C2) that turns each `ConnectorInstance` — two `(object, anchor)` references and nothing else — into a routed line on the resolved grid: a straight segment or a single-corner L of structural edges, with an arrowhead glyph at the target end.

## Why it exists

FR9 requires connectors bound to object anchors that re-resolve when shapes move or resize, so text and vector output stay in sync. Because the pipeline has no layout pass and must be pure (NFR3), the route can only be a function of the two resolved endpoint cells — which makes the routing strategy an explicit design decision ([ADR 013](../../adr/013-connector-routing.md)).

## How it works / behaves

- **Endpoints re-resolve on every render.** A `ConnectorInstance` stores no geometry. Each anchor reference is looked up in the endpoint shape's type (catalog), then mapped to a grid cell:
  - `Rect` instances (live pos/size): template coordinates map proportionally per axis, rounded to nearest (ties up) — side-midpoint anchors stay at side midpoints at any size.
  - `Pattern` instances (natural size): template coordinates offset by the instance position.
  - Missing anchor names or anchors declared outside the template extent are typed `RenderError`s (the §8.4.1 render-time invariant).
- **Straight or single-corner L (ADR 013).** Shared row/column → straight segment. Otherwise one corner cell: the final segment approaches the target perpendicular to its side when possible; otherwise the source's preference, then a fixed default. No multi-corner routes, no diagonals in v1 — routing is a pure function of the endpoints and their sides.
- **Routes are structural edges.** Every route cell stamps N/E/S/W connections through the same `CellState` mechanism as shape borders, so joins resolve through the C1 rule table: a line leaving a wall's midpoint renders as T-junction `+`, crossings merge cleanly. No connector cell is opaque content except the arrowhead.
- **Arrowhead at the target end.** The second-to-last route cell carries `<`/`>` (horizontal approach) or `v`/`^` (vertical), stamped as content over its edges — svgbob renders such a glyph as an arrow when a line connects to it, so text and SVG outputs agree. Omitted when endpoints are on the same or adjacent cells (degenerate routes draw nothing / stubs only).
- **Alphabet membership.** The arrows join the svgbob-alphabet check (`is_svgbob_glyph`), per ADR 001 v2's arrow set `< > ^ v V`; label lines containing them are quote-escaped like any other drawing glyph (C1 F4).
- **Scale expansion (FR8).** Arrowheads are content, so at scale ≥ 2 they land in their centered sub-cell while the line fills its whole block — frozen by the `connector-scale-2` golden.
- **Known v1 limitations** (ADR 013 consequences): no collision avoidance (routes may cross other objects' interiors as clean T-junctions); `ConnectorStyle::Dashed` renders solid (dash rendering is a follow-up decision).

## Relations

- **Render Pipeline (C2)** — routing is the connector branch of the place stage; its output feeds the same resolve stage as everything else (`doc/arc42/12-concepts/render-pipeline.md`).
- **Glyph Resolution (C1)** — route cells are structural edges resolved by the C1 rule table; clean joins come for free from F2 edge merging (`doc/arc42/12-concepts/glyph-resolution.md`).
- **Diagram Aggregate (C4)** — `ConnectorInstance` and `AnchorRef` live on the aggregate; connectors store only anchor references, so move/resize re-resolve them automatically (`doc/arc42/12-concepts/diagram-aggregate.md`).
- **Shape Template (C7)** — per-type anchor sets provide the template-space coordinates that routing maps to cells (`doc/arc42/12-concepts/shape-template.md`).

## Implementation

`crates/render-pipeline/src/connectors.rs` — `place_connector` (endpoint resolution + route stamping), `resolve_endpoint`, `route` (straight/L decision + corner choice), arrowhead placement; wired into the place stage via `crate::place::lookup_shape`. Typed failures: `RenderError::{UnknownAnchor, AnchorOutsideExtent}`. Pinned by unit tests in `connectors.rs`, five golden grids (`connector-horizontal/vertical/diagonal/mid-edge/scale-2`), and the extended proptest suite (random diagrams with connectors).
