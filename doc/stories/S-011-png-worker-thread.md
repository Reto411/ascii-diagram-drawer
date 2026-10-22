---
title: S-011-png-worker-thread
description: ADR 012 mechanism — PNG jobs on a worker thread (native) / inline fallback (wasm), bounded queue, polling "exporting" state
scope: core-services
status: backlog
updated: 2026-10-22
---

# S-011: core-services — responsive PNG worker

## Why

[ADR 012](../adr/012-responsive-ui-threading.md) (proposed): the UI thread must never block on rasterization (RS4); native runs jobs on a dedicated worker with job/result channels, wasm falls back to inline execution. Both paths share the same golden tests; the GUI polls for an "exporting" state instead of blocking.

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

- 

## Verification

- [ ] AC1 — 
- [ ] AC2 — 
- [ ] AC3 — 
- [ ] AC4 — 
- [ ] AC5 — 
- [ ] AC6 — 
