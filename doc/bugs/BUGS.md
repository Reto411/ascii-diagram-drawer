---
title: bug-board
description: persistent bug board — open / closed; single source of truth for bug status (reports live in doc/bugs/B-NNN-*.md)
scope: delivery
status: stable
updated: 2026-11-01
---

# Bug Board

Filed via the **bug-report skill** (`.agents/skills/bug-report/SKILL.md`). One file per bug under `doc/bugs/`; this board is the status source of truth.

## Open

| ID | Title | Severity | Status | File |
|----|-------|----------|--------|------|

## Closed

| ID | Title | Closed | Resolution |
|----|--------|--------|------------|
| [B-007](B-007-unknown-statement-trim-breaks-roundtrip.md) | Preserved unknown statements trimmed on save — round-trip broken | 2026-11-01 | fixed — `serialize_document` emitted `raw_line.trim()`, contradicting the documented verbatim/byte-for-byte contract for unknown statements; now re-emits the preserved line exactly as stored, so `parse(serialize(d)) == d` holds for forward-compatible files with leading/trailing whitespace on unknown lines (found by S-043's new `parse_ad` fuzz target in its first 10 s; pinned by a byte-exact serializer unit test + the fuzz QS7 invariant) |
| [B-006](B-006-ad-forward-reference-lossiness.md) | `.ad` forward references break round-tripping | 2026-11-01 | fixed — the streaming parser resolved endpoint ids only against earlier statements, but the canonical serializer emits in stored order (a connector may legally precede its target shape); `parse_document` is now two-phase (buffer all statements, install via the new trusted `Diagram::from_parts`) so forward references are legal and `parse(serialize(d)) == d` again; deferred model errors map back to their statement for positioned parse errors (found by S-043's operation-sequence round-trip property) |
| [B-005](B-005-set-endpoint-mutates-on-rejection.md) | `set_endpoint` leaves partial mutation behind when it rejects | 2026-11-01 | fixed — mutate-then-validate ordering left a zero-length connector in the diagram on rejection; `set_endpoint` now validates the prospective endpoints before mutating, so every rejected op leaves the aggregate unchanged (found by S-043's new operation-sequence properties; pinned by strengthened unit test + invariant/unchanged-state properties) |
| [B-004](B-004-window-edge-resize-not-working.md) | Window cannot be resized from its edges or corners on native | 2026-11-01 | fixed — the grips' one-shot `BeginResize(dir)` delegated resizing to the compositor (winit → xdg-toplevel `resize`), which did not work on KDE Plasma/KWin; the app now owns the gesture client-side: while a grip is held it queues `InnerSize` per frame (pointer-driven, clamped to the minimum) plus an `OuterPosition` re-anchor for west/north directions so the opposite edge stays fixed (ADR 017 v3); pinned by `resize_step` unit tests + tier-0b hover/drag/anchor tests; manual re-check on KWin recommended || [B-003](B-003-selection-outline-ignores-diagram-scale.md) | Selection outline ignores diagram scale | 2026-10-31 | fixed — `Viewport` and the canvas text-draw ignored the FR8 scale; the viewport is now scale-aware (model cells map to their rendered N×N blocks at `(col·N, row·N)`, hit-testing floor-divides by the scaled cell) and the expanded grid is anchored at the content origin's scaled position, so outline/anchors/hit-testing follow the drawn extent at every scale; pinned by Viewport unit tests + tier-0b regression test |
| [B-002](B-002-scale-duplicates-shape.md) | Scale duplicates shape | 2026-10-31 | as-designed — no duplication in any attached screenshot (each shows one exact FR8 expansion; last shot ×3, matching its toolbar + status bar); pipeline correct at HEAD; new tier-0b test pins single-copy rendering |
| [B-001](B-001-ellipse-resizing-does-not-work.md) | Ellipse resizing does not work | 2026-10-31 | fixed — the built-in ellipse was a `Pattern` stamped at natural size; it now has an `Ellipse` geometry kind and is re-rasterized deterministically at the instance pos/size (9×5 byte-identical to before), with proportional anchor mapping like `Rect` (ADR 013 v3); pinned by pipeline regression tests |
