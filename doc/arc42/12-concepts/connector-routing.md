---
title: connector-routing
description: how connectors are resolved to cells and routed in the render pipeline — anchor-bound or free endpoints, automatic straight/single-corner L or manual waypoint polylines, structural-edge joins, selectable per-end tips; read when touching connector rendering
scope: rendering
status: stable
updated: 2026-10-31
---

# Concept: Connector Routing

## Definition

The stage of the render pipeline (C2) that turns each `ConnectorInstance` into a routed line on the resolved grid. Each endpoint is either an `(object, anchor)` reference or a free cell position; the route is the automatic straight segment / single-corner L of structural edges — or, when the user has placed manual bends, the waypoint polyline through them — with a selectable tip (none / hollow arrow / empty rhombus / filled rhombus) at each end.

## Why it exists

FR9 requires connectors bound to object anchors that re-resolve when shapes move or resize, so text and vector output stay in sync. Because the pipeline has no layout pass and must be pure (NFR3), the route can only be a function of the two resolved endpoint cells — which makes the routing strategy an explicit design decision ([ADR 013](../../adr/013-connector-routing.md)).

## How it works / behaves

- **Endpoints re-resolve on every render.** A bound endpoint is an `(object, anchor)` reference; a free endpoint is a fixed model-space cell position (CR-010, S-031 — attach/detach in the GUI converts between the two forms). Each *bound* reference is looked up in the endpoint shape's type (catalog), then mapped to a grid cell:
  - `Rect` and `Ellipse` instances (live pos/size): template coordinates map proportionally per axis, rounded to nearest (ties up) — side-midpoint anchors stay at side midpoints at any size.
  - `Pattern` instances (custom templates, natural size): template coordinates offset by the instance position.
  - Missing anchor names or anchors declared outside the template extent are typed `RenderError`s (the §8.4.1 render-time invariant).
- **Straight or single-corner L (ADR 013) — automatic routing.** Shared row/column → straight segment. Otherwise one corner cell: the final segment approaches the target perpendicular to its side when possible; otherwise the source's preference, then a fixed default. No multi-corner routes, no diagonals in *automatic* routing — it is a pure function of the endpoints and their sides.
- **Manual bends (CR-007, S-028).** A connector may carry an ordered list of intermediate waypoints (model-space cells). When non-empty the route is the polyline `[end₁, bends…, end₂]` instead of the automatic L. Axis-aligned segments stamp structural edges exactly as before; a segment whose endpoints share neither row nor column is a straight diagonal stamped as `/` or `\` **content** (user-intentional routes may trade clean joins for flexibility — ADR 013 v4). The GUI constrains bend drags so diagonals only appear when explicitly placed.
- **Routes are structural edges.** Every axis-aligned route cell stamps N/E/S/W connections through the same `CellState` mechanism as shape borders, so joins resolve through the C1 rule table: a line leaving a wall's midpoint renders as T-junction `+`, crossings merge cleanly. No axis-aligned route cell is opaque content except tips.
- **Selectable tips at both ends (CR-008, S-029).** Each end carries a tip style: none, hollow arrow (`<`/`>` horizontal, `v`/`^` vertical approach — the existing glyph set), empty rhombus `◇`, or filled rhombus `◆`. Stamped at the second-to-last route cell as content, pointing along the final segment; omitted when endpoints are on the same or adjacent cells. `◇`/`◆` are content-only glyphs (treated like label characters — no ADR 001 alphabet change). Files without tip data keep the legacy default: source none, target hollow arrow.
- **GUI anchor markers (CR-008).** Connection points are shown in the GUI as small **arrow symbols** instead of plain dots, signaling that connecting adds an arrow; bound vs free ends must stay visually distinguishable (S-029/S-031).
- **Alphabet membership.** The arrow tips join the svgbob-alphabet check (`is_svgbob_glyph`), per ADR 001 v2's arrow set `< > ^ v V`; label lines containing them are quote-escaped like any other drawing glyph (C1 F4). `◇`/`◆` deliberately do **not** join that check — they are content, never escaped.
- **Scale expansion (FR8).** Tips are content, so at scale ≥ 2 they land in their centered sub-cell while the line fills its whole block — frozen by the `connector-scale-2` golden.
- **Known v1 limitations** (ADR 013 consequences): no collision avoidance (automatic routes may cross other objects' interiors as clean T-junctions); diagonal manual-bend segments don't join borders cleanly (opaque content); `ConnectorStyle::Dashed` renders solid (dash rendering is a follow-up decision).

## Relations

- **Render Pipeline (C2)** — routing is the connector branch of the place stage; its output feeds the same resolve stage as everything else (`doc/arc42/12-concepts/render-pipeline.md`).
- **Glyph Resolution (C1)** — route cells are structural edges resolved by the C1 rule table; clean joins come for free from F2 edge merging (`doc/arc42/12-concepts/glyph-resolution.md`).
- **Diagram Aggregate (C4)** — `ConnectorInstance` and `AnchorRef` live on the aggregate; connectors store only anchor references, so move/resize re-resolve them automatically (`doc/arc42/12-concepts/diagram-aggregate.md`).
- **Shape Template (C7)** — per-type anchor sets provide the template-space coordinates that routing maps to cells (`doc/arc42/12-concepts/shape-template.md`).

## Implementation

`crates/render-pipeline/src/connectors.rs` — `place_connector` (endpoint resolution + route stamping), `resolve_endpoint`, `route` (straight/L decision + corner choice), arrowhead placement; wired into the place stage via `crate::place::lookup_shape`. Typed failures: `RenderError::{UnknownAnchor, AnchorOutsideExtent}`. Pinned by unit tests in `connectors.rs`, five golden grids (`connector-horizontal/vertical/diagonal/mid-edge/scale-2`), and the extended proptest suite (random diagrams with connectors).

Pending (CR batch accepted 2026-10-31): manual waypoints + polyline stamping (S-028), per-end tip styles incl. `◇`/`◆` content glyphs (S-029), free endpoints (S-031) — each updates this section as it lands.
