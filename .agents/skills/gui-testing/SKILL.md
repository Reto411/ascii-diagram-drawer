---
name: gui-testing
description: How to test the eframe GUI headlessly in the isolated devcontainer — tier 0 headless unit tests for interaction logic, then three visual tiers (launch-alive, golden-screenshot diff, perceptual check with the local vision model), plus evidence rules for story Verification sections. Use when verifying any GUI story AC (S-012…S-019), running a GUI smoke or screenshot, generating/diffing goldens, or deciding whether a visual check is machine-verifiable.
metadata:
  scope: gui-app + devcontainer GUI tooling (xvfb/ImageMagick)
  status: stable
  updated: 2026-10-28
---

# GUI Testing

The devcontainer has **no display**. All GUI verification runs headless under `xvfb` inside the image (it ships `xvfb` + ImageMagick; see `.devcontainer/README.md`). The rule that makes this work: **GUI stories decompose interaction logic into headless unit tests** — gesture handlers against fake `Services`, pure grid→screen mapping / hit-test helpers — so nearly every AC is machine-verifiable without a single pixel. The visual tiers below exist only for what pixels actually answer.

## Tiers (cheapest first — stop as soon as a tier answers the AC)

### Tier 0 — headless unit tests (the default; most ACs end here)

Interaction logic lives in plain Rust inside `gui-app` and is tested without a window:

```sh
cargo test -p gui-app
cargo tarpaulin --engine Llvm -p gui-app   # ≥80% line coverage gate (AGENTS.md)
```

Covers: gesture handlers (pointer down/move/up → model mutation sequences), grid→screen mapping, cell hit-testing, snap-to-cell math, selection/resize rules. If an AC is phrased in terms of *model state after a user action*, it belongs here — write the unit test, do not reach for a screenshot.

### Tier 1 — launch-alive (does it start and stay up?)

```sh
sh .agents/skills/gui-testing/scripts/smoke.sh launch
```

Builds `gui-app`, runs it under `xvfb` for 10 s. Exit 124 from `timeout` = the app started and stayed up → **PASS**. Any early exit is a crash — capture stderr before moving on. Answers ACs like "app starts", "opens without panic", "stays responsive to idle".

### Tier 2 — golden-screenshot diff (pixel-exact regression guard)

```sh
sh .agents/skills/gui-testing/scripts/smoke.sh capture shot.png 3   # build, run, screenshot after 3 s idle
sh .agents/skills/gui-testing/scripts/smoke.sh diff shot.png gui-app/tests/goldens/<name>.png
```

- **Goldens live in `gui-app/tests/goldens/`** and are committed with the story that introduces them.
- Generate a golden **once, inside the same image build** — pixel-exact matches are only stable within one image build (fonts/renderer come from the image's apt packages). If the Dockerfile changes anything GUI-relevant (fonts, ImageMagick, eframe version), regenerate goldens and note it in the progress log.
- The capture script verifies the app is still alive before capturing — a crashed app fails loudly instead of producing a black-frame "pass".
- For meaningful goldens the startup state must be deterministic: fixed window size, fixed theme, no animations (see eframe notes below), and a known diagram — once `gui-app` takes CLI args (S-012+), capture with a fixture `.ad` file.

### Tier 3 — perceptual check (vision model; use sparingly)

Only when tiers 0–2 **genuinely cannot answer the AC** — a semantic property no pixel diff can express ("label is legible", "box drawn at clicked position"). Requirements:

1. The session model supports images (`"images": true` in `.devcontainer/models.json`) and Unsloth has a vision-capable variant loaded. **Status 2026-10-27:** the endpoint accepts `image_url` parts but the loaded GGUF is not vision-capable (HTTP 400 "does not support vision") — until a Qwen VL variant is loaded, tier 3 is unavailable and its ACs go to ⚠️ deferred-to-human.
2. The vision model runs on **CPU** — every call is slow. Downscale first: `magick shot.png -resize 320x shot-low.png`, then `read` the low-res PNG (structure and labels stay legible; tokens and inference time don't).
3. **Batch several perceptual assertions into one screenshot** instead of taking many.
4. Save the (low-res) screenshot as an artifact and cite its path + your observed findings as evidence — a human re-checks the PNG, not the agent's claim.

## Decision table

| AC kind | Tier |
|---|---|
| Model state after a user action (move/resize/connect/delete) | 0 |
| App starts, stays up, no panic on open | 1 |
| "Looks like before" — visual regression of a known screen | 2 |
| Semantic visual property (legibility, correct position in pixels) | 3 |
| Gesture → mutation sequence with fakes | 0 (never screenshot this) |

## Evidence rules (story Verification section)

- Every ✅ cites the **command actually run + observed result**: exit code, AE pixel count, or screenshot path. "Tests passed" is not evidence.
- Screenshot artifacts stay on disk (e.g. `gui-app/tests/goldens/` for goldens; a story-local path for one-off perceptual shots) and are cited by path.
- Only checks that fail **all** tiers close as ⚠️ *deferred to human*, with a concrete checklist written into Verification; every ⚠️ is flagged in the board's progress log. An agent never marks a visual AC ✅ without having seen it (tier ≥ 2 evidence).

## eframe determinism notes (stable goldens)

Pixel-exact diffs only work if the app renders identically on every run:

- Pin the viewport: fixed `InnerSize`, no user-resize between capture and diff; same xvfb screen size (default is fine — it's constant inside the image).
- Fix the theme (`Visuals::dark` or `light`) — never follow system settings.
- Disable animations / transitions (`Context::set_animations_enabled(false)`) so a mid-animation frame can't be captured.
- Capture after idle (the script's `sleep N`): egui repaints on events, so the settled frame is stable.
- Monospace font must resolve to the same file every run — it does inside one image build; another reason goldens are per-image-build.

## wasm target

This skill covers the **native** target (what runs under xvfb). The web milestone (S-019) verifies differently: `cargo check -p gui-app --target wasm32-unknown-unknown` + the headless core test suite on wasm — there is no browser in the isolated devcontainer, so no screenshots for wasm.
