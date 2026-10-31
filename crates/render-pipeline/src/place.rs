//! The place stage: stamp objects at their stored coordinates in document order
//! ([render pipeline](../../../doc/arc42/12-concepts/render-pipeline.md), C2).
//!
//! There is no layout pass — every object lands exactly where the model says.
//! Document order is z-order (later = on top, C6). Stamping rules:
//!
//! - `Rect` shapes (built-in box): borders are edge connections resolved at the
//!   instance's own pos/size, so instances of any size join cleanly; the label
//!   centers in the strict interior.
//! - `Ellipse` shapes (built-in ellipse/circle): the outline is re-rasterized
//!   deterministically at the instance's own pos/size (9×5 reproduces the natural
//!   template byte-for-byte, B-001); anchors map proportionally like `Rect`; the label
//!   centers over the full instance extent and may only land on space cells.
//! - `Pattern` shapes (all custom templates): the glyph pattern is stamped opaquely at
//!   its natural size — non-space cells verbatim, space cells transparent; anchor cells
//!   additionally carry *passive* base edges derived from their drawn neighbors (for
//!   connector joining in S-007); the label centers over the full pattern extent and may
//!   only land on space cells.
//! - Free text: stamped verbatim at its position; lines containing svgbob-alphabet
//!   characters are wrapped in quotes so they read as literal text in SVG output (C1).
//! - Connectors (ADR 013): endpoints re-resolved from anchor references, routed straight
//!   or single-corner L, stamped as structural edges with an arrowhead at the target end.
//!   See [`crate::connectors`].

use diagram_model::Diagram;
use diagram_model::objects::{Object, ShapeInstance, TextInstance};
use shape_registry::CellPos;
use shape_registry::{GeometryKind, ShapeRegistry, ShapeType, TemplateGrid};

use crate::edges::EdgeSet;
use crate::ellipse::rasterize_ellipse;
use crate::{RenderError, is_line_glyph, is_svgbob_glyph};

/// What one cell knows after the place stage (before glyph resolution, C1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CellState {
    /// Union of all edge connections stamped on this cell (structural; merges across objects).
    pub(crate) edges: EdgeSet,
    /// The last non-space character written by a content writer (shape art, label, text).
    pub(crate) content: Option<char>,
    /// Whether the last writer was structural (edges) rather than content. Resolution
    /// prefers content unless this is set — that is z-order for ambiguous cells (C1 F1/F2).
    pub(crate) structural_top: bool,
}

impl CellState {
    const fn empty() -> Self {
        Self {
            edges: EdgeSet::NONE,
            content: None,
            structural_top: false,
        }
    }

    /// Whether the cell will contribute a non-space glyph or edges to the output.
    pub(crate) fn occupied(&self) -> bool {
        !self.edges.is_empty() || self.content.is_some()
    }

    /// A content writer stamps a non-space glyph (z-order: becomes the top).
    pub(crate) fn stamp_content(&mut self, c: char) {
        self.content = Some(c);
        self.structural_top = false;
    }

    /// A content writer stamps a glyph together with passive base edges (anchor cell):
    /// the edges are recorded for future connector joining but do not claim the top.
    fn stamp_content_with_base_edges(&mut self, c: char, base: EdgeSet) {
        self.content = Some(c);
        self.edges |= base;
        self.structural_top = false;
    }

    /// A structural writer stamps edges (borders, connectors) and claims the top.
    pub(crate) fn stamp_edges(&mut self, edges: EdgeSet) {
        debug_assert!(
            !edges.is_empty(),
            "structural stamps carry at least one edge"
        );
        self.edges |= edges;
        self.structural_top = true;
    }
}

/// The placed (unresolved) grid. Grows on demand while stamping; the resolve stage
/// trims it to the bounding box of occupied cells (the rendered extent, C6).
#[derive(Debug)]
pub(crate) struct PlacedGrid {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) cells: Vec<CellState>,
}

