//! Presentation layer — [ADR 008](../doc/adr/008-gui-framework-choice.md).
//!
//! The eframe/egui application (native desktop + browser wasm). Scaffold stub:
//! the real app lands in S-012. May depend only on `ui-api`, `core-services` and
//! `diagram-model` value types — never on the other context crates directly.

#![allow(
    clippy::missing_const_for_fn,
    reason = "nursery false positive: `main` is a program entry point, not a const candidate"
)]

fn main() {
    // Intentionally empty until S-012 wires up eframe.
}
