//! Read-model types for the shape catalog (ADR 009 seam).
//!
//! `gui-app` may not depend on `shape-registry`, so catalog data crosses the seam as
//! these plain value types — built from registry types by the backend, using
//! `diagram-model`'s geometry vocabulary (the shared kernel of C3).

use diagram_model::geometry::{CellPos, CellSize};

/// A shape type as the GUI needs it: identity + size (S-016 sidebar/placement).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeInfo {
    /// The catalog key: bare name for built-ins, `collection/name` for custom shapes.
    pub type_id: String,
    /// Display name.
    pub name: String,
    /// The shape's footprint — the size of its glyph pattern.
    pub size: CellSize,
}

/// A named anchor point on a shape type (S-013 connectors).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorInfo {
    /// The anchor's name as declared in the template (`n`/`s`/`w`/`e` for built-ins).
    pub name: String,
    /// Position relative to the shape's top-left corner.
    pub position: CellPos,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The read models carry exactly what the GUI needs — construction is exercised
    /// through the backend mapping (core-services); here we pin the geometry types so
    /// a rename in `diagram-model` fails at the seam, not in the GUI.
    #[test]
    fn read_models_use_the_model_geometry_vocabulary() {
        let info = ShapeInfo {
            type_id: "box".into(),
            name: "Box".into(),
            size: CellSize::new(9, 5).expect("valid size"),
        };
        assert_eq!(info.size.width, 9);
        assert_eq!(info.size.height, 5);

        let anchor = AnchorInfo {
            name: "e".into(),
            position: CellPos::new(8, 2),
        };
        assert_eq!((anchor.position.x, anchor.position.y), (8, 2));
    }
}
