---
title: CR-010 - standalone arrows with attachable/detachable ends
description: arrows that need not be bound to shapes — each end either attached to a shape anchor or free at an explicit position, attachable and detachable interactively
scope: model + format + render pipeline + GUI
status: implemented
priority: medium
updated: 2026-10-31
---

# CR-010: Standalone arrows with attachable/detachable ends

## Requested change

It shall be possible to add **standalone "arrows"** — arrows whose ends are not bound to any shape — for more advanced diagrams. If one end of a line is attached to an anchor point, it shall **attach to that anchor** (follow the shape when it moves or resizes). Every end must be **detachable as well**, so it can be placed freely again.

## Motivation

User: "this is needed" for more advanced diagrams — annotation arrows, flows between regions, arrows pointing at nothing; today every connector is strictly shape-to-shape (`AnchorRef` on both ends), which cannot express that.

## Requirement impact

| Req | Current text (§14) | Proposed text |
|-----|--------------------|---------------|
| FR9 | "Connectors (lines/arrows) can be drawn between/around shapes; they bind to shape anchors and stay attached when the bound shape moves or resizes" | "Connectors (lines/arrows) can be drawn between/around shapes **or standalone with freely positioned ends**; each end is either bound to a shape anchor — staying attached when the bound shape moves or resizes — **or free at an explicit position; ends can be attached to and detached from anchors interactively**" |
| FR11 | "Every object type exposes anchor points for connectors (FR9), per type …" | unchanged — anchors stay per-type; a free end simply uses none |

## Conflict check

- **ADR 013 v2 (accepted):** both connector endpoints are `AnchorRef(object, anchor)` and the route is a pure function of the two resolved endpoint cells. The endpoint model extends to `Attached(AnchorRef) | Free(position)` → **version bump**. Routing generalizes unchanged: a free end resolves to its stored position, so the route stays a pure function of two cells — NFR3 holds, no search introduced.
- **ADR 004 / NFR2:** `.ad` connector entries gain per-endpoint encoding (attached `shape@anchor` vs free `x,y`) → format extension; existing files (both ends attached) stay valid and render identically.
- **CR-007 (manual bends):** orthogonal — standalone arrows can carry manual bends like any connector.
- **CR-008 (tips + anchor markers):** no conflict; the arrow-shaped anchor markers become the snap targets for attaching, and tip selection applies to both ends regardless of attach state.

## Impact analysis

- **Stories:** `diagram-model` (endpoint sum type), `ad-format` (per-endpoint encoding, round-trip), `render-pipeline` (resolve free ends; routing logic unchanged), `gui-app` (new "Add arrow" action → drag to create a standalone arrow; endpoint-drag gesture: near an anchor the end snaps/attaches, dragging a bound end away detaches it).
- **Assumptions (confirm at acceptance):** free ends stored in grid coordinates in diagram space; attach = drag an end onto/near an anchor; detach = drag a bound end off its shape → becomes free at the drop position; zero-length arrows rejected (minimum 1 cell); deleting a shape keeps today's behavior for connectors bound to it.
- **Format compatibility:** **yes** — `.ad` extension (per-endpoint encoding); ADR 004 round-trip / NFR2 at stake; old files stay valid.

## Decision plan

> What acceptance triggers, in one commit: §14 edit(s), ADR bump(s)/new ADR, concept/arc42 updates, skeleton story S-NNN (ACs written at implementation).

- §14: reword **FR9** with the proposed text.
- **ADR 013 version bump** (endpoint model `Attached | Free`) — sequential after the CR-007/CR-008 bumps; no new ADR (extension of the existing routing decision, purity argument unchanged).
- One line each: `connector-routing` concept (free endpoints) and `diagram-file-format` concept (per-endpoint encoding).
- Skeleton story **S-029** on `doc/stories/BOARD.md`, Why cites CR-010; no hard dependencies (the snap UX pairs with CR-008's anchor markers when that lands).

## Decision

> Human decision only — the agent never fills this without explicit approval ("Accept CR-NNN as written?").

- **Status:** accepted
- **Date:** 2026-10-31
- **Reason / notes:** Bulk acceptance per user directive (2026-10-31 session: implement all change requests). Story **S-031** — the decision plan's provisional story number collided with the existing backlog (S-019–S-021 already allocated); the final allocation for the whole batch is recorded on the board. ADR 013 → v6 (free endpoints).

## Implementation

> Filled when the story implementing it lands: S-NNN + commit hash; CR → implemented, board row → Closed. Empty until then.

- **S-031** (commit `d9ad17c`): `ConnectorEndpoint { Attached(AnchorRef), Free(CellPos) }` on both ends of every connector — zero/one/two bound ends legal, both-free at the same cell rejected by the aggregate (`ModelError`, the “minimum 1 cell” assumption); `Diagram::set_endpoint(id, end, endpoint)` converts one end between the forms with full re-validation (missing object / non-shape / zero-length result all typed errors). `.ad` `connect` accepts either form per end: attached `<obj>.<anchor>` unchanged, free as two bare integers `x y` — disambiguated by token shape (dot ⇒ attached), no keyword added; positioned parse errors; byte-stable round-trip; every pre-existing golden stayed byte-identical. Pipeline resolves a free end to its stored cell with no side preference (`side: None`) — routing, tips and bends otherwise unchanged (ADR 013 v6); new goldens `standalone-arrow.grid` + `mixed-endpoints.grid`; proptest suite extended so purity/determinism cover unattached endpoints. GUI: new “↘ Arrow” sidebar tool — press-drag-release creates an arrow whose ends are attached when exactly on a shape anchor, else free (press without movement draws nothing; zero-length refused); endpoint drag — `HitTarget::Endpoint` hits endpoint/tip cells below selected-shape anchors/resize handle and above bodies (declared bends keep priority), press without moving still cycles the tip style (S-029 preserved), a free end moves live (`set_endpoint(Free)`), an attached end detaches the moment it leaves its anchor, releasing over an anchor re-attaches (snaps); while drawing/dragging every shape's anchors are exposed as snap targets (`AnchorExposure::All` — same mechanism as the connect drag). ADR 013 v6.
