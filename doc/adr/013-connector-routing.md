---
title: 013 - connector-routing
description: connectors route orthogonally — straight when anchors share a row/column, otherwise a single-corner L — stamped as structural edges so they join borders through the C1 rule table; arrowhead glyph at the target end
scope: rendering
status: accepted
version: 6
updated: 2026-10-31
---

# ADR 013: Connector routing

## Context

Connectors are anchor-bound (FR9): a `ConnectorInstance` stores only `(object, anchor name)` references, and the render pipeline re-resolves both endpoints to cells at render time. The pipeline must decide **how** to draw the line between the two resolved cells. Constraints in play:

- The resolved grid uses svgbob's ASCII input alphabet ([ADR 001](001-glyph-resolution-alphabet.md)); SVG output is produced by passing that grid unchanged to `svgbob::to_svg` ([ADR 005](005-use-svgbob-as-vector-renderer.md)).
- Joins must be clean: no stray characters where a connector meets a shape border. The C1 rule table ([glyph resolution](../arc42/12-concepts/glyph-resolution.md)) already maps N/E/S/W edge sets to junction glyphs (`+` for T-junctions/corners) and reserves dangling single-direction stubs for connector endpoints.
- Rendering stays pure and deterministic (NFR3): the route must be a function of the two resolved endpoints alone — no search, no iteration order.
- The pipeline has no layout pass: objects land exactly where the model says; connectors are the first object kind whose geometry is *derived*.

## Decision

**Connectors route orthogonally, with at most one corner — unless the user has placed manual bends (v4); each end carries a selectable tip (v5); endpoints may be free positions instead of anchors (v6).**

