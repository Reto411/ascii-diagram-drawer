//! The canvas gesture state machine (S-012 placement/select/move, S-013 resize and
//! anchor-to-anchor connect) — pure interaction logic, unit-testable headlessly against
//! a real [`Diagram`] (T7 pattern).
//!
//! Per ADR 009 the GUI mutates the aggregate directly: this module turns semantic
//! pointer events into exactly those mutations. Selection, placement and in-flight
//! gestures are transient *view* state — they never touch the model.

use diagram_model::Diagram;
use diagram_model::anchors::AnchorRef;
use diagram_model::geometry::{CellPos, CellSize};
use diagram_model::objects::{ConnectorInstance, ConnectorStyle, Object, ObjectId, ShapeInstance};
use ui_api::ShapeInfo;

use crate::canvas::HitTarget;

/// Semantic canvas pointer events — egui input is mapped to these in `app.rs`. The
/// `target` is the precomputed interactive hit under the cell (see [`crate::canvas::hit_target`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PointerEvent {
    /// Primary button pressed on the given grid cell.
    Down { cell: (u32, u32), target: HitTarget },
    /// Pointer moved (primary held or hovering) over the given grid cell.
    Move { cell: (u32, u32), target: HitTarget },
    /// Primary button released; `target` is what was under the pointer at release.
    Up { target: HitTarget },
}

/// What a pointer event or key did to the model — drives the status bar and the tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOutcome {
    /// No state change (e.g. a move that did not cross a cell boundary).
    Nothing,
    /// The click hit no shape; the selection was cleared.
    Deselected,
    /// The click selected (and started dragging) a shape.
    Selected(ObjectId),
    /// A new shape instance entered the model at the clicked cell.
    Placed { id: ObjectId, type_id: String },
    /// A dragged shape crossed to a new cell and the model was updated live.
    Moved { id: ObjectId, to: CellPos },
    /// A resize drag changed the selected shape's extent (min 1×1, grid-aligned).
    Resized { id: ObjectId, to: CellSize },
    /// A connection gesture started from an anchor.
    Connecting { from: AnchorRef },
    /// A connector was created between two anchors.
    Connected { id: ObjectId },
    /// The selected object (and its connectors) left the model.
    Deleted { id: ObjectId, removed: usize },
    /// An in-flight gesture or placement was cancelled without mutation.
    Cancelled,
}

/// An in-flight primary-button gesture (S-013).
#[derive(Debug, Clone)]
enum Gesture {
    /// Move drag: which object and where on it the pointer grabbed.
    Move {
        id: ObjectId,
        offset_x: u32,
        offset_y: u32,
    },
    /// Resize drag on the selected shape's bottom-right handle (v1).
    Resize { id: ObjectId },
    /// Connect drag from a source anchor to wherever the pointer drops.
    Connect { from: AnchorRef },
}

/// Transient view state + gesture machine (ADR 009: selection/placement/gestures are
/// view state only — the model is mutated exclusively through [`Diagram`] operations).
#[derive(Debug, Default)]
pub struct EditorState {
    /// The template picked in the sidebar; the next canvas click places it.
    pub placing: Option<ShapeInfo>,
    /// The currently selected shape (if any).
    pub selection: Option<ObjectId>,
    gesture: Option<Gesture>,
}

impl EditorState {
    /// Clears all transient state (selection, placement, in-flight gesture) — used
    /// when a new document replaces the model (S-014 open), so no stale id survives.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Sidebar action: pick a template to place (replaces a previous pick, clears the
    /// selection so the next click is unambiguously a placement).
    pub fn select_template(&mut self, info: ShapeInfo) {
        self.placing = Some(info);
        self.selection = None;
    }

    /// The source anchor of an in-flight connection gesture, if any — the canvas uses
    /// it for target hit-testing and the drag overlay.
    #[must_use]
    pub fn connecting_source(&self) -> Option<&AnchorRef> {
        match &self.gesture {
            Some(Gesture::Connect { from }) => Some(from),
            _ => None,
        }
    }

    /// Escape: drops any in-flight gesture, exits placement mode and clears the
    /// selection — always without mutating the model.
    pub fn on_escape(&mut self) -> EditOutcome {
        let had_gesture = self.gesture.take().is_some();
        let was_placing = self.placing.take().is_some();
        if had_gesture || was_placing {
            return EditOutcome::Cancelled;
        }
        if self.selection.take().is_some() {
            return EditOutcome::Deselected;
        }
        EditOutcome::Nothing
    }