impl PlacedGrid {
    fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            cells: Vec::new(),
        }
    }

    /// Applies `f` to the cell at `(x, y)`, growing the grid as needed.
    pub(crate) fn with_cell(&mut self, x: u32, y: u32, f: impl FnOnce(&mut CellState)) {
        if x >= self.width {
            self.grow_width(x + 1);
        }
        if y >= self.height {
            self.grow_height(y + 1);
        }
        f(&mut self.cells[(y as usize) * self.width as usize + x as usize]);
    }

    fn grow_width(&mut self, new_width: u32) {
        let (w, h) = (self.width as usize, self.height as usize);
        let add = (new_width - self.width) as usize;
        let mut next = Vec::with_capacity((new_width as usize) * h);
        for y in 0..h {
            next.extend_from_slice(&self.cells[y * w..(y + 1) * w]);
            next.resize(next.len() + add, CellState::empty());
        }
        self.cells = next;
        self.width = new_width;
    }

    fn grow_height(&mut self, new_height: u32) {
        let w = self.width as usize;
        let add = ((new_height - self.height) as usize) * w;
        self.cells
            .resize(self.cells.len() + add, CellState::empty());
        self.height = new_height;
    }

    /// The cell state at `(x, y)` (in bounds).
    pub(crate) fn cell(&self, x: u32, y: u32) -> &CellState {
        &self.cells[(y as usize) * self.width as usize + x as usize]
    }
}

/// Stamps every object at its stored coordinates in document order (no layout pass).
/// Unknown shape types and unresolvable connector anchors are errors.
pub(crate) fn place(
    diagram: &Diagram,
    registry: &ShapeRegistry,
) -> Result<PlacedGrid, RenderError> {
    let mut grid = PlacedGrid::new();
    for object in diagram.objects() {
        match object {
            Object::Shape(shape) => place_shape(&mut grid, shape, registry)?,
            Object::Text(text) => place_text_object(&mut grid, text),
            Object::Connector(connector) => {
                crate::connectors::place_connector(&mut grid, diagram, registry, connector)?
            }
        }
    }
    Ok(grid)
}

/// Looks up the shape type for an instance; unknown type ids are a typed error.
pub(crate) fn lookup_shape<'a>(
    instance: &ShapeInstance,
    registry: &'a ShapeRegistry,
) -> Result<&'a ShapeType, RenderError> {
    registry
        .lookup(&instance.type_id)
        .map_err(|_| RenderError::UnknownShapeType {
            type_id: instance.type_id.clone(),
        })
}

fn place_shape(
    grid: &mut PlacedGrid,
    instance: &ShapeInstance,
    registry: &ShapeRegistry,
) -> Result<(), RenderError> {
    let shape = lookup_shape(instance, registry)?;
    match shape.geometry {
        GeometryKind::Rect => place_rect(grid, instance),
        GeometryKind::Ellipse => place_ellipse(grid, instance, shape),
        GeometryKind::Pattern => place_pattern(grid, instance, shape),
    }
    Ok(())
}

/// Stamps an `Ellipse` shape: the outline re-rasterized at the instance's own pos/size
/// (deterministic — [`rasterize_ellipse`]), passive base edges on anchor cells that map
/// onto drawn art, label over the space cells of the rasterized extent.
fn place_ellipse(grid: &mut PlacedGrid, instance: &ShapeInstance, shape: &ShapeType) {
    let px = instance.pos.x;
    let py = instance.pos.y;
    let w = instance.size.width;
    let h = instance.size.height;
    if w == 0 || h == 0 {
        return;
    }
    let glyph = &shape.glyph_pattern;
    let art = rasterize_ellipse(w, h);

    // 1. Opaque art stamp: non-space cells verbatim (they do not participate in glyph
    //    resolution — ADR 007); space cells are transparent.
    for dy in 0..h {
        for dx in 0..w {
            if let Some(c) = art.get(CellPos::new(dx, dy))
                && c != ' '
            {
                grid.with_cell(px + dx, py + dy, |s| s.stamp_content(c));
            }
        }
    }

    // 2. Anchor base edges (passive): anchors are declared in template coordinates and
    //    map proportionally onto the instance extent (identity at natural size); recorded
    //    on anchor cells that carry art so a connector binding there merges with the
    //    drawn neighbors. Invisible while unbound — content wins at resolution.
    for (_name, apos) in shape.anchor_set.iter() {
        let mapped = CellPos::new(
            map_proportional(apos.x, glyph.width(), w),
            map_proportional(apos.y, glyph.height(), h),
        );
        if let Some(c) = art.get(mapped)
            && c != ' '
        {
            let base = anchor_base_edges(&art, mapped);
            grid.with_cell(px + mapped.x, py + mapped.y, |s| {
                s.stamp_content_with_base_edges(c, base)
            });
        }
    }

    // 3. The label centers over the full instance extent; only space cells may receive it.
    stamp_label(
        grid,
        &instance.label,
        LabelRegion::Pattern {
            glyph: &art,
            px,
            py,
        },
    );
}

