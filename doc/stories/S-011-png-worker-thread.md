---
title: S-011-png-worker-thread
description: ADR 012 mechanism — PNG jobs on a worker thread (native) / inline fallback (wasm), bounded queue, polling "exporting" state
scope: core-services
status: done
updated: 2026-10-28
---

# S-011: core-services — responsive PNG worker

## Why

[ADR 012](../adr/012-responsive-ui-threading.md) (accepted): the UI thread must never block on rasterization (RS4); native runs jobs on a dedicated worker with job/result channels, wasm falls back to inline execution. Both paths share the same golden tests; the GUI polls for an "exporting" state instead of blocking.

## Dependencies

- S-010 — `Services` + exporter backend to wrap
- S-009 — the `to_png` work being offloaded

## Acceptance criteria

1. **AC1 — Job API.** A PNG export job API in `core-services`: submit(grid) → handle; polling yields Pending / Done(Png) / Error(ExportError); bounded queue (overflow → typed rejection, no unbounded growth). Check: unit tests incl. overflow case.
2. **AC2 — Native worker thread.** On native targets the job runs on a dedicated worker thread consuming a bounded mpsc channel; submitting does not block beyond enqueue. Check: unit test with a timing assertion (submit returns while rasterization of a large grid is still in flight) + test that results arrive correctly.
3. **AC3 — Wasm fallback.** On wasm32 the same API executes inline (cfg-gated); `cargo check -p core-services --target wasm32-unknown-unknown` exits 0. Check: compile check + shared golden test passes on the native path (wasm runtime execution tracked in S-019).
4. **AC4 — Shared goldens.** The same PNG golden (S-009) is produced through the worker API and the direct call — byte-identical. Check: unit test comparing both paths.
5. **AC5 — Gates green.** fmt/clippy/test pass; `cargo tarpaulin -p core-services` ≥ 80%.
6. **AC6 — ADR 012 flipped to accepted** (mechanism implemented + tested); overview tracker updated.

## Implementation notes

- New module `crates/core-services/src/worker.rs`; re-exported from the crate root (`PngWorker`, `PngJob`, `JobState`, `QueueFull`).
- **API:** `PngWorker::new()` (threaded, bounded queue, default capacity 4) / `with_capacity(n)` (native; clamped ≥ 1) / `inline()` (synchronous executor — the exact wasm32 code path, also used by tests). `submit(&ResolvedGrid) -> Result<PngJob, QueueFull>`; `PngJob::poll() -> JobState` is non-blocking (`try_recv`) and meant to be called once per frame.
- **JobState:** `Pending` / `Done(Vec<u8>)` / `Error(ExportError)` (the three AC1 states) + defensive `Abandoned` (result channel gone — only observable if the worker thread dies abnormally).
- **Ownership transfer, no shared mutable state** (ADR 012): the grid is cloned into a `PngWork` moved over a bounded `mpsc::sync_channel`; the result travels back over a per-job unbounded reply channel. The worker loop is a plain `for work in receiver.iter()`.
- **Drop semantics:** dropping the worker stops *new* submissions (sender gone → `QueueFull`) but the finite bounded buffer still drains — queued and in-flight jobs all complete; nothing submitted is lost. Pinned by test.
- **Wasm:** on `wasm32-unknown-unknown` the struct has no fields, `new()` = `inline()`, and `submit` rasterizes inline then hands back an already-complete job (first poll is `Done`). All threaded items are cfg-gated; zero warnings on the wasm target.
- **Timing basis for tests:** a 300×150 frame rasterizes in ≈154 ms (release, measured with a throwaway example) and slower in debug — submit must return in milliseconds, so the "first poll still Pending" assertion has >100× headroom.
- `PngWorker` sits beside `Services` (not inside it): the GUI story S-015 decides how the app struct holds both; ADR 012 only requires the worker be confined to core-services behind the seam, which it is.

## Verification

- [x] AC1 — `overflow_is_a_typed_rejection`: capacity 1 + in-flight job, three rapid submits → at least one (at most two) rejected with typed `QueueFull`; every accepted job still completes. Queue growth is bounded by construction (`sync_channel`).
- [x] AC2 — `submit_returns_while_rasterization_is_in_flight`: 300×150 grid (rasterization >100 ms measured), submit elapsed asserted < 50 ms, first poll asserts `Pending`, then the result arrives and is byte-equal to the direct call.
- [x] AC3 — `cargo check -p core-services --target wasm32-unknown-unknown` exits 0 with zero warnings (threaded items cfg-gated); `inline_executor_completes_on_first_poll` exercises the exact wasm code path on native (first poll already `Done`, bytes match the direct call).
- [x] AC4 — `worker_output_is_byte_identical_to_the_direct_call`: worker `Done(png)` == `exporters::to_png(grid)` byte-for-byte; same exporter call in both execution paths, so S-009's goldens apply unchanged.
- [x] AC5 — fmt/clippy(0 warnings)/test green workspace-wide (231 tests); tarpaulin: worker.rs 42/42 (100%), core-services crate ≥ 80% floor holds, workspace total 97.43%.
- [x] AC6 — ADR 012 → accepted (status flipped; the decision itself is unchanged — no version bump); overview tracker updated.
