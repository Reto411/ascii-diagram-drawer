---
name: doc-search
description: Search documentation without burning context — the project's own pages (MkDocs site under doc/: frontmatter index, targeted ripgrep, entry points per question type) plus the remote reference docs reachable through the egress proxy (Rust std/book/reference on doc.rust-lang.org, crate API docs on docs.rs, crate READMEs on crates.io). Use when asked about a decision (ADR), concept, objective, requirement, story status, "where is X documented", or Rust/crate API questions — especially inside the isolated devcontainer, where general web access is unavailable.
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

## Remote reference pages (reachable via the egress proxy)

Inside the devcontainer, `http(s)_proxy` env vars are already set, so plain
`curl <url>` (or pi's web tools) works against the whitelisted hosts below —
anything else gets a 403 by design. Verified 2026-10-28:

| Reference | URL |
|---|---|
| Rust std/core/alloc API docs | `https://doc.rust-lang.org/stable/std/` (swap `std` for `core`, `alloc`) |
| The Rust Book | `https://doc.rust-lang.org/book/` |
| The Rust Reference | `https://doc.rust-lang.org/reference/` |
| Third-party crate API docs (rustdoc) | `https://docs.rs/<crate>/` — e.g. `https://docs.rs/serde` |
| Crate README + versions/metadata | `https://crates.io/crates/<name>` |

Notes:
- Fetch the specific item page, not a site root, and extract what you need —
  these are full HTML pages; pipe through `head`/`grep` or save to a file and
  read it with offset/limit instead of dumping into context.
- **Offline alternative for dependency docs:** after any build, every
  dependency's source (with all doc comments) is in
  `$CARGO_HOME/registry/src/*/`, and `cargo doc --workspace` builds local
  rustdoc HTML with no network at all — prefer this when the question is
  about a crate already in the workspace.
- crates.io blocks curl's default user agent (403); use `-A "cargo-1.85.0"`
  or a browser UA for API/page fetches.

## Reading discipline

- Frontmatter is the triage layer: `status` (`draft` / `stable` / `superseded`) tells you whether a page is current; `scope` says what it covers. A `superseded` page is history — find the live one before relying on it.
- A `> ⚠️ out of date` marker in a body means that section is stale — treat the content as unverified and flag it.
- Never bulk-read chapters to "get context"; index → targeted read is always cheaper.
- After *editing* pages: keep the `mkdocs.yml` nav in sync and gate with `mkdocs build` (doc-only changes must still build).
