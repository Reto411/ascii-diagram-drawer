---
title: Arrow object behavior and Visio-style line handles
description: change request — connectors/arrows behave as normal objects (drag body moves the whole arrow), selected arrows show visible draggable handles at both endpoints and all bends, double-click on a segment inserts a bend, and an in-canvas dropdown selects the tip style per end; filed 2026-11-01
scope: change request — connector/arrow interaction model (FR9)
status: implemented
filed: 2026-11-01
priority: high
format-compat: none
---

# CR-020: Arrow object behavior and Visio-style line handles

## Requested change

Arrows (connectors) are still unintuitive: they do not behave like the shapes around them. They shall be viewed as **normal objects**, with **Visio-style handles** for the shape of the line:

1. **Move like an object.** Pressing a connector selects it; dragging its body moves the *whole* arrow (endpoints and bends translate together) — instead of today's "drag from body adds a bend". A bound end detaches while moving and re-attaches if dropped exactly on an anchor (same snap rule as endpoint drags, S-031).
2. **Visio line handles.** When selected, the arrow shows **visible square handles at both endpoints and at every bend**, all draggable — to reshape the line. Today only declared bends show a small handle; endpoints are draggable but invisible.
3. **Insert bends by double-click.** Double-clicking a segment inserts a new bend (waypoint) there (Visio behavior). Today the only way to add a bend is press-drag-release on the line body — which item 1 repurposes for moving.
4. **In-canvas tip dropdown.** Pressing onto a selected endpoint opens an in-canvas dropdown listing the four tip styles (none / hollow arrow / empty rhombus / filled rhombus) for that end; picking one applies it. This replaces the current press-to-cycle behavior. The trigger is a press, not right-click — right-click keeps its documented cancel/deselect role (S-013).
5. **Scope: both kinds** — shape-to-shape connectors and standalone arrows.

## Why

Improve user experience. Arrows are currently the least intuitive objects on the canvas: they cannot be moved like shapes, their endpoints give no visible affordance, adding bends is a hidden gesture, and tip selection is an undocumented press-cycle. The user has flagged arrow behavior repeatedly (CR-007/CR-008/CR-010 implemented routing, tips and free ends — the interaction layer still lags behind).

## Current requirement wording (§14)

> **FR9** — "Connectors (lines/arrows) can be drawn between/around shapes or standalone with freely positioned ends; each end is either bound to a shape anchor — staying attached when the bound shape moves or resizes — or free at an explicit position, and ends can be attached to and detached from anchors interactively; the user can modify a connector's shape — pressing onto it selects it, and dragging adds/moves intermediate bends (waypoints) so arrows can be routed manually for readability; connectors without manual bends keep the automatic routing; each end of a connector can carry a selectable tip style — no tip, hollow arrow, empty rhombus, filled rhombus — changed by pressing onto the tip; connection points are shown in the GUI as small arrow symbols to indicate that connecting adds an arrow"

## Proposed requirement wording (§14)

> **FR9** — "Connectors (lines/arrows) can be drawn between/around shapes or standalone with freely positioned ends; each end is either bound to a shape anchor — staying attached when the bound shape moves or resizes — or free at an explicit position, and ends can be attached to and detached from anchors interactively; connectors behave as normal objects: pressing onto one selects it, and dragging its body moves the whole connector — endpoints and bends translate together, a bound end detaches while moving and re-attaches if dropped exactly on an anchor; when selected, both endpoints and all bends show visible square handles that can be dragged to reshape the line, and double-clicking a segment inserts a new bend (waypoint) so arrows can be routed manually for readability; connectors without manual bends keep the automatic routing; each end of a connector can carry a selectable tip style — no tip, hollow arrow, empty rhombus, filled rhombus — chosen from an in-canvas dropdown opened by pressing onto that endpoint; connection points are shown in the GUI as small arrow symbols to indicate that connecting adds an arrow"

## Conflict check

