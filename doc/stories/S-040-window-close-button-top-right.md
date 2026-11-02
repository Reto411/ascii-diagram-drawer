---
title: S-040 - window-close-button-top-right
description: CR-019 — app-bar window controls ordered minimize, fullscreen, close (close at the top-right corner)
scope: gui-app
status: done
updated: 2026-11-01
---

# S-040: Window close button at the top right (rightmost of the app-bar controls)

## Why

[CR-019](../crs/CR-019-close-button-at-top-right.md) (accepted 2026-11-01, FR14 / [ADR 017 v2](../adr/017-material-design-title-bar.md)): on KDE Plasma — as on Windows — the platform convention places close at the right end of the window controls. Today the custom Material app bar orders the trio **close ×, fullscreen □, minimize ─**, so minimize occupies the top-right corner and the left-end close invites mis-clicks. The buttons shall read left-to-right: minimize, fullscreen, close.

## Dependencies

- S-027 (done) — the app bar this story reorders (`gui-app/src/appbar.rs`, `window_controls`)

## Acceptance criteria

Written from CR-019's requested change (left-to-right order minimize → fullscreen → close, close at the window's top-right corner):

- **AC1** — In a real headless frame (tier-0b), the app-bar control trio renders in left-to-right visual order **minimize ─ → fullscreen □ → close ×**: `x(─) < x(□) < x(×)` with all three inside the app-bar strip, and the close control rightmost — anchored to the window's top-right corner (right-aligned within the bar's edge margin). Pinned by a new tier-0b test.
- **AC2** — Commands and glyphs unchanged (S-027 AC3 stays valid): clicking each control still queues `Minimized(true)` / `Fullscreen(!state)` / `Close` respectively, with the same monospace glyphs ─ □ ×. The existing `window_controls_queue_viewport_commands` test must pass unmodified.
- **AC3** — Material goldens regenerated for the new order and pass pixel-exact (`GOLDEN_UPDATE=1`, then the diff test green).
- **Feature bar** — at least one `ui_decomposition_*` test drawing a representative diagram purely through the real UI (sidebar click → canvas placement, anchor→anchor connect drag, text entry), asserting model state + rendered content, with the app-bar order verified in the same live session — proving the reordered chrome coexists with a fully usable editor.

## Implementation notes

- Root cause confirmed: `window_controls` added the buttons in code order minimize → fullscreen → close *inside a `Layout::right_to_left` scope*, where egui places the first-added widget rightmost — so the rendered order was reversed (close leftmost, minimize at the top-right corner).
- Fix (`gui-app/src/appbar.rs`): the three button blocks are now added in reverse code order — close first, fullscreen second, minimize last — so the visual order is minimize ─ → fullscreen □ → close × with close rightmost. Commands, glyphs, hover texts, and sizes are untouched; a doc comment on `window_controls` pins the reversal rule so it isn't "fixed" back.
- Tests (`gui-app/src/gui_tests.rs`): new tier-0b test `window_controls_render_minimize_fullscreen_close_left_to_right` (AC1) and the feature-bar decomposition `ui_decomposition_window_controls` (two boxes via sidebar → canvas, an anchor→anchor connector drag, a typed text object, then the app-bar order re-verified in the same live session).
- Wording deviation: CR-019's "8 pt margin from the window's right edge" describes the control *group* background; the AC1 assertion targets the glyph rect (panel inner margin + button padding ≈ 20 pt) and pins "right-aligned within the bar's edge margin" instead — same visual fact, robust to layout constants.
- Goldens: both material goldens regenerated (`GOLDEN_UPDATE=1`); `compare -compose difference` shows the change confined to the control glyph row (89×6 pt at x=915, y=16 — top-right corner), nothing else moved.

## Verification

- [x] AC1 — `gui_tests::window_controls_render_minimize_fullscreen_close_left_to_right`: asserts all three glyphs in the app-bar strip with `x(─) < x(□) < x(×)` and close right-aligned to the window edge. Passes (failed before the reorder, as expected when first written).
- [x] AC2 — `gui_tests::window_controls_queue_viewport_commands` passes **unmodified**: each control still queues `Minimized(true)` / `Fullscreen(!state)` / `Close` with the same glyphs.
- [x] AC3 — goldens regenerated via `GOLDEN_UPDATE=1 cargo test -p gui-app material_theme_goldens`; diff test green without the env var; pixel diff vs HEAD confined to the top-right control row (89×6 pt) in both palettes.
- [x] Feature bar — `gui_tests::ui_decomposition_window_controls`: representative diagram (2 boxes + 1 connector + typed text "hello!") drawn purely through the real UI, model state asserted (4 objects, right kinds, typed content committed), canvas lines rendered, app-bar order re-verified in the same session.
- Gates: `cargo fmt --check` clean; `cargo clippy --workspace --all-targets` 0 warnings; workspace tests **693 passed, 0 failed** (gui-app 301); tarpaulin **93.08%** ≥ 80% (baseline 93.07%); mkdocs strict clean.
