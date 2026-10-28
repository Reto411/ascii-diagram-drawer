---
title: "015 - apache-2-0-license"
description: project is licensed Apache-2.0 (owner decision 2026-10-28) — full canonical text in LICENSE at repo root, all workspace crates inherit it
scope: legal / distribution
status: accepted
version: 1
updated: 2026-10-28
---

# ADR 015: Apache-2.0 project license

## Context

The repository is public on GitHub and intended as an open-source project, but it shipped with **no LICENSE file** — everything was all-rights-reserved by default, so legally nobody could use or contribute to the code. The workspace manifest additionally declared a stale `license = "MIT"` with no license text behind it. A license had to be chosen; the whole dependency tree (52 external crates) is permissive (MIT / Apache-2.0), so any permissive choice is compatible.

## Decision

The project is licensed under **Apache License 2.0**, copyright holder **Reto Bürgisser**. The full canonical text lives in `LICENSE` at the repo root; `[workspace.package].license = "Apache-2.0"` and all nine workspace members inherit it via `license.workspace = true`.

## Alternatives considered

| Alternative | Why rejected |
|-------------|--------------|
| MIT | Simplest and most permissive, but has no explicit patent grant — the owner wants the rights/patent structure to stay explicit (human decision 2026-10-28) |
| MIT OR Apache-2.0 dual | Common Rust convention, gives downstream a choice; single license is simpler and already covers the patent grant |
| GPL-3.0 (copyleft) | Would force every downstream derivative to be GPL — conflicts with the "paste-it-anywhere tool" goal and typical user expectations |

## Consequences

- **Positive:** clear legal terms for use and contribution; Apache-2.0's explicit patent grant protects author *and* users; 100% compatible with the current dependency tree (all permissive).
- **Negative / costs:** distributors must preserve the license/copyright notices (Apache §4); slightly more boilerplate than MIT.
- **Follow-ups:** none for now. If external contributors appear, consider a DCO/CLA to keep the copyright chain clean.

## Version history

- **v1 (2026-10-28):** initial decision — Apache-2.0, made directly by the project owner; `LICENSE` added and workspace license field corrected from stale `MIT`.
