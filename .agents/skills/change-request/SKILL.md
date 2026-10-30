---
name: change-request
description: Create change requests interactively with the user — request changes to or additions of requirements (arc42 §14). Ask in small batches, then clarify autonomously (map affected FR/NFRs verbatim, conflict check against ADRs/concepts, decision-relevant impact on stories/format compatibility), file the CR from the template into doc/crs/ and commit. Clarifies *what changes* — never *how it will be implemented*; implementation scoping is decided at handoff by the cr-implementation skill. The agent prepares but never applies: a CR stays "proposed" until the user explicitly accepts or rejects it. Use when the user wants different behavior than specified, requests a new feature/capability, or says "file a change request".
metadata:
  scope: whole repository (files under doc/crs/, applies to doc/arc42/14 + ADRs on acceptance)
  status: stable
  updated: 2026-10-30
---

# Change Request (interactive)

Mission: turn "I want X different / I need Y" into a filed, clarified change request — `doc/crs/CR-NNN-<slug>.md` + board row — prepared so far that **accepting is a one-word decision**. The skill pins down *what changes* (desired behavior, motivation, affected requirements and recorded decisions) — never *how it will be implemented*; implementation scoping happens at handoff when the **cr-implementation skill** (`.agents/skills/cr-implementation/SKILL.md`) works the story. On acceptance the skill applies the full doc change in one commit (§14 + ADRs + skeleton story).

## Ground rules

1. **AGENTS.md applies in full** — including security (public repo: no local paths or credentials in reports).
2. **The requirements are the contract; changing them is a human decision.** The agent *prepares* (exact proposed wording, conflict analysis, decision plan) but never *applies*. A CR stays `status: proposed` until the user explicitly accepts or rejects — in this conversation or later from disk. Vague enthusiasm ("sounds good") is not acceptance; confirm once: "Accept CR-NNN as written?"
3. **Bug vs change**: "current behavior is wrong" → that's a bug report (**bug-report skill**), not a CR. A CR is "behavior is as intended but I want it different" or "new capability". If the request turns out to be a defect while interviewing, stop and file a bug instead.
4. **Claims cite sources**: requirement IDs (FR-n/NFR-n), ADR numbers, story IDs — no unsourced impact estimates.
5. **Commit is part of filing**: `git add` CR + board, message `cr(CR-NNN): <title>`.

## State files

| File | Role |
|---|---|
| `doc/crs/CRS.md` | Board (Open / Closed). Single source of truth for CR status. |
| `doc/crs/CR-NNN-<slug>.md` | One file per CR, from `doc/templates/cr.md`. IDs sequential, never reused. |

## Workflow

### Phase 1 — Intake (ask ≤3 questions)

One message, plain language, numbered:

1. What change or addition do you want? (one sentence — becomes the title)
2. Why? (user need, problem, use case that motivates it)
3. Which part of the tool does it touch? (derive from answers 1–2 when possible — don't ask what's already answered)

### Phase 2 — Details (adapt to gaps)

- **Desired behavior** — concrete and observable: "the user can …, and sees …" not "works correctly".
- **Scope expectations** — v1 or later? Any constraints (performance, file-format compatibility, determinism NFR3)?
- **Existing requirement being changed?** Extract its current wording yourself from `doc/arc42/14-important-requirements.md` and show the user *current → proposed*; confirm the proposed wording rather than asking them to retype.

If the user says "just file it", file with what you have — mark missing fields `unknown`, never block on optional details.

### Phase 3 — Clarification (autonomous, no questions)

Goal: make the requested change precise and decidable. **Stop at clarifying what changes** — no implementation plan, no crate scoping, no effort estimates; those belong to the cr-implementation skill when the story is worked.

1. **Requirement mapping**: every affected FR-n / NFR-n quoted verbatim from §14 into the CR. Pure addition → propose the next free ID (scan §14.1/§14.2).
2. **Conflict check**: search ADRs, concepts (`doc/arc42/12-concepts/`), arc42 chapters for contradicting recorded decisions. A conflict does *not* block a CR — it is the core decision material — but it must be stated explicitly: which decision changes and how (ADR version bump + history entry).
3. **Decision-relevant impact**: affected stories (done → rework/regression risk, in-progress/backlog → scope change), and **file-format compatibility** — anything touching ADR 004 / NFR2 / round-trip is high-stakes; call it out.
4. **Decision plan**: the exact doc changes acceptance triggers in one commit — §14 edit(s), ADR bump(s)/new ADR (`status: proposed`), concept doc updates, and a skeleton story S-NNN on the board whose Why references CR-NNN (acceptance criteria are written by the cr-implementation skill when it starts).

### Phase 4 — File and commit

1. Next ID: `sh .agents/skills/change-request/scripts/next-id.sh` → e.g. `CR-001`.
2. Create `doc/crs/CR-NNN-<slug>.md` from `doc/templates/cr.md`; fill **every** section (`unknown`, never empty). Frontmatter `status: proposed`, `priority` (high|medium|low).
3. Board row in the Open table; commit `cr(CR-NNN): <title>`.
4. Tell the user what was filed and that it awaits their decision — with the one-line summary they need to decide.

### Decision phase (when the user accepts or rejects)

**Accepted** — confirm once ("Accept CR-NNN as written?"), then apply everything in **one commit**:

1. §14: replace/add the requirement rows exactly as proposed (new IDs appended, never renumber).
2. ADRs: version bump + history entry for each contradicted decision, or new ADR (`status: proposed`) for genuinely new decisions.
3. Concept docs / arc42 chapters touched by the change (same-commit rule), `overview.md` tracker if scope moves.
4. Story board: skeleton S-NNN whose Why cites CR-NNN (title + Why + scope only — acceptance criteria are written by the cr-implementation skill when it starts); dependencies set from the conflict/impact check.
5. CR file: Decision section filled (status `accepted`, date, user's words as reason if any); board row keeps Open until implemented.
6. Commit: `cr(CR-NNN): accepted — <title>`.

**Rejected** — record reason + date in the Decision section, status `rejected`, move the board row to Closed with a one-line resolution. No other doc changes. Commit: `cr(CR-NNN): rejected — <title>`.

**Later, when the work lands** (cr-implementation skill): the story's commit flips the CR to `implemented` (fill Implementation section with S-NNN + commit hash) and moves the board row to Closed.

## Question etiquette

- At most 2–3 questions per message; accept pasted examples/screenshots as answers and extract what you need.
- Never ask for something derivable from §14/ADRs/board — read it yourself and show the user your reading.
- Keep the requested change factual: "Requested change" states what should be, never critiques the current design — disagreements about direction go into Motivation or the conflict check, labeled as such.
