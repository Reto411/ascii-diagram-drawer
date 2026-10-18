---
title: documentation-home
description: entry point of the project documentation site — links to arc42, concepts, ADRs, templates
scope: documentation
status: stable
updated: 2026-10-18
---

# ascii-diagram-drawer — Documentation

A tool to draw ASCII (svgbob-compatible) diagrams visually. See [arc42 §1](arc42/01-introduction-and-context.md) for objectives.

## Where things live

| What | Where |
|------|-------|
| Architecture (arc42, one file per chapter) | [arc42/overview.md](arc42/overview.md) |
| **Objectives** (agents: always check before work) | [arc42/01-introduction-and-context.md](arc42/01-introduction-and-context.md) |
| Concepts | `arc42/12-concepts/` ([arc42 §12 index](arc42/12-concepts/index.md)) |
| Design decisions (ADRs) | `adr/` |
| Templates for new docs | `templates/` |

## Local preview

```bash
pip install mkdocs
mkdocs serve        # → http://127.0.0.1:8000
```
