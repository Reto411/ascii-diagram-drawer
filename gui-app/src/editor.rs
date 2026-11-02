//! The canvas gesture state machine (S-012 placement/select/move, S-013 resize and
//! anchor-to-anchor connect, S-018 free-text placement + inline editing) — pure
//! interaction logic, unit-testable headlessly against a real [`Diagram`] (T7 pattern).
//!
//! Per ADR 009 the GUI mutates the aggregate directly: this module turns semantic
//! pointer events into exactly those mutations. Selection, placement and in-flight
//! gestures are transient *view* state — they never touch the model.

use std::collections::BTreeSet;

use diagram_model::Diagram;
use diagram_model::anchors::{AnchorRef, ConnectorEndpoint};
use diagram_model::geometry::{CellPos, CellSize};
use diagram_model::objects::{
    ConnectorEnd, ConnectorInstance, ConnectorStyle, Object, ObjectId, ShapeInstance, TextInstance,
    TipStyle,
};

/// Initial content of a freshly placed text object (S-018) — the model rejects empty
/// content and inline editing opens at once, so this placeholder is replaced immediately.
const PLACEHOLDER_TEXT: &str = "text";
use ui_api::ShapeInfo;

use crate::canvas::HitTarget;

/// Semantic canvas pointer events — egui input is mapped to these in `app.rs`. The
/// `target` is the precomputed interactive hit under the cell (see [`crate::canvas::hit_target`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PointerEvent {
    /// Primary button pressed on the given grid cell. `additive` is true when Ctrl or
    /// Shift was held at the press — a body click then extends/toggles the selection
    /// instead of replacing it (S-035 FR20).
    Down {
        cell: (u32, u32),
        target: HitTarget,
        additive: bool,
    },
    /// Pointer moved (primary held or hovering) over the given grid cell.
    Move { cell: (u32, u32), target: HitTarget },
    /// Primary button released; `cell` is the release position in model space (`None`
    /// when released off-canvas) and `target` what it is over.
    Up {
        cell: Option<(u32, u32)>,
        target: HitTarget,
    },
    /// Double click (primary) on the given cell — opens inline editing of a text object
    /// under the pointer (S-018 AC1).
    DoubleClick { cell: (u32, u32), target: HitTarget },
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
    /// A new free-text object entered the model and inline editing opened (S-018 AC1).
    PlacedText { id: ObjectId },
    /// Inline editing of a text object started (double click — S-018 AC1).
    Editing { id: ObjectId },
    /// A shape instance was double-clicked (FR19 / CR-016): the app opens its
    /// region-content editor.
    DoubleClickShape { id: ObjectId },
    /// An inline edit committed its content to the model (Enter/blur — S-018 AC1).
    Edited { id: ObjectId },
    /// A dragged shape crossed to a new cell and the model was updated live.
    Moved { id: ObjectId, to: CellPos },
    /// A group move (S-035 FR20) crossed to new cells — `count` movable members were
    /// updated live with the same delta.
    GroupMoved { count: usize },
    /// A resize drag changed the selected shape's extent (min 1×1, grid-aligned).
    Resized { id: ObjectId, to: CellSize },
    /// A connection gesture started from an anchor.
    Connecting { from: AnchorRef },
    /// A connector was created between two anchors.
    Connected { id: ObjectId },
    /// The arrow tool created a new connector (CR-010) — its ends may be attached or
    /// free.
    ArrowPlaced { id: ObjectId },
    /// A connector end was re-attached to a shape anchor (CR-010).
    EndpointAttached { id: ObjectId, end: ConnectorEnd },
    /// An attached connector end was detached — it now follows the pointer (CR-010).
    EndpointDetached { id: ObjectId, end: ConnectorEnd },
    /// A free connector end moved to a new cell (CR-010).
    EndpointMoved {
        id: ObjectId,
        end: ConnectorEnd,
        to: CellPos,
    },
    /// A new bend entered the model at the dragged cell (CR-007 AC5) — live, on the
    /// first displacement of a body drag.
    BendAdded { id: ObjectId, at: CellPos },
    /// A bend followed the pointer (CR-007 AC5) — live during the drag; also reported
    /// after [`EditOutcome::BendAdded`] for the same gesture's later moves.
    BendMoved { id: ObjectId, to: CellPos },
    /// A dropped bend sat on the straight line between its neighbors and was removed
    /// (CR-007 AC6 — Visio-style cleanup).
    BendRemoved { id: ObjectId, index: usize },
    /// A press on a tip cell cycled that end's tip style (CR-008 AC2) — `tip` is the
    /// new value.
    TipChanged {
        id: ObjectId,
        end: ConnectorEnd,
        tip: TipStyle,
    },
    /// A press without movement on a selected connector's endpoint opened the
    /// in-canvas tip dropdown for that end (CR-020 AC4) — replacing S-029's
    /// press-to-cycle.
    TipDropdownOpened { id: ObjectId, end: ConnectorEnd },
    /// A body drag moved the whole connector (CR-020 AC1): endpoints and bends were
    /// translated live, bound ends detached along the way. The caller now re-attaches
    /// any free end that landed exactly on an anchor.
    ConnectorMoved { id: ObjectId },
    /// The selected object (and its connectors) left the model.
    Deleted { id: ObjectId, removed: usize },
    /// A multi-selection delete (S-035 FR20): `count` is the selection set's size and
    /// `removed` counts everything that actually left the model (the selected objects
    /// plus their cascaded connectors).
    BulkDeleted { count: usize, removed: usize },
    /// A modifier click changed the selection set without replacing it (S-035 FR20)
    /// — `count` is the set's size after the change.
    SelectionChanged { count: usize },
    /// An in-flight gesture or placement was cancelled without mutation.
    Cancelled,
}

/// A movable member of a group move (S-035 FR20): its id plus the grab offset — how
/// far the member's top-left sat from the press cell — so it follows the pointer with
/// exactly the same delta as every other member.
#[derive(Debug, Clone)]
struct GrabbedMember {
    id: ObjectId,
    offset_x: u32,
    offset_y: u32,
}

/// The in-canvas tip dropdown opened by pressing onto a selected endpoint (CR-020
/// AC4) — which connector end it offers styles for. Any canvas press, Escape or
/// right-click closes it without change; picking a style applies it.
#[derive(Debug, Clone)]
struct TipDropdown {
    id: ObjectId,
    end: ConnectorEnd,
}

/// An in-flight primary-button gesture (S-013).
#[derive(Debug, Clone)]
enum Gesture {
    /// Move drag (S-013; S-035 group move): every movable selected member with its
    /// grab offset — a plain click produces a one-member group, exactly today's
    /// single-object behavior; a modifier-clicked selection moves as one.
    Move { members: Vec<GrabbedMember> },
    /// Resize drag on the selected shape's bottom-right handle — with the catalog's
    /// minimum extent as the clamp (S-024 AC5).
    Resize { id: ObjectId, min_size: CellSize },
    /// Connect drag from a source anchor to wherever the pointer drops.
    Connect { from: AnchorRef },
    /// Dragging an existing bend (CR-007 AC5): `move_bend` live on every move; a drop
    /// on the straight line between its neighbors removes it (AC6). `prev`/`next` are
    /// the adjacent polyline points — fixed for the whole gesture. `moved` is false
    /// until the first cell crossing, so a press without movement never removes.
    BendMove {
        id: ObjectId,
        index: usize,
        prev: CellPos,
        next: CellPos,
        moved: bool,
    },
    /// Dragging a connector's body moves the whole arrow (CR-020 AC1): both endpoints
    /// and every declared bend translate by one delta. `from_cell`/`to_cell` are the
    /// resolved endpoint cells and `bends` the waypoints at press time — the snapshot
    /// the translation is applied to, so partial model rejections (e.g. both free ends
    /// on one cell) retry next frame instead of drifting. Bound ends detach in place
    /// on the first movement; release re-attaches them only exactly on an anchor.
    /// `moved` flips once the pointer has crossed a cell, so a press without movement
    /// only selects.
    ConnectorMove {
        id: ObjectId,
        pressed: CellPos,
        from_cell: CellPos,
        to_cell: CellPos,
        bends: Vec<CellPos>,
        moved: bool,
    },
    /// The arrow tool is drawing a connector (CR-010): `from` was resolved at press
    /// time (attached when the press landed on an anchor), `from_cell` its cell for
    /// the drag line; `moved` flips once the pointer has crossed a cell — a press
    /// without movement draws nothing.
    ArrowDrag {
        from: ConnectorEndpoint,
        from_cell: CellPos,
        moved: bool,
    },
    /// A connector endpoint is being dragged (CR-010): `at` is the end's current cell
    /// (kept in sync with every successful mutation); `was_attached` flips to `false`
    /// once an attached end has been detached; `moved` as in [`Gesture::ArrowDrag`] —
    /// a press without movement cycles the tip style at release instead (S-029).
    EndpointDrag {
        id: ObjectId,
        end: ConnectorEnd,
        at: CellPos,
        was_attached: bool,
        moved: bool,
    },
}

/// What a canvas click will place — the tool picked in the sidebar (S-012 shapes,
/// S-018 text).
#[derive(Debug, Clone)]
pub enum Placing {
    /// A shape template from the catalog. Boxed to keep the enum small
    /// (`clippy::large_enum_variant` — `ShapeInfo` carries preview rows and rules).
    Shape(Box<ShapeInfo>),
    /// A free-text object; inline editing opens immediately after placement (FR10).
    Text,
    /// The arrow tool (CR-010): press-drag-release draws a connector from the press
    /// cell to the release cell; ends attach when they land on shape anchors. Stays
    /// armed for further drags (Esc stops it).
    Arrow,
}

/// Transient view state + gesture machine (ADR 009: selection/placement/gestures are
/// view state only — the model is mutated exclusively through [`Diagram`] operations).
#[derive(Debug, Default)]
pub struct EditorState {
    /// The tool picked in the sidebar; the next canvas click places it.
    pub placing: Option<Placing>,
    /// The currently selected objects — a set since S-035 (FR20): Ctrl/Shift+click
    /// extends or toggles membership, a plain click replaces the set with one object.
    /// A `BTreeSet` so iteration order is deterministic (deletion order, overlays).
    pub selection: BTreeSet<ObjectId>,
    /// The text object being edited inline (S-018), if any.
    pub editing: Option<ObjectId>,
    /// The draft content of the inline edit — the egui `TextEdit` binds to this string
    /// directly; committed or discarded by [`Self::commit_edit`] / [`Self::cancel_edit`].
    pub edit_draft: String,
    gesture: Option<Gesture>,
    /// The open tip dropdown (CR-020 AC4), if any — the canvas draws it near the
    /// endpoint cell and offers the four styles for that end.
    tip_dropdown: Option<TipDropdown>,
}

impl EditorState {
    /// The open tip dropdown (CR-020 AC4) — the canvas anchors its menu at the
    /// endpoint cell of `(id, end)`.
    #[must_use]
    pub fn tip_dropdown(&self) -> Option<(&ObjectId, ConnectorEnd)> {
        self.tip_dropdown.as_ref().map(|d| (&d.id, d.end))
    }

    /// Closes an open tip dropdown without change (CR-020 AC4). Returns whether one
    /// was open — the canvas calls this for presses that did not pick a style.
    pub fn close_tip_dropdown(&mut self) -> bool {
        self.tip_dropdown.take().is_some()
    }

    /// Applies the style picked in the tip dropdown to that end (CR-020 AC4) and
    /// closes the menu. The press without movement that opened it already selected
    /// the connector, so the change is visible in context.
    pub fn apply_tip(
        &mut self,
        id: &ObjectId,
        end: ConnectorEnd,
        tip: TipStyle,
        diagram: &mut Diagram,
    ) -> EditOutcome {
        self.tip_dropdown = None;
        match diagram.set_tip(id, end, tip) {
            Ok(()) => EditOutcome::TipChanged {
                id: id.clone(),
                end,
                tip,
            },
            Err(_) => EditOutcome::Nothing, // defensive: the dropdown only opens for live ends
        }
    }

