---
name: gui-testing
description: How to test the eframe GUI headlessly in the isolated devcontainer — tier 0 headless unit tests for interaction logic, tier 0b full-app egui frame pumping (no window), then three visual tiers (launch-alive, golden-screenshot diff, perceptual check with the local vision model), plus evidence rules for story Verification sections. Use when verifying any GUI story AC (S-012…S-019), running a GUI smoke or screenshot, generating/diffing goldens, or deciding whether a visual check is machine-verifiable.
metadata:
  scope: gui-app + devcontainer GUI tooling (xvfb/ImageMagick)
  status: stable
  updated: 2026-10-29
---

# GUI Testing

The devcontainer has **no display**. All GUI verification runs headless under `xvfb` inside the image (it ships `xvfb` + ImageMagick; see `.devcontainer/README.md`). The rule that makes this work: **GUI stories decompose interaction logic into headless unit tests** — gesture handlers against fake `Services`, pure grid→screen mapping / hit-test helpers — so nearly every AC is machine-verifiable without a single pixel. The visual tiers below exist only for what pixels actually answer.

## Tiers (cheapest first — stop as soon as a tier answers the AC)

### Tier 0 — headless unit tests (the default; most ACs end here)

Interaction logic lives in plain Rust inside `gui-app` and is tested without a window:

```sh
cargo test -p gui-app
cargo tarpaulin --engine Llvm --workspace  # ≥80% line coverage gate (AGENTS.md, canonical)
```

> **Coverage-gate gotcha (S-014):** `tarpaulin -p gui-app` runs *only* gui-app's tests but reports every compiled crate — workspace deps show up untested and drag the total to ~40–50%, so the 80% floor is unreachable with that flag. Gate on `--workspace`, and cite per-module numbers (e.g. `fileio.rs: 57/57`) for the story's new logic — the egui draw glue + `main` bootstrap are window-bound and never covered.

Covers: gesture handlers (pointer down/move/up → model mutation sequences), grid→screen mapping, cell hit-testing, snap-to-cell math, selection/resize rules. If an AC is phrased in terms of *model state after a user action*, it belongs here — write the unit test, do not reach for a screenshot.

### Tier 0b — headless full-app frame pumping (egui without a window)

`gui-app/src/gui_tests.rs` drives the **real `DrawingApp` through real egui frames** with no window: a `Gui` harness feeds `RawInput` (pointer events, keys, text) into `Context::run` and asserts on `FullOutput` (galleys, lines, rects, circles) plus model state. This closes the gap between tier 0 (pure gesture machine, fake services) and tier 1 (a live process under xvfb): it exercises the *egui glue itself* — sidebar/topbar button hit areas, prompt TextEdits, the floating inline editor, clipboard commands, per-frame PNG worker ticks.

```sh
cargo test -p gui-app gui_tests   # the full suite; part of `cargo test --workspace`
```

Harness rules learned the hard way (do not re-derive):

- **One event batch per frame.** A click = press frame + release frame (+1 settle frame for rendering to catch up). egui drains its command list into the *release* frame's `FullOutput` — read clipboard/commands from that frame, not the idle one after.
- **egui 0.37 ignores `Event::Key.modifiers`.** Modifiers only change via `Event::ModifiersChanged` frames around the key press (see `key_with_mods`) — Ctrl+S sent as a plain `Key{modifiers: ctrl}` does nothing.
- **`response.hover_pos()` is suppressed on button-held frames** (egui stops hovering widgets while interacting with another). The canvas therefore falls back to the raw pointer position *only while a gesture is already running* (`app.rs`, cursor logic) — without it, drag/resize/connect freeze mid-drag.
- **Keyboard ownership is fought over per frame — design it explicitly.** egui 0.37 facts that bite: `request_focus` is an immediate write (the *last* field drawn in a frame wins); the outside-click surrender (`SurrenderFocusOn::Clicks`) runs inside the widget's own `interact()`, i.e. *before* any post-`TextEdit` code you write; `lost_focus(id)` uses a two-frame memory (fires one frame after focus moved); focus on a widget that stops being drawn is dead-lettered ~2 frames later. Consequence in `app.rs`: the inline editor re-requests focus every frame (one-shot focus made placement commit the placeholder immediately) **but yields while another text edit holds focus and skips the re-request on clicked-away frames**; conversely any field that *gains* focus while an edit is open commits it (blur = S-018 AC1). Regression-pinned by `gui_tests::path_prompt_keeps_the_keyboard_over_an_open_inline_edit` + friends — extend that pattern when adding any new text field.
- **Tracing keyboard ownership headlessly:** between pumped frames read `ctx.memory(|m| m.focused())` (the focused widget's `Id`) and `ctx.text_edit_focused()`; to see a transient state, pump press/release/keys one frame at a time (`gui.pump(vec![…])`) instead of using the click helpers. This is how the focus-steal bug was found: after opening the path prompt during an inline edit, `focused` still pointed at the canvas field every frame.
- **Canvas text is drawn row-by-row** (one galley per grid line), so a placed text object's content appears as an exact-match galley of its stamped row — and while editing, the draft adds a *second* identical galley (canvas row + editor field). Locate widgets by position, not just text.
- **Cells are rectangular** (`cell_w` = glyph advance, `cell_h` = line height) — see `Viewport::new(margin, cell_w, cell_h)`; square cells make adjacent text rows overlap.

Use tier 0b when an AC is about *what the GUI does with real egui input* (a button works, a prompt survives frames, an overlay lands on the right cell) — cheaper and more precise than screenshots, and it runs in any environment (no xvfb, no X11 libs).

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

Args after the command's own are passed through to gui-app — capture with a startup fixture:

```sh
sh .agents/skills/gui-testing/scripts/smoke.sh capture shot.png 3 gui-app/tests/fixtures/one-box.ad
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
| GUI behavior through real egui input (buttons, prompts, inline editor, overlays) | 0b |
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
