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
3. Follow nested `AGENTS.md` files for files in their respective directories.
4. Prefer repository-local instructions over global defaults when they conflict.

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

## Working agreements for agents

- **Always `git add` any files you create or modify** so new work is staged immediately.
- **Autocommit whenever a meaningful step is achieved** (a working feature, a finished doc section, a resolved issue — anything reviewable on its own). Write a concise, descriptive commit message for what was achieved. Don't bundle unrelated changes into one commit.
- Prefer small, reviewable changes; don't restructure the repo without being asked.
- Don't delete or rewrite `doc/arch/arc42.md` guidance notes — replace them with real content.
- If a request conflicts with a decision recorded in arc42, point out the conflict before implementing.
- If a task produces new durable knowledge (decisions, gotchas, conventions), record it in `.agents/` (or `doc/arch/arc42.md` for architecture) rather than only in chat.
- Keep this file updated when conventions change; it is short on purpose.
