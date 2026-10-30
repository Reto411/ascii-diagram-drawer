---
name: bug-report
description: Create bug reports interactively with the user — ask in small batches, then investigate autonomously (dedupe against open bugs, check whether the behavior is documented as intended, attempt a headless repro), file the report from the template into doc/bugs/, update the board, and commit. Use when the user reports a bug, says "file a bug", or describes broken behavior they want on record.
metadata:
  scope: whole repository (files under doc/bugs/)
  status: stable
  updated: 2026-10-30
---

# Bug Report (interactive)

Mission: turn "this is broken" into a filed, investigated, committed bug report — `doc/bugs/B-NNN-<slug>.md` + the board row. This skill **files and triages; it does not fix**. Fixing goes through the story board (confirmed bug → story S-NNN referencing B-NNN) or a direct `fix(B-NNN): …` commit made outside this skill.

## Ground rules

1. **AGENTS.md applies in full** — including security: reports are committed to a **public repo**. Never put local paths, usernames, credentials, or machine-specific data into the report; trim logs before pasting. If evidence contains something that looks like a secret, do not paste it — follow AGENTS.md's security section (ATTENTION.md).
2. **The docs are the contract.** Before calling anything a bug, check whether the behavior is *recorded as intended* (ADR / concept / arc42). A reported "bug" that matches a recorded decision is `as-designed`, not a bug — and changing it is an ADR version bump + story, not a fix.
3. **Evidence over assertion.** Every claim in the report cites something: a command + trimmed output, a test name, an asset file. "I think it crashes" is not evidence.
4. **Commit is part of filing**: `git add` report + board + assets, message `bug(B-NNN): <title>`.

## State files

| File | Role |
|---|---|
| `doc/bugs/BUGS.md` | Board (Open / Closed). Single source of truth for status. |
| `doc/bugs/B-NNN-<slug>.md` | One file per bug, from `doc/templates/bug.md`. IDs sequential, never reused. |
| `doc/bugs/assets/B-NNN/` | Screenshots, trimmed logs, minimal repro `.ad` files for that bug. |

## Workflow

### Phase 1 — Triage (ask ≤3 questions)

Ask in one message, plain language, numbered:

1. What happened? (one sentence — becomes the title)
2. Where / when? (which feature, screen, or action; which build — native or web)
3. Anything else you can share? (log output, screenshot, the `.ad` file that triggers it)

Derive what you can yourself instead of asking: commit (`git rev-parse --short HEAD`), workspace state (`git status --short`). If the user already pasted a log or file, answer those questions from it — don't ask again.

### Phase 2 — Details (adapt to what's missing)

Follow up only for gaps that matter:

- **Steps to reproduce** — numbered, concrete, minimal ("open `x.ad`, drag box right" not "use the app").
- **Expected vs actual** — one short paragraph each; expected should cite the requirement/concept when known.
- **Evidence** — save screenshots and repro files under `doc/bugs/assets/B-NNN/` (create the dir); link them from the report. A minimal `.ad` file that triggers the bug is the best possible evidence in this project.

If the user says "just file it", file with what you have — mark missing fields `unknown`, never block on optional details.

### Phase 3 — Investigation (autonomous, no questions)

1. **Dedupe**: read `doc/bugs/BUGS.md` + grep existing reports for the symptom. Same root issue → do not create a new ID; append to the existing report's Evidence and stop (note it was re-reported).
2. **Docs check**: search ADRs, concepts (`doc/arc42/12-concepts/`), arc42 chapters for the involved behavior. Record the outcome in the report: *not covered* / *contradicts <decision>* / *matches <decision> → as-designed*.
3. **Repro attempt** (skip only if clearly impossible headlessly, e.g. pure visual): run the relevant checks — targeted `cargo test`, a fixture through the pipeline/exporters, or the GUI smoke per the **gui-testing skill** (`scripts/smoke.sh`). Record each attempt: command + trimmed result. Reproduced → status `reproduced`; not reproduced → stay `open` with what was tried.

### Phase 4 — File and commit

1. Next ID: `sh .agents/skills/bug-report/scripts/next-id.sh` → e.g. `B-001`.
2. Create `doc/bugs/B-NNN-<slug>.md` from `doc/templates/bug.md`; fill **every** section (use `unknown`, never leave a section empty). Set frontmatter `status`:
   - `open` — reported, not yet reproduced · `reproduced` — confirmed with evidence · `in-progress` — being fixed
   - closed: `fixed` · `wontfix` · `as-designed` · `duplicate`
3. Update the board: Open table while open/reproduced/in-progress; move to Closed (date + one-line resolution) when it closes.
4. Commit: `bug(B-NNN): <title>` — report, board, and assets in the same commit.

### After filing (handoff, tell the user this)

- **Reproduced bug** → belongs on the story board as a story whose scope is the fix (or a direct `fix(B-NNN): …` commit if trivial); when it lands, flip the report to `fixed`, fill Resolution, update the board, commit.
- **As-designed** → ask whether the user wants the *decision* changed; that path is an ADR version bump + story, not a bug fix.

## Question etiquette

- At most 2–3 questions per message; accept pasted logs/screenshots as answers and extract what you need from them.
- Never ask for something derivable (commit hash) or already present in provided evidence.
- Keep the report's tone factual: "Actual: …" describes observed behavior, never assigns blame or guesses causes — speculation goes into Investigation only, labeled as such.
