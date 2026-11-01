//! Pure grid↔screen mapping and cell hit-testing for the WYSIWYG canvas (S-012 AC2,
//! S-013 anchor/resize targets).
//!
//! No egui types here — plain `f32`/`u32` math, unit-testable without a window.

use diagram_model::Diagram;
use diagram_model::anchors::AnchorRef;
use diagram_model::geometry::{CellPos, CellSize};
use diagram_model::objects::{
    ConnectorEnd, Object, ObjectId, ShapeInstance, TextInstance, TipStyle,
};
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
    /// The resize handle of the selected shape (its bottom-right cell) plus the
    /// smallest size an instance may reach — the gesture's clamp (S-024 AC5).
    ResizeHandle { id: ObjectId, min_size: CellSize },
    /// A declared bend of a connector (CR-007 AC4/AC5) — `index` is its position in
    /// the connector's bend list; `prev`/`next` are the adjacent polyline points
    /// (endpoints or neighboring bends), needed for axis snapping and the collinear
    /// drop check.
    Bend {
        id: ObjectId,
        index: usize,
        prev: CellPos,
        next: CellPos,
    },
    /// A cell of a connector's body — not an anchor or bend (CR-007 AC4/AC5) — where
    /// a drag adds a new bend. `segment` is the index of the polyline segment the
    /// cell lies on, i.e. the position the new bend would be inserted at; `prev`/
    /// `next` are that segment's endpoints.
    ConnectorBody {
        id: ObjectId,
        segment: usize,
        prev: CellPos,
        next: CellPos,
    },
    /// A connector's endpoint cell or its visible tip's glyph cell (CR-010): pressing
    /// starts an endpoint drag (attach/detach/move); a press without movement cycles
    /// that end's tip style (S-029 behavior preserved). `end` is which endpoint the
    /// cell belongs to, `at` the endpoint's own cell — the gesture detaches/moves
    /// relative to it. The tip cells follow the pipeline's stamping rule (ADR 013 v5).
    Endpoint {
        id: ObjectId,
        end: ConnectorEnd,
        at: CellPos,
    },
}