/// Proportional per-axis mapping of a template coordinate onto an instance extent
/// (shared with connector endpoint resolution in [`crate::connectors`]): natural-size
/// instances map to themselves, side coordinates stay on their side at any size, and
/// ties round up.
pub(crate) fn map_proportional(
    template_coord: u32,
    template_extent: u32,
    instance_extent: u32,
) -> u32 {
    if template_extent <= 1 || instance_extent <= 1 {
        return 0;
    }
    let num = template_coord * (instance_extent - 1);
    let den = template_extent - 1;
    // round(num / den), ties up.
    (2 * num + den) / (2 * den)
}

/// Stamps a `Rect` shape: borders from the instance's own pos/size (any size), label
/// in the strict interior. A zero-width or zero-height instance draws nothing.
fn place_rect(grid: &mut PlacedGrid, instance: &ShapeInstance) {
    let px = instance.pos.x;
    let py = instance.pos.y;
    let w = instance.size.width;
    let h = instance.size.height;
    if w == 0 || h == 0 {
        return;
    }
    for dy in 0..h {
        for dx in 0..w {
            let mut edges = EdgeSet::NONE;
            // Top and bottom sides run east-west; left and right sides run north-south.
            // Corners carry all four directions and resolve to `+` naturally.
            if (dy == 0 || dy == h - 1) && (dx == 0 || dx == w - 1) {
                edges |= EdgeSet::ALL;
            } else if dy == 0 || dy == h - 1 {
                edges |= EdgeSet::E | EdgeSet::W;
            } else if dx == 0 || dx == w - 1 {
                edges |= EdgeSet::N | EdgeSet::S;
            }
            // Interior cells carry no edges and are left untouched (transparent).
            if !edges.is_empty() {
                grid.with_cell(px + dx, py + dy, |c| c.stamp_edges(edges));
            }
        }
    }
    // The label needs a strict interior to live in.
    if w >= 3 && h >= 3 {
        stamp_label(
            grid,
            &instance.label,
            LabelRegion::Interior {
                x0: px + 1,
                y0: py + 1,
                w: w - 2,
                h: h - 2,
            },
        );
    }
}

/// Stamps a `Pattern` shape: opaque art at natural size (space cells transparent),
/// passive base edges on anchor cells that carry art, label over the space cells.
fn place_pattern(grid: &mut PlacedGrid, instance: &ShapeInstance, shape: &ShapeType) {
    let px = instance.pos.x;
    let py = instance.pos.y;
    let glyph = &shape.glyph_pattern;
    let tw = glyph.width();
    let th = glyph.height();

    // 1. Opaque art stamp: non-space cells verbatim (they do not participate in
    //    glyph resolution — ADR 007); space cells are transparent.
    for dy in 0..th {
        for dx in 0..tw {
            if let Some(c) = glyph.get(CellPos::new(dx, dy))
                && c != ' '
            {
                grid.with_cell(px + dx, py + dy, |s| s.stamp_content(c));
            }
        }
    }

    // 2. Anchor base edges (passive): recorded on anchor cells that carry art so a
    //    connector binding there in S-007 merges with the drawn neighbors. Invisible
    //    while unbound — content wins at resolution. Anchors on space cells (or out of
    //    bounds) are stamped nothing: they become live only when a connector binds.
    for (_name, apos) in shape.anchor_set.iter() {
        if let Some(c) = glyph.get(*apos)
            && c != ' '
        {
            let base = anchor_base_edges(glyph, *apos);
            grid.with_cell(px + apos.x, py + apos.y, |s| {
                s.stamp_content_with_base_edges(c, base)
            });
        }
    }

    // 3. The label centers over the full pattern extent; only space cells may receive it.
    stamp_label(
        grid,
        &instance.label,
        LabelRegion::Pattern { glyph, px, py },
    );
}

