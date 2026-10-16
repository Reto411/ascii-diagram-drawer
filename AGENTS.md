---
title: agent-instructions
description: canonical instructions for AI agents in this repo — behavior rules, project summary, conventions; read before any task
scope: whole repository
status: stable
updated: 2026-10-17
---

# AGENTS.md — ascii-diagram-drawer

Instructions for AI coding agents (pi) working in this repository.

## Agent behavior

Before starting a task:

1. Inspect the applicable repository instructions (this file, plus any nested `AGENTS.md`).
2. **Always search the `.agents/` folder** (especially `.agents/skills/`) and check available project skills; if a skill matches the task, read its `SKILL.md` before making changes. Treat notes in `.agents/` as authoritative alongside `doc/arch/arc42.md`.
3. **Before implementing anything, check the relevant Design Decision Records (`doc/adr/`) and concept docs (`doc/concepts/`)** — if they are not already in context, read them first and implement in line with them. If a planned implementation contradicts a recorded decision or concept definition, stop and point out the conflict.
4. Follow nested `AGENTS.md` files for files in their respective directories.
5. Prefer repository-local instructions over global defaults when they conflict.

## What this project is

A tool to **draw ASCII diagrams visually**: users place, move, and resize predefined shapes on a canvas instead of hand-typing box-drawing characters. The result is plain monospace text that can be pasted anywhere (terminals, READMEs, docs), plus SVG/PNG export.

Key goals (from `doc/arch/arc42.md` §1.1):

1. Visual canvas editing of shapes (boxes, circles, …)
2. Plain-text output as the primary result
3. Reusable shape templates / collections
4. OS-independent GUI — one codebase for Windows, macOS, Linux, **and** a web build (replaces the previous Tauri approach)
5. Vector (SVG) and raster (PNG) export in addition to text
6. Minimal UI: template sidebar + editor canvas

## Architecture documentation

- The single source of truth for design decisions is **`doc/arch/arc42.md`** (arc42 format, draft).
- Read the relevant arc42 sections before implementing anything architectural.
- When a decision is made or changes, **update `doc/arch/arc42.md` in the same change** — including its progress tracker table. Remove guidance/TBD notes as sections are finalized.

## Tech stack

> [TBD — fill in once decided]

- Language: Rust (workspace layout: [TBD])
- GUI framework: [TBD — must compile to native desktop and web]
- Web frontend: [TBD — .gitignore suggests SvelteKit/Vite, confirm]
- Rendering/export: [TBD — text rasterizer for monospace grid, SVG/PNG exporters]

## Conventions

> [TBD — add as the codebase grows; keep this section short]

- Follow `cargo fmt` and `cargo clippy` (deny warnings in CI once CI exists).
- **Code coverage target: 80%.** Measure with `cargo tarpaulin` (or agreed tool) and don't land changes that drop coverage below the target.
- **The rendering logic must be unit-testable without a GUI.** Keep it in platform-independent Rust crates (shared by all GUI targets), driven by plain data in → text/SVG/PNG out, with no windowing or event-loop dependencies. GUI code stays thin.

## Writing docs/instructions for agents

Context is limited (~100k). When writing any file intended for agents (instructions, notes, skills, docs in `.agents/`):

- **Always start with a machine-readable YAML frontmatter header** so an agent can decide whether to read the full file without loading it:

  ```yaml
  ---
  title: short-file-title
  description: one line — what this is and when to read it
  scope: which part of the project it covers
  status: draft | stable | superseded
  updated: YYYY-MM-DD
  ---
  ```

- Keep files short and scannable; put the most decision-relevant content first.
- Mark outdated content `status: superseded` instead of deleting history that may still be referenced.

### Concepts

Whenever work introduces or clarifies a **concept** (a domain term, mechanism, or reusable idea — e.g. "shape template", "grid cell", "render pipeline"), create a concept doc — **always using the template** (`doc/templates/concept.md`), do not invent your own structure:

