---
name: cr-implementation
description: Run the self-reliant implementation loop for an accepted change request (doc/crs/CR-NNN-*.md) — take the skeleton story created at acceptance, write its acceptance criteria from the CR's proposed requirement wording, implement with the standard quality gates and fresh-eyes review, then flip the CR to implemented and close it on the board. Use when asked to implement an accepted CR, "work CR-NNN", or after the change-request skill has recorded an acceptance.
metadata:
  scope: whole repository (closes files under doc/crs/)
  status: stable
  updated: 2026-10-30
---

# Change Request Implementation

Mission: take an **accepted** change request (`doc/crs/CR-NNN-<slug>.md`, `status: accepted`) to implemented — the skeleton story it triggered becomes a real, tested feature and the CR is closed. This is the handoff partner of the **change-request skill**: that one clarifies *what changes* (and applies the doc decision on acceptance); this one decides *how* and builds it.

## Ground rules

1. **AGENTS.md applies in full** — including security.
2. **The CR + accepted docs are the contract.** The requirement wording as it stands in §14 after acceptance is the spec; the ADR bump(s) recorded in the CR's Decision plan are binding decisions. If implementation shows the accepted wording is unbuildable, or a new decision is needed: stop — write a new ADR (`status: proposed`) or flag the CR for re-decision. Never silently deviate from the accepted text.
3. **This skill owns implementation scoping** — the part the change-request skill deliberately skipped: which crates/contexts per §8, approach, effort. Record it in the story's Implementation notes as you go.
4. **Docs are part of the change**: story, CR, boards, and any affected arc42/concept pages update in the *same commit* as the code.
5. **Mechanics follow the story-implementation skill** (session loop, quality gates, review cycle, interruption recovery) — this skill only changes where the spec comes from: a CR, not the §1.1/§8 breakdown.

## State files

| File | Role |
|---|---|
| `doc/crs/CR-NNN-<slug>.md` | The accepted CR — Requested change + Requirement impact are the spec; Implementation section closes it |
| `doc/stories/S-NNN-*.md` | The skeleton story created at acceptance (Why cites CR-NNN) |
| `doc/crs/CRS.md`, `doc/stories/BOARD.md` | Boards — status sources of truth |

## Workflow

1. **Load.** Read the CR in full; confirm `status: accepted`. Find its story (the one whose Why cites CR-NNN). Not accepted → stop: the change-request skill owns the decision phase.
2. **Write the ACs.** Turn the CR's proposed requirement wording into verifiable acceptance criteria on the skeleton story — a command, an inspectable artifact, or an observable user action per criterion; no "works correctly". GUI behavior decomposes into headless-testable pieces per the gui-testing skill. Scope check: if you are about to touch more than the CR's conflict check + impact cover, stop and flag it — scope creep needs a new CR.
3. **Work the story** with the story-implementation loop: mark in progress → implement in the correct bounded context (§8; rendering headless + unit-testable, GUI thin) → tests (regression/golden where deterministic output matters) → quality gates (`cargo fmt`, `clippy --workspace --all-targets` deny, `test --workspace`, `tarpaulin --engine Llvm --workspace` ≥80%; `mkdocs build` for doc parts) → verify every AC with recorded evidence → fresh-eyes review from disk.
4. **Close the CR in the story's commit**: CR frontmatter `status: implemented`, fill Implementation (S-NNN + commit hash), board row → Closed; flip any ADRs the CR introduced from `proposed` → `accepted` if fully implemented; story → Done on the board.
5. Commit: `S-NNN: <title>` — the CR close rides along in that same commit.

## Interruption recovery

Same as story-implementation (in-progress story at session start → finish or stash-discard with a labeled stash). Additionally: never leave an accepted CR with its story unstarted and the tree dirty without a board note saying what is pending.