/// The passive base edges of an anchor cell: one direction per drawn neighbor
/// (a neighbor is "drawn" when it holds a line glyph of the svgbob alphabet).
fn anchor_base_edges(glyph: &TemplateGrid, pos: CellPos) -> EdgeSet {
    let mut edges = EdgeSet::NONE;
    for (dir, dx, dy) in [
        (EdgeSet::N, 0i32, -1i32),
        (EdgeSet::E, 1i32, 0i32),
        (EdgeSet::S, 0i32, 1i32),
        (EdgeSet::W, -1i32, 0i32),
    ] {
        let nx = pos.x as i32 + dx;
        let ny = pos.y as i32 + dy;
        if nx >= 0
            && ny >= 0
            && let Some(c) = glyph.get(CellPos::new(nx as u32, ny as u32))
            && is_line_glyph(c)
        {
            edges |= dir;
        }
    }
    edges
}

/// Stamps a free-text object: each stamped line (C1 F4 quoting applied —
/// [`TextInstance::stamped_lines`]) at `pos + (column, row)`.
fn place_text_object(grid: &mut PlacedGrid, text: &TextInstance) {
    for (i, line) in text.stamped_lines().iter().enumerate() {
        for (j, c) in line.chars().enumerate() {
            if c != ' ' {
                grid.with_cell(text.pos.x + j as u32, text.pos.y + i as u32, |s| {
                    s.stamp_content(c)
                });
            }
        }
    }
}

/// Where a shape label may land: the strict interior of a `Rect` border, or the space
/// cells of a `Pattern` template at its instance position.
enum LabelRegion<'a> {
    Interior {
        x0: u32,
        y0: u32,
        w: u32,
        h: u32,
    },
    Pattern {
        glyph: &'a TemplateGrid,
        px: u32,
        py: u32,
    },
}

impl<'a> LabelRegion<'a> {
    /// The bounding box (in absolute cells) used for centering arithmetic.
    fn bounds(&self) -> (u32, u32, u32, u32) {
        match self {
            Self::Interior { x0, y0, w, h } => (*x0, *y0, *w, *h),
            Self::Pattern { glyph, px, py } => (*px, *py, glyph.width(), glyph.height()),
        }
    }

    /// Whether a label character may be stamped at `(x, y)`.
    fn allows(&self, x: u32, y: u32) -> bool {
        match self {
            Self::Interior { x0, y0, w, h } => {
                x >= *x0 && x < x0.saturating_add(*w) && y >= *y0 && y < y0.saturating_add(*h)
            }
            Self::Pattern { glyph, px, py } => {
                let dx = x.saturating_sub(*px);
                let dy = y.saturating_sub(*py);
                matches!(glyph.get(CellPos::new(dx, dy)), Some(' '))
            }
        }
    }
}

