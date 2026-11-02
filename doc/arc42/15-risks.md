---
title: "arc42-15-risks"
description: risk register — what could go wrong, impact, likelihood, mitigation or contingency
scope: architecture
status: draft
updated: 2026-11-02
---

# 15. Risks and Technical Risks

> *Risk register: what could go wrong, impact, likelihood, mitigation/contingency.*

| ID | Risk description | Impact (H/M/L) | Likelihood (H/M/L) | Mitigation / contingency | Owner |
|----|------------------|----------------|--------------------|--------------------------|-------|
| R1 | Web bundle size exceeds the proposed P3 budget (2 MB): S-019 measured JS+wasm = 13,137,832 B ≈ 12.5 MiB **uncompressed and unoptimized** (`wasm-opt` skipped — sandbox egress blocks the binaryen download; `data-wasm-opt="0"` in `gui-app/index.html`). Optimized size still unknown until measured where github.com is reachable. | M (P3 is "awaiting confirmation", not a gate — but 12+ MiB of wasm is slow to download/compile on constrained networks/devices) | H (already over budget before optimization is even applied) | Re-measure with `wasm-opt` enabled (drop the attribute); if still over budget, revisit P3 in §5.3/§14 — options: lazy-load wgpu/renderer init, trim eframe features, or raise the budget with a recorded rationale | Next release prep / S-021 doc pass |
