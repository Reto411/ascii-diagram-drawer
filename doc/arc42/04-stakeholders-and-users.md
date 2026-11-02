---
title: "arc42-4-stakeholders-and-users"
description: list of stakeholders and user groups, requirements from their perspective
scope: architecture
status: stable
updated: 2026-11-02
---

# 4. Stakeholders and Users

> *Concrete list of stakeholders and user groups, plus requirements from their perspective.*

## 4.1 List of Stakeholders

| Stakeholder | Role | Interest / concern |
|-------------|------|--------------------|
| **Diagram authors** (architects & developers who document systems) | Primary users | Full control over layout (nothing rearranges itself), fast iteration, output that looks good as plain text *and* image, no lock-in — files stay readable and editable outside the tool ([§3.3](03-business-scope.md#33-user-characteristics)) |
| **Output readers & tooling** (humans reading docs/READMEs/RFCs; CI that renders or diffs text) | Consumers of exported diagrams | Correct monospace rendering anywhere, byte-stable output on re-save (QS7), plain-text primary format with SVG/PNG as derivatives |
| **Project maintainer** | Product owner + quality gatekeeper | Coverage ≥ 80%, `clippy::all` deny, determinism invariants in CI ([§6](06-quality-model.md)), documentation as single source of truth (arc42 + ADRs current with every change) |
| **AI coding agents** (pi, working in the devcontainer) | Implementers on user stories | Reproducible quality gates that run headless in the podman container, story-driven board as persistent state, decisions recorded as ADRs/concepts so work is resumable across sessions |
| **Shape-library sharers** (teammates exchanging `.adc` collections via VCS) | Secondary users | Collections are plain text, versionable, shareable without installing the tool; template references (`collection/name`) stay stable across versions (C6/C8) |

## 4.2 List of Users / User Groups

- **Primary — technical documenters** ([§3.3](03-business-scope.md#33-user-characteristics)): architects and developers writing design docs, RFCs, READMEs. Comfortable with monospace text and version control; **no** ASCII-art fluency expected. Work on desktop (Windows/macOS/Linux) or in a browser; diagrams live in code repositories next to the documentation they describe.
- **Secondary — library maintainers**: users who build and share custom shape collections (`.adc`) for their team; need the template editor (anchors, text regions, resize rules) and stable reference forms.
- **Tertiary — output consumers**: readers of pasted ASCII diagrams and tools that process them; they only see the exported text/SVG/PNG and require nothing of the tool itself beyond correct, deterministic output.