/// Stamps a (possibly multi-line) label centered in its region. Lines that overflow the
/// region are clipped deterministically (C1 F3); quoted lines that do not fit whole fall
/// back to the raw line (C1 F4). Spaces never stamp — they are transparent everywhere.
fn stamp_label(grid: &mut PlacedGrid, label: &str, region: LabelRegion<'_>) {
    let (bx0, by0, bw, bh) = region.bounds();
    let lines: Vec<&str> = label.split('\n').collect();
    let y_off = ((bh as i64) - lines.len() as i64).div_euclid(2);

    for (i, line) in lines.iter().enumerate() {
        let abs_y = by0 as i64 + y_off + i as i64;
        if abs_y < 0 || abs_y >= by0.saturating_add(bh) as i64 {
            continue; // the whole line is outside the region
        }
        let y = abs_y as u32;

        let effective = quoted_if_needed(line);
        // F4: if quoting was applied but the quoted run does not fit the region whole,
        // stamp the raw line instead (deterministic fallback).
        let fits = |s: &str| -> bool {
            let x_off = (bw as i64 - s.chars().count() as i64).div_euclid(2);
            s.chars().enumerate().all(|(j, c)| {
                if c == ' ' {
                    return true;
                }
                let abs_x = bx0 as i64 + x_off + j as i64;
                abs_x >= 0 && region.allows(abs_x as u32, y)
            })
        };
        let stamp_line: &str = if effective != *line && !fits(&effective) {
            line
        } else {
            effective.as_str()
        };

        let x_off = (bw as i64 - stamp_line.chars().count() as i64).div_euclid(2);
        for (j, c) in stamp_line.chars().enumerate() {
            if c == ' ' {
                continue; // spaces are transparent
            }
            let abs_x = bx0 as i64 + x_off + j as i64;
            if abs_x < 0 {
                continue;
            }
            let x = abs_x as u32;
            if region.allows(x, y) {
                grid.with_cell(x, y, |s| s.stamp_content(c));
            }
        }
    }
}