    /// Delete/Backspace: removes the selected object and every connector referencing
    /// it (the aggregate keeps endpoints from dangling). No selection → no-op.
    pub fn delete_selection(&mut self, diagram: &mut Diagram) -> EditOutcome {
        let Some(id) = self.selection.clone() else {
            return EditOutcome::Nothing;
        };
        match diagram.remove(&id) {
            Ok(removed) => {
                self.selection = None;
                self.gesture = None;
                EditOutcome::Deleted { id, removed }
            }
            Err(_) => EditOutcome::Nothing, // defensive: the selection is always valid
        }
    }

    /// Applies one pointer event, mutating the model in place. Returns what happened.
    pub fn on_pointer(&mut self, event: PointerEvent, diagram: &mut Diagram) -> EditOutcome {
        match event {
            PointerEvent::Down { cell, target } => self.on_down(cell, target, diagram),
            PointerEvent::Move { cell, .. } => self.on_move(cell, diagram),
            PointerEvent::Up { target } => self.on_up(target, diagram),
        }
    }

    fn on_down(
        &mut self,
        (col, row): (u32, u32),
        target: HitTarget,
        diagram: &mut Diagram,
    ) -> EditOutcome {
        if let Some(info) = self.placing.clone() {
            return self.place(col, row, info, diagram);
        }
        match target {
            // Anchors are only offered for the selected shape (see `hit_target`), so a
            // pressed anchor always starts a connection from it.
            HitTarget::Anchor(object_id, name) => match AnchorRef::new(object_id.clone(), &name) {
                Ok(from) => {
                    self.gesture = Some(Gesture::Connect { from: from.clone() });
                    EditOutcome::Connecting { from }
                }
                // Defensive: catalog anchor names are valid single tokens.
                Err(_) => self.on_down_shape(col, row, diagram),
            },
            HitTarget::ResizeHandle(id) => {
                self.gesture = Some(Gesture::Resize { id: id.clone() });
                EditOutcome::Nothing
            }
            _ => self.on_down_shape(col, row, diagram),
        }
    }

    /// Press on a shape body (or empty space): select + start the move gesture.
    fn on_down_shape(&mut self, col: u32, row: u32, diagram: &mut Diagram) -> EditOutcome {
        match hit_shape(diagram, col, row) {
            Some((id, pos)) => {
                self.selection = Some(id.clone());
                // Grab immediately: one press-drag gesture both selects and moves.
                self.gesture = Some(Gesture::Move {
                    id: id.clone(),
                    offset_x: col.saturating_sub(pos.x),
                    offset_y: row.saturating_sub(pos.y),
                });
                EditOutcome::Selected(id)
            }
            None => {
                self.selection = None;
                EditOutcome::Deselected
            }
        }
    }

    fn place(&mut self, col: u32, row: u32, info: ShapeInfo, diagram: &mut Diagram) -> EditOutcome {
        let id = diagram.new_object_id();
        let instance = match ShapeInstance::new(
            id.clone(),
            &info.type_id,
            "",
            CellPos::new(col, row),
            info.size,
        ) {
            Ok(instance) => instance,
            // Defensive: catalog type ids are valid single tokens; a malformed one
            // (e.g. from a broken custom collection) is simply not placed.
            Err(_) => return EditOutcome::Nothing,
        };
        if diagram.place_shape(instance).is_err() {
            return EditOutcome::Nothing;
        }
        self.placing = None; // one placement per click (v1)
        self.selection = Some(id.clone());
        EditOutcome::Placed {
            id,
            type_id: info.type_id,
        }
    }

