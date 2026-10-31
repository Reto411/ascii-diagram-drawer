//! Pure grid↔screen mapping and cell hit-testing for the WYSIWYG canvas (S-012 AC2,
//! S-013 anchor/resize targets).
//!
//! No egui types here — plain `f32`/`u32` math, unit-testable without a window.

use diagram_model::Diagram;
use diagram_model::anchors::AnchorRef;
use diagram_model::objects::{Object, ObjectId, ShapeInstance, TextInstance};
use ui_api::{AnchorInfo, ShapeCatalog};

/// What the pointer is over (S-013). Computed by [`hit_target`] and consumed by the
/// gesture machine in `editor.rs` — anchors and the resize handle take priority over
/// plain object bodies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HitTarget {
    /// Nothing interactive at this cell.
    Empty,
    /// The body of a shape or free-text object (topmost on overlap).
    Body(ObjectId),
    /// A named anchor of a shape — starts/finishes a connection.
    Anchor(ObjectId, String),
    /// The resize handle of the selected shape (its bottom-right cell, v1).
    ResizeHandle(ObjectId),
}

/// Proportional per-axis mapping of a template coordinate onto an instance extent —
/// the exact rule the render pipeline uses for `Rect`-geometry anchors (C9 / ADR 013):
/// natural-size instances map to themselves, side coordinates stay on their side at
/// any size, and ties round up. Kept in lockstep with `render-pipeline::connectors` —
/// if that changes, this must change too (the GUI's anchor dots would drift from the
/// rendered endpoints).
#[must_use]
pub fn map_proportional(template_coord: u32, template_extent: u32, instance_extent: u32) -> u32 {
    if template_extent <= 1 || instance_extent <= 1 {
        return 0;
    }
    let num = template_coord * (instance_extent - 1);
    let den = template_extent - 1;
    // round(num / den), ties up.
    (2 * num + den) / (2 * den)
}

/// The model cell of a named anchor on a placed shape, as the catalog serves it.
///
/// Mirrors C9 / ADR 013: every resizable geometry (`Rect`, `Ellipse`) maps its
/// template anchors proportionally onto the instance extent; at natural size the
/// mapping is the identity — i.e. exactly the pipeline's plain offset. One rule
/// covers all of them.
#[must_use]
pub fn anchor_cell(
    shape: &ShapeInstance,
    catalog: &dyn ShapeCatalog,
    anchor: &AnchorInfo,
) -> Option<(u32, u32)> {
    let template = catalog.get(&shape.type_id)?;
    Some((
        shape.pos.x + map_proportional(anchor.position.x, template.size.width, shape.size.width),
        shape.pos.y + map_proportional(anchor.position.y, template.size.height, shape.size.height),
    ))
}

/// The interactive target under a model cell (S-013).
///
/// - **Connecting** (`connecting_from` set): every anchor except the source itself is a
///   valid drop target — that is what makes "drop outside an anchor cancels" work.
/// - **Otherwise**: the selected shape's anchors and its bottom-right resize cell first,
///   then the topmost shape body (document order = z-order, C5).
#[must_use]
pub fn hit_target(
    diagram: &Diagram,
    catalog: &dyn ShapeCatalog,
    selection: Option<&ObjectId>,
    connecting_from: Option<&AnchorRef>,
    cell: (u32, u32),
) -> HitTarget {
    let (col, row) = cell;
    if let Some(source) = connecting_from {
        for object in diagram.objects() {
            let Object::Shape(shape) = object else {
                continue;
            };
            let Some(anchors) = catalog.anchors(&shape.type_id) else {
                continue;
            };
            for anchor in anchors {
                let is_source = shape.id == *source.object() && anchor.name == source.anchor();
                if !is_source
                    && anchor_cell(shape, catalog, &anchor).is_some_and(|c| c == (col, row))
                {
                    return HitTarget::Anchor(shape.id.clone(), anchor.name);
                }
            }
        }
        return HitTarget::Empty;
    }

    if let Some(selected_id) = selection
        && let Some(Object::Shape(shape)) = diagram.get(selected_id)
    {
        if let Some(anchors) = catalog.anchors(&shape.type_id) {
            for anchor in anchors {
                if anchor_cell(shape, catalog, &anchor).is_some_and(|c| c == (col, row)) {
                    return HitTarget::Anchor(shape.id.clone(), anchor.name);
                }
            }
        }
        // v1: one resize handle — the selected shape's bottom-right cell. Built-ins
        // only: custom shapes are `collection/name` (qualified id) and fixed-size
        // until resize rules land (ADR 007 v3).
        if !shape.type_id.contains('/')
            && col >= shape.pos.x
            && col - shape.pos.x + 1 == shape.size.width
            && row >= shape.pos.y
            && row - shape.pos.y + 1 == shape.size.height
        {
            return HitTarget::ResizeHandle(shape.id.clone());
        }
    }

    diagram
        .objects()
        .iter()
        .rev()
        .find_map(|object| match object {
            Object::Shape(shape) => {
                let within = col >= shape.pos.x
                    && col.saturating_sub(shape.pos.x) < shape.size.width
                    && row >= shape.pos.y
                    && row.saturating_sub(shape.pos.y) < shape.size.height;
                within.then(|| HitTarget::Body(shape.id.clone()))
            }
            // Free text is hit-tested over its stamped extent (S-018).
            Object::Text(text) => {
                let (width, height) = text_hit_extent(text);
                let within = col >= text.pos.x
                    && col.saturating_sub(text.pos.x) < width
                    && row >= text.pos.y
                    && row.saturating_sub(text.pos.y) < height;
                within.then(|| HitTarget::Body(text.id.clone()))
            }
            Object::Connector(_) => None,
        })
        .unwrap_or(HitTarget::Empty)
}