/// Which shape anchors are exposed to the pointer under the current gesture (S-031).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorExposure<'a> {
    /// Selection mode: only the selected shape's anchors.
    SelectionOnly,
    /// A connection is in flight from `source`: every anchor except the source itself
    /// is a drop target — nothing else on the canvas is offered.
    Connect { source: &'a AnchorRef },
    /// Arrow tool / endpoint drag (CR-010): every anchor of every shape is a snap
    /// target; non-anchor cells fall through to the other hit targets.
    All,
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
/// Priority (top first):
/// 1. Anchor exposure per [`AnchorExposure`]: the selected shape's anchors in
///    selection mode, every anchor except the source while connecting (nothing else
///    is offered — that is what makes "drop outside an anchor cancels" work), or
///    every shape's anchors while drawing an arrow / dragging an endpoint (S-031).
/// 2. The selected shape's bottom-right resize cell (selection mode only; S-024 AC5).
/// 3. Connector endpoint cells and visible tip glyph cells (CR-010) — above shape
///    bodies, below the selected shape's anchors/resize handle; topmost connector
///    first. A declared bend on the same cell keeps winning over them.
/// 4. The topmost shape/text body (document order = z-order, C5).
/// 5. Connector bends and body segments (CR-007) — below shapes and text so a shape
///    painted over a route still wins; a declared bend at the cell wins over the
///    segment below it.
#[must_use]
pub fn hit_target(
    diagram: &Diagram,
    catalog: &dyn ShapeCatalog,
    selection: Option<&ObjectId>,
    exposure: AnchorExposure<'_>,
    cell: (u32, u32),
    connector_routes: &[(ObjectId, Vec<CellPos>)],
) -> HitTarget {
    let (col, row) = cell;

    // Anchor targets — which anchors are exposed depends on the gesture (S-031).
    for object in diagram.objects() {
        let Object::Shape(shape) = object else {
            continue;
        };
        let allowed = match exposure {
            AnchorExposure::SelectionOnly => selection.is_some_and(|id| id == &shape.id),
            AnchorExposure::Connect { source } => shape.id != *source.object(),
            AnchorExposure::All => true,
        };
        if !allowed {
            continue;
        }
        let Some(anchors) = catalog.anchors(&shape.type_id) else {
            continue;
        };
        for anchor in anchors {
            let is_source = matches!(
                exposure,
                AnchorExposure::Connect { source }
                    if shape.id == *source.object() && anchor.name == source.anchor()
            );
            if !is_source && anchor_cell(shape, catalog, &anchor).is_some_and(|c| c == (col, row)) {
                return HitTarget::Anchor(shape.id.clone(), anchor.name);
            }
        }
    }

    // While a connection is in flight, anchors are the only offered target — the
    // gesture resolves the drop from the hit at release time.
    if matches!(exposure, AnchorExposure::Connect { .. }) {
        return HitTarget::Empty;
    }

    if let Some(selected_id) = selection
        && let Some(Object::Shape(shape)) = diagram.get(selected_id)
        && matches!(exposure, AnchorExposure::SelectionOnly)
    {
        if let Some(anchors) = catalog.anchors(&shape.type_id) {
            for anchor in anchors {
                if anchor_cell(shape, catalog, &anchor).is_some_and(|c| c == (col, row)) {
                    return HitTarget::Anchor(shape.id.clone(), anchor.name);
                }
            }
        }
        // One resize handle — the selected shape's bottom-right cell. Offered iff the
        // catalog says the type is resizable: built-ins and rule-bearing custom shapes
        // (S-024 AC5); plain custom shapes stay fixed-size (FR12).
        if let Some(info) = catalog.get(&shape.type_id)
            && info.resizable
            && col >= shape.pos.x
            && col - shape.pos.x + 1 == shape.size.width
            && row >= shape.pos.y
            && row - shape.pos.y + 1 == shape.size.height
        {
            return HitTarget::ResizeHandle {
                id: shape.id.clone(),
                min_size: info.min_size,
            };
        }
    }

    // Connector endpoint cells and visible tip glyph cells (CR-010): above shape
    // bodies, below the selected shape's anchors/resize handle. Topmost connector
    // first. The tip cells mirror the pipeline's stamping rule (ADR 013 v5): the
    // source tip only when the route has more than three cells (otherwise it would
    // collide with the target endpoint); the target tip needs at least three. A
    // declared bend on the same cell keeps winning — that is checked in the pass
    // below, together with the segments.
    let pos = CellPos::new(col, row);
    for (id, route) in connector_routes.iter().rev() {
        let Some(Object::Connector(connector)) = diagram.get(id) else {
            continue; // stale read-model entry — trust the model
        };
        if route.len() < 2 || connector.bends.contains(&pos) {
            continue; // degenerate route, or a declared bend wins on its own cell
        }
        let last = *route.last().expect("length checked above");
        if pos == route[0]
            || (connector.from_tip != TipStyle::Bare && route.len() > 3 && pos == route[1])
        {
            return HitTarget::Endpoint {
                id: id.clone(),
                end: ConnectorEnd::Source,
                at: route[0],
            };
        }
        if pos == last
            || (connector.to_tip != TipStyle::Bare
                && route.len() >= 3
                && pos == route[route.len() - 2])
        {
            return HitTarget::Endpoint {
                id: id.clone(),
                end: ConnectorEnd::Target,
                at: last,
            };
        }
    }

    let body = diagram
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
        });
    if let Some(hit) = body {
        return hit;
    }

    // Connectors (CR-007 AC4) — below shapes and text, topmost first. A declared bend
    // at the cell wins over body cells: a bend lies on two segments and the explicit
    // waypoint must be draggable.
    for (id, route) in connector_routes.iter().rev() {
        let Some(Object::Connector(connector)) = diagram.get(id) else {
            continue; // stale read-model entry — trust the model
        };
        if route.len() < 2 {
            continue; // degenerate route: nothing to grab (ADR 013)
        }
        let bends = &connector.bends;
        let from = route[0];
        let to = *route.last().expect("length checked above");

        if let Some(index) = bends.iter().position(|bend| *bend == pos) {
            let prev = if index == 0 { from } else { bends[index - 1] };
            let next = if index + 1 == bends.len() {
                to
            } else {
                bends[index + 1]
            };
            return HitTarget::Bend {
                id: id.clone(),
                index,
                prev,
                next,
            };
        }

        for (segment, pair) in polyline_points(from, bends, to).windows(2).enumerate() {
            let (a, b) = (pair[0], pair[1]);
            if on_segment(a, b, pos) {
                return HitTarget::ConnectorBody {
                    id: id.clone(),
                    segment,
                    prev: a,
                    next: b,
                };
            }
        }
    }

    HitTarget::Empty
}