    fn on_move(&mut self, (col, row): (u32, u32), diagram: &mut Diagram) -> EditOutcome {
        let Some(gesture) = self.gesture.clone() else {
            return EditOutcome::Nothing;
        };
        match gesture {
            Gesture::Move {
                id,
                offset_x,
                offset_y,
            } => {
                // The shape follows the pointer with the grab offset preserved; saturating
                // subtraction clamps at the grid origin (positions are unsigned by design).
                let to = CellPos::new(col.saturating_sub(offset_x), row.saturating_sub(offset_y));
                let Some(Object::Shape(shape)) = diagram.get(&id) else {
                    return EditOutcome::Nothing;
                };
                if shape.pos == to {
                    return EditOutcome::Nothing; // no cell crossing — nothing to mutate
                }
                match diagram.move_object(&id, to) {
                    Ok(()) => EditOutcome::Moved { id, to },
                    Err(_) => EditOutcome::Nothing,
                }
            }
            Gesture::Resize { id } => {
                let Some(Object::Shape(shape)) = diagram.get(&id) else {
                    return EditOutcome::Nothing;
                };
                // The bottom-right corner follows the pointer: extent = distance from the
                // (fixed) top-left, minimum 1×1 by construction.
                let to = CellSize::new(
                    col.saturating_sub(shape.pos.x).saturating_add(1),
                    row.saturating_sub(shape.pos.y).saturating_add(1),
                )
                .expect("resize extents are ≥ 1");
                if shape.size == to {
                    return EditOutcome::Nothing; // no cell crossing — nothing to mutate
                }
                match diagram.resize(&id, to) {
                    Ok(()) => EditOutcome::Resized { id, to },
                    Err(_) => EditOutcome::Nothing,
                }
            }
            // Connect drags are pure hover until release (the highlight is drawn by the
            // canvas from the same hit computation).
            Gesture::Connect { .. } => EditOutcome::Nothing,
        }
    }

    fn on_up(&mut self, target: HitTarget, diagram: &mut Diagram) -> EditOutcome {
        let gesture = self.gesture.take();
        match (gesture, target) {
            (Some(Gesture::Connect { from }), HitTarget::Anchor(object_id, name)) => {
                // `hit_target` in connecting mode never reports the source anchor itself,
                // so a hit here is always a legal drop.
                let Ok(to) = AnchorRef::new(object_id, &name) else {
                    return EditOutcome::Nothing; // defensive: catalog names are valid
                };
                let id = diagram.new_object_id();
                let connector = ConnectorInstance::new(id.clone(), from, to, ConnectorStyle::Solid);
                match diagram.connect(connector) {
                    Ok(()) => EditOutcome::Connected { id },
                    Err(_) => EditOutcome::Nothing, // defensive: endpoints were just hit-tested
                }
            }
            // Dropping anywhere else cancels the gesture without mutation (AC2).
            _ => EditOutcome::Nothing,
        }
    }
}

