---
title: B-NNN - <kebab-case-slug>
description: one line — what breaks and where
scope: <crate or context, e.g. diagram-model | ad-format | shape-registry | render-pipeline | exporters | ui-api/core-services | gui-app | docs/tooling | build/devcontainer>
status: open
severity: minor
updated: YYYY-MM-DD
---

# B-NNN: <Title>

## Summary

<2–4 sentences: what is broken, where it shows up, impact. Factual, no cause-guessing.>

## Environment

| | |
|---|---|
| OS | <e.g. Linux (devcontainer) / macOS / Windows — or `unknown`> |
| Target | native \| web (wasm) |
| Commit | `<short hash>` |
| Other | <browser, display server, GPU — only if relevant> |

## Steps to reproduce

1. <concrete, minimal, numbered>
2. 

## Expected

<what should happen; cite the requirement/concept when known (FR-n, C-n).>

## Actual

<what happens instead; quote the error message verbatim if there is one.>

## Evidence

> Public repo: no local paths, usernames, or credentials in any pasted output. Screenshots/logs/repro files live in `assets/B-NNN/`.

- <asset link / trimmed log excerpt / minimal repro `.ad` file>

## Investigation

> Filled by the AI while triaging (bug-report skill): dedupe result, docs check, repro attempts with commands + results. Speculation is labeled as speculation.

- **Duplicate:** none | B-xxx
- **Docs check:** <not covered | contradicts <ADR/concept link> | matches <link> → as-designed>
- **Repro attempt:** `<command>` → <trimmed result> — reproduced: yes/no

## Resolution

> Filled when the bug closes: what fixed it (story S-NNN / commit hash), or why it won't be fixed. Empty while open.

- 
