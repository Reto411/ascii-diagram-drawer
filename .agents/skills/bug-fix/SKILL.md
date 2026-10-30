---
name: bug-fix
description: Run the self-reliant fix loop for a filed bug (doc/bugs/B-NNN-*.md) — reproduce it, do the root cause analysis, land a minimal fix in the right bounded context, verify against the report's evidence plus the standard quality gates, review with fresh eyes, then close the bug (status fixed + board). Use when asked to fix a bug, "work B-NNN", "fix the ellipse resize bug", or after a reproduced bug has been handed off from the bug-report skill.
metadata:
  scope: whole repository (closes files under doc/bugs/)
  status: stable
  updated: 2026-10-30
---

# Bug Fix (implementation)

Mission: take a filed bug (`doc/bugs/B-NNN-<slug>.md`, ideally `reproduced`) to closed — root cause found, minimal fix landed and verified, report flipped to `fixed`. This is the handoff partner of the **bug-report skill**: that one clarifies *what* is broken; this one answers *why* and fixes it.

## Ground rules

1. **AGENTS.md applies in full** — including security (public repo: no local paths or credentials in commits/reports).
2. **The report is the contract.** Fix the defect as filed: expected behavior is what the report's Expected section cites (requirement/ADR/concept). If root cause analysis shows the *filed expectation* is wrong (behavior actually matches a recorded decision): stop, close the bug `as-designed` with the citation, and point to the change-request skill if the user wants the decision changed. Do not silently "fix" your way to different behavior.
3. **Minimal fix in the right bounded context** (§8). No drive-by refactors, no new features. If the true fix is large (new capability, cross-cutting rework, needs a decision): file story S-NNN whose Why references B-NNN instead of patching — the bug stays `in-progress` until the story lands.
4. **Docs are part of the change**: report, board, and any affected arc42/concept/ADR pages update in the *same commit* as the fix.
5. **Commit message**: `fix(B-NNN): <what now works>` — one bug per commit.

## State files

| File | Role |
|---|---|
| `doc/bugs/B-NNN-<slug>.md` | The report — Summary/Steps/Expected are the acceptance criteria; Investigation holds prior clarification evidence |
| `doc/bugs/BUGS.md` | Board — status source of truth |

## Workflow

1. **Load.** Read the bug report in full + the board row. Status must be `open`, `reproduced`, or `in-progress` (resuming). Anything else (`as-designed`/`duplicate`/closed) → stop, nothing to fix here.
2. **Mark in progress**: frontmatter `status: in-progress` + board row updated (uncommitted local edit — this is the crash marker).
3. **Reproduce first.** Re-run the report's repro evidence headlessly against current HEAD (per its Investigation/Evidence sections; GUI per the **gui-testing skill**). If it no longer reproduces: find the commit that changed behavior and cite it, then close `fixed` — or keep `in-progress` with a note. Never close "can't reproduce" without evidence.
4. **Root cause.** Trace from the reproduced symptom to the faulty code/decision. Append findings to the report's Investigation section as a `**Fix analysis YYYY-MM-DD:**` block: where, why, and which recorded decision (if any) the fix must respect or change.
5. **Decide the path**:
   - **Trivial** — localized, no new decisions: fix directly now (steps 6–10).
   - **Non-trivial** — new capability, cross-crate, needs a decision/ADR: file story S-NNN from `doc/templates/story.md` (Why cites B-NNN; verifiable ACs including the report's repro), add the board row, commit `story(S-NNN): <title>`, and stop — bug stays `in-progress`. When the story later lands (story-implementation loop or a resumed session), close the bug per step 10 **in the story's commit**.
6. **Implement** (direct path only): in the correct crate/context per §8; rendering logic stays headless + unit-testable; GUI code stays thin. Add a regression test that fails before the fix and passes after, named for the symptom.
7. **Quality gates — all must pass**: `cargo fmt --all`, `cargo clippy --workspace --all-targets` (clippy::all deny), `cargo test --workspace`, `cargo tarpaulin --engine Llvm --workspace` ≥80% (`--engine Llvm` mandatory in the devcontainer). GUI behavior: headless verification per the gui-testing skill (tier 0 unit tests; smoke/golden where relevant).
8. **Verify against the report, not just tests**: re-run every repro command from the report — observed must now match Expected. Record each result as a line in the report's Resolution section.
9. **Fresh-eyes review** (re-read from disk): the report + full `git diff`. Scope check (only what B-NNN needs), evidence real (commands actually run, trimmed output cited), no secrets/credentials, docs current. Fix findings, re-run affected gates.
10. **Close**: report frontmatter `status: fixed`, fill Resolution (what changed + commit hash + verification lines), board row → Closed (date + one-line resolution). Commit everything: `fix(B-NNN): <title>`.

## Interruption recovery

`in-progress` bug at session start with uncommitted changes: finish it, or discard with `git stash push -m "B-NNN partial"` (recoverable — never an unscoped `git checkout -- .` / `reset --hard`). Note the recovery in the report's Resolution section.