/// The rendered extent of a free-text object in cells (S-018 hit-testing): the bounding
/// box of its stamped lines — C1 F4 quoting included, so clicks on the quote characters
/// land on the text. Width is the longest line, height the line count.
#[must_use]
pub fn text_hit_extent(text: &TextInstance) -> (u32, u32) {
    let lines = text.stamped_lines();
    let width = lines
        .iter()
        .map(|line| line.chars().count() as u32)
        .max()
        .unwrap_or(0);
    (width, lines.len() as u32)
}

/// One primitive of the interactive overlay layer (S-013), in model space — the
/// canvas maps these to pixels. Keeping the *data* here (not in the egui glue) makes
/// "what gets highlighted" unit-testable headlessly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    /// Filled dot at a cell center: an anchor — of the selected shape, or of every
    /// shape while a connection is in flight (the source's own dot is excluded; the
    /// canvas draws it together with the drag line).
    AnchorDot((u32, u32)),
    /// Small square on the selection's bottom-right **cell** — the same cell
    /// [`hit_target`] answers with [`HitTarget::ResizeHandle`], so the marker and the
    /// gesture target coincide (the outer corner cell is reserved for the `s`/`e`
    /// anchor dots of many shapes).
    ResizeHandle((u32, u32)),
}

/// The overlay primitives for the current model + view state (S-013 AC1/AC2):
/// anchor dots on the selected shape — or on every shape while connecting — and the
/// resize handle of the selection. Empty when nothing is selected and no connection
/// is in flight.
#[must_use]
pub fn overlays(
    diagram: &Diagram,
    catalog: &dyn ShapeCatalog,
    selection: Option<&ObjectId>,
    connecting_from: Option<&AnchorRef>,
) -> Vec<Overlay> {
    let mut out = Vec::new();
    for object in diagram.objects() {
        let Object::Shape(shape) = object else {
            continue;
        };
        let selected = selection.is_some_and(|id| id == &shape.id);
        if connecting_from.is_none() && !selected {
            continue; // dots only on the selection (or as connect targets)
        }

        if let Some(anchors) = catalog.anchors(&shape.type_id) {
            for anchor in anchors {
                let is_source = connecting_from
                    .is_some_and(|src| src.object() == &shape.id && src.anchor() == anchor.name);
                if !is_source && let Some(cell) = anchor_cell(shape, catalog, &anchor) {
                    out.push(Overlay::AnchorDot(cell));
                }
            }
        }

        // One resize handle: the selection's bottom-right cell (v1, built-ins only —
        // custom shapes are fixed-size per ADR 007 v3). The drawn marker and the
        // hit-test must agree on this exact cell.
        if selected && connecting_from.is_none() && !shape.type_id.contains('/') {
            out.push(Overlay::ResizeHandle((
                shape.pos.x + shape.size.width - 1,
                shape.pos.y + shape.size.height - 1,
            )));
        }
    }
    out
}

