---
title: verification facts must come from the tree, not memory
description: lesson + rule — after context compaction, every test name / gate number in story/board/CR docs is re-derived from code and logs before writing; incident 2026-11-02 (S-041)
scope: delivery process (story/CR documentation)
status: stable
updated: 2026-11-02
---

# Verification facts must come from the tree, not memory

## Incident (2026-11-02, S-041 / CR-020)

After a context compaction mid-story, the BOARD.md entries written for S-041 described a design that was **never implemented** ("sticky anchor drag", a `LineHandlesOverlay` with midpoint diamonds), cited test names that do not exist in the tree, and used stale gate numbers (697 / 92.71%) instead of the final ones (700 / 93.42%). The story file itself had been written correctly earlier; only the board draft was contaminated. Caught by re-grepping every cited test name against `gui-app/src/` before the CR status commit; fixed in `8156a73`.

## Rule (binding for all future stories/CRs)

Before writing any Verification section, board row, or CR Implementation section:

1. **Test names** — grep each one: `grep -rn "fn <name>" <crate>/src/`. No match ⇒ do not cite it.
2. **Gate numbers** — read the *final* log of the run (`/tmp/tarpaulin-*.log` tail for coverage; `cargo test --workspace | grep "^test result"` summed for the pass count). Never reuse numbers remembered from an earlier, intermediate run.
3. **Behavior claims** — re-read the actual diff (`git diff <base>..HEAD`) or the implementing code before describing gestures/mechanisms in prose.
4. After any compaction, treat all pre-compaction "facts" as unverified until re-checked this way.
