---
name: gui-testing
description: How to test the eframe GUI headlessly in the isolated devcontainer — tier 0 headless unit tests for interaction logic, tier 0b full-app egui frame pumping (no window), then visual tiers via the software screenshot rasterizer (golden PNGs without a GPU; xvfb capture is unavailable in this image — no EGL), plus evidence rules for story Verification sections. Use when verifying any GUI story AC (S-012…S-019), running a GUI smoke or screenshot, generating/diffing goldens, or deciding whether a visual check is machine-verifiable.
metadata:
  scope: gui-app + devcontainer GUI tooling (software rasterizer; xvfb path disabled — no EGL)
  status: stable
  updated: 2026-10-30
---

# GUI Testing

The devcontainer has **no display — and no working GPU path**: wgpu cannot create a surface (`FailedToCreateSurfaceForAnyBackend`, the image has no libEGL and apt egress is blocked by ADR 014), so `smoke.sh launch`/`capture` (xvfb) fail here. All GUI verification is headless: **GUI stories decompose interaction logic into unit tests** (tier 0/0b) — gesture handlers against fake `Services`, pure grid→screen mapping / hit-test helpers — and the visual tiers run on the **software screenshot rasterizer** (`gui-app/src/screenshot.rs`): it tessellates a pumped egui frame with epaint's own tessellator and rasterizes the triangles to PNG in pure Rust, no window/GL/X11. Verified pixel-exact against synthetic geometry (fills land on their exact pixel bounds).

> **Desktop launch (outside the container):** `cargo run -p gui-app` opens the ~1024×768 dark-theme window; a shareable RustRover run configuration for it lives in `.run/gui-app.xml` (`cargo run -p gui-app`, workspace root).

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

`gui-app/src/gui_tests.rs` drives the **real `DrawingApp` through real egui frames** with no window: a `Gui` harness feeds `RawInput` (pointer events, keys, text) into `Context::run` and asserts on `FullOutput` (galleys, lines, rects, circles) plus model state. This closes the gap between tier 0 (pure gesture machine, fake services) and the visual tiers: it exercises the *egui glue itself* — sidebar/topbar button hit areas, prompt TextEdits, the floating inline editor, clipboard commands, per-frame PNG worker ticks.

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

> **Unavailable in this image** — wgpu cannot create a surface (no libEGL; apt egress blocked). `smoke.sh launch` exits with `Wgpu(CreateSurfaceError …)`; treat that as an environment limitation, not an app regression. The tier-0b harness is the substitute: pumping many real egui frames through the real `DrawingApp` without panicking *is* the "stays up" proof (the whole suite does this on every run).

### Tier 2 — golden-screenshot diff (software rasterizer; pixel-exact regression guard)

```sh
GUI_DUMP=1 cargo test -p gui-app visual_dump   # writes /tmp/gui-dump/01-empty.png … 08-inline-edit.png
```

`gui_tests::visual_dump` drives the real app through eight known states (empty, text sample, two boxes, selection, connect-drag, template editor, path prompt, inline edit) and rasterizes each frame with `gui-app/src/screenshot.rs`. Add new states to that test as stories land; keep names `<nn>-<state>.png`.

