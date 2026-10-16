---
title: 002 - mkdocs-documentation-site
description: documentation moves to an MkDocs site; each arc42 chapter becomes its own markdown file under doc/arc42/
scope: tooling / documentation
status: accepted
updated: 2026-10-17
supersedes: null
---

# ADR 002: MkDocs documentation site, one file per arc42 chapter

## Context

The architecture doc was a single ~450-line `doc/arch/arc42.md`. Problems: heavy to load into limited agent context (~100k), hard to navigate, every small edit touches the same file. We want browsable documentation and targeted reading (e.g. "just read the objectives").

## Decision

- Documentation is an **MkDocs** site with `docs_dir: doc`, configured in `mkdocs.yml` at the repo root.
- **Each arc42 chapter lives in its own file**: `doc/arc42/<NN>-<slug>.md`. The monolithic `doc/arch/arc42.md` is retired (history stays in git).
- `doc/arc42/overview.md` holds the document header, usage notes, and the progress tracker.
- **Agents must always check the objectives file** (`doc/arc42/01-introduction-and-context.md`, §1.1) before starting work.
- All doc files carry the standard frontmatter header.

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| Keep one monolithic arc42.md | Context cost per read, navigation pain, edit conflicts |
| Docusaurus / other static-site tools | Heavier stack for docs-only needs; MkDocs is pure Python and minimal |
| Sphinx | More ceremony than needed for this doc set |

## Consequences

- **Positive:** targeted reads (one chapter instead of 450 lines), browsable site, parallel edits without conflicts
- **Negative / costs:** all existing references to `doc/arch/arc42.md` must be updated; local preview needs `pip install mkdocs` + `mkdocs serve`
- **Follow-ups:** consider CI doc build once CI exists