/// The arrow glyph for an anchor marker pointing outward from the shape's side
/// (CR-008 AC3): `<`/`>` on the left/right edge, `^`/`v` on the top/bottom. An anchor
/// not on any edge (or a degenerate 1-cell shape) falls back to a dot.
pub fn anchor_glyph(position: CellPos, size: CellSize) -> char {
    if position.x == 0 && size.width > 1 {
        '<'
    } else if position.x + 1 == size.width {
        '>'
    } else if position.y == 0 && size.height > 1 {
        '^'
    } else if position.y + 1 == size.height {
        'v'
    } else {
        '·'
    }
}

/// The polyline points of a connector's route in order — the resolved endpoints with
/// the declared bends between (ADR 013 v4).
fn polyline_points(from: CellPos, bends: &[CellPos], to: CellPos) -> Vec<CellPos> {
    let mut points = Vec::with_capacity(bends.len() + 2);
    points.push(from);
    points.extend_from_slice(bends);
    points.push(to);
    points
}

/// Whether `cell` lies on the route segment from `a` to `b` — axis-aligned segments by
/// exact row/column containment, diagonals by walking the same integer line the
/// pipeline stamps. The diagonal walk is a copy of
/// `render-pipeline::connectors::diagonal_line` (gui-app may not depend on that crate,
/// §8.3) — like [`map_proportional`] vs `route`, a change to either Bresenham must land
/// in both.
pub(crate) fn on_segment(a: CellPos, b: CellPos, cell: CellPos) -> bool {
    if a == b {
        return cell == a;
    }
    if a.x == b.x || a.y == b.y {
        let (lo_x, hi_x) = (a.x.min(b.x), a.x.max(b.x));
        let (lo_y, hi_y) = (a.y.min(b.y), a.y.max(b.y));
        return (a.y == b.y && cell.y == a.y && (lo_x..=hi_x).contains(&cell.x))
            || (a.x == b.x && cell.x == a.x && (lo_y..=hi_y).contains(&cell.y));
    }
    // Diagonal — walk the identical integer line and test membership.
    let (mut x, mut y) = (a.x as i64, a.y as i64);
    let (ex, ey) = (b.x as i64, b.y as i64);
    let dx = (ex - x).abs();
    let dy = (ey - y).abs();
    let sx = i64::from(ex > x) - i64::from(ex < x);
    let sy = i64::from(ey > y) - i64::from(ey < y);
    let mut err = dx - dy;
    loop {
        if (x, y) == (cell.x as i64, cell.y as i64) {
            return true;
        }
        if (x, y) == (ex, ey) {
            return false;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
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
    /// An arrow glyph at an anchor cell pointing outward from the shape's side
    /// (CR-008 AC3) — of the selected shape, or of every shape while a connection is
    /// in flight (the source's own marker is excluded; the canvas draws it together
    /// with the drag line).
    AnchorArrow { cell: (u32, u32), glyph: char },
    /// Small square on the selection's bottom-right **cell** — the same cell
    /// [`hit_target`] answers with [`HitTarget::ResizeHandle`], so the marker and the
    /// gesture target coincide (the outer corner cell is reserved for the `s`/`e`
    /// anchor dots of many shapes).
    ResizeHandle((u32, u32)),
    /// Small square at a waypoint of the selected connector (CR-007 AC7) — the same
    /// cell [`hit_target`] answers with [`HitTarget::Bend`].
    BendHandle((u32, u32)),
}

/// The overlay primitives for the current model + view state (S-013 AC1/AC2, CR-007
/// AC7): anchor dots on the selected shape — or on every shape while connecting /
/// drawing an arrow / dragging an endpoint (S-031) — the resize handle of a selected
/// shape, and bend handles on a selected connector. Empty when nothing is selected
/// and no gesture is in flight.
#[must_use]
pub fn overlays(
    diagram: &Diagram,
    catalog: &dyn ShapeCatalog,
    selection: Option<&ObjectId>,
    exposure: AnchorExposure<'_>,
) -> Vec<Overlay> {
    let mut out = Vec::new();

    // A selected connector shows a handle at each declared waypoint (CR-007 AC7).
    if matches!(exposure, AnchorExposure::SelectionOnly)
        && let Some(selected_id) = selection
        && let Some(Object::Connector(connector)) = diagram.get(selected_id)
    {
        for bend in &connector.bends {
            out.push(Overlay::BendHandle((bend.x, bend.y)));
        }
    }

    for object in diagram.objects() {
        let Object::Shape(shape) = object else {
            continue;
        };
        let selected = selection.is_some_and(|id| id == &shape.id);
        // Which shapes' anchors are snap targets (S-031): selection mode — only the
        // selection; connecting / arrow tool / endpoint drag — every shape.
        if !match exposure {
            AnchorExposure::SelectionOnly => selected,
            AnchorExposure::Connect { .. } | AnchorExposure::All => true,
        } {
            continue;
        }

        if let Some(anchors) = catalog.anchors(&shape.type_id) {
            let size = catalog.get(&shape.type_id).map(|t| t.size);
            for anchor in anchors {
                let is_source = matches!(
                    exposure,
                    AnchorExposure::Connect { source }
                        if shape.id == *source.object() && anchor.name == source.anchor()
                );
                if !is_source && let Some(cell) = anchor_cell(shape, catalog, &anchor) {
                    out.push(Overlay::AnchorArrow {
                        cell,
                        glyph: size.map_or('·', |s| anchor_glyph(anchor.position, s)),
                    });
                }
            }
        }

        // One resize handle: the selection's bottom-right cell, iff the catalog says
        // the type is resizable (S-024 AC5 — built-ins and rule-bearing custom shapes;
        // plain customs stay fixed-size per FR12). The drawn marker and the hit-test
        // must agree on this exact cell.
        if selected
            && matches!(exposure, AnchorExposure::SelectionOnly)
            && catalog
                .get(&shape.type_id)
                .is_some_and(|info| info.resizable)
        {
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
            hit_target(
                &diagram,
                &catalog,
                Some(&s1),
                AnchorExposure::SelectionOnly,
                (8, 2),
                &[]
            ),
            HitTarget::Anchor(ObjectId::new("s1").expect("valid"), "e".to_owned())
        );
        // Bottom-right cell of s1 = the resize handle with the catalog's minimum
        // extent (the same cell the overlay draws its marker on).
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                Some(&s1),
                AnchorExposure::SelectionOnly,
                (8, 4),
                &[]
            ),
            HitTarget::ResizeHandle {
                id: s1.clone(),
                min_size: CellSize::new(1, 1).expect("valid"),
            }
        );
        // Interior body cell → the shape itself.
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                Some(&s1),
                AnchorExposure::SelectionOnly,
                (3, 2),
                &[]
            ),
            HitTarget::Body(s1.clone())
        );
        // Empty space.
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                Some(&s1),
                AnchorExposure::SelectionOnly,
                (40, 40),
                &[]
            ),
            HitTarget::Empty
        );

        // s2 is not selected: its e anchor cell (13,2) offers no handle → plain body.
        let s2 = ObjectId::new("s2").expect("valid id");
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                Some(&s1),
                AnchorExposure::SelectionOnly,
                (13, 2),
                &[]
            ),
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
            hit_target(
                &diagram,
                &catalog,
                None,
                AnchorExposure::SelectionOnly,
                (6, 2),
                &[]
            ),
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
            hit_target(
                &diagram,
                &catalog,
                None,
                AnchorExposure::Connect { source: &source },
                (20, 2),
                &[]
            ),
            HitTarget::Anchor(ObjectId::new("s2").expect("valid"), "w".to_owned())
        );
        // …the source anchor itself is not…
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                None,
                AnchorExposure::Connect { source: &source },
                (8, 2),
                &[]
            ),
            HitTarget::Empty
        );
        // …nor are plain body cells.
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                None,
                AnchorExposure::Connect { source: &source },
                (3, 2),
                &[]
            ),
            HitTarget::Empty
        );
    }

    /// S-024 AC5: resizability is catalog-driven — a plain custom shape (no rules)
    /// offers no handle; a rule-bearing one does, carrying its minimum extent.
    #[test]
    fn resize_handle_follows_the_catalog_resizable_flag() {
        // A plain custom template from a collection: qualified type id, not resizable.
        let arrow = ShapeInfo {
            type_id: "mycol/arrow".to_owned(),
            name: "Arrow".to_owned(),
            size: CellSize::new(5, 3).expect("valid size"),
            collection: Some("mycol".to_owned()),
            preview: vec!["..#..".to_owned()],
            resizable: false,
            min_size: CellSize::new(1, 1).expect("valid size"),
            resize_rules: None,
        };
        // A rule-bearing custom shape: resizable down to its computed 2×2 floor.
        let star = ShapeInfo {
            type_id: "mycol/star".to_owned(),
            name: "Star".to_owned(),
            size: CellSize::new(5, 3).expect("valid size"),
            collection: Some("mycol".to_owned()),
            preview: vec!["..#..".to_owned()],
            resizable: true,
            min_size: CellSize::new(2, 2).expect("valid size"),
            resize_rules: None,
        };
        let anchors_for = |x: u32| {
            [("e", CellPos::new(x, 1))]
                .into_iter()
                .map(|(name, position)| AnchorInfo {
                    name: name.to_owned(),
                    position,
                })
                .collect()
        };
        let catalog = catalog()
            .add(arrow, anchors_for(4))
            .add(star, anchors_for(14));

        let mut diagram = Diagram::new();
        for (id, type_id, x) in [("a1", "mycol/arrow", 0u32), ("s1", "mycol/star", 10)] {
            diagram
                .place_shape(
                    ShapeInstance::new(
                        ObjectId::new(id).expect("valid id"),
                        type_id,
                        "",
                        CellPos::new(x, 0),
                        CellSize::new(5, 3).expect("valid size"),
                    )
                    .expect("valid instance"),
                )
                .expect("placement is valid");
        }
        let a1 = ObjectId::new("a1").expect("valid id");
        let s1 = ObjectId::new("s1").expect("valid id");

        // Plain custom: its bottom-right cell (4,2) selects the body — no handle…
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                Some(&a1),
                AnchorExposure::SelectionOnly,
                (4, 2),
                &[]
            ),
            HitTarget::Body(a1.clone())
        );
        let ov = overlays(&diagram, &catalog, Some(&a1), AnchorExposure::SelectionOnly);
        assert!(!ov.iter().any(|o| matches!(o, Overlay::ResizeHandle(_))));
        assert_eq!(ov.len(), 1, "just the single anchor dot");

        // Rule-bearing custom: its bottom-right cell (14,2) is a resize handle with
        // its min size…
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                Some(&s1),
                AnchorExposure::SelectionOnly,
                (14, 2),
                &[]
            ),
            HitTarget::ResizeHandle {
                id: s1.clone(),
                min_size: CellSize::new(2, 2).expect("valid size"),
            }
        );
        // …and the overlay carries its handle marker.
        let ov = overlays(&diagram, &catalog, Some(&s1), AnchorExposure::SelectionOnly);
        assert!(ov.contains(&Overlay::ResizeHandle((14, 2))));
    }

    /// Overlay data (S-013 AC1/AC2): selection → its dots + handle; connecting → every
    /// shape's dots except the source; nothing selected/connecting → empty.
    #[test]
    fn overlays_follow_selection_and_connecting_state() {
        let catalog = catalog();
        let diagram = diagram_with(&[("s1", 0, 0), ("s2", 20, 0)]);

        // Nothing selected, nothing in flight: no overlay at all.
        assert!(overlays(&diagram, &catalog, None, AnchorExposure::SelectionOnly).is_empty());

        // Selection: s1's four anchor dots + its bottom-right corner handle.
        let s1 = ObjectId::new("s1").expect("valid id");
        let sel = overlays(&diagram, &catalog, Some(&s1), AnchorExposure::SelectionOnly);
        assert_eq!(
            sel.iter()
                .filter(|o| matches!(o, Overlay::AnchorArrow { .. }))
                .count(),
            4,
            "the selected shape's four anchors"
        );
        // The handle marker sits on the bottom-right *cell* (8,4) — exactly where
        // `hit_target` answers with a resize gesture.
        assert!(sel.contains(&Overlay::ResizeHandle((8, 4))));
        // s2 (not selected) contributes nothing.
        assert!(!sel.iter().any(|o| matches!(
            o,
            Overlay::AnchorArrow {
                cell: (20..=28, _),
                ..
            }
        )));

        // Connecting from s1's e: dots on both shapes, minus the source dot, no handle.
        let source = AnchorRef::new(s1.clone(), "e").expect("valid ref");
        let connecting = overlays(
            &diagram,
            &catalog,
            None,
            AnchorExposure::Connect { source: &source },
        );
        assert_eq!(
            connecting
                .iter()
                .filter(|o| matches!(o, Overlay::AnchorArrow { .. }))
                .count(),
            7,
            "4 + 4 anchors minus the source dot"
        );
        assert!(!connecting.contains(&Overlay::AnchorArrow {
            cell: (8, 2),
            glyph: '>'
        }));
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

    // ---- CR-007: connector geometry -----------------------------------------

    fn pos(x: u32, y: u32) -> CellPos {
        CellPos::new(x, y)
    }

    #[test]
    fn on_segment_axis_aligned() {
        let a = pos(1, 2);
        let b = pos(9, 2);
        assert!(on_segment(a, b, pos(5, 2)));
        assert!(on_segment(a, b, pos(1, 2))); // endpoints included
        assert!(on_segment(a, b, pos(9, 2)));
        assert!(!on_segment(a, b, pos(5, 3)));
        assert!(!on_segment(a, b, pos(0, 2)));
        assert!(!on_segment(a, b, pos(10, 2)));

        let v = pos(4, 0);
        let w = pos(4, 7);
        assert!(on_segment(v, w, pos(4, 5)));
        assert!(!on_segment(v, w, pos(3, 5)));
    }

    #[test]
    fn on_segment_diagonal_walks_the_bresenham_line() {
        // (8,2) → (11,6): the pipeline's integer Bresenham stamps exactly these cells.
        let a = pos(8, 2);
        let b = pos(11, 6);
        for cell in [pos(8, 2), pos(9, 3), pos(9, 4), pos(10, 5), pos(11, 6)] {
            assert!(on_segment(a, b, cell), "{cell:?} is on the stamped line");
        }
        // Off-line cells are not reported — including the bounding-box interior.
        assert!(!on_segment(a, b, pos(10, 3)));
        assert!(!on_segment(a, b, pos(8, 6)));
        assert!(!on_segment(a, b, pos(9, 2)));

        // A rising segment: (11,6) → (14,2).
        let c = pos(11, 6);
        let d = pos(14, 2);
        for cell in [pos(11, 6), pos(12, 5), pos(12, 4), pos(13, 3), pos(14, 2)] {
            assert!(on_segment(c, d, cell), "{cell:?} is on the stamped line");
        }
        assert!(!on_segment(c, d, pos(13, 5)));
    }

    #[test]
    fn on_segment_degenerate_point() {
        let a = pos(3, 3);
        assert!(on_segment(a, a, a));
        assert!(!on_segment(a, a, pos(4, 3)));
    }

    /// Two 9×5 boxes with one bent connector between them — the canonical CR-007
    /// scene. s1 at (0,0) offers `e` at (8,2); s2 at (14,0) offers `w` at (14,2).
    /// The bend at (11,6) sits in open space; both route segments are diagonal and
    /// their stamped cells are all free of shape bodies.
    fn bent_connector_scene() -> (
        Diagram,
        core_services::fakes::FakeCatalog,
        ObjectId,
        Vec<CellPos>,
    ) {
        use diagram_model::objects::{ConnectorInstance, ConnectorStyle};
        let catalog = catalog();
        let mut diagram = Diagram::new();
        diagram.place_shape(box_at("s1", 0, 0)).expect("places");
        diagram.place_shape(box_at("s2", 14, 0)).expect("places");
        let c = ObjectId::new("c1").expect("valid id");
        diagram
            .connect(ConnectorInstance::between(
                c.clone(),
                AnchorRef::new(ObjectId::new("s1").expect("valid"), "e").expect("valid anchor"),
                AnchorRef::new(ObjectId::new("s2").expect("valid"), "w").expect("valid anchor"),
                ConnectorStyle::Solid,
            ))
            .expect("connects");
        diagram
            .add_bend(&c, 0, pos(11, 6))
            .expect("inserts the bend");
        let route = vec![pos(8, 2), pos(11, 6), pos(14, 2)];
        (diagram, catalog, c, route)
    }

    #[test]
    fn hit_target_reports_a_bend_on_its_cell() {
        let (diagram, catalog, c, route) = bent_connector_scene();
        let routes = vec![(c.clone(), route)];
        // The bend cell itself is a Bend — even though it lies on two segments.
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                None,
                AnchorExposure::SelectionOnly,
                (11, 6),
                &routes
            ),
            HitTarget::Bend {
                id: c,
                index: 0,
                prev: pos(8, 2),
                next: pos(14, 2)
            }
        );
    }

    /// CR-008 AC2/AC4: the cell adjacent to each endpoint is a tip cell when that end
    /// has a visible tip; bare ends stay plain body cells.
    #[test]
    fn hit_target_reports_tip_cells_at_the_route_ends() {
        use diagram_model::objects::{ConnectorInstance, ConnectorStyle};
        let catalog = catalog();
        let mut diagram = Diagram::new();
        diagram.place_shape(box_at("s1", 0, 0)).expect("places");
        diagram.place_shape(box_at("s2", 20, 0)).expect("places");
        let c = ObjectId::new("c1").expect("valid id");
        diagram
            .connect(ConnectorInstance::between(
                c.clone(),
                AnchorRef::new(ObjectId::new("s1").expect("valid"), "e").expect("valid anchor"),
                AnchorRef::new(ObjectId::new("s2").expect("valid"), "w").expect("valid anchor"),
                ConnectorStyle::Solid,
            ))
            .expect("connects");
        // The full route as the production seam provides it: every stamped cell.
        let route: Vec<CellPos> = (8..=20).map(|x| pos(x, 2)).collect();
        let routes = vec![(c.clone(), route)];

        // Default tips: bare source, hollow arrow at the target — only the target's
        // adjacent cell is a tip cell.
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                None,
                AnchorExposure::SelectionOnly,
                (19, 2),
                &routes
            ),
            HitTarget::Endpoint {
                id: c.clone(),
                end: ConnectorEnd::Target,
                at: pos(20, 2)
            }
        );
        match hit_target(
            &diagram,
            &catalog,
            None,
            AnchorExposure::SelectionOnly,
            (9, 2),
            &routes,
        ) {
            HitTarget::ConnectorBody { id, .. } => assert_eq!(id, c),
            other => panic!("bare source end is body, got {other:?}"),
        }

        // Cycled source tip: the first cell after the endpoint becomes a tip cell.
        diagram
            .set_tip(&c, ConnectorEnd::Source, TipStyle::HollowArrow)
            .expect("sets source tip");
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                None,
                AnchorExposure::SelectionOnly,
                (9, 2),
                &routes
            ),
            HitTarget::Endpoint {
                id: c,
                end: ConnectorEnd::Source,
                at: pos(8, 2)
            }
        );
    }

    /// CR-008: on bent routes the tip cells are the last/first stamped cells of the
    /// outer segments; declared bends keep priority on their own cells.
    #[test]
    fn bent_route_tip_cells_sit_on_the_outer_segments() {
        let (diagram, catalog, c, _waypoints) = bent_connector_scene();
        // Production route: every stamped cell of the two diagonal segments.
        let routes = vec![(
            c.clone(),
            vec![
                pos(8, 2),
                pos(9, 3),
                pos(10, 4),
                pos(11, 5),
                pos(11, 6),
                pos(12, 5),
                pos(13, 4),
                pos(14, 2),
            ],
        )];
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                None,
                AnchorExposure::SelectionOnly,
                (13, 4),
                &routes
            ),
            HitTarget::Endpoint {
                id: c.clone(),
                end: ConnectorEnd::Target,
                at: pos(14, 2)
            }
        );
        // The declared bend still wins on its own cell.
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                None,
                AnchorExposure::SelectionOnly,
                (11, 6),
                &routes
            ),
            HitTarget::Bend {
                id: c,
                index: 0,
                prev: pos(8, 2),
                next: pos(14, 2)
            }
        );
    }

    /// CR-010: free-endpoint cells and tip cells are hit above shape bodies, below
    /// the selected shape's anchors; `AnchorExposure::All` exposes every shape's
    /// anchors (arrow tool / endpoint-drag snap targets).
    #[test]
    fn hit_target_reports_endpoint_cells_above_bodies() {
        use diagram_model::anchors::ConnectorEndpoint;
        use diagram_model::objects::{ConnectorInstance, ConnectorStyle};
        let catalog = catalog();
        let mut diagram = Diagram::new();
        diagram.place_shape(box_at("s1", 0, 0)).expect("places");
        diagram.place_shape(box_at("s2", 20, 0)).expect("places");
        // A third box covering part of the route and the free endpoint's cell.
        diagram.place_shape(box_at("s3", 12, 0)).expect("places");
        let c = ObjectId::new("c1").expect("valid id");
        diagram
            .connect(ConnectorInstance::new(
                c.clone(),
                ConnectorEndpoint::Attached(
                    AnchorRef::new(ObjectId::new("s1").expect("valid"), "e").expect("valid anchor"),
                ),
                ConnectorEndpoint::Free(pos(15, 2)),
                ConnectorStyle::Solid,
            ))
            .expect("connects");
        // Route: s1.e (8,2) → free end (15,2), stamped horizontally.
        let route: Vec<CellPos> = (8..=15).map(|x| pos(x, 2)).collect();
        let routes = vec![(c.clone(), route)];

        // The free endpoint cell sits on s3's body — the endpoint wins (CR-010).
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                None,
                AnchorExposure::SelectionOnly,
                (15, 2),
                &routes
            ),
            HitTarget::Endpoint {
                id: c.clone(),
                end: ConnectorEnd::Target,
                at: pos(15, 2)
            }
        );
        // A plain route cell inside s3 is still the shape's body.
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                None,
                AnchorExposure::SelectionOnly,
                (13, 2),
                &routes
            ),
            HitTarget::Body(ObjectId::new("s3").expect("valid"))
        );

        // The attached source's cell is the selected shape's anchor — anchors keep
        // priority over endpoints.
        let s1 = ObjectId::new("s1").expect("valid id");
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                Some(&s1),
                AnchorExposure::SelectionOnly,
                (8, 2),
                &routes
            ),
            HitTarget::Anchor(s1.clone(), "e".to_owned())
        );

        // `All` exposes every shape's anchors even without a selection…
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                None,
                AnchorExposure::All,
                (28, 2),
                &routes
            ),
            HitTarget::Anchor(ObjectId::new("s2").expect("valid"), "e".to_owned())
        );
        // …and the overlay layer shows all their snap markers.
        let all = overlays(&diagram, &catalog, None, AnchorExposure::All);
        assert_eq!(
            all.iter()
                .filter(|o| matches!(o, Overlay::AnchorArrow { .. }))
                .count(),
            12,
            "three boxes × four anchors"
        );
    }

    #[test]
    fn hit_target_reports_body_cells_with_segment_endpoints() {
        let (diagram, catalog, c, route) = bent_connector_scene();
        let routes = vec![(c.clone(), route)];
        // Segment 0 is the diagonal (8,2)-(11,6): a cell on its stamped line.
        match hit_target(
            &diagram,
            &catalog,
            None,
            AnchorExposure::SelectionOnly,
            (9, 3),
            &routes,
        ) {
            HitTarget::ConnectorBody {
                id,
                segment,
                prev,
                next,
            } => {
                assert_eq!(id, c);
                assert_eq!(segment, 0);
                assert_eq!((prev, next), (pos(8, 2), pos(11, 6)));
            }
            other => panic!("expected ConnectorBody on (9,3), got {other:?}"),
        }
        // Segment 1 is the rising diagonal (11,6)-(14,2).
        match hit_target(
            &diagram,
            &catalog,
            None,
            AnchorExposure::SelectionOnly,
            (12, 5),
            &routes,
        ) {
            HitTarget::ConnectorBody {
                id,
                segment,
                prev,
                next,
            } => {
                assert_eq!(id, c);
                assert_eq!(segment, 1);
                assert_eq!((prev, next), (pos(11, 6), pos(14, 2)));
            }
            other => panic!("expected ConnectorBody on (12,5), got {other:?}"),
        }
    }

    #[test]
    fn hit_target_shapes_win_over_connector_bodies() {
        let (mut diagram, catalog, c, route) = bent_connector_scene();
        let routes = vec![(c.clone(), route)];
        // A shape added after the connector covers a route cell: the body wins.
        let s3 = ObjectId::new("s3").expect("valid id");
        diagram.place_shape(box_at("s3", 9, 3)).expect("places");
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                None,
                AnchorExposure::SelectionOnly,
                (9, 3),
                &routes
            ),
            HitTarget::Body(s3)
        );
    }

    #[test]
    fn hit_target_ignores_connectors_off_route_and_while_connecting() {
        let (diagram, catalog, c, route) = bent_connector_scene();
        let routes = vec![(c.clone(), route)];
        // Empty cell off the route stays empty.
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                None,
                AnchorExposure::SelectionOnly,
                (0, 9),
                &routes
            ),
            HitTarget::Empty
        );
        // While connecting, connector cells are not drop targets (anchors only).
        let source =
            AnchorRef::new(ObjectId::new("s1").expect("valid"), "e").expect("valid anchor");
        assert_eq!(
            hit_target(
                &diagram,
                &catalog,
                None,
                AnchorExposure::Connect { source: &source },
                (9, 3),
                &routes
            ),
            HitTarget::Empty
        );
    }

    #[test]
    fn overlays_show_bend_handles_only_for_the_selected_connector() {
        let (diagram, catalog, c, _route) = bent_connector_scene();
        // Not selected: no bend handles.
        assert!(
            overlays(&diagram, &catalog, None, AnchorExposure::SelectionOnly)
                .iter()
                .all(|o| !matches!(o, Overlay::BendHandle(_)))
        );
        // Selected: one handle at the declared waypoint — same cell `hit_target`
        // answers with a Bend.
        let handles = overlays(&diagram, &catalog, Some(&c), AnchorExposure::SelectionOnly);
        assert_eq!(
            handles
                .iter()
                .filter(|o| matches!(o, Overlay::BendHandle((11, 6))))
                .count(),
            1
        );
    }
}