/// The topmost shape containing the given cell — last in document order wins, which
/// is exactly the z-order (document order = z-order, C5). Connectors and text are not
/// selectable yet (S-018 for text; connectors follow their shapes).
fn hit_shape(diagram: &Diagram, col: u32, row: u32) -> Option<(ObjectId, CellPos)> {
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
                within.then(|| (shape.id.clone(), shape.pos))
            }
            Object::Connector(_) | Object::Text(_) => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::{HitTarget, hit_target};
    use core_services::fakes::FakeCatalog;
    use ui_api::{AnchorInfo, ShapeInfo};

    fn info(type_id: &str, name: &str, width: u32, height: u32) -> ShapeInfo {
        ShapeInfo {
            type_id: type_id.to_owned(),
            name: name.to_owned(),
            size: CellSize::new(width, height).expect("test sizes are valid"),
        }
    }

    /// The built-in `box` read model with its standard anchors (mirrors the catalog).
    fn box_entry() -> (ShapeInfo, Vec<AnchorInfo>) {
        let info = info("box", "Box", 9, 5);
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
        (info, anchors)
    }

    fn catalog() -> FakeCatalog {
        let (info, anchors) = box_entry();
        FakeCatalog::default().add(info, anchors)
    }

    /// Runs the same hit computation `app.rs` uses, so gesture tests exercise the real
    /// target a press on that cell would carry.
    fn down(editor: &mut EditorState, diagram: &mut Diagram, col: u32, row: u32) -> EditOutcome {
        let target = hit_target(
            diagram,
            &catalog(),
            editor.selection.as_ref(),
            None,
            (col, row),
        );
        editor.on_pointer(
            PointerEvent::Down {
                cell: (col, row),
                target,
            },
            diagram,
        )
    }

    fn move_to(editor: &mut EditorState, diagram: &mut Diagram, col: u32, row: u32) -> EditOutcome {
        let target = hit_target(
            diagram,
            &catalog(),
            editor.selection.as_ref(),
            None,
            (col, row),
        );
        editor.on_pointer(
            PointerEvent::Move {
                cell: (col, row),
                target,
            },
            diagram,
        )
    }

    fn up(
        editor: &mut EditorState,
        diagram: &mut Diagram,
        col: u32,
        row: u32,
        connecting_from: Option<&AnchorRef>,
    ) -> EditOutcome {
        let target = hit_target(diagram, &catalog(), None, connecting_from, (col, row));
        editor.on_pointer(PointerEvent::Up { target }, diagram)
    }

    fn first_shape(diagram: &Diagram) -> &ShapeInstance {
        diagram
            .objects()
            .iter()
            .find_map(|o| match o {
                Object::Shape(shape) => Some(shape),
                _ => None,
            })
            .expect("a shape exists")
    }

    /// A placed 9×5 box at (0,0) — the built-in `box` footprint.
    fn diagram_with_box() -> Diagram {
        let mut diagram = Diagram::new();
        let id = diagram.new_object_id();
        diagram
            .place_shape(
                ShapeInstance::new(
                    id,
                    "box",
                    "",
                    CellPos::new(0, 0),
                    CellSize::new(9, 5).unwrap(),
                )
                .expect("valid instance"),
            )
            .expect("placement is valid");
        diagram
    }

    #[test]
    fn clicking_a_template_then_the_canvas_places_at_the_clicked_cell() {
        let mut editor = EditorState::default();
        let mut diagram = Diagram::new();

        editor.select_template(info("box", "Box", 9, 5));
        // `new_object_id()` hands out ids starting at obj-1.
        assert_eq!(
            down(&mut editor, &mut diagram, 3, 4),
            EditOutcome::Placed {
                id: ObjectId::generated(1),
                type_id: "box".to_owned()
            }
        );

        let shape = first_shape(&diagram);
        assert_eq!((shape.pos.x, shape.pos.y), (3, 4));
        assert_eq!((shape.size.width, shape.size.height), (9, 5));
        assert!(shape.label.is_empty());
        // Placement mode is single-shot and the new object is selected.
        assert!(editor.placing.is_none());
        assert_eq!(editor.selection, Some(ObjectId::generated(1)));
    }

    #[test]
    fn document_order_is_z_order_for_multiple_placements() {
        let mut editor = EditorState::default();
        let mut diagram = Diagram::new();

        editor.select_template(info("box", "Box", 9, 5));
        down(&mut editor, &mut diagram, 0, 0);
        editor.select_template(info("ellipse", "Ellipse", 9, 5));
        down(&mut editor, &mut diagram, 12, 0);

        let ids: Vec<ObjectId> = diagram.objects().iter().map(Object::id).cloned().collect();
        assert_eq!(ids, vec![ObjectId::generated(1), ObjectId::generated(2)]);
    }

    #[test]
    fn clicking_an_empty_cell_deselects() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_box();
        down(&mut editor, &mut diagram, 2, 1);
        assert_eq!(
            down(&mut editor, &mut diagram, 30, 20),
            EditOutcome::Deselected
        );
        assert!(editor.selection.is_none());
    }

    #[test]
    fn clicking_a_shape_selects_the_topmost_on_overlap() {
        let mut editor = EditorState::default();
        let mut diagram = Diagram::new();

        // Two boxes overlapping at cell (5,2): the second is topmost (later in document order).
        for col in [0u32, 5] {
            let id = diagram.new_object_id();
            diagram
                .place_shape(
                    ShapeInstance::new(
                        id,
                        "box",
                        "",
                        CellPos::new(col, 0),
                        CellSize::new(9, 5).unwrap(),
                    )
                    .expect("valid"),
                )
                .expect("placement is valid");
        }

        assert_eq!(
            down(&mut editor, &mut diagram, 5, 2),
            EditOutcome::Selected(ObjectId::generated(2))
        );
    }

    #[test]
    fn a_single_gesture_both_selects_and_drags_the_shape() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_box(); // box at (0,0), 9×5

        // Press on cell (2,1) — inside the box, grab offset (2,1).
        assert_eq!(
            down(&mut editor, &mut diagram, 2, 1),
            EditOutcome::Selected(ObjectId::generated(1))
        );

        // Drag to cell (10,7): the box's top-left follows with the offset preserved.
        assert_eq!(
            move_to(&mut editor, &mut diagram, 10, 7),
            EditOutcome::Moved {
                id: ObjectId::generated(1),
                to: CellPos::new(8, 6)
            }
        );
        assert_eq!(
            (first_shape(&diagram).pos.x, first_shape(&diagram).pos.y),
            (8, 6)
        );

        // Moving within the same cell is a no-op — no redundant model mutation.
        assert_eq!(
            move_to(&mut editor, &mut diagram, 10, 7),
            EditOutcome::Nothing
        );

        // Release ends the drag; further moves do nothing.
        up(&mut editor, &mut diagram, 10, 7, None);
        assert_eq!(
            move_to(&mut editor, &mut diagram, 20, 7),
            EditOutcome::Nothing
        );
    }

    #[test]
    fn dragging_clamps_at_the_grid_origin() {
        let mut editor = EditorState::default();
        let mut diagram = Diagram::new();
        let id = diagram.new_object_id();
        diagram
            .place_shape(
                ShapeInstance::new(
                    id.clone(),
                    "box",
                    "",
                    CellPos::new(5, 5),
                    CellSize::new(9, 5).unwrap(),
                )
                .expect("valid"),
            )
            .expect("placement is valid");

        editor.on_pointer(
            PointerEvent::Down {
                cell: (7, 6),
                target: HitTarget::Shape(id.clone()),
            },
            &mut diagram,
        ); // offset (2,1)
        // Drag far to the top-left corner: (1,0) - (2,1) saturates to (0,0).
        assert_eq!(
            move_to(&mut editor, &mut diagram, 1, 0),
            EditOutcome::Moved {
                id,
                to: CellPos::new(0, 0)
            }
        );
    }

    #[test]
    fn moves_without_a_drag_do_nothing() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_box();
        assert_eq!(
            move_to(&mut editor, &mut diagram, 4, 2),
            EditOutcome::Nothing
        );
        assert_eq!(
            (first_shape(&diagram).pos.x, first_shape(&diagram).pos.y),
            (0, 0)
        );
    }

    /// A drag that never crosses a cell boundary mutates nothing (no-op moves).
    #[test]
    fn a_drag_without_cell_crossing_is_a_noop() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_box(); // 9×5 at (0,0)

        // Move gesture: press inside, hover the same cell again.
        down(&mut editor, &mut diagram, 1, 1);
        assert_eq!(
            move_to(&mut editor, &mut diagram, 1, 1),
            EditOutcome::Nothing
        );

        // Resize gesture: press the handle, hover it again.
        down(&mut editor, &mut diagram, 8, 4);
        assert_eq!(
            move_to(&mut editor, &mut diagram, 8, 4),
            EditOutcome::Nothing
        );
        assert_eq!(first_shape(&diagram).size, CellSize::new(9, 5).unwrap());
    }

    /// `connecting_source` exposes the in-flight connection for hit-testing/overlays.
    #[test]
    fn connecting_source_tracks_the_inflight_connection() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes(); // s1 at (0,0), s2 elsewhere
        assert!(editor.connecting_source().is_none());

        down(&mut editor, &mut diagram, 1, 1); // select s1 + start move
        down(&mut editor, &mut diagram, 8, 2); // its e anchor → connect gesture
        let from = editor.connecting_source().expect("in flight");
        assert_eq!(from.object(), &ObjectId::generated(1));
        assert_eq!(from.anchor(), "e");

        editor.on_escape();
        assert!(editor.connecting_source().is_none());
    }

    #[test]
    fn escape_cancels_placement_and_returns_to_selection_mode() {
        let mut editor = EditorState::default();
        let mut diagram = Diagram::new();

        editor.select_template(info("box", "Box", 9, 5));
        assert_eq!(editor.on_escape(), EditOutcome::Cancelled);
        // Back in selection mode: an empty-cell press only deselects.
        assert_eq!(
            down(&mut editor, &mut diagram, 1, 1),
            EditOutcome::Deselected
        );
        assert_eq!(diagram.len(), 0);
    }

    #[test]
    fn a_malformed_template_type_id_is_not_placed() {
        let mut editor = EditorState::default();
        let mut diagram = Diagram::new();

        // Whitespace in the type id violates the model invariant — placement is refused.
        editor.select_template(info("bad id", "Bad", 3, 3));
        assert_eq!(down(&mut editor, &mut diagram, 1, 1), EditOutcome::Nothing);
        assert_eq!(diagram.len(), 0);
    }

    // ── S-013: resize ────────────────────────────────────────────────────────────

    #[test]
    fn dragging_the_resize_handle_grows_the_shape_grid_aligned() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_box(); // 9×5 at (0,0)

        // Select first (handles are only offered on the selection)…
        assert_eq!(
            down(&mut editor, &mut diagram, 1, 1),
            EditOutcome::Selected(ObjectId::generated(1))
        );
        // …then press the bottom-right cell (8,4): that is the resize handle, not a move.
        assert_eq!(down(&mut editor, &mut diagram, 8, 4), EditOutcome::Nothing);
        assert_eq!(first_shape(&diagram).size, CellSize::new(9, 5).unwrap());

        // Drag to (12,7): extent becomes 13×8.
        assert_eq!(
            move_to(&mut editor, &mut diagram, 12, 7),
            EditOutcome::Resized {
                id: ObjectId::generated(1),
                to: CellSize::new(13, 8).unwrap()
            }
        );
        let shape = first_shape(&diagram);
        assert_eq!(
            (shape.pos.x, shape.pos.y),
            (0, 0),
            "the top-left stays fixed"
        );
        assert_eq!((shape.size.width, shape.size.height), (13, 8));

        // Shrinking works too.
        assert_eq!(
            move_to(&mut editor, &mut diagram, 4, 2),
            EditOutcome::Resized {
                id: ObjectId::generated(1),
                to: CellSize::new(5, 3).unwrap()
            }
        );
    }

    #[test]
    fn resize_clamps_at_one_by_one() {
        let mut editor = EditorState::default();
        let mut diagram = Diagram::new();
        let id = diagram.new_object_id();
        diagram
            .place_shape(
                ShapeInstance::new(
                    id.clone(),
                    "box",
                    "",
                    CellPos::new(5, 5),
                    CellSize::new(9, 5).unwrap(),
                )
                .expect("valid"),
            )
            .expect("placement is valid");

        // Select first (the handle only exists for the selected shape)…
        assert_eq!(
            down(&mut editor, &mut diagram, 6, 6),
            EditOutcome::Selected(id.clone())
        );
        // …then press the bottom-right cell (13,9) and drag far past the top-left:
        // the extent saturates at 1×1.
        down(&mut editor, &mut diagram, 13, 9);
        assert_eq!(
            move_to(&mut editor, &mut diagram, 5, 5),
            EditOutcome::Resized {
                id,
                to: CellSize::new(1, 1).unwrap()
            }
        );
    }

    // ── S-013: connect ───────────────────────────────────────────────────────────

    /// Two boxes with a known gap: s1 at (0,0), s2 at (20,0) — both 9×5.
    fn two_boxes() -> Diagram {
        let mut diagram = Diagram::new();
        for col in [0u32, 20] {
            let id = diagram.new_object_id();
            diagram
                .place_shape(
                    ShapeInstance::new(
                        id,
                        "box",
                        "",
                        CellPos::new(col, 0),
                        CellSize::new(9, 5).unwrap(),
                    )
                    .expect("valid"),
                )
                .expect("placement is valid");
        }
        diagram
    }

    #[test]
    fn dragging_between_anchors_creates_a_connector() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        let s1 = ObjectId::generated(1);
        let s2 = ObjectId::generated(2);

        // Select s1, then press its e anchor (8,2) — the connection starts.
        assert_eq!(
            down(&mut editor, &mut diagram, 1, 1),
            EditOutcome::Selected(s1.clone())
        );
        assert_eq!(
            down(&mut editor, &mut diagram, 8, 2),
            EditOutcome::Connecting {
                from: AnchorRef::new(s1.clone(), "e").unwrap()
            }
        );

        // Hover mid-gap: pure hover, no mutation.
        assert_eq!(
            move_to(&mut editor, &mut diagram, 14, 3),
            EditOutcome::Nothing
        );
        assert_eq!(diagram.len(), 2);

        // Release over s2's w anchor (20,2): the connector lands.
        let source = AnchorRef::new(s1.clone(), "e").unwrap();
        assert_eq!(
            up(&mut editor, &mut diagram, 20, 2, Some(&source)),
            EditOutcome::Connected {
                id: ObjectId::generated(3)
            }
        );

        let connector = diagram
            .objects()
            .iter()
            .find_map(|o| match o {
                Object::Connector(c) => Some(c),
                _ => None,
            })
            .expect("the connector exists");
        assert_eq!(connector.from, AnchorRef::new(s1, "e").unwrap());
        assert_eq!(connector.to, AnchorRef::new(s2, "w").unwrap());
        assert_eq!(connector.style, ConnectorStyle::Solid);
    }

    #[test]
    fn dropping_outside_an_anchor_cancels_without_mutation() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        let s1 = ObjectId::generated(1);

        assert_eq!(
            down(&mut editor, &mut diagram, 1, 1),
            EditOutcome::Selected(s1.clone())
        );
        down(&mut editor, &mut diagram, 8, 2); // start connecting from e
        let source = AnchorRef::new(s1, "e").unwrap();

        // Drop on empty space — no connector, gesture over.
        assert_eq!(
            up(&mut editor, &mut diagram, 14, 9, Some(&source)),
            EditOutcome::Nothing
        );
        assert_eq!(diagram.len(), 2);

        // A second attempt dropped on a shape body (not an anchor) also cancels.
        down(&mut editor, &mut diagram, 8, 2);
        assert_eq!(
            up(&mut editor, &mut diagram, 24, 1, Some(&source)),
            EditOutcome::Nothing
        );
        assert_eq!(diagram.len(), 2);
    }

    #[test]
    fn dropping_on_the_source_anchor_cancels() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        let s1 = ObjectId::generated(1);

        assert_eq!(
            down(&mut editor, &mut diagram, 1, 1),
            EditOutcome::Selected(s1.clone())
        );
        down(&mut editor, &mut diagram, 8, 2); // start connecting from e
        let source = AnchorRef::new(s1, "e").unwrap();

        // The source anchor is never reported as a target (hit_target excludes it)…
        assert_eq!(
            up(&mut editor, &mut diagram, 8, 2, Some(&source)),
            EditOutcome::Nothing
        );
        assert_eq!(diagram.len(), 2);
    }

    // ── S-013: delete + escape ───────────────────────────────────────────────────

    #[test]
    fn delete_removes_the_selection_and_its_connectors() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        let s1 = ObjectId::generated(1);
        let s2 = ObjectId::generated(2);
        // Two connectors out of s1 (e→w on both… use distinct anchors for validity).
        diagram
            .connect(ConnectorInstance::new(
                diagram.new_object_id(),
                AnchorRef::new(s1.clone(), "e").unwrap(),
                AnchorRef::new(s2.clone(), "w").unwrap(),
                ConnectorStyle::Solid,
            ))
            .expect("valid connector");
        diagram
            .connect(ConnectorInstance::new(
                diagram.new_object_id(),
                AnchorRef::new(s1.clone(), "n").unwrap(),
                AnchorRef::new(s2.clone(), "s").unwrap(),
                ConnectorStyle::Solid,
            ))
            .expect("valid connector");
        assert_eq!(diagram.len(), 4);

        assert_eq!(
            down(&mut editor, &mut diagram, 1, 1),
            EditOutcome::Selected(s1.clone())
        );
        assert_eq!(
            editor.delete_selection(&mut diagram),
            EditOutcome::Deleted {
                id: s1.clone(),
                removed: 3
            }
        );

        // No dangling endpoints: only s2 remains.
        assert_eq!(diagram.len(), 1);
        assert!(diagram.get(&s2).is_some());
        assert!(editor.selection.is_none());
    }

    #[test]
    fn delete_without_a_selection_is_a_noop() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        assert_eq!(editor.delete_selection(&mut diagram), EditOutcome::Nothing);
        assert_eq!(diagram.len(), 2);
    }

    #[test]
    fn escape_cancels_gestures_placement_and_selection() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        let s1 = ObjectId::generated(1);

        // Cancels an in-flight connection.
        assert_eq!(
            down(&mut editor, &mut diagram, 1, 1),
            EditOutcome::Selected(s1.clone())
        );
        down(&mut editor, &mut diagram, 8, 2);
        assert_eq!(editor.on_escape(), EditOutcome::Cancelled);

        // Cancels placement mode.
        editor.select_template(info("box", "Box", 9, 5));
        assert_eq!(editor.on_escape(), EditOutcome::Cancelled);
        assert!(editor.placing.is_none());

        // Deselects: the press also starts a move gesture, so the first escape drops
        // the gesture and the second clears the selection.
        down(&mut editor, &mut diagram, 1, 1);
        assert_eq!(editor.on_escape(), EditOutcome::Cancelled);
        assert_eq!(editor.on_escape(), EditOutcome::Deselected);
        // And is a no-op when there is nothing to cancel.
        assert_eq!(editor.on_escape(), EditOutcome::Nothing);
    }
}