1. **Straight or single-corner L (automatic routing).** If the two resolved endpoint cells share a row or a column, the route is the straight segment between them. Otherwise it is an L: two axis-aligned segments meeting in exactly one corner cell. No multi-corner (Z/U) routes and no diagonal segments in automatic routing.
2. **Deterministic corner choice.** The corner is chosen so that the *final* segment approaches the target anchor perpendicular to its side when possible: a target on a left/right side gets a horizontal final segment; a target on a top/bottom side gets a vertical one. If the target anchor has no side preference (corner/interior cell), the source's preference is used, then a fixed default (vertical-first corner `(from.x, to.y)`). The route is therefore a pure function of the two endpoints and their sides — no search.
3. **Manual bends override automatic routing (v4).** A connector may carry an ordered list of intermediate **waypoints** (cell coordinates in model space). When non-empty, the route is the polyline `[endpoint₁, waypoints…, endpoint₂]` and points 1–2 do not apply. Each segment stamps like automatic segments: axis-aligned segments are structural edges (clean C1 joins everywhere); segments whose endpoints share neither row nor column are straight diagonals stamped as `/` or `\` **content** — user-intentional routes may trade clean joins for flexibility, the same trade ADR 013 v1 made when rejecting diagonals in *automatic* routing. The GUI constrains bend editing so that axis-aligned segments stay axis-aligned; a diagonal is only produced if the user explicitly drags a bend off both its neighbors' row and column.
4. **Routes are structural edges.** Every route cell of an axis-aligned segment stamps N/E/S/W edge connections toward its route neighbors (the same `CellState` mechanism as shape borders). Connectors therefore join borders through the existing C1 rule table and F2 merge: a connector leaving a wall's midpoint renders as a T-junction `+`, an elbow landing on a wall corner merges into it, and a route crossing another border mid-edge produces a clean T-junction. No axis-aligned route cell is ever stamped as opaque content except tips (below).
5. **Selectable tips at both ends (v5).** Each endpoint carries a tip style: **none**, **hollow arrow** (`< > ^ v` — the existing glyph set, chosen by approach direction), **empty rhombus** `◇`, or **filled rhombus** `◆`. The tip is stamped at the cell adjacent to the endpoint (second-to-last route cell), as content, pointing along the final segment; omitted when there is no room. `◇`/`◆` are **content-only** glyphs — they never participate in edge resolution and are treated exactly like label characters (no ADR 001 alphabet change, no quote-escaping needed). Files without tip data render as before: source end none, target end hollow arrow — existing diagrams stay byte-identical.
6. **Anchor resolution per geometry.** `Rect` and `Ellipse` instances have live pos/size: a template anchor coordinate maps proportionally onto the instance extent per axis, rounded to nearest (ties up), so side-midpoint anchors stay at side midpoints at any instance size and natural-size instances map to themselves. `Pattern` instances (custom templates) stamp at natural size, so their anchors are the template coordinates offset by the instance position. An anchor name missing from the type's anchor set, or declared outside the template extent (which cannot resolve inside the shape's bounds — the §8.4.1 render-time invariant), is a typed `RenderError`.
7. **Free endpoints (v6).** An endpoint may be a free cell position in model space instead of an `(object, anchor)` reference. Free endpoints never re-resolve with any shape — they are fixed coordinates; the route simply starts/ends there. Attaching a free end to an anchor and detaching a bound end are GUI operations that convert between the two endpoint forms (the model accepts both). A connector may have zero, one, or two bound ends.
8. **Degenerate cases.** Coincident endpoints draw nothing; adjacent endpoints draw only the two stub edges (no tip). Both are deterministic and render-safe.
5. **Anchor resolution per geometry.** `Rect` and `Ellipse` instances have live pos/size: a template anchor coordinate maps proportionally onto the instance extent per axis, rounded to nearest (ties up), so side-midpoint anchors stay at side midpoints at any instance size and natural-size instances map to themselves. `Pattern` instances (custom templates) stamp at natural size, so their anchors are the template coordinates offset by the instance position. An anchor name missing from the type's anchor set, or declared outside the template extent (which cannot resolve inside the shape's bounds — the §8.4.1 render-time invariant), is a typed `RenderError`.
6. **Degenerate cases.** Coincident endpoints draw nothing; adjacent endpoints draw only the two stub edges (no arrowhead). Both are deterministic and render-safe.

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Straight lines only (error when anchors share no row/column) | Most common layouts (side-by-side boxes at different heights, stacked boxes offset horizontally) would be unconnectable; the error case would dominate over the success case. |
| Multi-corner Manhattan routing (Z/U routes with corner-set selection) | Needs a collision-avoidance policy (which of the two+ possible corner sets to pick when one crosses another object) — a second design decision without a v1 use case. Single-corner L covers anchored boxes in v1 and stays trivially deterministic; revisit if layouts demand it. |
| Diagonal segments (`/` `\`) | Diagonals do not map onto the N/E/S/W edge model, so they cannot join borders through the C1 rule table — they would have to be stamped as opaque content, producing exactly the stray-character joins this ADR exists to avoid. Orthogonal-only keeps every route cell structural. |
| Curved / bezier routing | svgbob's input alphabet has no curve glyphs (ADR 001); curves would have to be approximated with stair-step diagonals, losing the plain-ASCII character of the output and breaking clean joins for the same reason as the diagonal alternative. |

## Consequences

- **Positive:**
  - Routing is a pure function of resolved endpoints — determinism (NFR3) holds by construction; `move_object`/`resize` re-resolve anchors and the route follows with byte-identical output (S-007 AC3).
  - Clean joins for free: connectors reuse the C1 rule table, so every join cell is a standard junction glyph — no special-case merge code.
  - Arrowheads are inside svgbob's documented vocabulary; text and SVG outputs render them identically (arrow + connecting line).
- **Negative / costs:**
  - L routes can take long detours for strongly offset anchors, and the route has no collision avoidance — it may cross other objects' interiors (rendered as clean T-junctions over their borders). Acceptable for v1; a documented limitation.
  - `ConnectorStyle::Dashed` is **not honored in v1**: dashed connectors render solid. Dashes would require alternating content cells along the route, which breaks structural joins at crossings and interacts badly with scale expansion (content glyphs occupy one sub-cell). The style remains parsed/serialized by `ad-format`; rendering it is a follow-up decision (dash vocabulary per ADR 001: `- - -`, `~`, `_`, `:`, `!`).
  - Arrowhead glyphs `< > ^ v V` join the svgbob-alphabet membership check, so label lines containing them are quote-escaped per C1 F4 (e.g. a label `hive` renders as `"hive"`). Deterministic and correct per ADR 001's alphabet, at the cost of visible quotes on such labels.
  - Because tips are content (not line glyphs), scale expansion (FR8) places them in the centered sub-cell instead of filling their block: at scale ≥ 2 a horizontal route shows the tip on one row of the doubled line with a gap on the other. Deterministic and frozen by the `connector-scale-2` golden; revisit only if scaled connectors look wrong in practice.
  - Manual bends (v4): diagonal segments are opaque content — they do not join borders through the C1 rule table, so a user-placed diagonal touching a wall can leave stray characters. Documented limitation of manual routing; axis-aligned segments keep clean joins. Waypoint lists grow the connector's model + `.ad` statement (format extension lands with S-028).
  - Rhombus tips (v5): `◇`/`◆` are Unicode content glyphs — svgbob renders them as text in SVG output, so the vector tip is font-dependent (same as every label character). In terminals they display as diamond characters in any font that covers U+25C7/U+25C6.
  - Free endpoints (v6): a free end is inert under shape movement — exactly what the user asked for, but it means a "bound" look can silently become stale when shapes move; the GUI must make bound vs free ends visually distinguishable (S-031).
- **Follow-ups:**
  - Dashed rendering decision once S-008 lands SVG emission and dash behavior can be verified against `svgbob`.
  - Multi-corner *automatic* routing / collision avoidance if real layouts demand it (manual bends now cover the user-driven case).
  - ~~Arrowhead presence/shape as a `ConnectorStyle` extension in the `.ad` format~~ settled by v5 (per-end tip styles, S-029).

## Version history

> When a decision changes: bump `version` in the header, update Context/Decision to the *current* state, and add an entry here (newest first) recording what changed, why, and when. Never create a superseding ADR.

- **v6 (2026-10-31):** CR-010 accepted — new point 7: endpoints may be free model-space cell positions instead of `(object, anchor)` references; zero/one/two bound ends; attach/detach converts between the endpoint forms in the GUI. Implementation: S-031.
- **v5 (2026-10-31):** CR-008 accepted — point 4 replaced by point 5: both ends carry a selectable tip style (none / hollow arrow `< > ^ v` / empty rhombus `◇` / filled rhombus `◆`); rhombi are content-only glyphs (no ADR 001 bump — same treatment as label characters); files without tip data keep the old default (source none, target hollow arrow) so existing renders stay byte-identical. Implementation: S-029.
- **v4 (2026-10-31):** CR-007 accepted — new point 3: connectors may carry ordered manual waypoints; the route becomes the polyline through them, axis-aligned segments structural (clean joins), explicit diagonals stamped as `/` `\` content. Automatic routing unchanged for bend-free connectors. Implementation: S-028.
- **v3 (2026-10-31):** point 5 extended — the built-in ellipse gained an `Ellipse` geometry kind and is re-rasterized at its instance pos/size (B-001: resizing was a no-op because it stamped the natural 9×5 art); its anchors now map proportionally like `Rect`. Custom templates stay `Pattern` (natural size).
- **v2 (2026-10-28):** decision implemented — S-007 landed `crates/render-pipeline/src/connectors.rs` exactly as recorded: endpoint re-resolution from anchor references, straight/single-corner L with target-side corner preference, structural-edge stamping, lowercase arrowheads `< > ^ v` at the target end (uppercase `V` joins alphabet membership only, per ADR 001 v2), typed `RenderError::{UnknownAnchor, AnchorOutsideExtent}`. Five golden grids pin horizontal/vertical/L/mid-edge/scale-2 routing; status flipped to `accepted`.
- **v1 (2026-10-26):** initial decision — orthogonal straight/single-corner routing, structural-edge routes, arrowhead at the target end, proportional Rect anchor mapping; dashed style deferred.