    /// Re-attaches a free endpoint dropped exactly on an anchor (CR-020 AC1) — the
    /// model op behind [`EditOutcome::ConnectorMoved`]: the caller resolves which
    /// anchor sits at the end's cell and hands it over.
    pub fn reattach_endpoint(
        &mut self,
        id: &ObjectId,
        end: ConnectorEnd,
        reference: AnchorRef,
        diagram: &mut Diagram,
    ) -> EditOutcome {
        match diagram.set_endpoint(id, end, ConnectorEndpoint::Attached(reference)) {
            Ok(()) => EditOutcome::EndpointAttached {
                id: id.clone(),
                end,
            },
            Err(_) => EditOutcome::Nothing, // defensive: the caller just hit-tested it
        }
    }
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
        self.placing = Some(Placing::Shape(Box::new(info)));
        self.selection.clear();
    }

    /// Sidebar action: pick the text tool (S-018). Same semantics as
    /// [`Self::select_template`]: replaces a previous pick, clears the selection.
    pub fn select_text_tool(&mut self) {
        self.placing = Some(Placing::Text);
        self.selection.clear();
    }

    /// Sidebar action: pick the arrow tool (CR-010). The next press-drag on the canvas
    /// draws a connector; the tool stays armed for further drags (Esc stops it).
    pub fn select_arrow_tool(&mut self) {
        self.placing = Some(Placing::Arrow);
        self.selection.clear();
    }

    /// The anchor exposure for canvas hit-testing/overlays under the current state
    /// (S-031): selection mode shows only the selected shape's anchors; a connection,
    /// an arrow drag or an endpoint drag exposes every shape's anchors as snap targets.
    #[must_use]
    pub fn anchor_exposure(&self) -> crate::canvas::AnchorExposure<'_> {
        match &self.gesture {
            Some(Gesture::Connect { from }) => {
                crate::canvas::AnchorExposure::Connect { source: from }
            }
            Some(Gesture::ArrowDrag { .. }) | Some(Gesture::EndpointDrag { .. }) => {
                crate::canvas::AnchorExposure::All
            }
            _ => match &self.placing {
                Some(Placing::Arrow) => crate::canvas::AnchorExposure::All,
                _ => crate::canvas::AnchorExposure::SelectionOnly,
            },
        }
    }

    /// The arrow tool's in-flight drag source (CR-010): the press cell and whether the
    /// source end attached to it — the canvas draws the drag line from that cell. Only
    /// set while an arrow is being drawn.
    #[must_use]
    pub fn arrow_drag_source(&self) -> Option<(CellPos, bool)> {
        match &self.gesture {
            Some(Gesture::ArrowDrag {
                from, from_cell, ..
            }) => Some((*from_cell, matches!(from, ConnectorEndpoint::Attached(_)))),
            _ => None,
        }
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
        // Escape inside an inline edit cancels the edit (AC1) — before anything else.
        if self.editing.is_some() {
            return self.cancel_edit();
        }
        // CR-020 AC4 — Escape closes an open tip dropdown without change; it never
        // coexists with a gesture (the menu opens at release), so this comes first.
        if self.tip_dropdown.take().is_some() {
            return EditOutcome::Cancelled;
        }
        let had_gesture = self.gesture.take().is_some();
        let was_placing = self.placing.take().is_some();
        if had_gesture || was_placing {
            return EditOutcome::Cancelled;
        }
        if !self.selection.is_empty() {
            self.selection.clear();
            return EditOutcome::Deselected;
        }
        EditOutcome::Nothing
    }

    /// Delete/Backspace (S-013; S-035 FR20): removes every selected object and, with
    /// each of them, every connector referencing it (the aggregate keeps endpoints
    /// from dangling). Objects already gone (cascaded by an earlier removal in the
    /// same pass) are skipped. No selection → no-op.
    pub fn delete_selection(&mut self, diagram: &mut Diagram) -> EditOutcome {
        if self.selection.is_empty() {
            return EditOutcome::Nothing;
        }
        let ids: Vec<ObjectId> = self.selection.iter().cloned().collect();
        let mut removed_total = 0usize;
        for id in &ids {
            // `remove` errors on a missing id — one selection member may have been
            // cascaded away by an earlier member's connector removal.
            if diagram.get(id).is_some()
                && let Ok(removed) = diagram.remove(id)
            {
                removed_total += removed;
            }
        }
        if removed_total == 0 {
            // Defensive: every member was already gone — nothing actually happened.
            return EditOutcome::Nothing;
        }
        self.selection.clear();
        self.gesture = None;
        match ids.len() {
            1 => EditOutcome::Deleted {
                id: ids.into_iter().next().expect("len == 1"),
                removed: removed_total,
            },
            _ => EditOutcome::BulkDeleted {
                count: ids.len(),
                removed: removed_total,
            },
        }
    }

    /// Applies one pointer event, mutating the model in place. Returns what happened.
    pub fn on_pointer(&mut self, event: PointerEvent, diagram: &mut Diagram) -> EditOutcome {
        match event {
            PointerEvent::Down {
                cell,
                target,
                additive,
            } => self.on_down(cell, target, additive, diagram),
            PointerEvent::Move { cell, .. } => self.on_move(cell, diagram),
            PointerEvent::Up { cell, target } => self.on_up(cell, target, diagram),
            PointerEvent::DoubleClick { cell, target } => {
                self.on_double_click(cell, target, diagram)
            }
        }
    }

    /// Commits the inline text edit with the current draft (Enter or focus loss — AC1).
    /// An empty draft cancels without mutation (the model rejects empty content).
    pub fn commit_edit(&mut self, diagram: &mut Diagram) -> EditOutcome {
        let Some(id) = self.editing.take() else {
            return EditOutcome::Nothing;
        };
        let draft = std::mem::take(&mut self.edit_draft);
        if draft.is_empty() || diagram.set_text(&id, &draft).is_err() {
            // Empty or (defensively) non-text target: cancel without mutation.
            return EditOutcome::Cancelled;
        }
        EditOutcome::Edited { id }
    }

    /// Cancels the inline text edit without mutation (Escape — AC1).
    pub fn cancel_edit(&mut self) -> EditOutcome {
        if self.editing.take().is_some() {
            self.edit_draft.clear();
            return EditOutcome::Cancelled;
        }
        EditOutcome::Nothing
    }

    fn on_down(
        &mut self,
        (col, row): (u32, u32),
        target: HitTarget,
        additive: bool,
        diagram: &mut Diagram,
    ) -> EditOutcome {
        // A pointer press while editing commits the inline edit (blur semantics — AC1)
        // and consumes the click; the next click acts on the canvas normally.
        if self.editing.is_some() {
            return self.commit_edit(diagram);
        }
        // Any canvas press closes an open tip dropdown first (CR-020 AC4) — the press
        // then proceeds normally (a press on the same endpoint reopens it at release).
        self.tip_dropdown = None;
        match self.placing.clone() {
            Some(Placing::Shape(info)) => return self.place(col, row, *info, diagram),
            Some(Placing::Text) => return self.place_text(col, row, diagram),
            // CR-010 — the arrow tool starts a drag instead of placing an object; it
            // stays armed for further drags.
            Some(Placing::Arrow) => {
                let outcome = self.start_arrow(col, row, target);
                self.placing = Some(Placing::Arrow);
                return outcome;
            }
            None => {}
        }
        match target {
            // Anchors are only offered for the selected shape (see `hit_target`), so a
            // pressed anchor always starts a connection from it.
            HitTarget::Anchor(object_id, name) => {
                let Ok(from) = AnchorRef::new(object_id.clone(), &name) else {
                    // Defensive: catalog anchor names are valid single tokens.
                    return self.grab(col, row, object_id, additive, diagram);
                };
                self.gesture = Some(Gesture::Connect { from: from.clone() });
                EditOutcome::Connecting { from }
            }
            HitTarget::ResizeHandle { id, min_size } => {
                self.gesture = Some(Gesture::Resize {
                    id: id.clone(),
                    min_size,
                });
                EditOutcome::Nothing
            }
            // Press on a shape or text body: plain = select + start the move gesture
            // (S-013 AC2); Ctrl/Shift = extend/toggle the selection set and, when a
            // member was added, grab the whole group (S-035 FR20).
            HitTarget::Body(id) => self.grab(col, row, id, additive, diagram),
            // CR-007 AC4 — pressing a connector (bend or body) selects it.
            HitTarget::Bend {
                id,
                index,
                prev,
                next,
            } => {
                let Some(Object::Connector(_)) = diagram.get(&id) else {
                    return EditOutcome::Nothing; // defensive: stale hit
                };
                self.select_for_gesture(id.clone(), additive);
                self.gesture = Some(Gesture::BendMove {
                    id: id.clone(),
                    index,
                    prev,
                    next,
                    moved: false,
                });
                self.press_outcome(additive, id)
            }
            // CR-020 AC1 — pressing a connector's body selects it and starts moving
            // the WHOLE arrow (endpoints + bends translate together); bend creation
            // moved to double-click (AC3).
            HitTarget::ConnectorBody {
                id,
                from_cell,
                to_cell,
                ..
            } => {
                let Some(Object::Connector(connector)) = diagram.get(&id) else {
                    return EditOutcome::Nothing; // defensive: stale hit
                };
                self.select_for_gesture(id.clone(), additive);
                self.gesture = Some(Gesture::ConnectorMove {
                    id: id.clone(),
                    pressed: CellPos::new(col, row),
                    from_cell,
                    to_cell,
                    bends: connector.bends.clone(),
                    moved: false,
                });
                self.press_outcome(additive, id)
            }
            // CR-010 — pressing an endpoint (or its tip cell) starts a drag: a free
            // end follows the pointer live, an attached end detaches on its first
            // move; a press without movement cycles the tip style at release
            // (S-029 behavior preserved).
            HitTarget::Endpoint { id, end, at } => {
                let Some(Object::Connector(connector)) = diagram.get(&id) else {
                    return EditOutcome::Nothing; // defensive: stale hit
                };
                self.select_for_gesture(id.clone(), additive);
                self.gesture = Some(Gesture::EndpointDrag {
                    id: id.clone(),
                    end,
                    at,
                    was_attached: match end {
                        ConnectorEnd::Source => connector.from.is_attached(),
                        ConnectorEnd::Target => connector.to.is_attached(),
                    },
                    moved: false,
                });
                self.press_outcome(additive, id)
            }
            // Plain press on empty space clears the selection (S-013); a modifier
            // press leaves it untouched (S-035 FR20).
            HitTarget::Empty => {
                if additive {
                    EditOutcome::Nothing
                } else {
                    self.selection.clear();
                    EditOutcome::Deselected
                }
            }
        }
    }

    /// Selection update for a press on a connector part (bend / body / endpoint):
    /// plain press replaces the set with that connector; a modifier press toggles its
    /// membership, keeping every other selected object (S-035 FR20).
    fn select_for_gesture(&mut self, id: ObjectId, additive: bool) {
        if additive {
            if !self.selection.remove(&id) {
                self.selection.insert(id);
            }
        } else {
            self.selection.clear();
            self.selection.insert(id);
        }
    }

    /// The press outcome for a target that also starts a gesture: plain presses report
    /// the (replaced) selection as before; modifier presses report the set's new size.
    fn press_outcome(&self, additive: bool, id: ObjectId) -> EditOutcome {
        if additive {
            EditOutcome::SelectionChanged {
                count: self.selection.len(),
            }
        } else {
            EditOutcome::Selected(id)
        }
    }

    /// The arrow tool's press (CR-010): the source end attaches when the press lands
    /// on a shape anchor and stays free at the pressed cell otherwise. The drag line
    /// runs from that cell; the target end is resolved at release ([`Self::on_up`]).
    fn start_arrow(&mut self, col: u32, row: u32, target: HitTarget) -> EditOutcome {
        let from = match target {
            HitTarget::Anchor(object_id, name) => AnchorRef::new(object_id, &name)
                .map(ConnectorEndpoint::Attached)
                .unwrap_or_else(|_| ConnectorEndpoint::Free(CellPos::new(col, row))),
            _ => ConnectorEndpoint::Free(CellPos::new(col, row)),
        };
        self.gesture = Some(Gesture::ArrowDrag {
            from,
            from_cell: CellPos::new(col, row),
            moved: false,
        });
        EditOutcome::Nothing
    }

    /// Replaces the selection set with exactly this object — the plain-click
    /// semantics every non-modifier press uses (S-035 FR20).
    fn replace_selection(&mut self, id: &ObjectId) {
        self.selection.clear();
        self.selection.insert(id.clone());
    }

    /// Press on an object body (S-013 AC2; S-035 FR20):
    /// - plain press — selects + starts the move gesture. If the pressed object is
    ///   already part of a multi-selection, the set is kept and the *whole group*
    ///   moves (FR20: dragging any selected member moves every member); otherwise
    ///   the selection becomes exactly this object (S-013's single-object behavior);
    /// - Ctrl/Shift press — toggles membership: an already-selected member is removed
    ///   (no drag); otherwise it joins the set and the *whole* group moves, each
    ///   member keeping its grab offset from the press cell.
    fn grab(
        &mut self,
        col: u32,
        row: u32,
        id: ObjectId,
        additive: bool,
        diagram: &Diagram,
    ) -> EditOutcome {
        if additive && self.selection.remove(&id) {
            // Toggled off — no drag, the rest of the group stays selected.
            return EditOutcome::SelectionChanged {
                count: self.selection.len(),
            };
        }
        if object_pos(diagram, &id).is_none() {
            // Defensive: stale hit (the object left the model this frame).
            if self.selection.is_empty() {
                return EditOutcome::Deselected;
            }
            return EditOutcome::Nothing;
        }
        let in_group = self.selection.contains(&id) && self.selection.len() > 1;
        if !additive && !in_group {
            // Plain click on an unselected object (or the only selected one):
            // replace the set — S-013's behavior, unchanged.
            self.selection.clear();
        }
        self.selection.insert(id.clone());
        // Grab immediately: one press-drag gesture both selects and moves — now the
        // whole set (S-035), connectors included in the selection but not movable.
        let members = self
            .selection
            .iter()
            .filter_map(|member| {
                object_pos(diagram, member).map(|p| GrabbedMember {
                    id: member.clone(),
                    offset_x: col.saturating_sub(p.x),
                    offset_y: row.saturating_sub(p.y),
                })
            })
            .collect();
        self.gesture = Some(Gesture::Move { members });
        match (additive, in_group) {
            // The set changed (Ctrl/Shift join) or stayed a group: report the size.
            (true, _) | (false, true) => EditOutcome::SelectionChanged {
                count: self.selection.len(),
            },
            // Plain single selection: today's exact outcome.
            (false, false) => EditOutcome::Selected(id),
        }
    }

    /// Double click (S-018 AC1): opens inline editing of the text object under the
    /// pointer; CR-020 AC3: double-clicking a connector's body segment inserts a bend
    /// at the clicked cell. Double-clicking shapes opens their region editor (FR19);
    /// anything else does nothing.
    fn on_double_click(
        &mut self,
        cell: (u32, u32),
        target: HitTarget,
        diagram: &mut Diagram,
    ) -> EditOutcome {
        // CR-020 AC3 — a double-click on a segment inserts a waypoint there (Visio);
        // the two single clicks of the pair only selected (no movement).
        if let HitTarget::ConnectorBody { id, segment, .. } = &target {
            let Some(Object::Connector(_)) = diagram.get(id) else {
                return EditOutcome::Nothing; // defensive: stale hit
            };
            self.replace_selection(id);
            let at = CellPos::new(cell.0, cell.1);
            match diagram.add_bend(id, *segment, at) {
                Ok(()) => EditOutcome::BendAdded { id: id.clone(), at },
                Err(_) => EditOutcome::Nothing, // defensive: the hit was just computed
            }
        } else {
            let HitTarget::Body(id) = target else {
                return EditOutcome::Nothing;
            };
            self.handle_double_click_body(&id, diagram)
        }
    }

    /// The body half of [`Self::on_double_click`] (S-018 AC1 / FR19).
    fn handle_double_click_body(&mut self, id: &ObjectId, diagram: &mut Diagram) -> EditOutcome {
        match diagram.get(id) {
            Some(Object::Text(text)) => {
                if self.editing.as_ref() != Some(id) {
                    self.editing = Some(id.clone());
                    self.edit_draft = text.text.clone();
                    self.replace_selection(id);
                }
                EditOutcome::Editing { id: id.clone() }
            }
            // FR19 (CR-016): double-clicking a shape opens its region-content editor
            // (the app decides whether it has regions; empty ones get a hint).
            Some(Object::Shape(_)) => EditOutcome::DoubleClickShape { id: id.clone() },
            _ => EditOutcome::Nothing,
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
        self.replace_selection(&id);
        EditOutcome::Placed {
            id,
            type_id: info.type_id,
        }
    }

    /// Places a free-text object at the clicked cell and opens inline editing of it
    /// immediately (S-018 AC1 — placement is followed by content entry in one gesture).
    fn place_text(&mut self, col: u32, row: u32, diagram: &mut Diagram) -> EditOutcome {
        let id = diagram.new_object_id();
        // Placeholder content — the inline editor opens at once and the user replaces it;
        // the model rejects empty content, so a placeholder is required.
        let instance = match TextInstance::new(id.clone(), CellPos::new(col, row), PLACEHOLDER_TEXT)
        {
            Ok(instance) => instance,
            Err(_) => return EditOutcome::Nothing, // defensive: the placeholder is always valid
        };
        if diagram.place_text(instance).is_err() {
            return EditOutcome::Nothing;
        }
        self.placing = None; // one placement per click (v1)
        self.replace_selection(&id);
        self.editing = Some(id.clone());
        self.edit_draft = PLACEHOLDER_TEXT.to_owned();
        EditOutcome::PlacedText { id }
    }

    fn on_move(&mut self, (col, row): (u32, u32), diagram: &mut Diagram) -> EditOutcome {
        let Some(gesture) = self.gesture.clone() else {
            return EditOutcome::Nothing;
        };
        match gesture {
            Gesture::Move { members } => {
                // Every movable member follows the pointer with its grab offset
                // preserved (S-035 FR20 — one delta for the whole group); saturating
                // subtraction clamps at the grid origin (positions are unsigned by
                // design). Connectors in the selection have no position of their own
                // and are skipped — attached endpoints follow their bound shapes.
                let mut moved = Vec::new();
                for member in &members {
                    let to = CellPos::new(
                        col.saturating_sub(member.offset_x),
                        row.saturating_sub(member.offset_y),
                    );
                    let at = match diagram.get(&member.id) {
                        Some(Object::Shape(shape)) => shape.pos,
                        Some(Object::Text(text)) => text.pos,
                        _ => continue,
                    };
                    if at == to {
                        continue; // no cell crossing for this member
                    }
                    if diagram.move_object(&member.id, to).is_ok() {
                        moved.push((member.id.clone(), to));
                    }
                }
                match (members.len(), moved.len()) {
                    (_, 0) => EditOutcome::Nothing,
                    (1, 1) => EditOutcome::Moved {
                        id: moved[0].0.clone(),
                        to: moved[0].1,
                    },
                    _ => EditOutcome::GroupMoved { count: moved.len() },
                }
            }
            Gesture::Resize { id, min_size } => {
                let Some(Object::Shape(shape)) = diagram.get(&id) else {
                    return EditOutcome::Nothing;
                };
                // The bottom-right corner follows the pointer: extent = distance from the
                // (fixed) top-left, clamped per axis to the shape's minimum extent
                // (S-024 AC5 — built-ins 1×1, rule-bearing customs their computed floor).
                let to = CellSize::new(
                    col.saturating_sub(shape.pos.x)
                        .saturating_add(1)
                        .max(min_size.width),
                    row.saturating_sub(shape.pos.y)
                        .saturating_add(1)
                        .max(min_size.height),
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
            Gesture::BendMove {
                id,
                index,
                prev,
                next,
                ..
            } => {
                // AC5 — free 2-D drag on the discrete grid: sharing a row/column with a
                // neighbor is exact, so axis-alignment needs no pull; a diagonal to a
                // neighbor appears exactly when the pointer is off both of its axes.
                let snapped = CellPos::new(col, row);
                if diagram.move_bend(&id, index, snapped).is_ok() {
                    // Persist the flag: this gesture is live until release (AC6's
                    // "moved" check reads it on drop).
                    self.gesture = Some(Gesture::BendMove {
                        id: id.clone(),
                        index,
                        prev,
                        next,
                        moved: true,
                    });
                    EditOutcome::BendMoved { id, to: snapped }
                } else {
                    EditOutcome::Nothing
                }
            }
            // CR-020 AC1 — whole-arrow move: every part (both endpoints + all bends)
            // translates by the pointer's delta from the press cell. Targets are
            // derived from the press-time snapshot, so a frame where one mutation is
            // refused (e.g. clamping at the canvas edge) retries next frame without
            // drift. A bound end is released by setting it free at its target in one
            // step — "detaches at its anchor cell" on the first movement.
            Gesture::ConnectorMove {
                id,
                pressed,
                from_cell,
                to_cell,
                bends,
                moved,
            } => {
                let cur = CellPos::new(col, row);
                if !moved && cur == pressed {
                    // Still on the pressed cell — a press without movement only selects.
                    return EditOutcome::Nothing;
                }
                let dx = i64::from(cur.x) - i64::from(pressed.x);
                let dy = i64::from(cur.y) - i64::from(pressed.y);
                let shift = |p: CellPos| -> CellPos {
                    CellPos::new(
                        (i64::from(p.x) + dx).clamp(0, i64::from(u32::MAX)) as u32,
                        (i64::from(p.y) + dy).clamp(0, i64::from(u32::MAX)) as u32,
                    )
                };
                let mut detached = None;
                for (end, cell) in [
                    (ConnectorEnd::Source, from_cell),
                    (ConnectorEnd::Target, to_cell),
                ] {
                    let Some(Object::Connector(connector)) = diagram.get(&id) else {
                        return EditOutcome::Nothing; // defensive: stale hit
                    };
                    let attached = match end {
                        ConnectorEnd::Source => connector.from.is_attached(),
                        ConnectorEnd::Target => connector.to.is_attached(),
                    };
                    let to = shift(cell);
                    if diagram
                        .set_endpoint(&id, end, ConnectorEndpoint::Free(to))
                        .is_ok()
                        && attached
                        && detached.is_none()
                    {
                        detached = Some(end);
                    }
                }
                for (index, bend) in bends.iter().enumerate() {
                    let to = shift(*bend);
                    if to != *bend {
                        // Rejection leaves the part where it is; next frame retries.
                        let _ = diagram.move_bend(&id, index, to);
                    }
                }
                self.gesture = Some(Gesture::ConnectorMove {
                    id: id.clone(),
                    pressed,
                    from_cell,
                    to_cell,
                    bends: bends.clone(),
                    moved: true,
                });
                match detached {
                    Some(end) => EditOutcome::EndpointDetached { id, end },
                    None => EditOutcome::Nothing,
                }
            }
            // CR-010 — arrow drag: pure hover until release; remember that the pointer
            // has crossed a cell so a bare click draws nothing.
            Gesture::ArrowDrag { from_cell, .. } => {
                if CellPos::new(col, row) != from_cell
                    && let Some(Gesture::ArrowDrag { moved, .. }) = self.gesture.as_mut()
                {
                    *moved = true;
                }
                EditOutcome::Nothing
            }
            // CR-010 — endpoint drag: a free end follows the pointer live; an attached
            // end detaches on its first move. Invalid targets (e.g. both-free on one
            // cell) are refused by the model and leave the gesture running.
            Gesture::EndpointDrag {
                id,
                end,
                at,
                was_attached,
                ..
            } => {
                let to = CellPos::new(col, row);
                if to == at {
                    return EditOutcome::Nothing;
                }
                match diagram.set_endpoint(&id, end, ConnectorEndpoint::Free(to)) {
                    Ok(()) => {
                        self.gesture = Some(Gesture::EndpointDrag {
                            id: id.clone(),
                            end,
                            at: to,
                            was_attached: false,
                            moved: true,
                        });
                        if was_attached {
                            EditOutcome::EndpointDetached { id, end }
                        } else {
                            EditOutcome::EndpointMoved { id, end, to }
                        }
                    }
                    Err(_) => EditOutcome::Nothing,
                }
            }
        }
    }

    fn on_up(
        &mut self,
        cell: Option<(u32, u32)>,
        target: HitTarget,
        diagram: &mut Diagram,
    ) -> EditOutcome {
        let gesture = self.gesture.take();
        match (gesture, cell, target) {
            (Some(Gesture::Connect { from }), _, HitTarget::Anchor(object_id, name)) => {
                // `hit_target` in connecting mode never reports the source anchor itself,
                // so a hit here is always a legal drop.
                let Ok(to) = AnchorRef::new(object_id, &name) else {
                    return EditOutcome::Nothing; // defensive: catalog names are valid
                };
                let id = diagram.new_object_id();
                let connector =
                    ConnectorInstance::between(id.clone(), from, to, ConnectorStyle::Solid);
                match diagram.connect(connector) {
                    Ok(()) => EditOutcome::Connected { id },
                    Err(_) => EditOutcome::Nothing, // defensive: endpoints were just hit-tested
                }
            }
            // CR-007 AC6 — dropping a *moved* bend on the straight line between its
            // neighbors removes it (Visio-style cleanup); otherwise it stays where the
            // live moves left it. A press without movement never removes.
            // CR-010 — arrow tool: a press without movement draws nothing; otherwise
            // the target end attaches when released on an anchor and stays free at the
            // release cell. Both-free on one cell is refused by the model (criterion 1).
            (Some(Gesture::ArrowDrag { from, moved, .. }), Some((col, row)), target) => {
                if !moved {
                    return EditOutcome::Nothing;
                }
                let to = match target {
                    HitTarget::Anchor(object_id, name) => AnchorRef::new(object_id, &name)
                        .map(ConnectorEndpoint::Attached)
                        .unwrap_or_else(|_| ConnectorEndpoint::Free(CellPos::new(col, row))),
                    _ => ConnectorEndpoint::Free(CellPos::new(col, row)),
                };
                let id = diagram.new_object_id();
                match diagram.connect(ConnectorInstance::new(
                    id.clone(),
                    from,
                    to,
                    ConnectorStyle::Solid,
                )) {
                    Ok(()) => EditOutcome::ArrowPlaced { id },
                    Err(_) => EditOutcome::Nothing, // defensive: endpoints were just hit-tested
                }
            }
            (Some(Gesture::ArrowDrag { .. }), None, _) => EditOutcome::Nothing,
            // CR-010/CR-020 — endpoint drag: a press without movement opens the
            // in-canvas tip dropdown for that end (AC4, replacing S-029's cycle);
            // otherwise the end attaches when released on an anchor and stays free at
            // the release cell.
            (
                Some(Gesture::EndpointDrag {
                    id, end, at, moved, ..
                }),
                cell,
                target,
            ) => {
                if !moved {
                    self.tip_dropdown = Some(TipDropdown {
                        id: id.clone(),
                        end,
                    });
                    return EditOutcome::TipDropdownOpened { id, end };
                }
                match target {
                    HitTarget::Anchor(object_id, name) => {
                        let Ok(reference) = AnchorRef::new(object_id, &name) else {
                            return EditOutcome::Nothing; // defensive: catalog names are valid
                        };
                        match diagram.set_endpoint(&id, end, ConnectorEndpoint::Attached(reference))
                        {
                            Ok(()) => EditOutcome::EndpointAttached { id, end },
                            Err(_) => EditOutcome::Nothing,
                        }
                    }
                    _ => {
                        let to = match cell {
                            Some((col, row)) if (col, row) != (at.x, at.y) => {
                                CellPos::new(col, row)
                            }
                            _ => at,
                        };
                        if to == at {
                            EditOutcome::Nothing
                        } else if diagram
                            .set_endpoint(&id, end, ConnectorEndpoint::Free(to))
                            .is_ok()
                        {
                            EditOutcome::EndpointMoved { id, end, to }
                        } else {
                            EditOutcome::Nothing
                        }
                    }
                }
            }
            (
                Some(Gesture::BendMove {
                    id,
                    index,
                    prev,
                    next,
                    moved,
                }),
                _,
                _,
            ) => {
                if moved
                    && let Some(Object::Connector(connector)) = diagram.get(&id)
                    && let Some(bend) = connector.bends.get(index)
                    && crate::canvas::on_segment(prev, next, *bend)
                {
                    match diagram.remove_bend(&id, index) {
                        Ok(()) => EditOutcome::BendRemoved { id, index },
                        Err(_) => EditOutcome::Nothing,
                    }
                } else {
                    EditOutcome::Nothing
                }
            }
            // CR-020 AC1 — release ends the whole-arrow move; the parts are already
            // wherever they last translated to. The caller (app) now re-attaches any
            // free end dropped exactly on an anchor.
            (Some(Gesture::ConnectorMove { id, moved, .. }), _, _) => {
                if moved {
                    EditOutcome::ConnectorMoved { id }
                } else {
                    EditOutcome::Nothing
                }
            }
            // Dropping anywhere else cancels the gesture without mutation (AC2).
            _ => EditOutcome::Nothing,
        }
    }
}

/// The top-left position of a movable object (shapes and free text); connectors have no
/// position of their own.
fn object_pos(diagram: &Diagram, id: &ObjectId) -> Option<CellPos> {
    match diagram.get(id)? {
        Object::Shape(shape) => Some(shape.pos),
        Object::Text(text) => Some(text.pos),
        Object::Connector(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::{HitTarget, hit_target};
    use core_services::fakes::FakeCatalog;
    use ui_api::{AnchorInfo, ShapeInfo};

    fn info(type_id: &str, name: &str, width: u32, height: u32) -> ShapeInfo {
        // Qualified ids (`collection/name`) belong to a collection group (ADR 009 v5).
        let collection = type_id
            .split('/')
            .next()
            .filter(|_| type_id.contains('/'))
            .map(str::to_owned);
        ShapeInfo {
            type_id: type_id.to_owned(),
            name: name.to_owned(),
            size: CellSize::new(width, height).expect("test sizes are valid"),
            collection,
            preview: Vec::new(),
            resizable: true,
            min_size: CellSize::new(1, 1).expect("valid size"),
            resize_rules: None,
            regions: Vec::new(),
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
    fn down(
        editor: &mut EditorState,
        diagram: &mut Diagram,
        col: u32,
        row: u32,
        routes: &[(ObjectId, Vec<CellPos>)],
    ) -> EditOutcome {
        let target = hit_target(
            diagram,
            &catalog(),
            &editor.selection,
            editor.anchor_exposure(),
            (col, row),
            routes,
        );
        editor.on_pointer(
            PointerEvent::Down {
                cell: (col, row),
                target,
                additive: false,
            },
            diagram,
        )
    }

    /// A Ctrl/Shift press — the S-035 additive variant of [`down`].
    fn down_additive(
        editor: &mut EditorState,
        diagram: &mut Diagram,
        col: u32,
        row: u32,
        routes: &[(ObjectId, Vec<CellPos>)],
    ) -> EditOutcome {
        let target = hit_target(
            diagram,
            &catalog(),
            &editor.selection,
            editor.anchor_exposure(),
            (col, row),
            routes,
        );
        editor.on_pointer(
            PointerEvent::Down {
                cell: (col, row),
                target,
                additive: true,
            },
            diagram,
        )
    }

    fn move_to(
        editor: &mut EditorState,
        diagram: &mut Diagram,
        col: u32,
        row: u32,
        routes: &[(ObjectId, Vec<CellPos>)],
    ) -> EditOutcome {
        let target = hit_target(
            diagram,
            &catalog(),
            &editor.selection,
            editor.anchor_exposure(),
            (col, row),
            routes,
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
        routes: &[(ObjectId, Vec<CellPos>)],
    ) -> EditOutcome {
        let target = hit_target(
            diagram,
            &catalog(),
            &editor.selection,
            editor.anchor_exposure(),
            (col, row),
            routes,
        );
        editor.on_pointer(
            PointerEvent::Up {
                cell: Some((col, row)),
                target,
            },
            diagram,
        )
    }

    /// A double click at a cell (S-018 / CR-020 AC3).
    fn double_click(
        editor: &mut EditorState,
        diagram: &mut Diagram,
        col: u32,
        row: u32,
        routes: &[(ObjectId, Vec<CellPos>)],
    ) -> EditOutcome {
        let target = hit_target(
            diagram,
            &catalog(),
            &editor.selection,
            editor.anchor_exposure(),
            (col, row),
            routes,
        );
        editor.on_pointer(
            PointerEvent::DoubleClick {
                cell: (col, row),
                target,
            },
            diagram,
        )
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
            down(&mut editor, &mut diagram, 3, 4, &[]),
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
        assert_eq!(
            editor.selection,
            std::iter::once(ObjectId::generated(1)).collect::<std::collections::BTreeSet<_>>(),
        );
    }

    /// S-016 AC3 — a custom (collection) template places at its fixed pattern size;
    /// the qualified id lands on the instance verbatim (the renderer stamps it,
    /// canvas.rs pins that it gets no resize handle).
    #[test]
    fn custom_templates_place_at_their_fixed_size() {
        let mut editor = EditorState::default();
        let mut diagram = Diagram::new();

        let star = info("stars/star", "star", 3, 3);
        assert_eq!(star.collection.as_deref(), Some("stars"));
        editor.select_template(star);
        assert_eq!(
            down(&mut editor, &mut diagram, 10, 2, &[]),
            EditOutcome::Placed {
                id: ObjectId::generated(1),
                type_id: "stars/star".to_owned()
            }
        );

        let shape = first_shape(&diagram);
        assert_eq!((shape.pos.x, shape.pos.y), (10, 2));
        // Placed at the template's natural size; whether it can be resized is the
        // catalog's `resizable` flag (S-024 AC5 — canvas.rs pins both cases).
        assert_eq!((shape.size.width, shape.size.height), (3, 3));
    }

    #[test]
    fn document_order_is_z_order_for_multiple_placements() {
        let mut editor = EditorState::default();
        let mut diagram = Diagram::new();

        editor.select_template(info("box", "Box", 9, 5));
        down(&mut editor, &mut diagram, 0, 0, &[]);
        editor.select_template(info("ellipse", "Ellipse", 9, 5));
        down(&mut editor, &mut diagram, 12, 0, &[]);

        let ids: Vec<ObjectId> = diagram.objects().iter().map(Object::id).cloned().collect();
        assert_eq!(ids, vec![ObjectId::generated(1), ObjectId::generated(2)]);
    }

    #[test]
    fn clicking_an_empty_cell_deselects() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_box();
        down(&mut editor, &mut diagram, 2, 1, &[]);
        assert_eq!(
            down(&mut editor, &mut diagram, 30, 20, &[]),
            EditOutcome::Deselected
        );
        assert!(editor.selection.is_empty());
    }

    // ── S-035: multi-select + group move (FR20) ─────────────────────────────

    /// A plain click selects exactly one object; a Ctrl/Shift click on a second
    /// object extends the set instead of replacing it (AC2).
    #[test]
    fn ctrl_click_on_a_second_shape_extends_the_selection() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        let s1 = ObjectId::generated(1);
        let s2 = ObjectId::generated(2);

        assert_eq!(
            down(&mut editor, &mut diagram, 4, 2, &[]),
            EditOutcome::Selected(s1.clone())
        );
        assert_eq!(editor.selection.len(), 1);

        // s2's body spans cols 20..29; (24,2) is well inside.
        assert_eq!(
            down_additive(&mut editor, &mut diagram, 24, 2, &[]),
            EditOutcome::SelectionChanged { count: 2 }
        );
        assert!(editor.selection.contains(&s1));
        assert!(editor.selection.contains(&s2));
    }

    /// A Ctrl/Shift click on an already-selected member removes it from the set
    /// (AC2 — toggle), without starting a drag.
    #[test]
    fn ctrl_click_on_a_selected_member_toggles_it_off() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        let s1 = ObjectId::generated(1);
        let s2 = ObjectId::generated(2);

        down(&mut editor, &mut diagram, 4, 2, &[]);
        down_additive(&mut editor, &mut diagram, 24, 2, &[]);
        assert_eq!(editor.selection.len(), 2);

        // A drag may have started after the extending click — release it first so
        // the toggle click is a clean press.
        up(&mut editor, &mut diagram, 24, 2, &[]);
        assert_eq!(
            down_additive(&mut editor, &mut diagram, 4, 2, &[]),
            EditOutcome::SelectionChanged { count: 1 }
        );
        assert!(!editor.selection.contains(&s1));
        assert!(editor.selection.contains(&s2));
    }

    /// Dragging after a Ctrl/Shift click moves the WHOLE group by the same delta —
    /// each member keeps its grab offset, so relative positions are preserved (AC4).
    #[test]
    fn drag_after_ctrl_click_moves_the_whole_group() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        let s1 = ObjectId::generated(1);
        let s2 = ObjectId::generated(2);

        down(&mut editor, &mut diagram, 4, 2, &[]); // select s1 (0,0)
        up(&mut editor, &mut diagram, 4, 2, &[]);
        // Ctrl-click s2's body: press cell (24,2) → offsets s1:(24,2), s2:(4,2).
        down_additive(&mut editor, &mut diagram, 24, 2, &[]);

        let moved = move_to(&mut editor, &mut diagram, 30, 5, &[]);
        assert_eq!(moved, EditOutcome::GroupMoved { count: 2 });

        // Both moved by the same (+6,+3) delta; the 20-col spacing is preserved.
        let p1 = object_pos(&diagram, &s1).expect("s1 exists");
        let p2 = object_pos(&diagram, &s2).expect("s2 exists");
        assert_eq!((p1.x, p1.y), (6, 3));
        assert_eq!((p2.x, p2.y), (26, 3));
        assert_eq!(p2.x - p1.x, 20);
    }

    /// A Ctrl/Shift press on empty space leaves the selection untouched (AC5) —
    /// only a plain click deselects.
    #[test]
    fn additive_press_on_empty_space_keeps_the_selection() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        let s1 = ObjectId::generated(1);

        down(&mut editor, &mut diagram, 4, 2, &[]);
        up(&mut editor, &mut diagram, 4, 2, &[]);

        assert_eq!(
            down_additive(&mut editor, &mut diagram, 30, 20, &[]),
            EditOutcome::Nothing
        );
        assert_eq!(editor.selection.len(), 1);
        assert!(editor.selection.contains(&s1));
    }

    /// Connectors can be part of the selection (Ctrl-click on their body) but have no
    /// position of their own: a group move shifts only the movable members while the
    /// connector follows its bound shapes via its endpoints (AC4).
    #[test]
    fn group_move_skips_connectors_in_the_selection() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        let s1 = ObjectId::generated(1);
        let s2 = ObjectId::generated(2);
        let c = diagram.new_object_id();
        diagram
            .connect(ConnectorInstance::between(
                c.clone(),
                AnchorRef::new(s1.clone(), "e").unwrap(),
                AnchorRef::new(s2.clone(), "w").unwrap(),
                ConnectorStyle::Solid,
            ))
            .expect("valid connector");

        let routes = vec![(c.clone(), vec![CellPos::new(8, 2), CellPos::new(20, 2)])];

        down(&mut editor, &mut diagram, 4, 2, &[]); // s1
        up(&mut editor, &mut diagram, 4, 2, &[]);
        down_additive(&mut editor, &mut diagram, 24, 2, &routes); // + s2
        up(&mut editor, &mut diagram, 24, 2, &routes);
        // Ctrl-click the connector body (its route runs along row 2 between the
        // boxes) — it joins the selection via the BendAdd press…
        assert_eq!(
            down_additive(&mut editor, &mut diagram, 15, 2, &routes),
            EditOutcome::SelectionChanged { count: 3 }
        );
        up(&mut editor, &mut diagram, 15, 2, &routes); // …released without moving

        // A plain drag on any selected member moves the movable ones (the two boxes);
        // the connector has no position of its own and follows its bound shapes.
        down(&mut editor, &mut diagram, 4, 2, &routes); // s1 — keeps the group
        let moved = move_to(&mut editor, &mut diagram, 30, 5, &routes);
        assert_eq!(moved, EditOutcome::GroupMoved { count: 2 });
        assert_eq!(diagram.len(), 3);
        assert!(diagram.get(&c).is_some()); // the connector survived with both ends
    }

    /// Deleting a multi-selection removes every member plus their connectors in one
    /// pass (AC6) and clears the selection.
    #[test]
    fn delete_with_a_multi_selection_removes_everything() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        let s1 = ObjectId::generated(1);
        let s2 = ObjectId::generated(2);
        diagram
            .connect(ConnectorInstance::between(
                diagram.new_object_id(),
                AnchorRef::new(s1.clone(), "e").unwrap(),
                AnchorRef::new(s2.clone(), "w").unwrap(),
                ConnectorStyle::Solid,
            ))
            .expect("valid connector");
        assert_eq!(diagram.len(), 3);

        down(&mut editor, &mut diagram, 4, 2, &[]); // s1
        up(&mut editor, &mut diagram, 4, 2, &[]);
        down_additive(&mut editor, &mut diagram, 24, 2, &[]); // + s2
        up(&mut editor, &mut diagram, 24, 2, &[]);

        assert_eq!(
            editor.delete_selection(&mut diagram),
            EditOutcome::BulkDeleted {
                count: 2,
                removed: 3
            }
        );
        assert_eq!(diagram.len(), 0);
        assert!(editor.selection.is_empty());
    }

    /// Escape clears the WHOLE selection at once (S-035 AC4 — FR20), not just one object.
    #[test]
    fn escape_clears_the_whole_selection() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();

        down(&mut editor, &mut diagram, 4, 2, &[]); // s1
        up(&mut editor, &mut diagram, 4, 2, &[]);
        down_additive(&mut editor, &mut diagram, 24, 2, &[]); // + s2
        up(&mut editor, &mut diagram, 24, 2, &[]);
        assert_eq!(editor.selection.len(), 2);

        assert_eq!(editor.on_escape(), EditOutcome::Deselected);
        assert!(editor.selection.is_empty());
    }

    /// FR20 — after a multi-selection exists, a PLAIN click-drag on any selected
    /// member keeps the set and moves the whole group (not just that member).
    #[test]
    fn plain_click_on_a_selected_member_moves_the_group() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        let s1 = ObjectId::generated(1);
        let s2 = ObjectId::generated(2);

        down(&mut editor, &mut diagram, 4, 2, &[]); // s1
        up(&mut editor, &mut diagram, 4, 2, &[]);
        down_additive(&mut editor, &mut diagram, 24, 2, &[]); // + s2
        up(&mut editor, &mut diagram, 24, 2, &[]);

        // Plain press on the already-selected s2: the group is kept…
        assert_eq!(
            down(&mut editor, &mut diagram, 24, 2, &[]),
            EditOutcome::SelectionChanged { count: 2 }
        );
        assert_eq!(editor.selection.len(), 2);

        // …and the drag moves both members (press cell (24,2): offsets s1:(24,2),
        // s2:(4,2) → target (30,5) gives +6/+3 for each).
        let moved = move_to(&mut editor, &mut diagram, 30, 5, &[]);
        assert_eq!(moved, EditOutcome::GroupMoved { count: 2 });
        let p1 = object_pos(&diagram, &s1).expect("s1 exists");
        let p2 = object_pos(&diagram, &s2).expect("s2 exists");
        assert_eq!((p1.x, p1.y), (6, 3));
        assert_eq!((p2.x, p2.y), (26, 3));
    }

    /// S-013's base case is preserved: a plain click on the ONLY selected object
    /// still selects exactly that one and moves only it.
    #[test]
    fn plain_click_on_the_only_selected_object_moves_only_it() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        let s1 = ObjectId::generated(1);
        let s2 = ObjectId::generated(2);

        down(&mut editor, &mut diagram, 4, 2, &[]); // s1 (0,0)
        up(&mut editor, &mut diagram, 4, 2, &[]);

        assert_eq!(
            down(&mut editor, &mut diagram, 4, 2, &[]),
            EditOutcome::Selected(s1.clone())
        );
        let moved = move_to(&mut editor, &mut diagram, 10, 6, &[]);
        assert_eq!(
            moved,
            EditOutcome::Moved {
                id: s1.clone(),
                to: CellPos::new(6, 4)
            }
        );
        // s2 did not move.
        let p2 = object_pos(&diagram, &s2).expect("s2 exists");
        assert_eq!((p2.x, p2.y), (20, 0));
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
            down(&mut editor, &mut diagram, 5, 2, &[]),
            EditOutcome::Selected(ObjectId::generated(2))
        );
    }

    #[test]
    fn a_single_gesture_both_selects_and_drags_the_shape() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_box(); // box at (0,0), 9×5

        // Press on cell (2,1) — inside the box, grab offset (2,1).
        assert_eq!(
            down(&mut editor, &mut diagram, 2, 1, &[]),
            EditOutcome::Selected(ObjectId::generated(1))
        );

        // Drag to cell (10,7): the box's top-left follows with the offset preserved.
        assert_eq!(
            move_to(&mut editor, &mut diagram, 10, 7, &[]),
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
            move_to(&mut editor, &mut diagram, 10, 7, &[]),
            EditOutcome::Nothing
        );

        // Release ends the drag; further moves do nothing.
        up(&mut editor, &mut diagram, 10, 7, &[]);
        assert_eq!(
            move_to(&mut editor, &mut diagram, 20, 7, &[]),
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
                target: HitTarget::Body(id.clone()),
                additive: false,
            },
            &mut diagram,
        ); // offset (2,1)
        // Drag far to the top-left corner: (1,0) - (2,1) saturates to (0,0).
        assert_eq!(
            move_to(&mut editor, &mut diagram, 1, 0, &[]),
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
            move_to(&mut editor, &mut diagram, 4, 2, &[]),
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
        down(&mut editor, &mut diagram, 1, 1, &[]);
        assert_eq!(
            move_to(&mut editor, &mut diagram, 1, 1, &[]),
            EditOutcome::Nothing
        );

        // Resize gesture: press the handle, hover it again.
        down(&mut editor, &mut diagram, 8, 4, &[]);
        assert_eq!(
            move_to(&mut editor, &mut diagram, 8, 4, &[]),
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

        down(&mut editor, &mut diagram, 1, 1, &[]); // select s1 + start move
        down(&mut editor, &mut diagram, 8, 2, &[]); // its e anchor → connect gesture
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
            down(&mut editor, &mut diagram, 1, 1, &[]),
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
            down(&mut editor, &mut diagram, 1, 1, &[]),
            EditOutcome::Nothing
        );
        assert_eq!(diagram.len(), 0);
    }

    // ── S-013: resize ────────────────────────────────────────────────────────────

    #[test]
    fn dragging_the_resize_handle_grows_the_shape_grid_aligned() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_box(); // 9×5 at (0,0)

        // Select first (handles are only offered on the selection)…
        assert_eq!(
            down(&mut editor, &mut diagram, 1, 1, &[]),
            EditOutcome::Selected(ObjectId::generated(1))
        );
        // …then press the bottom-right cell (8,4): that is the resize handle, not a move.
        assert_eq!(
            down(&mut editor, &mut diagram, 8, 4, &[]),
            EditOutcome::Nothing
        );
        assert_eq!(first_shape(&diagram).size, CellSize::new(9, 5).unwrap());

        // Drag to (12,7): extent becomes 13×8.
        assert_eq!(
            move_to(&mut editor, &mut diagram, 12, 7, &[]),
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
            move_to(&mut editor, &mut diagram, 4, 2, &[]),
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
            down(&mut editor, &mut diagram, 6, 6, &[]),
            EditOutcome::Selected(id.clone())
        );
        // …then press the bottom-right cell (13,9) and drag far past the top-left:
        // the extent saturates at 1×1.
        down(&mut editor, &mut diagram, 13, 9, &[]);
        assert_eq!(
            move_to(&mut editor, &mut diagram, 5, 5, &[]),
            EditOutcome::Resized {
                id,
                to: CellSize::new(1, 1).unwrap()
            }
        );
    }

    /// S-024 AC5 — rule-bearing customs clamp at their catalog minimum (not 1×1):
    /// dragging past the top-left stops at the per-axis `min_size` floor.
    #[test]
    fn resize_clamps_to_the_shape_minimum_extent() {
        let mut editor = EditorState::default();
        let mut diagram = Diagram::new();
        let id = diagram.new_object_id();
        diagram
            .place_shape(
                ShapeInstance::new(
                    id.clone(),
                    "stars/star",
                    "",
                    CellPos::new(5, 5),
                    CellSize::new(5, 3).unwrap(),
                )
                .expect("valid"),
            )
            .expect("placement is valid");

        editor.selection.clear();
        editor.selection.insert(id.clone());
        // Press the bottom-right handle (9,7) carrying the catalog's 2×2 floor.
        editor.on_pointer(
            PointerEvent::Down {
                cell: (9, 7),
                target: HitTarget::ResizeHandle {
                    id: id.clone(),
                    min_size: CellSize::new(2, 2).unwrap(),
                },
                additive: false,
            },
            &mut diagram,
        );

        // Drag onto the top-left corner itself: both axes clamp at 2×2.
        assert_eq!(
            editor.on_pointer(
                PointerEvent::Move {
                    cell: (5, 5),
                    target: HitTarget::Empty
                },
                &mut diagram,
            ),
            EditOutcome::Resized {
                id: id.clone(),
                to: CellSize::new(2, 2).unwrap()
            }
        );
        // One axis can still grow independently of the clamp.
        assert_eq!(
            editor.on_pointer(
                PointerEvent::Move {
                    cell: (9, 5),
                    target: HitTarget::Empty
                },
                &mut diagram,
            ),
            EditOutcome::Resized {
                id,
                to: CellSize::new(5, 2).unwrap()
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
            down(&mut editor, &mut diagram, 1, 1, &[]),
            EditOutcome::Selected(s1.clone())
        );
        assert_eq!(
            down(&mut editor, &mut diagram, 8, 2, &[]),
            EditOutcome::Connecting {
                from: AnchorRef::new(s1.clone(), "e").unwrap()
            }
        );

        // Hover mid-gap: pure hover, no mutation.
        assert_eq!(
            move_to(&mut editor, &mut diagram, 14, 3, &[]),
            EditOutcome::Nothing
        );
        assert_eq!(diagram.len(), 2);

        // Release over s2's w anchor (20,2): the connector lands.
        assert_eq!(
            up(&mut editor, &mut diagram, 20, 2, &[]),
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
        use diagram_model::anchors::ConnectorEndpoint;
        assert_eq!(
            connector.from,
            ConnectorEndpoint::Attached(AnchorRef::new(s1, "e").unwrap())
        );
        assert_eq!(
            connector.to,
            ConnectorEndpoint::Attached(AnchorRef::new(s2, "w").unwrap())
        );
        assert_eq!(connector.style, ConnectorStyle::Solid);
    }

    #[test]
    fn dropping_outside_an_anchor_cancels_without_mutation() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        let s1 = ObjectId::generated(1);

        assert_eq!(
            down(&mut editor, &mut diagram, 1, 1, &[]),
            EditOutcome::Selected(s1.clone())
        );
        down(&mut editor, &mut diagram, 8, 2, &[]); // start connecting from e

        // Drop on empty space — no connector, gesture over.
        assert_eq!(
            up(&mut editor, &mut diagram, 14, 9, &[]),
            EditOutcome::Nothing
        );
        assert_eq!(diagram.len(), 2);

        // A second attempt dropped on a shape body (not an anchor) also cancels.
        down(&mut editor, &mut diagram, 8, 2, &[]);
        assert_eq!(
            up(&mut editor, &mut diagram, 24, 1, &[]),
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
            down(&mut editor, &mut diagram, 1, 1, &[]),
            EditOutcome::Selected(s1.clone())
        );
        down(&mut editor, &mut diagram, 8, 2, &[]); // start connecting from e

        // The source anchor is never reported as a target (hit_target excludes it)…
        assert_eq!(
            up(&mut editor, &mut diagram, 8, 2, &[]),
            EditOutcome::Nothing
        );
        assert_eq!(diagram.len(), 2);
    }

    // ── S-031 (CR-010): arrow tool + endpoint drag ─────────────────────────────

    /// Two boxes with a straight connector A.e → B.w, drawn through the real gesture
    /// path (select A, drag its e anchor onto B's w anchor).
    fn connected_boxes() -> (EditorState, Diagram) {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        down(&mut editor, &mut diagram, 1, 1, &[]); // select A
        down(&mut editor, &mut diagram, 8, 2, &[]); // start connecting from A's e anchor
        move_to(&mut editor, &mut diagram, 14, 2, &[]);
        up(&mut editor, &mut diagram, 20, 2, &[]); // drop on B's w anchor
        (editor, diagram)
    }

    fn connector_of(diagram: &Diagram) -> &ConnectorInstance {
        diagram
            .objects()
            .iter()
            .find_map(|o| match o {
                Object::Connector(c) => Some(c),
                _ => None,
            })
            .expect("the connector exists")
    }

    /// The connected scene with the target end detached to a free cell at (14,7).
    fn free_endpoint_scene() -> (EditorState, Diagram, ObjectId) {
        let (editor, mut diagram) = connected_boxes();
        let c = connector_of(&diagram).id.clone();
        diagram
            .set_endpoint(
                &c,
                ConnectorEnd::Target,
                ConnectorEndpoint::Free(CellPos::new(14, 7)),
            )
            .expect("detach is valid");
        (editor, diagram, c)
    }

    /// Route read-model for the free-endpoint scene: A.e (8,2) → free end (14,7).
    fn free_endpoint_routes(c: &ObjectId) -> Vec<(ObjectId, Vec<CellPos>)> {
        vec![(c.clone(), vec![CellPos::new(8, 2), CellPos::new(14, 7)])]
    }

    #[test]
    fn arrow_tool_draws_a_standalone_free_arrow() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        editor.select_arrow_tool();

        // Press on empty space: the source end stays free at the press cell…
        assert_eq!(
            down(&mut editor, &mut diagram, 3, 5, &[]),
            EditOutcome::Nothing
        );
        assert_eq!(
            move_to(&mut editor, &mut diagram, 14, 7, &[]),
            EditOutcome::Nothing
        );
        // …and the drop cell becomes the free target end.
        let outcome = up(&mut editor, &mut diagram, 14, 7, &[]);
        assert!(matches!(outcome, EditOutcome::ArrowPlaced { .. }));

        let connector = connector_of(&diagram);
        assert_eq!(connector.from, ConnectorEndpoint::Free(CellPos::new(3, 5)));
        assert_eq!(connector.to, ConnectorEndpoint::Free(CellPos::new(14, 7)));
    }

    #[test]
    fn arrow_tool_snaps_its_ends_to_shape_anchors() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        let s1 = ObjectId::generated(1);
        let s2 = ObjectId::generated(2);
        editor.select_arrow_tool();

        // Press exactly on A's e anchor: the source end attaches…
        assert_eq!(
            down(&mut editor, &mut diagram, 8, 2, &[]),
            EditOutcome::Nothing
        );
        move_to(&mut editor, &mut diagram, 14, 2, &[]);
        // …and dropping on B's w anchor attaches the target end.
        let outcome = up(&mut editor, &mut diagram, 20, 2, &[]);
        assert!(matches!(outcome, EditOutcome::ArrowPlaced { .. }));

        let connector = connector_of(&diagram);
        assert_eq!(
            connector.from,
            ConnectorEndpoint::Attached(AnchorRef::new(s1, "e").unwrap())
        );
        assert_eq!(
            connector.to,
            ConnectorEndpoint::Attached(AnchorRef::new(s2, "w").unwrap())
        );
    }

    #[test]
    fn arrow_tool_press_without_movement_draws_nothing() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        editor.select_arrow_tool();

        assert_eq!(
            down(&mut editor, &mut diagram, 5, 5, &[]),
            EditOutcome::Nothing
        );
        assert_eq!(
            up(&mut editor, &mut diagram, 5, 5, &[]),
            EditOutcome::Nothing
        );
        assert_eq!(diagram.len(), 2, "no connector was created");
    }

    #[test]
    fn arrow_tool_refuses_a_zero_length_standalone_arrow() {
        let mut editor = EditorState::default();
        let mut diagram = two_boxes();
        editor.select_arrow_tool();

        // Drag out and back onto the press cell: both ends free at one cell — the
        // model's minimum-1-cell rule refuses it (AC1).
        assert_eq!(
            down(&mut editor, &mut diagram, 5, 5, &[]),
            EditOutcome::Nothing
        );
        assert_eq!(
            move_to(&mut editor, &mut diagram, 6, 5, &[]),
            EditOutcome::Nothing
        );
        assert_eq!(
            move_to(&mut editor, &mut diagram, 5, 5, &[]),
            EditOutcome::Nothing
        );
        assert_eq!(
            up(&mut editor, &mut diagram, 5, 5, &[]),
            EditOutcome::Nothing
        );
        assert_eq!(diagram.len(), 2, "the zero-length arrow was refused");
    }

    #[test]
    fn pressing_an_endpoint_without_moving_opens_the_tip_dropdown() {
        let (mut editor, mut diagram, c) = free_endpoint_scene();
        let routes = free_endpoint_routes(&c);

        // Press on the free endpoint cell: selects + starts the endpoint drag…
        assert_eq!(
            down(&mut editor, &mut diagram, 14, 7, &routes),
            EditOutcome::Selected(c.clone())
        );
        // …and a release without movement changes nothing (CR-020 AC2: the tip is no
        // longer cycled by press-without-move)…
        assert_eq!(
            up(&mut editor, &mut diagram, 14, 7, &routes),
            EditOutcome::TipDropdownOpened {
                id: c.clone(),
                end: ConnectorEnd::Target,
            }
        );
        let connector = connector_of(&diagram);
        assert_eq!(
            connector.to_tip,
            TipStyle::HollowArrow,
            "the tip is untouched"
        );
        // …but it opens the in-canvas dropdown for exactly that end (AC4).
        let (id, end) = editor.tip_dropdown().expect("the dropdown is open");
        assert_eq!(id, &c);
        assert_eq!(end, ConnectorEnd::Target);
    }

    #[test]
    fn applying_a_tip_from_the_dropdown_changes_only_that_end() {
        let (mut editor, mut diagram, c) = free_endpoint_scene();
        let routes = free_endpoint_routes(&c);

        down(&mut editor, &mut diagram, 14, 7, &routes);
        assert_eq!(
            up(&mut editor, &mut diagram, 14, 7, &routes),
            EditOutcome::TipDropdownOpened {
                id: c.clone(),
                end: ConnectorEnd::Target,
            }
        );

        let outcome = editor.apply_tip(
            &c,
            ConnectorEnd::Target,
            TipStyle::FilledRhombus,
            &mut diagram,
        );
        assert_eq!(
            outcome,
            EditOutcome::TipChanged {
                id: c.clone(),
                end: ConnectorEnd::Target,
                tip: TipStyle::FilledRhombus,
            }
        );
        assert!(
            editor.tip_dropdown().is_none(),
            "the menu closes after a pick"
        );
        let connector = connector_of(&diagram);
        assert_eq!(connector.to_tip, TipStyle::FilledRhombus);
        assert_eq!(
            connector.from_tip,
            TipStyle::Bare,
            "the other end keeps its default (bare) tip"
        );
    }

    /// CR-020 AC4 — Escape closes an open tip dropdown without any model change.
    /// Right-click routes through the same `on_escape`, so this covers both exits.
    #[test]
    fn escape_closes_the_tip_dropdown_without_mutation() {
        let (mut editor, mut diagram, c) = free_endpoint_scene();
        let routes = free_endpoint_routes(&c);

        down(&mut editor, &mut diagram, 14, 7, &routes);
        assert_eq!(
            up(&mut editor, &mut diagram, 14, 7, &routes),
            EditOutcome::TipDropdownOpened {
                id: c.clone(),
                end: ConnectorEnd::Target,
            }
        );

        assert_eq!(editor.on_escape(), EditOutcome::Cancelled);
        assert!(editor.tip_dropdown().is_none(), "the menu is closed");
        let connector = connector_of(&diagram);
        assert_eq!(
            connector.to_tip,
            TipStyle::HollowArrow,
            "no tip was changed"
        );
    }

    #[test]
    fn dragging_a_free_endpoint_moves_it_live() {
        let (mut editor, mut diagram, c) = free_endpoint_scene();
        let routes = free_endpoint_routes(&c);

        assert_eq!(
            down(&mut editor, &mut diagram, 14, 7, &routes),
            EditOutcome::Selected(c.clone())
        );
        // Each move re-positions the end (AC6: a free end follows the pointer).
        assert_eq!(
            move_to(&mut editor, &mut diagram, 18, 9, &routes),
            EditOutcome::EndpointMoved {
                id: c.clone(),
                end: ConnectorEnd::Target,
                to: CellPos::new(18, 9),
            }
        );
        // Releasing where it already sits changes nothing.
        assert_eq!(
            up(&mut editor, &mut diagram, 18, 9, &routes),
            EditOutcome::Nothing
        );
        assert_eq!(
            connector_of(&diagram).to,
            ConnectorEndpoint::Free(CellPos::new(18, 9))
        );
    }

    #[test]
    fn dropping_a_free_endpoint_on_an_anchor_attaches_it() {
        let (mut editor, mut diagram, c) = free_endpoint_scene();
        let routes = free_endpoint_routes(&c);
        let s2 = ObjectId::generated(2);

        assert_eq!(
            down(&mut editor, &mut diagram, 14, 7, &routes),
            EditOutcome::Selected(c.clone())
        );
        // Moving over B's w anchor re-positions the end there…
        assert_eq!(
            move_to(&mut editor, &mut diagram, 20, 2, &routes),
            EditOutcome::EndpointMoved {
                id: c.clone(),
                end: ConnectorEnd::Target,
                to: CellPos::new(20, 2),
            }
        );
        // …and the release snaps it onto the anchor (AC6).
        assert_eq!(
            up(&mut editor, &mut diagram, 20, 2, &routes),
            EditOutcome::EndpointAttached {
                id: c.clone(),
                end: ConnectorEnd::Target,
            }
        );
        assert_eq!(
            connector_of(&diagram).to,
            ConnectorEndpoint::Attached(AnchorRef::new(s2, "w").unwrap())
        );
    }

    #[test]
    fn dragging_an_attached_endpoint_off_its_anchor_detaches_it() {
        let (mut editor, mut diagram) = connected_boxes();
        let c = connector_of(&diagram).id.clone();
        let routes = vec![(c.clone(), vec![CellPos::new(8, 2), CellPos::new(20, 2)])];

        // Press on the attached target's anchor cell: endpoint drag starts…
        assert_eq!(
            down(&mut editor, &mut diagram, 20, 2, &routes),
            EditOutcome::Selected(c.clone())
        );
        // …and the first move off the anchor detaches it (AC6).
        assert_eq!(
            move_to(&mut editor, &mut diagram, 14, 7, &routes),
            EditOutcome::EndpointDetached {
                id: c.clone(),
                end: ConnectorEnd::Target,
            }
        );
        // Releasing on empty space leaves it free at the drop cell.
        assert_eq!(
            up(&mut editor, &mut diagram, 14, 7, &routes),
            EditOutcome::Nothing
        );
        assert_eq!(
            connector_of(&diagram).to,
            ConnectorEndpoint::Free(CellPos::new(14, 7))
        );
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
            .connect(ConnectorInstance::between(
                diagram.new_object_id(),
                AnchorRef::new(s1.clone(), "e").unwrap(),
                AnchorRef::new(s2.clone(), "w").unwrap(),
                ConnectorStyle::Solid,
            ))
            .expect("valid connector");
        diagram
            .connect(ConnectorInstance::between(
                diagram.new_object_id(),
                AnchorRef::new(s1.clone(), "n").unwrap(),
                AnchorRef::new(s2.clone(), "s").unwrap(),
                ConnectorStyle::Solid,
            ))
            .expect("valid connector");
        assert_eq!(diagram.len(), 4);

        assert_eq!(
            down(&mut editor, &mut diagram, 1, 1, &[]),
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
        assert!(editor.selection.is_empty());
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
            down(&mut editor, &mut diagram, 1, 1, &[]),
            EditOutcome::Selected(s1.clone())
        );
        down(&mut editor, &mut diagram, 8, 2, &[]);
        assert_eq!(editor.on_escape(), EditOutcome::Cancelled);

        // Cancels placement mode.
        editor.select_template(info("box", "Box", 9, 5));
        assert_eq!(editor.on_escape(), EditOutcome::Cancelled);
        assert!(editor.placing.is_none());

        // Deselects: the press also starts a move gesture, so the first escape drops
        // the gesture and the second clears the selection.
        down(&mut editor, &mut diagram, 1, 1, &[]);
        assert_eq!(editor.on_escape(), EditOutcome::Cancelled);
        assert_eq!(editor.on_escape(), EditOutcome::Deselected);
        // And is a no-op when there is nothing to cancel.
        assert_eq!(editor.on_escape(), EditOutcome::Nothing);
    }

    // ── S-018: free text ───────────────────────────────────────────────────────

    /// A diagram with one text object at (2, 3) — "hi" is 2×1.
    fn diagram_with_text() -> Diagram {
        let mut diagram = Diagram::new();
        let id = diagram.new_object_id();
        diagram
            .place_text(TextInstance::new(id.clone(), CellPos::new(2, 3), "hi").expect("valid"))
            .expect("placement is valid");
        diagram
    }

    #[test]
    fn the_text_tool_places_a_text_object_and_opens_editing() {
        let mut editor = EditorState::default();
        let mut diagram = Diagram::new();

        editor.select_text_tool();
        assert!(matches!(editor.placing, Some(Placing::Text)));

        // One click places and opens inline editing immediately (AC1) — the tool is
        // consumed, so a second click would act on the canvas normally.
        let outcome = editor.on_pointer(
            PointerEvent::Down {
                cell: (4, 2),
                target: HitTarget::Empty,
                additive: false,
            },
            &mut diagram,
        );
        assert_eq!(
            outcome,
            EditOutcome::PlacedText {
                id: ObjectId::generated(1)
            }
        );
        assert!(editor.placing.is_none());
        assert_eq!(editor.editing, Some(ObjectId::generated(1)));
        assert_eq!(editor.edit_draft, PLACEHOLDER_TEXT);

        let text = diagram
            .objects()
            .first()
            .and_then(|o| match o {
                Object::Text(t) => Some(t),
                _ => None,
            })
            .expect("the text object exists");
        assert_eq!(text.pos, CellPos::new(4, 2));
    }

    #[test]
    fn committing_an_inline_edit_writes_the_content_to_the_model() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_text();
        let id = ObjectId::generated(1);

        // Double click inside the text (its 2×1 extent at (2,3)) opens editing.
        assert_eq!(
            editor.on_pointer(
                PointerEvent::DoubleClick {
                    cell: (3, 3),
                    target: HitTarget::Body(id.clone()),
                },
                &mut diagram,
            ),
            EditOutcome::Editing { id: id.clone() }
        );
        assert_eq!(editor.edit_draft, "hi");

        // The user types (the egui TextEdit binds to the draft directly) and commits.
        editor.edit_draft = "hello\nworld".to_owned();
        assert_eq!(
            editor.commit_edit(&mut diagram),
            EditOutcome::Edited { id: id.clone() }
        );
        assert!(editor.editing.is_none());

        let text = diagram
            .get(&id)
            .and_then(|o| match o {
                Object::Text(t) => Some(t),
                _ => None,
            })
            .expect("still a text object");
        assert_eq!(text.text, "hello\nworld");
    }

    #[test]
    fn committing_an_empty_inline_edit_cancels_without_mutation() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_text();
        let id = ObjectId::generated(1);

        assert_eq!(
            editor.on_pointer(
                PointerEvent::DoubleClick {
                    cell: (2, 3),
                    target: HitTarget::Body(id.clone()),
                },
                &mut diagram,
            ),
            EditOutcome::Editing { id: id.clone() }
        );
        editor.edit_draft.clear(); // the user deleted everything
        assert_eq!(editor.commit_edit(&mut diagram), EditOutcome::Cancelled);
        assert!(editor.editing.is_none());
        let text = match diagram.get(&id) {
            Some(Object::Text(t)) => t,
            other => panic!("expected the original text object, got {other:?}"),
        };
        assert_eq!(text.text, "hi"); // untouched
    }

    #[test]
    fn escape_cancels_an_inline_edit_without_mutation() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_text();
        let id = ObjectId::generated(1);

        assert_eq!(
            editor.on_pointer(
                PointerEvent::DoubleClick {
                    cell: (2, 3),
                    target: HitTarget::Body(id.clone()),
                },
                &mut diagram,
            ),
            EditOutcome::Editing { id: id.clone() }
        );
        editor.edit_draft = "draft".to_owned();
        assert_eq!(editor.on_escape(), EditOutcome::Cancelled);
        assert!(editor.editing.is_none());
        assert!(editor.edit_draft.is_empty());
        let text = match diagram.get(&id) {
            Some(Object::Text(t)) => t,
            other => panic!("expected the original text object, got {other:?}"),
        };
        assert_eq!(text.text, "hi"); // untouched
    }

    #[test]
    fn a_pointer_press_while_editing_commits_and_consumes_the_click() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_text();
        let id = ObjectId::generated(1);

        assert_eq!(
            editor.on_pointer(
                PointerEvent::DoubleClick {
                    cell: (2, 3),
                    target: HitTarget::Body(id.clone()),
                },
                &mut diagram,
            ),
            EditOutcome::Editing { id: id.clone() }
        );
        editor.edit_draft = "done".to_owned();

        // Pressing the canvas commits (blur semantics) and consumes the click — no move
        // gesture starts, so a following drag does not move the object.
        assert_eq!(
            editor.on_pointer(
                PointerEvent::Down {
                    cell: (2, 3),
                    target: HitTarget::Body(id.clone()),
                    additive: false,
                },
                &mut diagram,
            ),
            EditOutcome::Edited { id: id.clone() }
        );
        assert_eq!(
            editor.selection,
            std::iter::once(id.clone()).collect::<std::collections::BTreeSet<_>>(),
        ); // stays selected
        assert_eq!(
            move_to(&mut editor, &mut diagram, 9, 9, &[]),
            EditOutcome::Nothing
        );
        let text = match diagram.get(&id) {
            Some(Object::Text(t)) => t,
            other => panic!("expected the updated text object, got {other:?}"),
        };
        assert_eq!(text.text, "done");
    }

    /// FR19 (CR-016) — a double click on a shape body reports `DoubleClickShape` so
    /// the app layer can offer the region editor for region-bearing templates;
    /// unlike text objects it never opens inline text editing.
    #[test]
    fn double_click_on_a_shape_reports_the_body_not_inline_editing() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_box(); // box at (0,0), 9×5
        assert_eq!(
            editor.on_pointer(
                PointerEvent::DoubleClick {
                    cell: (4, 2),
                    target: HitTarget::Body(ObjectId::generated(1)),
                },
                &mut diagram,
            ),
            EditOutcome::DoubleClickShape {
                id: ObjectId::generated(1)
            }
        );
        assert!(editor.editing.is_none());
    }

    #[test]
    fn text_objects_are_movable_with_the_same_gesture_as_shapes() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_text(); // "hi" at (2,3)
        let id = ObjectId::generated(1);

        // Press inside the text: select + grab (offset (1,0)).
        assert_eq!(
            down(&mut editor, &mut diagram, 3, 3, &[]),
            EditOutcome::Selected(id.clone())
        );
        // Drag two cells right: grid-aligned move (AC2).
        assert_eq!(
            move_to(&mut editor, &mut diagram, 5, 3, &[]),
            EditOutcome::Moved {
                id,
                to: CellPos::new(4, 3)
            }
        );
    }

    #[test]
    fn deleting_a_text_object_removes_it() {
        let mut editor = EditorState::default();
        let mut diagram = diagram_with_text();
        let id = ObjectId::generated(1);

        assert_eq!(
            down(&mut editor, &mut diagram, 2, 3, &[]),
            EditOutcome::Selected(id.clone())
        );
        assert_eq!(
            editor.delete_selection(&mut diagram),
            EditOutcome::Deleted {
                id: id.clone(),
                removed: 1
            }
        );
        assert_eq!(diagram.len(), 0);
    }

    // ---- CR-007: bend gestures -----------------------------------------------

    /// `two_boxes` plus one solid connector s1.e (8,2) → s2.w (20,2); with an optional
    /// seed bend. Returns the diagram, the connector id and its route polyline.
    fn bent_scene(bend: Option<CellPos>) -> (Diagram, ObjectId, Vec<CellPos>) {
        let mut diagram = two_boxes();
        let c = diagram.new_object_id(); // obj-3
        diagram
            .connect(ConnectorInstance::between(
                c.clone(),
                AnchorRef::new(ObjectId::generated(1), "e").expect("valid anchor"),
                AnchorRef::new(ObjectId::generated(2), "w").expect("valid anchor"),
                ConnectorStyle::Solid,
            ))
            .expect("connects");
        if let Some(b) = bend {
            diagram.add_bend(&c, 0, b).expect("inserts the bend");
        }
        let route = match bend {
            Some(b) => vec![CellPos::new(8, 2), b, CellPos::new(20, 2)],
            None => vec![CellPos::new(8, 2), CellPos::new(20, 2)],
        };
        (diagram, c, route)
    }

    fn bends_of(diagram: &Diagram, id: &ObjectId) -> Vec<CellPos> {
        match diagram.get(id).expect("the connector exists") {
            Object::Connector(c) => c.bends.clone(),
            _ => panic!("expected a connector"),
        }
    }

    #[test]
    fn pressing_a_bend_selects_the_connector_and_moves_it_live() {
        let (mut diagram, c, route) = bent_scene(Some(CellPos::new(14, 6)));
        let mut editor = EditorState::default();
        let routes = [(c.clone(), route)];

        // Press the bend cell: selects the connector and starts a live move.
        assert_eq!(
            down(&mut editor, &mut diagram, 14, 6, &routes),
            EditOutcome::Selected(c.clone())
        );

        // Live moves land where the pointer is (AC5 free drag on a cell grid).
        assert_eq!(
            move_to(&mut editor, &mut diagram, 15, 5, &routes),
            EditOutcome::BendMoved {
                id: c.clone(),
                to: CellPos::new(15, 5)
            }
        );
        assert_eq!(bends_of(&diagram, &c), vec![CellPos::new(15, 5)]);

        // Drop off the straight line between the neighbors: the bend stays.
        assert_eq!(
            up(&mut editor, &mut diagram, 16, 7, &routes),
            EditOutcome::Nothing
        );
        assert_eq!(bends_of(&diagram, &c), vec![CellPos::new(15, 5)]);
    }

    #[test]
    fn dropping_a_moved_bend_on_the_neighbor_line_removes_it() {
        let (mut diagram, c, route) = bent_scene(Some(CellPos::new(14, 6)));
        let mut editor = EditorState::default();
        let routes = [(c.clone(), route)];

        down(&mut editor, &mut diagram, 14, 6, &routes); // Selected + BendMove
        // Drag it back onto the straight line between (8,2) and (20,2).
        assert_eq!(
            move_to(&mut editor, &mut diagram, 14, 2, &routes),
            EditOutcome::BendMoved {
                id: c.clone(),
                to: CellPos::new(14, 2)
            }
        );
        // Drop: collinear with both neighbors → removed (AC6).
        assert_eq!(
            up(&mut editor, &mut diagram, 14, 2, &routes),
            EditOutcome::BendRemoved {
                id: c.clone(),
                index: 0
            }
        );
        assert!(bends_of(&diagram, &c).is_empty());
    }

    #[test]
    fn pressing_a_bend_without_moving_changes_nothing() {
        // Seed bend on the straight line between its neighbors: a press+release on the
        // same cell must not remove it (AC6 — only *moved* bends are cleaned up).
        let (mut diagram, c, route) = bent_scene(Some(CellPos::new(14, 2)));
        let mut editor = EditorState::default();
        let routes = [(c.clone(), route)];

        assert_eq!(
            down(&mut editor, &mut diagram, 14, 2, &routes),
            EditOutcome::Selected(c.clone())
        );
        assert_eq!(
            up(&mut editor, &mut diagram, 14, 2, &routes),
            EditOutcome::Nothing
        );
        assert_eq!(bends_of(&diagram, &c), vec![CellPos::new(14, 2)]);
    }

    #[test]
    fn pressing_a_connector_body_and_dragging_moves_the_whole_arrow() {
        let (mut diagram, c, route) = bent_scene(Some(CellPos::new(14, 6)));
        let mut editor = EditorState::default();
        let routes = [(c.clone(), route)];

        // Press a body cell on segment 0: selects + starts the whole-arrow move.
        assert_eq!(
            down(&mut editor, &mut diagram, 9, 3, &routes),
            EditOutcome::Selected(c.clone())
        );
        // Still on the pressed cell: nothing moved yet (CR-020 AC3 — a press without
        // movement only selects).
        assert_eq!(
            move_to(&mut editor, &mut diagram, 9, 3, &routes),
            EditOutcome::Nothing
        );
        let connector = connector_of(&diagram);
        assert!(connector.from.is_attached() && connector.to.is_attached());

        // First displacement translates EVERYTHING by the delta (+3, +1) (AC1): both
        // ends detach in place and follow; the bend rides along.
        assert_eq!(
            move_to(&mut editor, &mut diagram, 12, 4, &routes),
            EditOutcome::EndpointDetached {
                id: c.clone(),
                end: ConnectorEnd::Source,
            }
        );
        let connector = connector_of(&diagram);
        assert_eq!(
            connector.from,
            ConnectorEndpoint::Free(CellPos::new(11, 3)) // (8,2)+(3,1)
        );
        assert_eq!(
            connector.to,
            ConnectorEndpoint::Free(CellPos::new(23, 3)) // (20,2)+(3,1)
        );
        assert_eq!(connector.bends, vec![CellPos::new(17, 7)]); // (14,6)+(3,1)

        // Later moves keep translating from the press-time snapshot (+4, +2).
        assert_eq!(
            move_to(&mut editor, &mut diagram, 13, 5, &routes),
            EditOutcome::Nothing
        );
        let connector = connector_of(&diagram);
        assert_eq!(
            connector.from,
            ConnectorEndpoint::Free(CellPos::new(12, 4)) // (8,2)+(4,2)
        );
        assert_eq!(
            connector.to,
            ConnectorEndpoint::Free(CellPos::new(24, 4)) // (20,2)+(4,2)
        );
        assert_eq!(connector.bends, vec![CellPos::new(18, 8)]); // (14,6)+(4,2)

        // Drop off every anchor: the ends stay free and the gesture is over (AC1).
        assert_eq!(
            up(&mut editor, &mut diagram, 13, 5, &routes),
            EditOutcome::ConnectorMoved { id: c.clone() }
        );
    }

    #[test]
    fn double_clicking_a_connector_body_inserts_a_bend_there() {
        let (mut diagram, c, route) = bent_scene(None); // straight (8,2)-(20,2)
        let mut editor = EditorState::default();
        let routes = [(c.clone(), route)];

        // Double-click a body cell on the segment: inserts the waypoint there (AC3).
        assert_eq!(
            double_click(&mut editor, &mut diagram, 11, 2, &routes),
            EditOutcome::BendAdded {
                id: c.clone(),
                at: CellPos::new(11, 2),
            }
        );
        assert_eq!(bends_of(&diagram, &c), vec![CellPos::new(11, 2)]);
        // The double-click's press selected the connector.
        assert!(editor.selection.contains(&c));
    }

    /// CR-020 AC3 — double-clicking a bend, an endpoint, or empty space inserts
    /// nothing (only body segments take waypoints).
    #[test]
    fn double_clicking_a_bend_endpoint_or_empty_space_inserts_nothing() {
        let (mut diagram, c, route) = bent_scene(Some(CellPos::new(14, 6)));
        let mut editor = EditorState::default();
        let routes = [(c.clone(), route)];

        // On the existing bend's cell: no second bend appears.
        assert_eq!(
            double_click(&mut editor, &mut diagram, 14, 6, &routes),
            EditOutcome::Nothing
        );
        // On an endpoint cell (the route ends at (8,2) and (20,2)).
        assert_eq!(
            double_click(&mut editor, &mut diagram, 8, 2, &routes),
            EditOutcome::Nothing
        );
        // On empty space.
        assert_eq!(
            double_click(&mut editor, &mut diagram, 3, 9, &routes),
            EditOutcome::Nothing
        );

        let bends = bends_of(&diagram, &c);
        assert_eq!(bends, vec![CellPos::new(14, 6)], "no bend was inserted");
    }

    #[test]
    fn bend_dragging_is_free_with_exact_axis_alignment() {
        let (mut diagram, c, route) = bent_scene(Some(CellPos::new(14, 6)));
        let mut editor = EditorState::default();
        let routes = [(c.clone(), route)];

        down(&mut editor, &mut diagram, 14, 6, &routes);
        // Pointer on the neighbors' shared row (y=2): both segments axis-aligned.
        assert_eq!(
            move_to(&mut editor, &mut diagram, 9, 2, &routes),
            EditOutcome::BendMoved {
                id: c.clone(),
                to: CellPos::new(9, 2)
            }
        );
        // Pointer off both axes of both neighbors: diagonals on both sides.
        assert_eq!(
            move_to(&mut editor, &mut diagram, 15, 5, &routes),
            EditOutcome::BendMoved {
                id: c.clone(),
                to: CellPos::new(15, 5)
            }
        );
    }
}
