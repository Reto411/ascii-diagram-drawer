---
title: agent-instructions
description: canonical instructions for AI agents in this repo — behavior rules, project summary, conventions; read before any task
scope: whole repository
status: stable
updated: 2026-10-18
---

# AGENTS.md — ascii-diagram-drawer

Instructions for AI coding agents (pi) working in this repository.

## Agent behavior

Before starting a task:

1. Inspect the applicable repository instructions (this file, plus any nested `AGENTS.md`).
2. **Always read the objectives file** — `doc/arc42/01-introduction-and-context.md` (§1.1). It defines what we are building; every task should be checkable against it.
3. **Always search the `.agents/` folder** (especially `.agents/skills/`) and check available project skills; if a skill matches the task, read its `SKILL.md` before making changes. Treat notes in `.agents/` as authoritative alongside the arc42 docs (`doc/arc42/`).
4. **Before implementing anything, check the relevant Design Decision Records (`doc/adr/`) and concept docs (`doc/arc42/12-concepts/`)** — if they are not already in context, read them first and implement in line with them. If a planned implementation contradicts a recorded decision or concept definition, stop and point out the conflict.
5. Follow nested `AGENTS.md` files for files in their respective directories.
6. Prefer repository-local instructions over global defaults when they conflict.

## What this project is

A tool to **draw ASCII diagrams visually**: users place, move, and resize predefined shapes on a canvas instead of hand-typing box-drawing characters. The result is plain monospace text (Unicode box-drawing) that can be pasted anywhere, plus SVG/PNG export.

**Objectives: see the objectives file** `doc/arc42/01-introduction-and-context.md` §1.1 — it is the single source of truth for goals; do not duplicate the list here.

## Architecture documentation (MkDocs, one file per chapter)

- The single source of truth for design decisions is the **arc42 documentation**: `doc/arc42/` — one markdown file per chapter, [ADR 002](doc/adr/002-mkdocs-documentation-site.md). `doc/arc42/overview.md` holds the progress tracker.
- Documentation is an **MkDocs** site (`mkdocs.yml` at repo root, `docs_dir: doc`). New doc pages should be added to its `nav`.
- **Diagrams in documentation use PlantUML** (text-based, in ```plantuml fences) until ascii-diagram-drawer itself can produce them — [ADR 003](doc/adr/003-plantuml-for-doc-diagrams.md).
- Read only the relevant chapter(s) before implementing — not the whole set (context budget).
- When a decision is made or changes, **update the affected chapter file(s) in the same change** — including the progress tracker in `overview.md`. Remove guidance/TBD notes as chapters are finalized.

## Tech stack

- Language: Rust — workspace: 5 headless core crates (`diagram-model`, `ad-format`, `shape-registry`, `render-pipeline`, `exporters`) + service seam (`ui-api` traits, `core-services` backends/composition) + `gui-app`, organized as **bounded contexts per DDD** ([ADR 011](doc/adr/011-domain-driven-design.md); the ADR 009 seam = application layer) — [ADR 009](doc/adr/009-ui-api-service-layer.md), [§8](doc/arc42/08-building-block-view.md)
- GUI framework: **egui / eframe** (immediate-mode Rust, compiles to native desktop + browser wasm) — [ADR 008](doc/adr/008-gui-framework-choice.md), status `proposed` (Slint = recorded fallback)
- Web frontend: none separate — eframe builds the same app to wasm; the SvelteKit/Vite hints in `.gitignore` are obsolete
- Rendering/export: text = resolved grid as-is; SVG via `svgbob` ([ADR 005](doc/adr/005-use-svgbob-as-vector-renderer.md)); PNG = optional backend, rasterizer TBD ([ADR 009](doc/adr/009-ui-api-service-layer.md))

## Conventions

> [TBD — add as the codebase grows; keep this section short]

- Follow `cargo fmt` and `cargo clippy`: **all Clippy lints are active at deny** — any lint violation fails the build ([ADR 010](doc/adr/010-deny-all-clippy-lints.md)).
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

- Location: `doc/arc42/12-concepts/<kebab-case-name>.md` (inside the arc42 §12 chapter folder)
- Must carry the standard frontmatter header (`scope` = the concept's area)
- Content: one-line definition, why it exists, how it works/behaves, relations to other concepts (link them), and where it is implemented (once code exists)
- **Before creating any concept doc, always check whether the concept already exists** — search `doc/arc42/12-concepts/` (the arc42 §12 chapter — index plus concept docs) and grep the repo for the term. If it exists, update that doc; never create a second file for the same concept.
- One concept per file
- Keep arc42 §12 (Concepts) as the index: add a one-line pointer per concept there

### Design Decision Records (ADRs)

Whenever a **technical or design decision** has to be made (technology choice, architecture, API/format, trade-off between alternatives), create a Design Decision Record — **always using the template** (`doc/templates/adr.md`), do not invent your own structure:

- Location: `doc/adr/<NNN>-<kebab-case-slug>.md` — sequential numbering, e.g. `001-gui-framework-choice.md`
- Must carry the standard frontmatter header (`status`: proposed | accepted, `version`: integer)
- Content: **Context** (the problem/question), **Decision** (what was chosen), **Alternatives considered** (with why they were rejected), **Consequences** (trade-offs, follow-ups), **Version history** (see below)
- **Check existing ADRs first** (`doc/adr/` + arc42) — if the decision was already made, follow it or bump its `version` (see below); never silently reverse a recorded decision
- **Versioning instead of superseding:** when a recorded decision changes, bump `version` in the header, update the body to reflect the *current* decision, and add an entry to the "Version history" section (newest first) recording what changed, why, and when. Do not write a new ADR that supersedes an old one; keep the original number.
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

- `doc/arc42/*.md` chapter files — when architecture, scope, or quality goals are affected; plus the progress tracker in `doc/arc42/overview.md`
- `doc/arc42/12-concepts/*.md` — when behavior, definitions, relations, or implementation locations of a documented concept change (incl. the "Implementation" section once code lands)
- `doc/adr/*.md` — flip `status: proposed` → `accepted` when the decision is implemented; bump `version` + add a Version history entry when a decision changes (no superseding ADRs)
- `.agents/` notes — when conventions or gotchas change

If a change makes a doc wrong and you cannot fix it fully, mark the affected part `> ⚠️ out of date — <why>` rather than leaving it silently wrong.

## Working agreements for agents

- **Always `git add` any files you create or modify** so new work is staged immediately.
- **Autocommit whenever a meaningful step is achieved** (a working feature, a finished doc section, a recorded decision/ADR/concept — anything reviewable on its own). Write a concise, descriptive commit message for what was achieved. Don't bundle unrelated changes into one commit.
- **The commit is mechanical, not optional: if you created or modified files in this turn and the work is complete, commit before ending the turn.** Never leave finished docs or decisions uncommitted "for review" — the commit itself is the review artifact. Holding work back is only allowed while it is genuinely incomplete.
- Prefer small, reviewable changes; don't restructure the repo without being asked.
- Don't delete or rewrite arc42 chapter guidance notes (`doc/arc42/*.md`) — replace them with real content.
- If a request conflicts with a decision recorded in arc42 or an ADR, point out the conflict before implementing.
- If a task produces new durable knowledge (decisions, gotchas, conventions), record it in `.agents/` (or the arc42 docs / an ADR for architecture) rather than only in chat.
- Keep this file updated when conventions change; it is short on purpose.
