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
| B-003 | Selection outline ignores diagram scale | major | reproduced | [B-003](B-003-selection-outline-ignores-diagram-scale.md) |

## Closed

| ID | Title | Closed | Resolution |
|----|--------|--------|------------|
| [B-002](B-002-scale-duplicates-shape.md) | Scale duplicates shape | 2026-10-31 | as-designed — no duplication in any attached screenshot (each shows one exact FR8 expansion; last shot ×3, matching its toolbar + status bar); pipeline correct at HEAD; new tier-0b test pins single-copy rendering |
| [B-001](B-001-ellipse-resizing-does-not-work.md) | Ellipse resizing does not work | 2026-10-31 | fixed — the built-in ellipse was a `Pattern` stamped at natural size; it now has an `Ellipse` geometry kind and is re-rasterized deterministically at the instance pos/size (9×5 byte-identical to before), with proportional anchor mapping like `Rect` (ADR 013 v3); pinned by pipeline regression tests |