/// Wraps a line in double quotes when it contains any svgbob-alphabet character, so the
/// characters read as literal text in SVG output (C1). Lines without alphabet characters
/// are returned unchanged.
fn quoted_if_needed(line: &str) -> String {
    if line.chars().any(is_svgbob_glyph) {
        format!("\"{line}\"")
    } else {
        line.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Model-space geometry (the explicit import shadows the registry types from `super::*`);
    // template/anchor coordinates in these tests are qualified as `shape_registry::CellPos`.
    use diagram_model::Diagram;
    use diagram_model::geometry::{CellPos, CellSize};
    use diagram_model::objects::ObjectId;

    fn box_at(x: u32, y: u32, w: u32, h: u32) -> ShapeInstance {
        box_at_id(0, x, y, w, h)
    }

    /// A box with an explicit object id (diagram ids must be unique per diagram).
    fn box_at_id(id: u32, x: u32, y: u32, w: u32, h: u32) -> ShapeInstance {
        ShapeInstance::new(
            ObjectId::generated(id),
            "box",
            "",
            CellPos::new(x, y),
            CellSize::new(w, h).expect("non-zero test size"),
        )
        .expect("valid shape")
    }

    fn ellipse_at(x: u32, y: u32) -> ShapeInstance {
        ShapeInstance::new(
            ObjectId::generated(1),
            "ellipse",
            "",
            CellPos::new(x, y),
            CellSize::new(9, 5).expect("non-zero test size"),
        )
        .expect("valid shape")
    }

    fn diagram_with(objects: Vec<Object>) -> Diagram {
        let mut d = Diagram::new();
        for o in objects {
            match o {
                Object::Shape(s) => d.place_shape(s).unwrap(),
                Object::Text(t) => d.place_text(t).unwrap(),
                Object::Connector(c) => d.connect(c).unwrap(),
            }
        }
        d
    }

    fn placed(objects: Vec<Object>) -> PlacedGrid {
        place(&diagram_with(objects), &ShapeRegistry::builtin()).expect("known shapes")
    }

    #[test]
    fn rect_border_from_instance_geometry() {
        let g = placed(vec![Object::Shape(box_at(3, 2, 5, 3))]);
        // Top and bottom edges: E|W; sides: N|S; corners: all four.
        assert_eq!(g.cell(3, 2).edges, EdgeSet::ALL);
        assert_eq!(g.cell(5, 2).edges, EdgeSet::E | EdgeSet::W);
        assert_eq!(g.cell(3, 3).edges, EdgeSet::N | EdgeSet::S);
        assert_eq!(g.cell(7, 4).edges, EdgeSet::ALL);
        // Interior: untouched.
        assert!(!g.cell(5, 3).occupied());
    }

    #[test]
    fn rect_minimum_size_is_a_single_junction() {
        // Zero sizes are unrepresentable in the model (CellSize rejects them); the
        // smallest box is 1x1: a single cell carrying all four directions.
        let g = placed(vec![Object::Shape(box_at(0, 0, 1, 1))]);
        assert_eq!(g.cell(0, 0).edges, EdgeSet::ALL);
    }

    #[test]
    fn pattern_stamps_art_verbatim_and_keeps_spaces_transparent() {
        let g = placed(vec![Object::Shape(ellipse_at(1, 1))]);
        // Art cells verbatim (template offset by the instance position).
        assert_eq!(g.cell(1, 3).content, Some('('));
        assert_eq!(g.cell(9, 3).content, Some(')'));
        assert_eq!(g.cell(5, 1).content, Some('_'));
        // Space cells stay transparent (nothing stamped).
        assert!(!g.cell(5, 3).occupied());
    }

    /// B-001: an ellipse instance larger than its natural 9×5 template is re-rasterized
    /// at the instance's own extent — not stamped at natural size.
    #[test]
    fn resized_ellipse_is_rasterized_at_instance_extent() {
        let mut shape = ellipse_at(2, 1);
        shape.size = CellSize::new(13, 8).expect("non-zero test size");
        let g = placed(vec![Object::Shape(shape)]);
        // The grid follows the instance extent, not the template's 9×5.
        assert_eq!(g.width, 2 + 13);
        assert_eq!(g.height, 1 + 8);
        // Rasterized art verbatim at instance coordinates (local (4,0)='_', (0,2)'(',
        // (11,1)='.', (6,7)='-').
        assert_eq!(g.cell(6, 1).content, Some('_'));
        assert_eq!(g.cell(2, 3).content, Some('('));
        assert_eq!(g.cell(13, 2).content, Some('.'));
        assert_eq!(g.cell(8, 8).content, Some('-'));
        // Interior space cells stay transparent.
        assert!(!g.cell(8, 4).occupied());
    }

    /// B-001: the label of a resized ellipse centers over the instance extent and lands
    /// only on space cells of the rasterized outline.
    #[test]
    fn resized_ellipse_label_centers_on_space_cells() {
        let mut shape = ellipse_at(2, 1);
        shape.size = CellSize::new(13, 8).expect("non-zero test size");
        shape.label = "AB".to_owned();
        let g = placed(vec![Object::Shape(shape)]);
        // 13×8 region: x_off=(13-2)/2=5, y_off=(8-1)/2=3 → local (5,3),(6,3) — space cells.
        assert_eq!(g.cell(7, 4).content, Some('A'));
        assert_eq!(g.cell(8, 4).content, Some('B'));
    }

    #[test]
    fn pattern_anchor_cells_carry_passive_base_edges() {
        let g = placed(vec![Object::Shape(ellipse_at(0, 0))]);
        // n-anchor (4,0)='_': drawn neighbors west and east.
        assert_eq!(g.cell(4, 0).content, Some('_'));
        assert_eq!(g.cell(4, 0).edges, EdgeSet::W | EdgeSet::E);
        assert!(
            !g.cell(4, 0).structural_top,
            "unbound anchors do not claim the top"
        );
        // s-anchor (4,4)='-': drawn neighbors west and east.
        assert_eq!(g.cell(4, 4).edges, EdgeSet::W | EdgeSet::E);
        // w/e anchors: no drawn neighbor inside the template.
        assert_eq!(g.cell(0, 2).edges, EdgeSet::NONE);
        assert_eq!(g.cell(8, 2).edges, EdgeSet::NONE);
    }

    #[test]
    fn label_centers_in_rect_interior() {
        let mut shape = box_at(0, 0, 9, 5);
        shape.label = "User".to_owned();
        let g = placed(vec![Object::Shape(shape)]);
        // Interior is 7x3 (abs x=1..7); "User" (4 wide) centers at abs x=2..5,
        // on the middle row (y=2).
        for (x, c) in [(2u32, 'U'), (3, 's'), (4, 'e'), (5, 'r')] {
            assert_eq!(g.cell(x, 2).content, Some(c));
        }
    }

    #[test]
    fn label_wider_than_interior_clips_centered() {
        let mut shape = box_at(0, 0, 7, 3); // interior 5x1 (abs x=1..5)
        shape.label = "abcdefgh".to_owned(); // 8 wide -> x_off=(5-8)/2=-1 -> abs x=0..7
        let g = placed(vec![Object::Shape(shape)]);
        // Floor centering: x_off = floor((5-8)/2) = -2, so 'a','b' are clipped on the
        // left and 'h' on the right; 'c'..'g' land at abs x=1..5 (F3).
        assert_eq!(g.cell(0, 1).content, None);
        assert_eq!(g.cell(1, 1).content, Some('c'));
        assert_eq!(g.cell(5, 1).content, Some('g'));
        // 'h' at abs x=6 is the border cell — clipped.
        assert_eq!(g.cell(6, 1).content, None);
    }

    #[test]
    fn label_with_alphabet_chars_is_quoted() {
        let mut shape = box_at(0, 0, 13, 3); // interior 11x1
        shape.label = "a+b".to_owned();
        let g = placed(vec![Object::Shape(shape)]);
        // "a+b" quoted -> \"a+b\" (6 wide) centers in the 11-wide interior at abs x=3..8.
        let row: String = (0..13)
            .map(|x| g.cell(x, 1).content.unwrap_or(' '))
            .collect();
        assert_eq!(row.trim(), "\"a+b\"");
    }

    #[test]
    fn quoted_label_that_does_not_fit_falls_back_to_raw() {
        let mut shape = box_at(0, 0, 5, 3); // interior 3x1 (abs x=1..3)
        shape.label = "-x-".to_owned(); // raw (3) fits; quoted "\"-x-\"" (5) does not
        let g = placed(vec![Object::Shape(shape)]);
        let row: String = (0..5)
            .map(|x| g.cell(x, 1).content.unwrap_or(' '))
            .collect();
        assert_eq!(row, " -x- ", "raw line centered in the interior");
    }

    #[test]
    fn label_on_pattern_may_only_land_on_space_cells() {
        let mut shape = ellipse_at(0, 0);
        shape.label = "User".to_owned();
        let g = placed(vec![Object::Shape(shape)]);
        // Centers over the full 9-wide extent at x=2..5 on the middle row (y=2): all spaces.
        for (x, c) in [(2u32, 'U'), (3, 's'), (4, 'e'), (5, 'r')] {
            assert_eq!(g.cell(x, 2).content, Some(c));
        }
    }

    #[test]
    fn text_object_stamps_verbatim_with_quoting() {
        let t = TextInstance::new(ObjectId::generated(9), CellPos::new(1, 1), "hi + there")
            .expect("valid text");
        let g = placed(vec![Object::Text(t)]);
        // Line contains `+` -> quoted: `"hi + there"` (12 chars) at (1,1)..(12,1).
        let row: String = (0..13)
            .map(|x| g.cell(x, 1).content.unwrap_or(' '))
            .collect();
        assert_eq!(row.trim(), "\"hi + there\"");
    }

    #[test]
    fn z_order_later_content_wins_over_earlier_edges() {
        // Border first, text on top: the text character wins.
        let t = TextInstance::new(ObjectId::generated(9), CellPos::new(3, 2), "X").expect("valid");
        let g = placed(vec![Object::Shape(box_at(0, 0, 7, 5)), Object::Text(t)]);
        assert_eq!(g.cell(3, 2).content, Some('X'));
        assert!(!g.cell(3, 2).structural_top);
    }

    #[test]
    fn z_order_later_edges_win_over_earlier_content() {
        // Ellipse art first, box border on top of it: the border wins (structural top).
        let g = placed(vec![
            Object::Shape(ellipse_at(0, 0)),
            Object::Shape(box_at_id(2, 8, 0, 3, 5)),
        ]);
        // The box's left border column x=8 crosses the ellipse's `)` at (8,2).
        assert_eq!(g.cell(8, 2).edges, EdgeSet::N | EdgeSet::S);
        assert!(g.cell(8, 2).structural_top, "later border claims the top");
    }

    #[test]
    fn edges_merge_across_objects() {
        // Two adjacent boxes share a wall: the shared column carries N|S from both.
        let g = placed(vec![
            Object::Shape(box_at(0, 0, 5, 5)),
            Object::Shape(box_at_id(1, 5, 0, 5, 5)),
        ]);
        assert_eq!(g.cell(4, 2).edges, EdgeSet::N | EdgeSet::S);
        assert_eq!(g.cell(5, 2).edges, EdgeSet::N | EdgeSet::S);
    }

    #[test]
    fn unknown_shape_type_is_an_error() {
        let s = ShapeInstance::new(
            ObjectId::generated(0),
            "nope",
            "",
            CellPos::new(0, 0),
            CellSize::new(3, 3).expect("non-zero test size"),
        )
        .expect("valid shape");
        let d = diagram_with(vec![Object::Shape(s)]);
        let err = place(&d, &ShapeRegistry::builtin()).unwrap_err();
        assert_eq!(
            err,
            RenderError::UnknownShapeType {
                type_id: "nope".to_owned()
            }
        );
    }

    #[test]
    fn custom_template_stamps_opaquely_with_live_anchors() {
        // Template (3x2):
        //   - -
        //   ---
        let mut registry = ShapeRegistry::builtin();
        let collection = shape_registry::Collection {
            name: "mine".to_owned(),
            types: vec![shape_registry::ShapeType {
                type_id: "mine/star".to_owned(),
                name: "star".to_owned(),
                geometry: GeometryKind::Pattern,
                glyph_pattern: TemplateGrid::from_rows(&["- -", "---"]).expect("valid art"),
                anchor_set: {
                    let mut a = shape_registry::AnchorSet::new();
                    // On drawn art ('-') at the right edge.
                    a.add("e", shape_registry::CellPos::new(2, 1)).unwrap();
                    // On a space cell: must not be stamped at all (live only once bound).
                    a.add("n", shape_registry::CellPos::new(1, 0)).unwrap();
                    a
                },
                resize_rules: None,
            }],
        };
        registry.add_collection(collection);
        let s = ShapeInstance::new(
            ObjectId::generated(0),
            "mine/star",
            "",
            CellPos::new(2, 1),
            CellSize::new(3, 2).expect("non-zero test size"),
        )
        .expect("valid shape");
        let g = place(&diagram_with(vec![Object::Shape(s)]), &registry).expect("known");
        // Art verbatim at the instance position.
        assert_eq!(g.cell(2, 1).content, Some('-'));
        assert_eq!(g.cell(3, 2).content, Some('-'));
        // The `e` anchor (template (2,1) -> abs (4,2)) carries passive base edges from its
        // drawn neighbors: north '-' and west '-' (south/east are out of bounds).
        assert_eq!(g.cell(4, 2).content, Some('-'));
        assert_eq!(g.cell(4, 2).edges, EdgeSet::N | EdgeSet::W);
        assert!(
            !g.cell(4, 2).structural_top,
            "unbound anchors do not claim the top"
        );
        // The `n` anchor sits on a space cell (template (1,0) -> abs (3,1)): nothing stamped.
        assert!(!g.cell(3, 1).occupied());
    }

    #[test]
    fn spaces_are_transparent_between_objects() {
        // A box under an ellipse: the ellipse's space cells do not erase the border.
        let g = placed(vec![
            Object::Shape(box_at(0, 0, 9, 5)),
            Object::Shape(ellipse_at(0, 0)),
        ]);
        // (1,0): box top border (E|W); the ellipse has a space there, so the border
        // stays exactly as stamped.
        assert_eq!(g.cell(1, 0).edges, EdgeSet::E | EdgeSet::W);
        assert_eq!(g.cell(1, 0).content, None);
        assert!(
            g.cell(1, 0).structural_top,
            "the box border is the only writer"
        );
        // (5,0): under the ellipse's `_` art — later content wins over the earlier
        // border edges (F1), and does not claim structural top.
        assert_eq!(g.cell(5, 0).content, Some('_'));
        assert!(!g.cell(5, 0).structural_top);
    }
}