- **Goldens live in `gui-app/tests/goldens/`** and are committed with the story that introduces them. Diff a fresh dump against a golden with any pixel-diff tool (`cmp` on raw RGBA, or ImageMagick's `compare -metric AE` if present).
- The rasterizer is **deterministic pure Rust over epaint's tessellator output** — more stable than GPU captures: goldens survive as long as the egui/epaint version + fonts in the image don't change. If either changes (eframe bump, font packages), regenerate goldens and note it in the progress log.
- The dump is opt-in (`GUI_DUMP=1`) so plain `cargo test` stays fast; coverage runs should set it (`GUI_DUMP=1 cargo tarpaulin --engine Llvm --workspace`).

**How to "see" a screenshot as an agent:** models here cannot view images. Two working methods:

1. **ASCII downscale** — `python3 .agents/skills/gui-testing/scripts/ascii_view.py <png> [cols]` (stdlib-only; brighter → `| - # @`). Good for layout, panels, dialogs, "is the canvas empty or not".
2. **Pixel scans** — decode the PNG and print alpha/luminance transitions along chosen rows/columns (a ~30-line Python script with `zlib`+`struct`, same decoder as `ascii_view.py`). Good for exact positions: "the selection border starts at x=180", "row y=24 is transparent, y=25 opaque".

Use both together: ASCII view to orient, scans to verify the AC's exact claim.

### Tier 3 — perceptual check (vision model; use sparingly)

Only when tiers 0–2 **genuinely cannot answer the AC**. Requirements:

1. The session model supports images (`"images": true` in `.devcontainer/models.json`) and Unsloth has a vision-capable variant loaded. **Status 2026-10-30:** this session's model cannot view images at all — use tier 2's ASCII view + pixel scans instead; reserve ⚠️ deferred-to-human for truly semantic judgments ("looks professional").
2. If a vision-capable endpoint is available: the vision model runs on **CPU** — every call is slow. Downscale first, then `read` the low-res PNG.
3. **Batch several perceptual assertions into one screenshot** instead of taking many.
4. Save the screenshot as an artifact and cite its path + your observed findings (ASCII excerpt / scan lines) as evidence — a human re-checks the PNG, not the agent's claim.

## Decision table

| AC kind | Tier |
|---|---|
| Model state after a user action (move/resize/connect/delete) | 0 |
| GUI behavior through real egui input (buttons, prompts, inline editor, overlays) | 0b |
| App starts, stays up, no panic on open | 0b (frame pumping; tier 1 unavailable in this image) |
| "Looks like before" — visual regression of a known screen | 2 (software rasterizer dump vs golden) |
| Semantic visual property (legibility, correct position in pixels) | 2 + pixel scans / ASCII view (vision model = tier 3, see below) |

## Evidence rules (story Verification section)

- Every ✅ cites the **command actually run + observed result**: exit code, AE pixel count, or screenshot path. "Tests passed" is not evidence.
- Screenshot artifacts stay on disk (`gui-app/tests/goldens/` for goldens; a story-local path for one-off shots) and are cited by path. `/tmp/gui-dump/*.png` from `visual_dump` is fine as working evidence but is **not durable** — copy anything worth keeping into the repo.
- Only checks that fail **all** tiers close as ⚠️ *deferred to human*, with a concrete checklist written into Verification; every ⚠️ is flagged in the board's progress log. An agent never marks a visual AC ✅ without tier ≥ 2 evidence (an inspected dump: ASCII view excerpt or scan lines proving the claim).

## Determinism notes (stable goldens)

Pixel-exact diffs only work if the app renders identically on every run. The software rasterizer removes most of the usual hazards (no GPU driver, no compositor, no timing) — the harness already pins what matters:

- Fixed viewport: the `Gui` harness pumps at a fixed 1024×768, ppp 1.0.
- Fixed theme: `DrawingApp` sets its visuals explicitly — never follow system settings.
- Deterministic state: each dump state is reached by an explicit event sequence (no timing, no idle sleep needed); the last pumped frame is captured directly.
- Font atlas comes from the image's fontconfig + epaint's rasterizer — stable within one image build; regenerate goldens on eframe/epaint bumps or font package changes.

Known rasterizer conventions (why the PNGs match what a GPU would show): epaint vertex colors are straight sRGBA, untextured vertices sample `uv=(0,0)` which egui keeps as an opaque white texel in the font atlas, and blending is straight-alpha "over". If you change any of this in `screenshot.rs`, re-run the synthetic check (`dbg_raster_synth` pattern: two known rects must land on their exact pixel bounds).

## wasm target

This skill covers the **native** target (headless frame pumping + software rasterizer). The web milestone (S-019) verifies differently: `cargo check -p gui-app --target wasm32-unknown-unknown` + the headless core test suite on wasm — there is no browser in the isolated devcontainer, so no screenshots for wasm.