/// The canvas viewport: where cell `(0, 0)` starts on screen and how big one cell is.
///
/// Cells are **rectangular** (`cell_w` × `cell_h`), not square: a monospace glyph's
/// line height is taller than its advance, so a cell must be exactly as wide as the
/// glyph advance and as tall as the font's line height for the rendered rows to match
/// the terminal layout (WYSIWYG). Square cells would make adjacent text rows overlap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// Pixel position of the top-left corner of cell `(0, 0)`, relative to the canvas area.
    pub origin_x: f32,
    /// Pixel position (vertical) of the top-left corner of cell `(0, 0)`.
    pub origin_y: f32,
    /// Pixels per grid column (the monospace glyph advance).
    pub cell_w: f32,
    /// Pixels per grid row (the font's line height at the canvas font size).
    pub cell_h: f32,
    /// FR8 expansion factor — every model cell renders as a `scale × scale` block of
    /// screen cells, so all mappings work in the scaled layout the text is drawn in.
    pub scale: u32,
}

impl Viewport {
    /// A viewport anchored at an explicit canvas-relative origin: model cell `(0, 0)`
    /// starts at `(origin_x, origin_y)`. A margin-based viewport is
    /// `with_origin(margin, margin, …)`; panning (S-022) shifts the origin, left/up
    /// past the margin makes it negative. The mapping math is unchanged and callers
    /// are responsible for clamping the pan itself (nothing exists before cell 0).
    /// `scale < 1` is treated as 1 (the model guarantees ≥ 1).
    #[must_use]
    pub fn with_origin(origin_x: f32, origin_y: f32, cell_w: f32, cell_h: f32, scale: u32) -> Self {
        Self {
            origin_x,
            origin_y,
            cell_w,
            cell_h,
            scale: scale.max(1),
        }
    }

    /// The pixel center of the rendered block of the model cell at `(col, row)` —
    /// where anchor dots and the resize-handle marker are drawn.
    #[must_use]
    pub fn center_of(&self, col: u32, row: u32) -> (f32, f32) {
        let (x, y) = self.cell_origin(col, row);
        let s = self.scale as f32;
        (x + s * self.cell_w / 2.0, y + s * self.cell_h / 2.0)
    }

    /// The top-left pixel of the rendered block of the model cell at `(col, row)`.
    #[must_use]
    pub fn cell_origin(&self, col: u32, row: u32) -> (f32, f32) {
        let s = self.scale as f32;
        (
            self.origin_x + col as f32 * s * self.cell_w,
            self.origin_y + row as f32 * s * self.cell_h,
        )
    }

    /// The pixel rectangle `(x, y, w, h)` covering the rendered blocks of a
    /// `width`×`height` model cell area starting at `(col, row)` (FR8-scaled).
    #[must_use]
    pub fn cell_block(&self, col: u32, row: u32, width: u32, height: u32) -> (f32, f32, f32, f32) {
        let (x, y) = self.cell_origin(col, row);
        let s = self.scale as f32;
        (
            x,
            y,
            width as f32 * s * self.cell_w,
            height as f32 * s * self.cell_h,
        )
    }

