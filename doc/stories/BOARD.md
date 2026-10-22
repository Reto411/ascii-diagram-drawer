---
title: story-board
description: implementation board for ascii-diagram-drawer — story statuses + progress log; single source of truth for what is done, in flight, and next; any agent session resumes from here
scope: project delivery
status: active
updated: 2026-10-22
---

# Story Board

> **How to work this board:** follow the story-implementation skill (`.agents/skills/story-implementation/SKILL.md`). The board + story files are the only persistent state — any session can resume from disk alone.

**Overall status:** ⬜ breakdown pending → 🟨 in progress → ✅ complete

## Backlog

| ID | Title | Scope | Deps | Priority |
|----|-------|-------|------|----------|

*(empty — run the Breakdown phase of the story-implementation skill to populate)*

## In progress

| ID | Title | Started | Note |
|----|-------|---------|------|

## Blocked

| ID | Title | Reason | Unblocks when |
|----|-------|--------|---------------|

## Done

| ID | Title | Completed | Commit |
|----|-------|-----------|--------|

## Progress log

> Newest first. One entry per story completion, block, recovery, or significant state change. Facts only: what landed, gate results, follow-ups.

### 2026-10-22 — Board created

- Story board + story template added; story-implementation agent (skill) in place.
- Breakdown phase pending — first run of the skill populates S-001… from the arc42 docs, ADRs, and concepts.
