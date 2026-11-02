//! Deterministic diagram fixtures for benchmarks and determinism invariants (S-020).
//!
//! [`mixed_diagram`] builds an `n`-object diagram from a fixed seed: the same call
//! yields the same model on every platform, at any time — no RNG-system dependence.
//! Objects are laid out on a 25×20 grid of 8×5 cells with positions clamped so the
//! rendered grid stays inside the 200×100-cell envelope that P1/QS12 (§5.3/§6)
//! reference; dense placement means overlapping shapes, which the pipeline handles.

use diagram_model::{
    Diagram,
    anchors::AnchorRef,
    geometry::{CellPos, CellSize},
    objects::{ConnectorInstance, ConnectorStyle, ObjectId, ShapeInstance, TextInstance},
};

/// SplitMix64 — deterministic on every platform (plain integer arithmetic).
struct Lcg(u64);

impl Lcg {
    /// Uniform value in `0..range` (panics for `range == 0`).
    fn below(&mut self, range: u64) -> u64 {
        assert!(range > 0, "range must be positive");
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        (z ^ (z >> 31)) % range
    }
}

/// The fixed seed — changing it invalidates recorded benchmark baselines.
const SEED: u64 = 0xA5C1_2026;
/// Shape kinds cycled through the fixture (built-in type ids).
const KINDS: [&str; 3] = ["box", "ellipse", "class"];
/// The P1/QS12 envelope.
const ENVELOPE_W: u32 = 200;
const ENVELOPE_H: u32 = 100;

/// Builds a deterministic `n`-object diagram: a mix of shapes (box/ellipse/class),
/// free text and solid connectors between consecutive shapes, laid out inside the
/// 200×100-cell envelope.
#[must_use]
pub fn mixed_diagram(n_objects: u32) -> Diagram {
    let mut rng = Lcg(SEED);
    let n_objects = n_objects.max(1);
    // Roughly one connector per six objects; the rest are shapes/text.
    let n_connectors = (n_objects / 6).max(1).min(n_objects - 1);
    let n_placed = n_objects - n_connectors;

    let mut diagram = Diagram::new();
    let mut shape_ids: Vec<ObjectId> = Vec::new();

    for i in 0..n_placed {
        let col = i % 25;
        let row = i / 25;
        if i % 5 == 3 {
            // Free text object (one line, short label — always inside the envelope).
            let id = diagram.new_object_id();
            let text = format!("t{i}");
            let x = (col * 8 + rng.below(2) as u32).min(ENVELOPE_W - text.len() as u32);
            let y = (row * 5 + rng.below(2) as u32).min(ENVELOPE_H - 1);
            diagram
                .place_text(TextInstance::new(id, CellPos::new(x, y), &text).expect("non-empty"))
                .expect("fits");
        } else {
            let kind = KINDS[(i % 3) as usize];
            let (w, h) = match kind {
                "box" => (8 + rng.below(5) as u32, 3 + rng.below(2) as u32),
                "ellipse" => (7 + rng.below(4) as u32, 3 + rng.below(2) as u32),
                // The class keeps its natural extent (S-042).
                _ => (13, 10),
            };
            let x = (col * 8 + rng.below(2) as u32).min(ENVELOPE_W - w);
            let y = (row * 5 + rng.below(2) as u32).min(ENVELOPE_H - h);
            let id = diagram.new_object_id();
            let shape = ShapeInstance::new(
                id.clone(),
                kind,
                &format!("{kind}-{i}"),
                CellPos::new(x, y),
                CellSize::new(w, h).expect("non-zero"),
            )
            .expect("valid shape");
            diagram.place_shape(shape).expect("fits");
            shape_ids.push(id);
        }
    }

    // Solid connectors between consecutive shapes (east anchor → west anchor),
    // one per `n_connectors` slots, spread across the placed shapes.
    for k in 0..n_connectors as usize {
        if shape_ids.len() < 2 {
            break;
        }
        let a = (k * (shape_ids.len() - 1) / n_connectors as usize).min(shape_ids.len() - 1);
        let b = (a + 1).min(shape_ids.len() - 1);
        let id = diagram.new_object_id();
        let connector = ConnectorInstance::between(
            id,
            AnchorRef {
                anchor: "e".into(),
                object: shape_ids[a].clone(),
            },
            AnchorRef {
                anchor: "w".into(),
                object: shape_ids[b].clone(),
            },
            ConnectorStyle::Solid,
        );
        diagram.connect(connector).expect("valid endpoints");
    }

    diagram
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fixture is deterministic: same seed → equal models (QS1 input side).
    #[test]
    fn same_seed_yields_equal_models() {
        assert_eq!(mixed_diagram(50), mixed_diagram(50));
        assert_eq!(mixed_diagram(500), mixed_diagram(500));
    }

    /// The object count is exactly `n` for representative sizes.
    #[test]
    fn object_count_is_exact() {
        for n in [1u32, 7, 60, 500, 1000] {
            assert_eq!(mixed_diagram(n).objects().len(), n as usize, "n = {n}");
        }
    }

    /// 500 objects fit the 200×100-cell envelope referenced by P1/QS12.
    #[test]
    fn five_hundred_objects_fit_the_envelope() {
        let grid = crate::render(
            &mixed_diagram(500),
            &shape_registry::ShapeRegistry::builtin(),
        )
        .expect("renders");
        assert!(grid.width() <= ENVELOPE_W, "width {} > 200", grid.width());
        assert!(
            grid.height() <= ENVELOPE_H,
            "height {} > 100",
            grid.height()
        );
    }

    /// Even the smallest diagrams are valid (no division-by-zero in the layout).
    #[test]
    fn tiny_diagrams_are_valid() {
        for n in [1u32, 2, 3] {
            let d = mixed_diagram(n);
            assert!(!d.objects().is_empty());
        }
    }
}
