---
title: change-request-board
description: persistent change-request board — open / closed; single source of truth for CR status (CRs live in doc/crs/CR-NNN-*.md)
scope: delivery
status: stable
updated: 2026-10-30
---

# Change Request Board

Filed via the **change-request skill** (`.agents/skills/change-request/SKILL.md`). One file per CR under `doc/crs/`; this board is the status source of truth. A CR stays Open while `proposed` (awaiting decision) and `accepted` (awaiting implementation); it closes as `implemented` or `rejected`.

## Open

| ID | Title | Priority | Status | File |
|----|-------|----------|--------|------|
| CR-001 | Native file dialogs for Open, Save and Export | medium | proposed | [CR-001-native-file-dialogs.md](CR-001-native-file-dialogs.md) |
| CR-002 | Grouped dropdown menus in the toolbar | low | proposed | [CR-002-grouped-dropdown-menus.md](CR-002-grouped-dropdown-menus.md) |

## Closed

| ID | Title | Closed | Resolution |
|----|--------|--------|------------|
| — | — | — | — |
