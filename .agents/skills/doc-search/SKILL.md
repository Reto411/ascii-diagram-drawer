---
name: doc-search
description: Search the project's documentation pages (the MkDocs site under doc/) without burning context — frontmatter index, targeted ripgrep, and entry points per question type. Use when asked about a decision (ADR), concept, objective, requirement, or story status, or "where is X documented" — especially inside the isolated devcontainer, where web search is unavailable and these pages are the reference material.
metadata:
  scope: doc/ documentation site
  status: stable
  updated: 2026-10-28
---

# Doc Search

The project's knowledge base is an MkDocs site rooted at `doc/` (`docs_dir: doc`, nav in `mkdocs.yml`). Pages are plain Markdown — **read them directly; no server needed**. The point of this skill is to reach the right page fast without bulk-reading chapters (context budget, per AGENTS.md).

## Workflow

1. **Index first** — one line per page from its YAML frontmatter, no bodies loaded:
   ```sh
   sh .agents/skills/doc-search/scripts/pages.sh index            # all pages
   sh .agents/skills/doc-search/scripts/pages.sh index connector  # only matching frontmatter
   ```
2. **Full-text search** when you have a term but not the page (ripgrep, case-insensitive; extra rg flags pass through):
   ```sh
   sh .agents/skills/doc-search/scripts/pages.sh find "anchor" -C 2
   ```
3. **Read only the winners** — with `read`, using offset/limit for long chapters. Follow relative links from a hit instead of re-searching.

## Where things live (skip straight here when you know the question type)

| Question | Start at |
|---|---|
| Objectives / "what are we building" | `doc/arc42/01-introduction-and-context.md` §1.1 |
| Architecture, building blocks, runtime, deployment | `doc/arc42/08-building-block-view.md`, `09-…`, `10-…` |
| Requirements (FR/NFR) | `doc/arc42/14-important-requirements.md` |
| A concept (term / mechanism) | `doc/arc42/12-concepts/index.md` → the concept doc |
| A decision / "is X decided?" | `doc/adr/NNN-slug.md` — frontmatter carries `status: proposed\|accepted` and `version` |
| Story status / progress | `doc/stories/BOARD.md` + `doc/stories/S-NNN-*.md`; arc42 tracker in `doc/arc42/overview.md` |
| Templates (concept / ADR / story) | `doc/templates/*.md` |

## Reading discipline

- Frontmatter is the triage layer: `status` (`draft` / `stable` / `superseded`) tells you whether a page is current; `scope` says what it covers. A `superseded` page is history — find the live one before relying on it.
- A `> ⚠️ out of date` marker in a body means that section is stale — treat the content as unverified and flag it.
- Never bulk-read chapters to "get context"; index → targeted read is always cheaper.
- After *editing* pages: keep the `mkdocs.yml` nav in sync and gate with `mkdocs build` (doc-only changes must still build).
