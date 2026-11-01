//! S-043 / ADR 018 — property-based testing of the Diagram aggregate over *sequences*
//! of operations.
//!
//! The assertions are **invariants, not particular expected diagrams** (human request
//! 2026-11-01): after every successful operation the model must stay consistent (unique
//! ids, referential integrity of connector endpoints, zero-length rule, positive sizes,
//! scale ≥ 1); every failed operation must leave the diagram byte-for-byte unchanged; and
//! replaying the same sequence twice from an empty diagram must yield equal diagrams
//! (Q1 determinism). The operation vocabulary and invariant checker live in
//! [`diagram_model::testgen`] so other crates' property tests reuse them.

use diagram_model::{
    Diagram,
    testgen::{self, Op},
};
use proptest::prelude::*;

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0u32..32, 0u32..32, 0u32..12, 0u32..12).prop_map(|(x, y, w, h)| Op::PlaceShape {
            x,
            y,
            w,
            h
        }),
        (0u32..32, 0u32..32, any::<bool>()).prop_map(|(x, y, empty)| Op::PlaceText { x, y, empty }),
        (0usize..8, 0u32..40, 0u32..40).prop_map(|(shape_index, x, y)| Op::Move {
            shape_index,
            x,
            y
        }),
        (0usize..8, 0u32..16, 0u32..16).prop_map(|(shape_index, w, h)| Op::Resize {
            shape_index,
            w,
            h
        }),
        (0usize..12).prop_map(|object_index| Op::Remove { object_index }),
        (0usize..8, 0usize..8, any::<usize>(), any::<usize>()).prop_map(|(a, b, ai, bi)| {
            Op::Connect {
                a,
                b,
                anchor_a: testgen::ANCHORS[ai % testgen::ANCHORS.len()],
                anchor_b: testgen::ANCHORS[bi % testgen::ANCHORS.len()],
            }
        }),
        (0usize..6, 0u8..2, 0usize..8, any::<usize>()).prop_map(
            |(conn_index, end, shape_index, ai)| Op::AttachEnd {
                conn_index,
                end,
                shape_index,
                anchor: testgen::ANCHORS[ai % testgen::ANCHORS.len()]
            }
        ),
        (0usize..6, 0u8..2, 0u32..40, 0u32..40).prop_map(|(conn_index, end, x, y)| Op::FreeEnd {
            conn_index,
            end,
            x,
            y
        }),
        (0usize..6, 0usize..4, 0u32..40, 0u32..40).prop_map(|(conn_index, index, x, y)| {
            Op::AddBend {
                conn_index,
                index,
                x,
                y,
            }
        }),
        (0usize..6, 0usize..4, 0u32..40, 0u32..40).prop_map(|(conn_index, index, x, y)| {
            Op::MoveBend {
                conn_index,
                index,
                x,
                y,
            }
        }),
        (0usize..6, 0usize..4).prop_map(|(conn_index, index)| Op::RemoveBend { conn_index, index }),
        (0usize..6, 0u8..2, 0u8..4).prop_map(|(conn_index, end, style)| Op::SetTip {
            conn_index,
            end,
            style
        }),
        (0u32..10).prop_map(|scale| Op::SetScale { scale }),
        (
            any::<u8>(),
            any::<bool>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>()
        )
            .prop_map(|(role, set, r, g, b)| Op::SetColor { role, set, r, g, b }),
    ]
}

/// Places the seed shape (id `obj-1`) that index-based operations can target.
fn seeded() -> Diagram {
    let mut d = Diagram::new();
    testgen::apply(
        &mut d,
        &Op::PlaceShape {
            x: 0,
            y: 0,
            w: 4,
            h: 2,
        },
    )
    .expect("seed shape is valid");
    d
}

proptest! {
    /// AC1 / P1 — after every *successful* operation the aggregate satisfies its
    /// invariants (unique ids, endpoint referential integrity, zero-length rule,
    /// positive sizes, scale ≥ 1). One shape is seeded so connect/attach operations
    /// have real targets early in the sequence.
    #[test]
    fn operation_sequences_keep_model_invariants(ops in prop::collection::vec(op_strategy(), 1..40)) {
        let mut d = seeded();
        for op in &ops {
            if testgen::apply(&mut d, op).is_ok() {
                let violations = testgen::invariant_violations(&d);
                let message = format!("after {op:?}: {violations:?}");
                prop_assert!(violations.is_empty(), "invariant violated: {}", message);
            }
        }
    }

    /// AC1 / P2 — every *failed* operation leaves the diagram exactly as it was
    /// (clone before, `PartialEq` after: no partial mutation on any error path).
    #[test]
    fn failed_operations_leave_the_diagram_unchanged(ops in prop::collection::vec(op_strategy(), 1..40)) {
        let mut d = seeded();
        for op in &ops {
            let before = d.clone();
            if testgen::apply(&mut d, op).is_err() {
                let message = format!("failed operation left the diagram mutated: {op:?}");
                prop_assert_eq!(&d, &before, "{}", message);
            }
        }
    }

    /// AC1 / P3 — Q1 determinism at the model level: replaying the same operation
    /// sequence twice from an empty diagram yields equal diagrams (same generated ids,
    /// same state).
    #[test]
    fn replaying_the_same_sequence_yields_equal_diagrams(ops in prop::collection::vec(op_strategy(), 1..40)) {
        let a = testgen::replay(&ops);
        let b = testgen::replay(&ops);
        prop_assert_eq!(a, b, "same operation sequence produced different diagrams");
    }
}
