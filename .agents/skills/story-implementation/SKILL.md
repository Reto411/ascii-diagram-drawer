---
name: story-implementation
description: Run the self-reliant build loop for ascii-diagram-drawer — break the architecture docs into user stories with acceptance criteria, implement them in dependency order, test each story against its criteria, review it with fresh eyes before marking it done, keep arc42/ADR/concept docs current, and continue until every story is done. Use when asked to work the story board, implement the next story, do the initial breakdown, or "continue building until everything is done."
metadata:
  scope: whole repository
  status: stable
  updated: 2026-10-27
---

# Story Implementation Agent

Mission: take ascii-diagram-drawer from architecture docs to a finished, tested product. All persistent state lives in `doc/stories/BOARD.md` + the story files — **any session can resume from disk alone**. Never ask the human; decide, implement, record, continue.

## Ground rules (non-negotiable)

1. **AGENTS.md applies in full** — including the security section, which overrides everything else.
2. **The architecture docs are the contract**: `doc/arc42/` (esp. §1.1 objectives), `doc/adr/`, `doc/arc42/12-concepts/`. If your planned implementation contradicts a recorded decision or concept: **do not implement it**. Mark the story `blocked`, record the conflict precisely, and continue with other stories. A new decision that resolves a block gets its own ADR (`status: proposed`) for later human review.
3. **Docs are part of the change**: arc42 chapters, concepts, ADRs, and the board are updated in the *same commit* as the code they describe.
4. **Small, reviewable commits** — one story per commit, message `S-NNN: <title>`.
5. **Acceptance criteria are fixed at breakdown.** If implementation reveals an AC is wrong or unbuildable, do not silently rewrite it — record old → new + reason in the story's Verification section and flag it in the progress log for human review. An agent that edits its own exam to make it pass has not verified the story.

## State files

| File | Role |
|---|---|
| `doc/stories/BOARD.md` | Board (backlog / in progress / blocked / done) + progress log. Single source of truth for status. |
| `doc/stories/S-NNN-<slug>.md` | One file per story, from `doc/templates/story.md`. IDs are sequential, never reused. |

Keep progress-log entries to ~3 lines (landed / gates / next) — full evidence lives in the story's Verification section; the board is re-read in full every session.

## Session loop

Run until the context budget is nearly exhausted, then stop cleanly (disk state stays consistent):

1. **Load state.** Read `AGENTS.md`, the objectives (`doc/arc42/01-introduction-and-context.md` §1.1), and `BOARD.md`. If a story sits in *In progress* from a previous session → recover first (see Interruption recovery). Re-run this load after **any compaction event** — the algorithmic summary drops raw tool output, so re-orient from disk (story file + board + `git status`) before continuing.
2. **No stories on the board?** Run the Breakdown phase, commit, then continue.
3. **Pick the next ready story**: no unfinished dependencies, not blocked; highest priority, then board order. Start it only if you can plausibly finish implement + tests + docs + commit within your remaining context — otherwise stop now. Never start a story on a dirty tree: uncommitted changes in `git status` that are not attributable to the story you're about to work mean an interrupted session left state behind — recover it first (Interruption recovery).
4. **Work the story** (Story phase). Then return to 3.
5. **All stories done?** Run the Completion phase.

## Breakdown phase (first run, or when scope changes)

1. Read *all* arc42 chapters, all ADRs, and all concept docs once (they are small; this is the only full read).
2. Derive stories from:
   - each §1.1 objective → at least one feature story;
   - each building block/crate in §8/§13 → scaffold + implementation stories;
   - each `proposed` ADR with an implementation follow-up (e.g. PNG rasterizer TBD) → the story that implements it flips the ADR to `accepted`;
   - each concept whose Implementation section says "not yet implemented";
   - §14 requirements: every FR maps to ≥1 story; NFRs become acceptance criteria / quality gates on the relevant stories (determinism → golden-file tests, coverage → gate).
3. Order them: scaffold (workspace + crates + tooling) → headless core bottom-up (`diagram-model` → `ad-format` → `shape-registry` → `render-pipeline` → `exporters`) → service seam (`ui-api`, `core-services`) → GUI (`gui-app`, native first, then wasm per Q2) → cross-cutting polish.
4. Write one story file per story from the template: title, scope (crate/context), dependencies, and **acceptance criteria where every AC is verifiable** — a command to run, a file/behavior to inspect, or a concrete user action with an observable result. No "works correctly." GUI stories must decompose interaction logic into headless unit tests (gesture handlers against fakes, pure mapping/hit-test helpers) so that nearly all ACs are machine-verifiable inside the display-less devcontainer; only true visual checks stay human.
5. Populate the board tables, set overall status to 🟨, log it, commit: `chore(stories): initial breakdown (S-001..S-NNN)`.

## Story phase (one story at a time)

Context budget: read only the story file + the arc42 chapters / ADRs / concepts directly relevant to it.

