//! The canvas gesture state machine (S-012 AC3/AC4) — pure interaction logic,
//! unit-testable headlessly against a real [`Diagram`] (T7 pattern).
//!
//! Per ADR 009 the GUI mutates the aggregate directly: this module turns semantic
//! pointer events into exactly those mutations. Selection and placement mode are
//! transient *view* state — they never touch the model.

use diagram_model::Diagram;
use diagram_model::geometry::CellPos;
use diagram_model::objects::{Object, ObjectId, ShapeInstance};
use ui_api::ShapeInfo;

/// Semantic canvas pointer events — egui input is mapped to these in `app.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerEvent {
    /// Primary button pressed on the given grid cell.
    Down { col: u32, row: u32 },
    /// Pointer moved (primary held) over the given grid cell.
    Move { col: u32, row: u32 },
    /// Primary button released anywhere.
    Up,
}

/// What a pointer event did to the model — drives the status bar and the tests.
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
}

/// An in-progress drag: which object and where on it the pointer grabbed.
#[derive(Debug, Clone)]
struct Drag {
    id: ObjectId,
    offset_x: u32,
    offset_y: u32,
}

/// Transient view state + gesture machine (ADR 009: selection/placement are view
/// state only — the model is mutated exclusively through [`Diagram`] operations).
#[derive(Debug, Default)]
pub struct EditorState {
    /// The template picked in the sidebar; the next canvas click places it.
    pub placing: Option<ShapeInfo>,
    /// The currently selected shape (if any).
    pub selection: Option<ObjectId>,
    drag: Option<Drag>,
}

impl EditorState {
    /// Sidebar action: pick a template to place (replaces a previous pick, clears the
    /// selection so the next click is unambiguously a placement).
    pub fn select_template(&mut self, info: ShapeInfo) {
        self.placing = Some(info);
        self.selection = None;
    }

    /// Cancel placement mode (Esc / right-click) and return to selection/drag.
    pub fn cancel_placing(&mut self) {
        self.placing = None;
    }

    /// Applies one pointer event, mutating the model in place. Returns what happened.
    pub fn on_pointer(&mut self, event: PointerEvent, diagram: &mut Diagram) -> EditOutcome {
        match event {
            PointerEvent::Down { col, row } => self.on_down(col, row, diagram),
            PointerEvent::Move { col, row } => self.on_move(col, row, diagram),
            PointerEvent::Up => {
                self.drag = None;
                EditOutcome::Nothing
            }
        }
    }

    fn on_down(&mut self, col: u32, row: u32, diagram: &mut Diagram) -> EditOutcome {
        if let Some(info) = self.placing.clone() {
            return self.place(col, row, info, diagram);
        }
        match hit_shape(diagram, col, row) {
            Some((id, pos)) => {
                self.selection = Some(id.clone());
                // Grab immediately: one press-drag gesture both selects and moves.
                self.drag = Some(Drag {
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

    fn on_move(&mut self, col: u32, row: u32, diagram: &mut Diagram) -> EditOutcome {
        let Some(drag) = self.drag.as_ref() else {
            return EditOutcome::Nothing;
        };
        // The shape follows the pointer with the grab offset preserved; saturating
        // subtraction clamps at the grid origin (positions are unsigned by design).
        let to = CellPos::new(
            col.saturating_sub(drag.offset_x),
            row.saturating_sub(drag.offset_y),
        );
        let Some(Object::Shape(shape)) = diagram.get(&drag.id) else {
            return EditOutcome::Nothing;
        };
        if shape.pos == to {
            return EditOutcome::Nothing; // no cell crossing — nothing to mutate
        }
        match diagram.move_object(&drag.id, to) {
            Ok(()) => EditOutcome::Moved {
                id: drag.id.clone(),
                to,
            },
            Err(_) => EditOutcome::Nothing,
        }
    }
}

/// The topmost shape containing the given cell — last in document order wins, which
/// is exactly the z-order (document order = z-order, C5). Connectors and text are not
/// selectable yet (S-013/S-018).
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
    use diagram_model::geometry::CellSize;

    fn info(type_id: &str, name: &str, width: u32, height: u32) -> ShapeInfo {
        ShapeInfo {
            type_id: type_id.to_owned(),
            name: name.to_owned(),
            size: CellSize::new(width, height).expect("test sizes are valid"),
        }
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
            editor.on_pointer(PointerEvent::Down { col: 3, row: 4 }, &mut diagram),
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
        editor.on_pointer(PointerEvent::Down { col: 0, row: 0 }, &mut diagram);
        editor.select_template(info("ellipse", "Ellipse", 9, 5));
        editor.on_pointer(PointerEvent::Down { col: 12, row: 0 }, &mut diagram);

        let ids: Vec<ObjectId> = diagram.objects().iter().map(Object::id).cloned().collect();
        assert_eq!(ids, vec![ObjectId::generated(1), ObjectId::generated(2)]);
    }

    #[test]
    fn clicking_an_empty_cell_deselects() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_box();
        editor.on_pointer(PointerEvent::Down { col: 2, row: 1 }, &mut diagram);
        assert!(matches!(
            editor.on_pointer(PointerEvent::Down { col: 30, row: 20 }, &mut diagram),
            EditOutcome::Deselected
        ));
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
            editor.on_pointer(PointerEvent::Down { col: 5, row: 2 }, &mut diagram),
            EditOutcome::Selected(ObjectId::generated(2))
        );
    }

    #[test]
    fn a_single_gesture_both_selects_and_drags_the_shape() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_box(); // box at (0,0), 9×5

        // Press on cell (2,1) — inside the box, grab offset (2,1).
        assert_eq!(
            editor.on_pointer(PointerEvent::Down { col: 2, row: 1 }, &mut diagram),
            EditOutcome::Selected(ObjectId::generated(1))
        );

        // Drag to cell (10,7): the box's top-left follows with the offset preserved.
        assert_eq!(
            editor.on_pointer(PointerEvent::Move { col: 10, row: 7 }, &mut diagram),
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
            editor.on_pointer(PointerEvent::Move { col: 10, row: 7 }, &mut diagram),
            EditOutcome::Nothing
        );

        // Release ends the drag; further moves do nothing.
        assert_eq!(
            editor.on_pointer(PointerEvent::Up, &mut diagram),
            EditOutcome::Nothing
        );
        assert_eq!(
            editor.on_pointer(PointerEvent::Move { col: 20, row: 7 }, &mut diagram),
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

        editor.on_pointer(PointerEvent::Down { col: 7, row: 6 }, &mut diagram); // offset (2,1)
        // Drag far to the top-left corner: (1,0) - (2,1) saturates to (0,0).
        assert_eq!(
            editor.on_pointer(PointerEvent::Move { col: 1, row: 0 }, &mut diagram),
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
            editor.on_pointer(PointerEvent::Move { col: 4, row: 2 }, &mut diagram),
            EditOutcome::Nothing
        );
        assert_eq!(
            (first_shape(&diagram).pos.x, first_shape(&diagram).pos.y),
            (0, 0)
        );
    }

    #[test]
    fn cancel_placing_returns_to_selection_mode() {
        let mut editor = EditorState::default();
        let mut diagram = Diagram::new();

        editor.select_template(info("box", "Box", 9, 5));
        editor.cancel_placing();
        assert_eq!(
            editor.on_pointer(PointerEvent::Down { col: 1, row: 1 }, &mut diagram),
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
        assert_eq!(
            editor.on_pointer(PointerEvent::Down { col: 1, row: 1 }, &mut diagram),
            EditOutcome::Nothing
        );
        assert_eq!(diagram.len(), 0);
    }
}
