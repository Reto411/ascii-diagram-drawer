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
| B-001 | Ellipse resizing does not work | major | reproduced | [B-001](B-001-ellipse-resizing-does-not-work.md) |
| B-003 | Selection outline ignores diagram scale | major | reproduced | [B-003](B-003-selection-outline-ignores-diagram-scale.md) |

## Closed

| ID | Title | Closed | Resolution |
|----|--------|--------|------------|
| [B-002](B-002-scale-duplicates-shape.md) | Scale duplicates shape | 2026-10-31 | as-designed — no duplication in any attached screenshot (each shows one exact FR8 expansion; last shot ×3, matching its toolbar + status bar); pipeline correct at HEAD; new tier-0b test pins single-copy rendering |