    /// Hit-test: the model-space grid cell under a canvas-relative pixel position, or
    /// `None` if the position is before the origin (the margin region). Any sub-cell of
    /// a rendered `scale × scale` block maps back to its model cell. Model space is
    /// unbounded — any non-negative cell is a valid placement target, so there is no
    /// extent argument.
    #[must_use]
    pub fn screen_to_model_cell(&self, x: f32, y: f32) -> Option<(u32, u32)> {
        let s = self.scale as f32;
        let col = ((x - self.origin_x) / (s * self.cell_w)).floor();
        let row = ((y - self.origin_y) / (s * self.cell_h)).floor();
        if col < 0.0 || row < 0.0 {
            return None;
        }
        Some((col as u32, row as u32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_model::geometry::{CellPos, CellSize};
    use ui_api::ShapeInfo;

    /// A catalog carrying the built-in `box` (9×5, standard mid-side anchors) — the seam
    /// fake keeps this test hermetic.
    fn catalog() -> core_services::fakes::FakeCatalog {
        let box_info = ShapeInfo::builtin("box", "Box", CellSize::new(9, 5).expect("valid size"));
        let anchors = [
            ("n", CellPos::new(4, 0)),
            ("s", CellPos::new(4, 4)),
            ("w", CellPos::new(0, 2)),
            ("e", CellPos::new(8, 2)),
        ]
        .into_iter()
        .map(|(name, position)| AnchorInfo {
            name: name.to_owned(),
            position,
        })
        .collect();
        core_services::fakes::FakeCatalog::default().add(box_info, anchors)
    }

    /// A 9×5 `box` at `(x, y)` with the given id.
    fn box_at(id: &str, x: u32, y: u32) -> ShapeInstance {
        ShapeInstance::new(
            ObjectId::new(id).expect("valid id"),
            "box",
            "",
            CellPos::new(x, y),
            CellSize::new(9, 5).expect("valid size"),
        )
        .expect("valid instance")
    }

    fn diagram_with(ids_and_pos: &[(&str, u32, u32)]) -> Diagram {
        let mut diagram = Diagram::new();
        for (id, x, y) in ids_and_pos {
            diagram
                .place_shape(box_at(id, *x, *y))
                .expect("placement is valid");
        }
        diagram
    }

    fn vp() -> Viewport {
        // 8 px margin, 16 px cells, scale ×1.
        Viewport::with_origin(8.0, 8.0, 16.0, 16.0, 1)
    }

    /// A rectangular viewport (8 px margin, 8×16 px cells) — the real canvas shape:
    /// monospace glyphs are taller than they are wide.
    fn vp_rect() -> Viewport {
        Viewport::with_origin(8.0, 8.0, 8.0, 16.0, 1)
    }

    #[test]
    fn viewport_scale_maps_blocks_and_hit_tests_roundtrip() {
        let v = Viewport::with_origin(8.0, 8.0, 16.0, 16.0, 2);
        // Model cell (3, 2) renders as a 2×2 block starting at screen cells (6, 4).
        assert_eq!(v.cell_origin(3, 2), (8.0 + 6.0 * 16.0, 8.0 + 4.0 * 16.0));
        // Anchor dots / the handle marker sit at the block's center.
        assert_eq!(v.center_of(3, 2), (8.0 + 7.0 * 16.0, 8.0 + 5.0 * 16.0));
        // A 4×2 extent at (1, 1) covers model cells [1..5) × [1..3) → screen
        // blocks [2..10) × [2..6).
        let (x, y, w, h) = v.cell_block(1, 1, 4, 2);
        assert_eq!(
            (x, y, w, h),
            (
                8.0 + 2.0 * 16.0,
                8.0 + 2.0 * 16.0,
                4.0 * 2.0 * 16.0,
                2.0 * 2.0 * 16.0
            )
        );
        // Every sub-cell of a rendered block hit-tests back to its model cell.
        for dx in 0..2u32 {
            for dy in 0..2u32 {
                let px = v.cell_origin(3, 2).0 + (dx as f32 + 0.5) * 16.0;
                let py = v.cell_origin(3, 2).1 + (dy as f32 + 0.5) * 16.0;
                assert_eq!(v.screen_to_model_cell(px, py), Some((3, 2)));
            }
        }
        // The margin is still outside the model grid.
        assert_eq!(v.screen_to_model_cell(4.0, 4.0), None);
    }

    #[test]
    fn viewport_scale_one_is_the_identity_mapping() {
        let scaled = Viewport::with_origin(8.0, 8.0, 16.0, 16.0, 1);
        let plain = vp();
        assert_eq!(scaled.cell_origin(4, 2), plain.cell_origin(4, 2));
        assert_eq!(scaled.center_of(4, 2), plain.center_of(4, 2));
        assert_eq!(scaled.cell_block(1, 1, 5, 3), plain.cell_block(1, 1, 5, 3));
    }

    #[test]
    fn panned_viewport_maps_from_a_negative_origin() {
        // Panned right by 40 px and down by 24 px: cell (0,0) starts at (-40, -24)
        // relative to the canvas area (S-022).
        let v = Viewport::with_origin(-40.0, -24.0, 16.0, 16.0, 1);
        assert_eq!(v.cell_origin(0, 0), (-40.0, -24.0));
        // The canvas pixel (0, 0) now sits inside model cell (2, 1): the content
        // moved left/up by exactly the pan.
        assert_eq!(v.screen_to_model_cell(0.0, 0.0), Some((2, 1)));
        // Round trip: cell (5, 3) starts at (-40 + 80, -24 + 48) and every pixel of
        // its block hit-tests back to it.
        assert_eq!(v.cell_origin(5, 3), (40.0, 24.0));
        for dx in [0.0f32, 7.9, 15.9] {
            for dy in [0.0f32, 7.9, 15.9] {
                assert_eq!(v.screen_to_model_cell(40.0 + dx, 24.0 + dy), Some((5, 3)));
            }
        }
        // A panned viewport with a scale factor still maps sub-cells to the block:
        // cell (3, 2) renders as a 2×2 block starting at (-8 + 6·16, -16 + 4·16).
        let s = Viewport::with_origin(-8.0, -16.0, 16.0, 16.0, 2);
        assert_eq!(s.cell_origin(3, 2), (-8.0 + 6.0 * 16.0, -16.0 + 4.0 * 16.0));
        for (dx, dy) in [(0.5f32, 0.5), (15.9, 15.9)] {
            assert_eq!(
                s.screen_to_model_cell(s.cell_origin(3, 2).0 + dx, s.cell_origin(3, 2).1 + dy),
                Some((3, 2))
            );
        }
    }

    #[test]
    fn cell_origin_lands_on_the_grid() {
        let vp = vp();
        assert_eq!(vp.cell_origin(0, 0), (8.0, 8.0));
        assert_eq!(vp.cell_origin(3, 2), (8.0 + 48.0, 8.0 + 32.0));
    }

    #[test]
    fn rectangular_cells_scale_each_axis_independently() {
        let vp = vp_rect(); // 8 px wide, 16 px tall cells
        assert_eq!(vp.cell_origin(5, 3), (8.0 + 40.0, 8.0 + 48.0));
        assert_eq!(vp.cell_block(1, 2, 9, 5), (16.0, 40.0, 72.0, 80.0));
    }

    #[test]
    fn rectangular_hit_test_round_trips_every_cell() {
        let vp = vp_rect();
        for col in 0..12u32 {
            for row in 0..7u32 {
                // Probe the middle of each cell.
                let (x, y) = vp.cell_origin(col, row);
                assert_eq!(
                    vp.screen_to_model_cell(x + 4.0, y + 8.0),
                    Some((col, row)),
                    "cell ({col}, {row})"
                );
            }
        }
    }

    #[test]
    fn cell_block_covers_exactly_the_requested_cells() {
        let vp = vp();
        assert_eq!(vp.cell_block(1, 2, 9, 5), (24.0, 40.0, 144.0, 80.0));
    }

    #[test]
    fn hit_test_round_trips_every_cell() {
        let vp = vp();
        for col in 0..12u32 {
            for row in 0..7u32 {
                // Probe the middle of each cell.
                let (x, y) = vp.cell_origin(col, row);
                assert_eq!(
                    vp.screen_to_model_cell(x + 8.0, y + 8.0),
                    Some((col, row)),
                    "cell ({col}, {row})"
                );
            }
        }
    }

    #[test]
    fn hit_test_is_inclusive_of_the_top_left_edge_and_exclusive_of_the_right_bottom() {
        let vp = vp();
        // Top-left corner of cell (2, 1) is inside it…
        assert_eq!(vp.screen_to_model_cell(40.0, 24.0), Some((2, 1)));
        // …but the right/bottom edge belongs to the next cell.
        assert_eq!(vp.screen_to_model_cell(72.0, 24.0), Some((4, 1)));
        assert_eq!(vp.screen_to_model_cell(40.0, 40.0), Some((2, 2)));
    }

    /// The proportional mapping is the pipeline's rule (C9): identity at natural size,
    /// side coordinates pinned to their side, ties round up, degenerate extents → 0.
    #[test]
    fn map_proportional_matches_the_pipeline_rule() {
        // Natural size: identity.
        assert_eq!(map_proportional(8, 9, 9), 8);
        assert_eq!(map_proportional(2, 5, 5), 2);
        // Side coordinates stay on their side at any size.
        assert_eq!(map_proportional(0, 9, 3), 0);
        assert_eq!(map_proportional(8, 9, 3), 2);
        assert_eq!(map_proportional(4, 5, 2), 1);
        // Ties round up: template 9 → instance 3, coord 4 = midpoint → (4*2)/8 = 1.0…
        assert_eq!(map_proportional(4, 9, 3), 1);
        // Degenerate extents have only one possible cell.
        assert_eq!(map_proportional(8, 9, 1), 0);
        assert_eq!(map_proportional(0, 1, 5), 0);
    }

    /// Anchor cells mirror the pipeline: proportional for resized built-ins, plain
    /// offset at natural size.
    #[test]
    fn anchor_cell_maps_like_the_render_pipeline() {
        let catalog = catalog();
        // Natural-size box at (3,4): e=(8,2) → (11,6).
        let shape = box_at("s1", 3, 4);
        let e = catalog
            .anchors("box")
            .expect("anchors")
            .into_iter()
            .find(|a| a.name == "e")
            .expect("e exists");
        assert_eq!(anchor_cell(&shape, &catalog, &e), Some((11, 6)));

        // Resized box (3×2) at (3,4): e → x = round(8·2/8) = 2, y = round(2·1/4)
        // = 0.5, ties up → 1. So (3+2, 4+1).
        let mut small = shape.clone();
        small.size = CellSize::new(3, 2).expect("valid size");
        assert_eq!(anchor_cell(&small, &catalog, &e), Some((5, 5)));
    }

    /// Selection mode: anchors and the resize handle of the selected shape beat its body;
    /// unselected shapes offer no handles.
    #[test]
    fn hit_target_in_selection_mode() {
        let catalog = catalog();
        let diagram = diagram_with(&[("s1", 0, 0), ("s2", 5, 0)]);
        let s1 = ObjectId::new("s1").expect("valid id");

        // e anchor of the selected s1 at (8,2) — an anchor even though the body is there.
        assert_eq!(
            hit_target(&diagram, &catalog, Some(&s1), None, (8, 2)),
            HitTarget::Anchor(ObjectId::new("s1").expect("valid"), "e".to_owned())
        );
        // Bottom-right cell of s1 = the resize handle (the same cell the overlay
        // draws its marker on).
        assert_eq!(
            hit_target(&diagram, &catalog, Some(&s1), None, (8, 4)),
            HitTarget::ResizeHandle(s1.clone())
        );
        // Interior body cell → the shape itself.
        assert_eq!(
            hit_target(&diagram, &catalog, Some(&s1), None, (3, 2)),
            HitTarget::Body(s1.clone())
        );
        // Empty space.
        assert_eq!(
            hit_target(&diagram, &catalog, Some(&s1), None, (40, 40)),
            HitTarget::Empty
        );

        // s2 is not selected: its e anchor cell (13,2) offers no handle → plain body.
        let s2 = ObjectId::new("s2").expect("valid id");
        assert_eq!(
            hit_target(&diagram, &catalog, Some(&s1), None, (13, 2)),
            HitTarget::Body(s2.clone())
        );
    }

    /// Overlap: the topmost shape body wins (document order = z-order).
    #[test]
    fn hit_target_prefers_the_topmost_shape_body() {
        let catalog = catalog();
        // s2 overlaps s1 at columns 5..8; nothing selected → plain body hit.
        let diagram = diagram_with(&[("s1", 0, 0), ("s2", 5, 0)]);
        assert_eq!(
            hit_target(&diagram, &catalog, None, None, (6, 2)),
            HitTarget::Body(ObjectId::new("s2").expect("valid id"))
        );
    }

    /// Connecting mode: every anchor except the source is a target; anything else is empty.
    #[test]
    fn hit_target_in_connecting_mode() {
        let catalog = catalog();
        let diagram = diagram_with(&[("s1", 0, 0), ("s2", 20, 0)]);
        let source =
            AnchorRef::new(ObjectId::new("s1").expect("valid id"), "e").expect("valid ref");

        // s2's w anchor at (20,2) is a valid target…
        assert_eq!(
            hit_target(&diagram, &catalog, None, Some(&source), (20, 2)),
            HitTarget::Anchor(ObjectId::new("s2").expect("valid"), "w".to_owned())
        );
        // …the source anchor itself is not…
        assert_eq!(
            hit_target(&diagram, &catalog, None, Some(&source), (8, 2)),
            HitTarget::Empty
        );
        // …nor are plain body cells.
        assert_eq!(
            hit_target(&diagram, &catalog, None, Some(&source), (3, 2)),
            HitTarget::Empty
        );
    }

    /// ADR 007 v3: custom shapes (`collection/name`) are fixed-size in v1 — no resize
    /// handle is offered for them, even when selected; their body still selects/moves.
    #[test]
    fn custom_shapes_offer_no_resize_handle() {
        // A custom template from a collection: qualified type id.
        let arrow = ShapeInfo {
            type_id: "mycol/arrow".to_owned(),
            name: "Arrow".to_owned(),
            size: CellSize::new(5, 3).expect("valid size"),
            collection: Some("mycol".to_owned()),
            preview: vec!["..#..".to_owned()],
        };
        let catalog = catalog().add(
            arrow,
            [("e", CellPos::new(4, 1))]
                .into_iter()
                .map(|(name, position)| AnchorInfo {
                    name: name.to_owned(),
                    position,
                })
                .collect(),
        );

        let mut diagram = Diagram::new();
        diagram
            .place_shape(
                ShapeInstance::new(
                    ObjectId::new("a1").expect("valid id"),
                    "mycol/arrow",
                    "",
                    CellPos::new(0, 0),
                    CellSize::new(5, 3).expect("valid size"),
                )
                .expect("valid instance"),
            )
            .expect("placement is valid");
        let a1 = ObjectId::new("a1").expect("valid id");

        // Its bottom-right cell (4,2) selects the body — not a resize handle…
        assert_eq!(
            hit_target(&diagram, &catalog, Some(&a1), None, (4, 2)),
            HitTarget::Body(a1.clone())
        );
        // …and the overlay has dots but no handle.
        let overlays = overlays(&diagram, &catalog, Some(&a1), None);
        assert!(
            !overlays
                .iter()
                .any(|o| matches!(o, Overlay::ResizeHandle(_)))
        );
        assert_eq!(overlays.len(), 1, "just the single anchor dot");
    }

    /// Overlay data (S-013 AC1/AC2): selection → its dots + handle; connecting → every
    /// shape's dots except the source; nothing selected/connecting → empty.
    #[test]
    fn overlays_follow_selection_and_connecting_state() {
        let catalog = catalog();
        let diagram = diagram_with(&[("s1", 0, 0), ("s2", 20, 0)]);

        // Nothing selected, nothing in flight: no overlay at all.
        assert!(overlays(&diagram, &catalog, None, None).is_empty());

        // Selection: s1's four anchor dots + its bottom-right corner handle.
        let s1 = ObjectId::new("s1").expect("valid id");
        let sel = overlays(&diagram, &catalog, Some(&s1), None);
        assert_eq!(
            sel.iter()
                .filter(|o| matches!(o, Overlay::AnchorDot(_)))
                .count(),
            4,
            "the selected shape's four anchors"
        );
        // The handle marker sits on the bottom-right *cell* (8,4) — exactly where
        // `hit_target` answers with a resize gesture.
        assert!(sel.contains(&Overlay::ResizeHandle((8, 4))));
        // s2 (not selected) contributes nothing.
        assert!(
            !sel.iter()
                .any(|o| matches!(o, Overlay::AnchorDot((20..=28, _))))
        );

        // Connecting from s1's e: dots on both shapes, minus the source dot, no handle.
        let source = AnchorRef::new(s1.clone(), "e").expect("valid ref");
        let connecting = overlays(&diagram, &catalog, None, Some(&source));
        assert_eq!(
            connecting
                .iter()
                .filter(|o| matches!(o, Overlay::AnchorDot(_)))
                .count(),
            7,
            "4 + 4 anchors minus the source dot"
        );
        assert!(!connecting.contains(&Overlay::AnchorDot((8, 2))));
        assert!(
            connecting
                .iter()
                .all(|o| !matches!(o, Overlay::ResizeHandle(_)))
        );
    }

    #[test]
    fn hit_test_is_unbounded_but_rejects_the_margin_region() {
        let vp = vp();
        // Far beyond any content — still a valid model cell.
        assert_eq!(
            vp.screen_to_model_cell(10_000.0, 10_000.0),
            Some((624, 624))
        );
        // Left/above the grid origin (margin region) is not.
        assert_eq!(vp.screen_to_model_cell(7.0, 7.0), None);
        assert_eq!(vp.screen_to_model_cell(-1.0, 40.0), None);
    }
}