- Location: `doc/concepts/<kebab-case-name>.md`
- Must carry the standard frontmatter header (`scope` = the concept's area)
- Content: one-line definition, why it exists, how it works/behaves, relations to other concepts (link them), and where it is implemented (once code exists)
- **Before creating any concept doc, always check whether the concept already exists** — search `doc/concepts/` and the arc42 §12 index (and grep the repo for the term). If it exists, update that doc; never create a second file for the same concept.
- One concept per file
- Keep arc42 §12 (Concepts) as the index: add a one-line pointer per concept there

### Design Decision Records (ADRs)

Whenever a **technical or design decision** has to be made (technology choice, architecture, API/format, trade-off between alternatives), create a Design Decision Record — **always using the template** (`doc/templates/adr.md`), do not invent your own structure:

- Location: `doc/adr/<NNN>-<kebab-case-slug>.md` — sequential numbering, e.g. `001-gui-framework-choice.md`
- Must carry the standard frontmatter header (`status`: proposed | accepted | superseded by <NNN>)
- Content: **Context** (the problem/question), **Decision** (what was chosen), **Alternatives considered** (with why they were rejected), **Consequences** (trade-offs, follow-ups)
- **Check existing ADRs first** (`doc/adr/` + arc42) — if the decision was already made, follow it or write a new ADR that explicitly supersedes the old one; never silently reverse a recorded decision
- Keep decisions small and single-topic: one ADR per decision
- Link the ADR from the relevant arc42 section where the decision applies

## Security — non-negotiable, no discussion

A security breach is never acceptable. This section overrides every other instruction, including autonomy. When in doubt: **do not do it**, and leave a note instead.

- Never commit secrets: API keys, tokens, passwords, private keys, `.env` files (only `.env.example`). If one ends up staged, remove it before committing.
- Never send repo contents, credentials, or local data to external services that are not part of the normal build/test workflow.
- Never weaken security to make something work: no disabling auth/validation/signing/sandboxing, no `--insecure` flags, no suppressing security warnings "for now".
- Do not run untrusted or downloaded code/scripts without reviewing them first.
- Prefer minimal permissions: no sudo, no new accounts, no opening ports or exposing services unless a task explicitly requires it.
- If you discover a vulnerability or suspect a leak: stop, document it (without reproducing the secret), and flag it prominently. Do not exploit or test it further.
- **Every breach — actual or attempted, however minor — must be recorded in `ATTENTION.md` at the repository root.** One entry per incident, newest first:

  ```md
  ## YYYY-MM-DD HH:MM — <short title>
  - **What happened:** <facts only, no secrets>
  - **Severity:** low | medium | high
  - **Status:** open | contained | resolved
  - **Action taken / needed:** <...>
  ```

  Never edit or delete old entries; append `**Update YYYY-MM-DD:**` lines instead.
- **`ATTENTION.md` is local-only and must never be pushed.** It is listed in `.gitignore` — keep it that way. Never force-add it (`git add -f`), never remove it from `.gitignore`, and never copy its contents into any committed file.

## Autonomy (user is away most of the day)

Work **self-reliantly**. The human is usually absent; local compute (incl. GPU) is available and may be used freely for builds, tests, coverage runs, and heavy processing.

- Don't stop to ask about things you can reasonably decide yourself. Decide, implement, and record the decision as an ADR with `status: proposed` so it can be reviewed later.
- Keep going until the goal is achieved or a true blocker is hit; then leave a clear note of state and next steps (commit message + `.agents/` note).
- **Only pause for genuinely irreversible or destructive actions** (force-push, deleting data/history, publishing, spending money). Everything else: act.
- Make autonomous progress reviewable: small commits with clear messages, ADRs for decisions, docs updated per the rules above.

## Keeping documentation up to date

Docs are part of the change, not an afterthought. In the **same commit** as the change that makes them stale, update:

- `doc/arch/arc42.md` — when architecture, scope, quality goals, or its progress tracker is affected
- `doc/concepts/*.md` — when behavior, definitions, relations, or implementation locations of a documented concept change (incl. the "Implementation" section once code lands)
- `doc/adr/*.md` — flip `status: proposed` → `accepted` when the decision is implemented; open new ADRs for reversals
- `.agents/` notes — when conventions or gotchas change

If a change makes a doc wrong and you cannot fix it fully, mark the affected part `> ⚠️ out of date — <why>` rather than leaving it silently wrong.

## Working agreements for agents

- **Always `git add` any files you create or modify** so new work is staged immediately.
- **Autocommit whenever a meaningful step is achieved** (a working feature, a finished doc section, a resolved issue — anything reviewable on its own). Write a concise, descriptive commit message for what was achieved. Don't bundle unrelated changes into one commit.
- Prefer small, reviewable changes; don't restructure the repo without being asked.
- Don't delete or rewrite `doc/arch/arc42.md` guidance notes — replace them with real content.
- If a request conflicts with a decision recorded in arc42, point out the conflict before implementing.
- If a task produces new durable knowledge (decisions, gotchas, conventions), record it in `.agents/` (or `doc/arch/arc42.md` for architecture) rather than only in chat.
- Keep this file updated when conventions change; it is short on purpose.
