//! Read-model types for the shape catalog (ADR 009 seam).
//!
//! `gui-app` may not depend on `shape-registry`, so catalog data crosses the seam as
//! these plain value types — built from registry types by the backend, using
//! `diagram-model`'s geometry vocabulary (the shared kernel of C3).

use diagram_model::Diagram;
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
    /// Group label for the sidebar (ADR 009 v5): `None` = built-in group, `Some(name)`
    /// = a loaded collection. Custom shapes are always qualified (`collection/name`).
    pub collection: Option<String>,
    /// The glyph pattern rows, verbatim — the sidebar draws them as a small preview
    /// (ADR 009 v5). Same bytes the renderer stamps on placement.
    pub preview: Vec<String>,
    /// Whether instances of this shape can be resized (S-024 AC5): built-ins and
    /// rule-bearing custom shapes are resizable; plain custom shapes are not (FR12).
    pub resizable: bool,
    /// The smallest size an instance may reach on each axis — the resize gesture's
    /// clamp (S-024 AC5). Built-ins: 1×1; rule-bearing customs: computed by pure
    /// shrink simulation; plain customs: their natural size (the only valid one).
    pub min_size: CellSize,
    /// The template's declared resize rules, when it has any (S-024) — the editor
    /// restores them when opening an existing rule-bearing template for editing,
    /// so saving does not silently drop them.
    pub resize_rules: Option<ResizeRuleSpec>,
    /// The template's declared text regions (FR19 / CR-016), sorted by name — the
    /// editor restores them when reopening a region-bearing template, and double-
    /// clicking an instance of it offers one field per region (S-037 AC5).
    pub regions: Vec<RegionInfo>,
}

impl ShapeInfo {
    /// A built-in shape with no preview rows (test/placeholder construction).
    #[must_use]
    pub fn builtin(type_id: impl Into<String>, name: impl Into<String>, size: CellSize) -> Self {
        Self {
            type_id: type_id.into(),
            name: name.into(),
            size,
            collection: None,
            preview: Vec::new(),
            resizable: true,
            min_size: CellSize::new(1, 1).expect("1×1 is a valid cell size"),
            resize_rules: None,
            regions: Vec::new(),
        }
    }
}

/// A per-edge resize rule as the GUI submits it (S-024 AC4) — plain seam data
/// mirroring `shape_registry::EdgeRule`; no registry type crosses the seam (C3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeRuleSpec {
    /// Character inserted when the shape grows on this edge.
    pub insert: char,
    /// Character removed when it shrinks — a removed cell differing from this is an error.
    pub remove: char,
    /// Override pair used when the target extent on that axis is even (C10 parity).
    pub even: Option<ParityCharsSpec>,
    /// Override pair used when the target extent is odd.
    pub odd: Option<ParityCharsSpec>,
}

/// A parity-variant character pair for one edge (S-024 AC4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParityCharsSpec {
    /// Insert character for this parity.
    pub insert: char,
    /// Remove character for this parity.
    pub remove: char,
}

/// A complete resize rule set — all four edges (S-024 AC4; a rule set is complete by
/// construction, mirroring `shape_registry::ResizeRules`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResizeRuleSpec {
    /// Rule for the top edge.
    pub top: EdgeRuleSpec,
    /// Rule for the bottom edge.
    pub bottom: EdgeRuleSpec,
    /// Rule for the left edge.
    pub left: EdgeRuleSpec,
    /// Rule for the right edge.
    pub right: EdgeRuleSpec,
}

/// A named anchor point on a shape type (S-013 connectors).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorInfo {
    /// The anchor's name as declared in the template (`n`/`s`/`w`/`e` for built-ins).
    pub name: String,
    /// Position relative to the shape's top-left corner.
    pub position: CellPos,
}

/// A text region (text box) declared by a template (FR19 / CR-016): a named rect in
/// template cells; instances carry per-region content that renders inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionInfo {
    /// The region's name as declared in the template — a single token, unique per
    /// template.
    pub name: String,
    /// Top-left corner relative to the shape's top-left corner.
    pub pos: CellPos,
    /// Extent in cells (width/height ≥ 1).
    pub size: CellSize,
}

impl RegionInfo {
    /// Whether the cell `(x, y)` falls inside this region.
    #[must_use]
    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.pos.x
            && y >= self.pos.y
            && x < self.pos.x.saturating_add(self.size.width)
            && y < self.pos.y.saturating_add(self.size.height)
    }

    /// Whether this region intersects the half-open rect `x0..x1 × y0..y1` (template
    /// cells).
    #[must_use]
    pub fn intersects(&self, x0: u32, y0: u32, x1: u32, y1: u32) -> bool {
        self.pos.x < x1
            && self.pos.y < y1
            && x0 < self.pos.x.saturating_add(self.size.width)
            && y0 < self.pos.y.saturating_add(self.size.height)
    }
}

/// A new or edited user-defined template, as the GUI submits it (S-017, FR3/FR11).
///
/// Plain seam-safe data — the backend validates it into a registry `ShapeType`
/// (printable-ASCII cells, in-bounds unique anchors) and persists it to the
/// collection's `.adc` file. `rows` may be ragged; the backend right-pads them to
/// the longest row (the parser's rule), so the editor only ever tracks typed chars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTemplate {
    /// The template name — a single token, unique within its collection.
    pub name: String,
    /// The glyph grid rows, top to bottom; cells are printable ASCII (space = empty).
    pub rows: Vec<String>,
    /// Declared anchors; at least one is required for saving (S-017 AC2).
    pub anchors: Vec<AnchorInfo>,
    /// Optional resize rules (S-024 AC4) — present iff the editor's resize toolset
    /// is enabled. The backend validates completeness (all four edges' base rules)
    /// and character constraints, then persists them as `.adc` `resize` statements.
    pub resize_rules: Option<ResizeRuleSpec>,
    /// Declared text regions (FR19 / CR-016) — each rect must fit inside the glyph
    /// grid; names are single tokens, unique per template. Persisted as `.adc`
    /// `region` statements.
    pub regions: Vec<RegionInfo>,
}

/// A loaded `.ad` document plus forward-compat metadata (ADR 009 v4, RS2):
/// how many unknown statements of a newer minor version the reader skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedDocument {
    /// The parsed diagram content (scale + objects in file order).
    pub diagram: Diagram,
    /// Unknown statements skipped at load; they are **not** part of `diagram` and
    /// would be lost if the document is saved. `0` = lossless load.
    pub skipped_unknowns: usize,
}

impl LoadedDocument {
    /// A load that dropped nothing (the common case, incl. in-memory stores).
    #[must_use]
    pub fn lossless(diagram: Diagram) -> Self {
        Self {
            diagram,
            skipped_unknowns: 0,
        }
    }
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
            collection: None,
            preview: vec!["+-------+".to_owned()],
            resizable: true,
            min_size: CellSize::new(1, 1).expect("valid size"),
            resize_rules: None,
            regions: Vec::new(),
        };
        assert_eq!(info.size.width, 9);
        assert_eq!(info.size.height, 5);

        // ADR 009 v5: the sidebar group + preview fields — built-ins have no collection.
        let custom = ShapeInfo::builtin("col/star", "Star", CellSize::new(3, 3).expect("valid"));
        assert_eq!(custom.collection, None);
        assert!(custom.preview.is_empty());

        let anchor = AnchorInfo {
            name: "e".into(),
            position: CellPos::new(8, 2),
        };
        assert_eq!((anchor.position.x, anchor.position.y), (8, 2));
    }
}