- **ADR 013 (connector routing, v6) — no conflict.** Routing stays a pure function of the two resolved endpoints: moving an arrow changes endpoint/bend positions through existing model operations (`set_endpoint`, bend ops); the automatic L vs. waypoint-polyline decision is untouched. No ADR version bump needed.
- **S-028 (manual bends, done) — gesture rework.** AC4/AC5's "drag from body adds a bend at drop" is replaced by whole-arrow move; the double-click insertion takes over bend creation. Bend-handle dragging and the axis-aligned-segment constraint stay as implemented.
- **S-029 (selectable tips, done) — interaction rework.** Press-without-move on an endpoint currently cycles the tip (`editor.rs::cycle_tip`); it becomes the dropdown trigger with explicit selection. Tip styles and their rendering are unchanged.
- **S-031 (free endpoints, done) — extension only.** Endpoint drag/detach/reattach semantics stay; the change adds a visible handle to the existing gesture.
- **S-013 (editing gestures, done) — untouched.** Right-click keeps its cancel-priority role, so no conflict with the new dropdown trigger.
- **Concept C9 (connector-routing)** — its "GUI:" paragraph documents the old gestures; it is updated on acceptance together with FR9.

## Impact analysis

**Affected stories:** S-013 (done — selection/cancel foundation), S-028 (done — body-drag gesture rework + double-click insertion), S-029 (done — press-to-cycle → dropdown), S-031 (done — visible endpoint handles). All done → **rework**; no in-progress story is touched.

**Format compatibility:** none — GUI-only change. Every model operation needed already exists and is serialized (bends, free/attached endpoints, tip styles — S-028/S-029/S-031). ADR 004 / NFR2 are not involved; `.ad` files are unaffected.

## Decision plan

On acceptance: update §14 FR9 to the proposed wording (this CR keeps the verbatim copy); update concept C9's GUI paragraph; create a skeleton story on the board (**S-041** — S-040 is already planned for CR-019) with this Why, depending on S-028/S-029/S-031 (all done); no ADR changes.

## Decision

- [x] **Accept** — proceed as proposed
- [ ] **Accept with adjustments** — <what changes>
- [ ] **Reject** — <reason>

Decided: accepted (2026-11-01) — blanket acceptance by the user: "Fix all bugs, implement all change requests and do extensive regression testing including End to end tests". Applied on acceptance: §14 FR9 updated, concept C9 GUI paragraph updated, skeleton story S-041 on the board. ADR 013 untouched (routing stays a pure function of endpoint/bend positions — no version bump).

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

Landed as **S-041**, commit `55abd09` (2026-11-02), all in `gui-app` (no model/format changes — every needed operation pre-existed):

1. **Move like an object** — `editor.rs` gained `Gesture::ConnectorMove`: pressing a body cell selects the connector and snapshots press-time endpoints + bends; dragging translates *both* ends and every bend by one delta, a bound end detaching in place (free at its anchor cell) on first displacement; release reports `ConnectorMoved`. Re-attach lives in `app.rs::reattach_moved_ends` — after a move, each free end dropped exactly on an anchor is re-attached. The old S-028 “drag from body adds a bend” gesture is gone.
2. **Visio line handles** — `canvas.rs::overlays` now emits square `EndpointHandle`s at both resolved endpoints and `BendHandle`s at every declared bend of each selected connector; `hit_target` exposes the endpoint cells (plus the visible tip glyph cell when non-bare) above bodies, so the drawn marker and the gesture target coincide. Bend handles keep S-028's axis rule + collinear-drop removal.
3. **Double-click inserts a bend** — `on_double_click` routes `HitTarget::ConnectorBody` to `diagram.add_bend(id, segment, cell)` at the clicked cell (`BendAdded`, connector selected); double-click on a bend/endpoint/empty space is a no-op.
4. **In-canvas tip dropdown** — press-without-move on an endpoint opens an egui area anchored at that endpoint listing the four tip styles; picking one applies `set_tip` to that end only and closes. Right-click/Escape close without change (S-013 cancel role kept); any other canvas press closes the menu first. Press-to-cycle is replaced entirely — `cycle_tip` no longer exists in the tree.
5. **Both kinds** — every gesture is endpoint-model-driven, so shape-to-shape connectors and standalone arrows (S-031) share it unchanged.

Feature bar: `ui_decomposition_arrow_handles` builds two boxes + a connector + a standalone arrow purely through the real UI, then drives body-drag, double-click insertion, handle drags and the dropdown through it. 10 new tests (6 editor + 3 tier-0b + 1 app) + harness helpers `settle(n)`/`right_click_at`; WYSIWYG intact — handles are overlay-only, all goldens byte-identical. Gates: workspace fmt/clippy(0)/test green (700 passed); tarpaulin **93.42%** ≥ 80%; mkdocs strict clean.