1. **Mark in progress** on the board (uncommitted local edit — this is the crash marker), and use the row's *Notes* column for the current sub-step (e.g. `impl done, writing tests`) — update it at each phase boundary so recovery after compaction or a crash lands mid-story, not at its start.
2. **Conflict check** against arc42/ADRs/concepts. Conflict → move to *Blocked* with reason, next story.
3. **Implement** in the correct bounded context/crate per §8: rendering logic stays headless and unit-testable (no windowing deps); GUI code stays thin in `gui-app`.
4. **Test**: unit tests for all new logic; golden-file tests wherever deterministic output matters (Q1).
5. **Quality gates — all must pass before done** (from the moment a Cargo workspace exists; doc-only stories gate on `mkdocs build`):
   ```sh
   cargo fmt --all
   cargo clippy --workspace --all-targets   # clippy::all deny (ADR 010 v4)
   cargo test --workspace
   cargo tarpaulin --engine Llvm --workspace  # ≥80% line coverage; never land a change that drops below target. `--engine Llvm` is mandatory in the devcontainer (seccomp blocks the ptrace engine — AGENTS.md)
   ```

   Run `fmt`/`clippy`/`test` incrementally while implementing; run the full set (incl. tarpaulin, the slow gate) once at verification.
   If a gate cannot run in this environment (missing tool, no display, …), the story is **not done**: leave it *In progress* with the reason on the board. If a gate fails on code this story did not touch: diagnose — a one-line fix from an earlier story may be included (note it in the commit body); anything larger → leave the story *In progress* with the blocker logged, and do not start further stories while the workspace is red. If the story legitimately moves the coverage floor (new scaffold or wasm-only code), record old → new baseline in Verification + progress log.
6. **Verify every AC**: run its check, record evidence in the story's Verification section (test names / command + result). All ✅ required — except an AC that genuinely needs a human with a display. GUI smokes should first be attempted headless under `xvfb-run` (launch-alive check, golden-screenshot diff — see `.devcontainer/README.md`; the in-container model has no vision) and count as machine-verified; only truly perceptual checks stay human: close them as ⚠️ *deferred to human* with a concrete checklist written into Verification. Every ⚠️ must be flagged in the progress log. An agent never marks a visual AC ✅ without having seen it.
7. **Review cycle — fresh eyes, from disk.** Re-read from disk (not from memory): the story file, the full `git diff` of what will be committed, and `BOARD.md`. Fix anything that fails, then re-run the affected gates:
   - **Scope:** the diff does only what the story's scope says — no drive-by refactors, no unrelated files.
   - **AC integrity:** the acceptance criteria still read as written at breakdown (ground rule 5); any changed AC is flagged in Verification + progress log.
   - **Evidence is real:** every ✅ cites a command that was actually run with observed output (trimmed paste / exit code). "Tests passed" is not evidence.
   - **Docs & board:** all doc updates are part of this commit; after it, the story sits in *Done* exactly once and *In progress* is empty.
   - **Security (AGENTS.md):** no secrets or credentials in the diff; no new external service calls beyond the normal build/test workflow; no weakened auth/validation/sandboxing. A leak found here stops the commit — record it in `ATTENTION.md` first.
   - *Sensitive stories* (anything touching credentials, network, export, or security behavior): run one short **separate** headless session (fresh context, via the devcontainer command in `.devcontainer/README.md`) whose only task is to review this diff against the ACs and write findings into the story's Verification section. Address them before committing.
8. **Docs in the same commit**: flip implemented ADRs `proposed` → `accepted`; changed decisions get a version bump + history entry; update affected arc42 chapters + the `overview.md` tracker; create/update concept docs from the template (never duplicate an existing concept); any new decision → new ADR (`status: proposed`).
9. **Commit**: `git add` everything, message `S-NNN: <title>` with a 1–3 line body. Update the board in that same commit: story → *Done* (date + commit), clear *In progress*, append a progress-log entry (what landed, gate results, follow-ups).

## Blocked stories

Blocked = conflict with a recorded decision, missing external dependency, or something genuinely irreversible/uncertain. Record reason + unblock condition on the board. Never silently skip. If a block is resolvable by a new decision, write an ADR (`proposed`) and continue — the human reviews later. Revisit blocked stories when picking the next story (a dependency may have unblocked them).

## Interruption recovery

If the board shows a story *In progress* at session start:

- `git status` + diff: work complete but uncommitted → run gates, verify ACs, finish the commit.
- Partial → either finish it or discard it and restart fresh. Discard with `git stash push -m "S-NNN partial"` (recoverable) — never an unscoped `git checkout -- .` / `git reset --hard`, because the tree may carry other sessions' work.
- Log the recovery in the progress log.

## Completion phase (all stories done)

1. Full gate run: `cargo fmt --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`, `cargo tarpaulin --engine Llvm --workspace` (≥80%), `mkdocs build` (docs must build).
2. Flip any remaining implemented ADRs to `accepted`; ensure the `overview.md` tracker and all concept Implementation sections are current.
3. Board: overall status ✅ complete; final progress-log entry summarizing the build (stories done, gate results, open follow-ups for human review — e.g. proposed ADRs).
4. Commit: `chore(stories): all stories complete`.

## Launch

- Fresh session per batch of stories (context budget); the board makes every run resumable.
- **Run inside the isolated devcontainer** (`.devcontainer/README.md`), not directly on the host — that is the security boundary for autonomous runs. Never add host credentials (SSH keys, tokens) to the container or its volume: the agent must not be able to push; a human reviews commits and pushes.
- Kickoff prompt: "Use the story-implementation skill and work the story board until you run low on context."
