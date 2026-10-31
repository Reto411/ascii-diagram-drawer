---
title: bug-board
description: persistent bug board — open / closed; single source of truth for bug status (reports live in doc/bugs/B-NNN-*.md)
scope: delivery
status: stable
updated: 2026-10-31
---

# Bug Board

Filed via the **bug-report skill** (`.agents/skills/bug-report/SKILL.md`). One file per bug under `doc/bugs/`; this board is the status source of truth.

## Open

| ID | Title | Severity | Status | File |
|----|-------|----------|--------|------|
| — | (none) | | | |

## Closed

| ID | Title | Closed | Resolution |
|----|--------|--------|------------|
| [B-003](B-003-selection-outline-ignores-diagram-scale.md) | Selection outline ignores diagram scale | 2026-10-31 | fixed — `Viewport` and the canvas text-draw ignored the FR8 scale; the viewport is now scale-aware (model cells map to their rendered N×N blocks at `(col·N, row·N)`, hit-testing floor-divides by the scaled cell) and the expanded grid is anchored at the content origin's scaled position, so outline/anchors/hit-testing follow the drawn extent at every scale; pinned by Viewport unit tests + tier-0b regression test |
| [B-002](B-002-scale-duplicates-shape.md) | Scale duplicates shape | 2026-10-31 | as-designed — no duplication in any attached screenshot (each shows one exact FR8 expansion; last shot ×3, matching its toolbar + status bar); pipeline correct at HEAD; new tier-0b test pins single-copy rendering |
| [B-001](B-001-ellipse-resizing-does-not-work.md) | Ellipse resizing does not work | 2026-10-31 | fixed — the built-in ellipse was a `Pattern` stamped at natural size; it now has an `Ellipse` geometry kind and is re-rasterized deterministically at the instance pos/size (9×5 byte-identical to before), with proportional anchor mapping like `Rect` (ADR 013 v3); pinned by pipeline regression tests |
