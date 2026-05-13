//! Port of `xyflow-core/src/xydrag/XYDrag.ts` — node and selection
//! drag manager.
//!
//! Wraps [`rgraph_drag::DragBehavior`] and adds:
//!   - selection vs single-node drag,
//!   - parent-extent + grid snapping,
//!   - auto-pan on edge (driven by the consumer via
//!     [`XYDrag::auto_pan_tick`]),
//!   - node-drag-threshold gating,
//!   - subject offset (the user's "grab point" stays under the cursor).
//!
//! Status: implemented (phase 5).
//!
//! ## Differences vs TS
//!
//! * TS attaches d3-drag to a DOM node and reads
//!   `getBoundingClientRect()` on `start`. The Rust port takes the
//!   container bounds as a `Rect` set via [`XYDrag::set_container_bounds`].
//! * TS runs `requestAnimationFrame(autoPan)` inside the manager. We
//!   leave the rAF loop to the consumer: at each animation tick the
//!   consumer calls [`XYDrag::auto_pan_tick`].
//! * Store access uses a `Fn() -> StoreSnapshot` closure instead of a
//!   reactive store reference.

#![allow(clippy::module_name_repetitions)]

pub mod utils;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rgraph_drag::{DragBehavior, DragEvent, PointerId, PointerInput};

use crate::promise::Promise;
use crate::types::geometry::{CoordinateExtent, Rect, Transform, XYPosition};
use crate::types::nodes::{InternalNode, Node, NodeDragItem, NodeLookup, NodeOrigin, PointerEventLike};
use crate::types::viewport::SnapGrid;
use crate::utils::dom::{get_event_position, get_pointer_position, ContainerBounds, GetPointerPositionParams};
use crate::utils::general::{calc_auto_pan, snap_position};
use crate::utils::graph::{calculate_node_position, CalculateNodePositionParams, GraphOnErrorFn};
use crate::xydrag::utils::{
    calculate_snap_offset, get_drag_items, get_event_handler_params, has_selector,
    DragEventHandlerParams,
};

// ---------------------------------------------------------------------------
// Public type aliases
// ---------------------------------------------------------------------------

/// Closure that pans the viewport by the given delta. Returns a
/// [`Promise<bool>`] that resolves with `true` if the transform
/// actually changed. The store snapshot's `pan_by` field uses this.
pub type PanByFn = Rc<dyn Fn(XYPosition) -> Promise<bool>>;

/// Closure called when the user drags one or more nodes. Mirrors the
/// TS `OnDrag` signature.
pub type OnDragFn<D> = Rc<
    dyn Fn(&PointerEventLike, &HashMap<String, NodeDragItem>, &Option<Node<D>>, &[Node<D>]),
>;

/// Closure called when the user-selection variant of a drag event
/// fires. Mirrors the TS `OnSelectionDrag = (event, nodes) => void`.
pub type OnSelectionDragFn<D> = Rc<dyn Fn(&PointerEventLike, &[Node<D>])>;

/// Closure that updates node positions in the consumer's store. The
/// `dragging` flag is `true` during the drag and `false` on end.
pub type UpdateNodePositionsFn = Rc<dyn Fn(&HashMap<String, NodeDragItem>, bool)>;

/// Closure that clears node and edge selection in the consumer's
/// store.
pub type UnselectAllFn = Rc<dyn Fn()>;

/// Closure called when the gesture starts on a single node — used by
/// the consumer to mark the node selected before the drag delta
/// accumulates (mirrors `onNodeMouseDown` in TS).
pub type OnNodeMouseDownFn = Rc<dyn Fn(&str)>;

// ---------------------------------------------------------------------------
// Snapshot of the React-Flow / Svelte-Flow store
// ---------------------------------------------------------------------------

/// Snapshot of the consumer's store needed by `XYDrag`. Mirrors the
/// TS `StoreItems<OnNodeDrag>` shape, but heavily trimmed: callbacks
/// are stored on the [`XYDrag`] struct directly via the `XYDragParams`,
/// not duplicated into the snapshot every tick.
pub struct StoreSnapshot<D: Clone> {
    pub node_lookup: Rc<NodeLookup<D>>,
    pub node_extent: CoordinateExtent,
    pub snap_grid: SnapGrid,
    pub snap_to_grid: bool,
    pub node_origin: NodeOrigin,
    pub multi_selection_active: bool,
    pub transform: Transform,
    pub auto_pan_on_node_drag: bool,
    pub nodes_draggable: bool,
    pub select_nodes_on_drag: bool,
    pub node_drag_threshold: f64,
    pub auto_pan_speed: f64,
    /// Pan-by closure — typically a thin wrapper around
    /// `XYPanZoom::set_viewport_constrained`.
    pub pan_by: PanByFn,
    pub update_node_positions: UpdateNodePositionsFn,
    pub unselect_nodes_and_edges: UnselectAllFn,
    pub on_error: Option<GraphOnErrorFn>,
}

/// Fetcher closure that produces a fresh [`StoreSnapshot`] on demand.
pub type GetStoreItemsFn<D> = Rc<dyn Fn() -> StoreSnapshot<D>>;

// ---------------------------------------------------------------------------
// XYDrag construction parameters
// ---------------------------------------------------------------------------

/// Construction parameters for [`XYDrag::new`].
pub struct XYDragParams<D: Clone> {
    pub get_store_items: GetStoreItemsFn<D>,
    pub on_drag_start: Option<OnDragFn<D>>,
    pub on_drag: Option<OnDragFn<D>>,
    pub on_drag_stop: Option<OnDragFn<D>>,
    pub on_node_drag_start: Option<OnDragFn<D>>,
    pub on_node_drag: Option<OnDragFn<D>>,
    pub on_node_drag_stop: Option<OnDragFn<D>>,
    pub on_selection_drag_start: Option<OnSelectionDragFn<D>>,
    pub on_selection_drag: Option<OnSelectionDragFn<D>>,
    pub on_selection_drag_stop: Option<OnSelectionDragFn<D>>,
    pub on_node_mouse_down: Option<OnNodeMouseDownFn>,
}

/// Per-call options applied to the engine via [`XYDrag::update`].
#[derive(Debug, Clone)]
pub struct DragUpdateParams {
    /// CSS class that suppresses dragging on matching ancestors.
    pub no_drag_class_name: Option<String>,
    /// CSS selector for elements that act as drag handles. Drag
    /// is only accepted when the event target has this selector
    /// among its ancestors.
    pub handle_selector: Option<String>,
    pub is_selectable: bool,
    /// Node id for single-node drags. `None` means selection drag.
    pub node_id: Option<String>,
    pub node_click_distance: f64,
}

impl Default for DragUpdateParams {
    fn default() -> Self {
        DragUpdateParams {
            no_drag_class_name: None,
            handle_selector: None,
            is_selectable: false,
            node_id: None,
            node_click_distance: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// XYDrag
// ---------------------------------------------------------------------------

/// Pure-state drag manager wrapping [`rgraph_drag::DragBehavior`].
///
/// Generic over the user-data type `D`. Cloning is cheap (`Rc::clone`).
pub struct XYDrag<D: Clone + 'static> {
    inner: Rc<XYDragInner<D>>,
}

impl<D: Clone + 'static> Clone for XYDrag<D> {
    fn clone(&self) -> Self {
        XYDrag {
            inner: Rc::clone(&self.inner),
        }
    }
}

struct XYDragInner<D: Clone + 'static> {
    drag: DragBehavior<()>,
    params: XYDragParams<D>,
    /// Currently configured update params (last `update` call).
    update_params: RefCell<DragUpdateParams>,
    /// Container bounds in viewport coords.
    container_bounds: RefCell<ContainerBounds>,
    /// Per-gesture state.
    gesture: RefCell<GestureState>,
    /// Per-gesture drag items.
    drag_items: RefCell<HashMap<String, NodeDragItem>>,
    destroyed: RefCell<bool>,
}

#[derive(Debug, Clone, Default)]
struct GestureState {
    /// Last known pointer position in flow coordinates (snapped if
    /// snapping is on).
    last_pos: Option<XYPosition>,
    /// Mouse position in container-relative pixels (used by auto-pan).
    mouse_position: XYPosition,
    /// True after the threshold has been crossed and `start` callbacks
    /// were fired.
    drag_started: bool,
    /// True iff the gesture must be aborted (multitouch / node deleted).
    abort_drag: bool,
    /// True iff at least one position changed during the gesture.
    node_positions_changed: bool,
    /// True while the auto-pan loop is active.
    auto_pan_started: bool,
}

impl<D: Clone + 'static> XYDrag<D> {
    /// Construct a fresh `XYDrag` instance.
    pub fn new(params: XYDragParams<D>) -> Self {
        let drag = DragBehavior::<()>::new();
        // d3-drag's defaultSubject requires `Default` for the datum;
        // we work around it by setting a custom subject closure that
        // anchors at the pointer position.
        drag.subject(|ctx| {
            Some(rgraph_drag::Subject {
                x: ctx.x,
                y: ctx.y,
                datum: (),
            })
        });

        XYDrag {
            inner: Rc::new(XYDragInner {
                drag,
                params,
                update_params: RefCell::new(DragUpdateParams::default()),
                container_bounds: RefCell::new(None),
                gesture: RefCell::new(GestureState::default()),
                drag_items: RefCell::new(HashMap::new()),
                destroyed: RefCell::new(false),
            }),
        }
    }

    /// Replace the cached container bounds (from the consumer's
    /// `MountedData::get_client_rect()` pre-measurement).
    pub fn set_container_bounds(&self, bounds: Option<Rect>) {
        *self.inner.container_bounds.borrow_mut() = bounds;
    }

    /// Mirrors the TS `update()` call. Re-applies the click-distance
    /// threshold and the filter (which honours `no_drag_class_name` /
    /// `handle_selector`).
    pub fn update(&self, params: DragUpdateParams) {
        if *self.inner.destroyed.borrow() {
            return;
        }
        self.inner.drag.click_distance(params.node_click_distance);
        *self.inner.update_params.borrow_mut() = params;
    }

    /// Detach the manager. Subsequent calls to `handle_pointer` and
    /// `update` are no-ops.
    pub fn destroy(&self) {
        *self.inner.destroyed.borrow_mut() = true;
        self.inner.drag.on("start", None);
        self.inner.drag.on("drag", None);
        self.inner.drag.on("end", None);
    }

    /// Consumer-facing pointer-event filter. Mirrors the TS d3-drag
    /// `filter` closure: rejects right-clicks, ancestors marked with
    /// `no_drag_class_name`, and (when set) anything that doesn't
    /// match `handle_selector`.
    ///
    /// `ancestor_class_lists` is a list (target → root) of class-name
    /// arrays; the Dioxus consumer collects this from its component
    /// tree.
    #[must_use]
    pub fn accept_event(&self, button: u8, ancestor_class_lists: &[Vec<String>]) -> bool {
        if button != 0 {
            return false;
        }
        let params = self.inner.update_params.borrow();
        if let Some(no_drag) = params.no_drag_class_name.as_deref() {
            if has_selector(ancestor_class_lists, no_drag) {
                return false;
            }
        }
        if let Some(handle) = params.handle_selector.as_deref() {
            if !has_selector(ancestor_class_lists, handle) {
                return false;
            }
        }
        true
    }

    // -----------------------------------------------------------------
    // Pointer-input forwarders
    // -----------------------------------------------------------------

    /// Forward a `Down` pointer event. Mirrors the TS d3-drag `start`
    /// handler: caches container bounds (already done via
    /// [`Self::set_container_bounds`]), records mouse position, and —
    /// if `node_drag_threshold == 0` — fires `start` callbacks
    /// immediately.
    pub fn handle_pointer_down(
        &self,
        id: PointerId,
        event: &PointerEventLike,
        button: u8,
        ctrl: bool,
    ) -> bool {
        if *self.inner.destroyed.borrow() {
            return false;
        }
        let consumed = self.inner.drag.handle(PointerInput::Down {
            id,
            x: event.client_x,
            y: event.client_y,
            button,
            ctrl,
            datum: None,
        });
        if !consumed {
            return false;
        }
        self.on_drag_start_internal(event);
        true
    }

    /// Forward a `Move` pointer event.
    pub fn handle_pointer_move(&self, id: PointerId, event: &PointerEventLike) -> bool {
        if *self.inner.destroyed.borrow() {
            return false;
        }
        let consumed = self.inner.drag.handle(PointerInput::Move {
            id,
            x: event.client_x,
            y: event.client_y,
        });
        if consumed {
            self.on_drag_internal(event);
        }
        consumed
    }

    /// Forward an `Up` pointer event.
    pub fn handle_pointer_up(&self, id: PointerId, event: &PointerEventLike) -> bool {
        if *self.inner.destroyed.borrow() {
            return false;
        }
        let consumed = self.inner.drag.handle(PointerInput::Up {
            id,
            x: event.client_x,
            y: event.client_y,
        });
        if consumed {
            self.on_drag_end_internal(event);
        }
        consumed
    }

    /// Forward a `Cancel` pointer event.
    pub fn handle_pointer_cancel(&self, id: PointerId, event: &PointerEventLike) -> bool {
        if *self.inner.destroyed.borrow() {
            return false;
        }
        let consumed = self.inner.drag.handle(PointerInput::Cancel { id });
        if consumed {
            self.on_drag_end_internal(event);
        }
        consumed
    }

    /// Auto-pan tick. The consumer should call this from each animation
    /// frame while a drag is in progress. Returns the resolved
    /// pan-by promise (already-resolved-false when no movement).
    pub fn auto_pan_tick(&self) -> Promise<bool> {
        let bounds = match *self.inner.container_bounds.borrow() {
            Some(b) => b,
            None => return Promise::resolved(false),
        };
        let store = (self.inner.params.get_store_items)();
        if !store.auto_pan_on_node_drag {
            self.inner.gesture.borrow_mut().auto_pan_started = false;
            return Promise::resolved(false);
        }
        let mouse = self.inner.gesture.borrow().mouse_position;
        let (x_movement, y_movement) = calc_auto_pan(
            mouse,
            crate::types::geometry::Dimensions {
                width: bounds.width,
                height: bounds.height,
            },
            store.auto_pan_speed,
            40.0,
        );
        if x_movement == 0.0 && y_movement == 0.0 {
            return Promise::resolved(false);
        }
        // Update last_pos so the next move event computes the right delta.
        if let Some(last) = self.inner.gesture.borrow_mut().last_pos.as_mut() {
            last.x -= x_movement / store.transform.scale().max(f64::MIN_POSITIVE);
            last.y -= y_movement / store.transform.scale().max(f64::MIN_POSITIVE);
        }
        let p = (store.pan_by)(XYPosition::new(x_movement, y_movement));
        // After pan, re-run updates with the (potentially mutated) last_pos.
        if let Some(last) = self.inner.gesture.borrow().last_pos {
            self.update_nodes(last);
        }
        p
    }

    /// Returns whether a drag gesture has been started (i.e. the
    /// threshold was crossed and `start` callbacks were fired).
    #[must_use]
    pub fn is_drag_started(&self) -> bool {
        self.inner.gesture.borrow().drag_started
    }

    /// Returns the current per-gesture drag items snapshot.
    #[must_use]
    pub fn drag_items(&self) -> HashMap<String, NodeDragItem> {
        self.inner.drag_items.borrow().clone()
    }

    // -----------------------------------------------------------------
    // Internal handlers
    // -----------------------------------------------------------------

    fn on_drag_start_internal(&self, event: &PointerEventLike) {
        let store = (self.inner.params.get_store_items)();
        let bounds = *self.inner.container_bounds.borrow();
        let pointer_pos = get_pointer_position(
            event,
            GetPointerPositionParams {
                transform: store.transform,
                snap_grid: store.snap_grid,
                snap_to_grid: store.snap_to_grid,
                container_bounds: bounds,
            },
        );

        let mut g = self.inner.gesture.borrow_mut();
        g.drag_started = false;
        g.abort_drag = false;
        g.node_positions_changed = false;
        g.last_pos = Some(XYPosition::new(pointer_pos.x_snapped, pointer_pos.y_snapped));
        g.mouse_position = {
            let evp = get_event_position(event, bounds);
            XYPosition::new(evp.x, evp.y)
        };
        drop(g);

        // If the threshold is 0 we begin the drag immediately;
        // otherwise we wait for the first move past the threshold.
        if store.node_drag_threshold == 0.0 {
            self.start_drag(event, &store);
        }
    }

    fn start_drag(&self, event: &PointerEventLike, store: &StoreSnapshot<D>) {
        let update_params = self.inner.update_params.borrow().clone();
        let node_id = update_params.node_id.clone();
        let pointer_pos = match self.inner.gesture.borrow().last_pos {
            Some(p) => p,
            None => return,
        };

        // Selection-on-drag bookkeeping.
        if (!store.select_nodes_on_drag || !update_params.is_selectable)
            && !store.multi_selection_active
            && node_id.is_some()
        {
            let nid = node_id.as_deref().unwrap();
            let already_selected = store
                .node_lookup
                .get(nid)
                .map(|n| n.user.selected.unwrap_or(false))
                .unwrap_or(false);
            if !already_selected {
                (store.unselect_nodes_and_edges)();
            }
        }

        if update_params.is_selectable && store.select_nodes_on_drag {
            if let (Some(cb), Some(nid)) = (
                self.inner.params.on_node_mouse_down.as_ref(),
                node_id.as_deref(),
            ) {
                cb(nid);
            }
        }

        let drag_items = get_drag_items(
            &store.node_lookup,
            store.nodes_draggable,
            pointer_pos,
            node_id.as_deref(),
        );
        *self.inner.drag_items.borrow_mut() = drag_items.clone();
        self.inner.gesture.borrow_mut().drag_started = true;

        if drag_items.is_empty() {
            return;
        }
        let params = get_event_handler_params(node_id.as_deref(), &drag_items, &store.node_lookup, true);
        self.fire_drag_callback(
            CallbackKind::Start,
            event,
            &drag_items,
            &params,
            node_id.as_deref(),
        );
    }

    fn on_drag_internal(&self, event: &PointerEventLike) {
        let store = (self.inner.params.get_store_items)();
        let bounds = *self.inner.container_bounds.borrow();
        let pointer_pos = get_pointer_position(
            event,
            GetPointerPositionParams {
                transform: store.transform,
                snap_grid: store.snap_grid,
                snap_to_grid: store.snap_to_grid,
                container_bounds: bounds,
            },
        );
        let update_params = self.inner.update_params.borrow().clone();
        let node_id = update_params.node_id.clone();

        // Multi-touch / node deletion → abort.
        let mut should_abort = false;
        if let Some(nid) = node_id.as_deref() {
            if !store.node_lookup.contains_key(nid) {
                should_abort = true;
            }
        }
        if should_abort {
            self.inner.gesture.borrow_mut().abort_drag = true;
        }
        if self.inner.gesture.borrow().abort_drag {
            return;
        }

        // Auto-pan kick-off.
        let drag_started_now = self.inner.gesture.borrow().drag_started;
        if !self.inner.gesture.borrow().auto_pan_started && store.auto_pan_on_node_drag && drag_started_now {
            self.inner.gesture.borrow_mut().auto_pan_started = true;
            // The actual rAF loop is consumer-driven; the consumer
            // detects `is_drag_started()` and starts ticking
            // `auto_pan_tick()`.
        }

        // Threshold gating.
        if !drag_started_now {
            let current_mouse_event_pos = get_event_position(event, bounds);
            let current_mouse = XYPosition::new(current_mouse_event_pos.x, current_mouse_event_pos.y);
            let prev_mouse = self.inner.gesture.borrow().mouse_position;
            let dx = current_mouse.x - prev_mouse.x;
            let dy = current_mouse.y - prev_mouse.y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > store.node_drag_threshold {
                self.start_drag(event, &store);
            }
        }

        // Skip events without snapped-position movement.
        let last_pos = self.inner.gesture.borrow().last_pos;
        let snapped = XYPosition::new(pointer_pos.x_snapped, pointer_pos.y_snapped);
        if Some(snapped) == last_pos {
            return;
        }
        if !self.inner.drag_items.borrow().is_empty() && self.inner.gesture.borrow().drag_started {
            let evp = get_event_position(event, bounds);
            self.inner.gesture.borrow_mut().mouse_position = XYPosition::new(evp.x, evp.y);
            self.update_nodes(snapped);
        }
    }

    fn update_nodes(&self, pointer_xy: XYPosition) {
        let store = (self.inner.params.get_store_items)();
        let mut drag_items = self.inner.drag_items.borrow_mut();
        let is_multi_drag = drag_items.len() > 1;
        let nodes_box = if is_multi_drag {
            Some(crate::utils::general::rect_to_box(
                crate::utils::graph::get_internal_nodes_bounds(
                    &as_internal_node_lookup(&drag_items),
                    crate::utils::graph::GetInternalNodesBoundsParams::default(),
                ),
            ))
        } else {
            None
        };
        let multi_drag_snap_offset = if is_multi_drag && store.snap_to_grid {
            calculate_snap_offset(&drag_items, store.snap_grid, pointer_xy.x, pointer_xy.y)
        } else {
            None
        };

        let mut has_change = false;
        let mut updates: Vec<(String, XYPosition, XYPosition)> = Vec::new();

        for (id, drag_item) in drag_items.iter() {
            if !store.node_lookup.contains_key(id) {
                continue;
            }
            let mut next_position = XYPosition {
                x: pointer_xy.x - drag_item.distance.x,
                y: pointer_xy.y - drag_item.distance.y,
            };
            if store.snap_to_grid {
                next_position = match multi_drag_snap_offset {
                    Some(off) => XYPosition {
                        x: (next_position.x + off.x).round(),
                        y: (next_position.y + off.y).round(),
                    },
                    None => snap_position(next_position, store.snap_grid),
                };
            }

            let mut adjusted_extent = None;
            if is_multi_drag {
                if let Some(nb) = nodes_box {
                    let p_abs = drag_item.position_absolute;
                    let x1 = p_abs.x - nb.x + store.node_extent[0][0];
                    let x2 = p_abs.x + drag_item.measured.width - nb.x2 + store.node_extent[1][0];
                    let y1 = p_abs.y - nb.y + store.node_extent[0][1];
                    let y2 = p_abs.y + drag_item.measured.height - nb.y2 + store.node_extent[1][1];
                    adjusted_extent = Some([[x1, y1], [x2, y2]]);
                }
            }
            let extent_to_use = adjusted_extent.unwrap_or(store.node_extent);

            let on_error_ref = store.on_error.as_ref();
            let calc = calculate_node_position(CalculateNodePositionParams {
                node_id: id,
                next_position,
                node_lookup: &store.node_lookup,
                node_origin: store.node_origin,
                node_extent: Some(extent_to_use),
                on_error: on_error_ref,
            });

            if drag_item.position.x != calc.position.x || drag_item.position.y != calc.position.y {
                has_change = true;
            }
            updates.push((id.clone(), calc.position, calc.position_absolute));
        }

        if !has_change {
            return;
        }

        for (id, position, position_absolute) in updates {
            if let Some(item) = drag_items.get_mut(&id) {
                item.position = position;
                item.position_absolute = position_absolute;
            }
        }
        drop(drag_items);

        self.inner.gesture.borrow_mut().node_positions_changed = true;
        self.inner.gesture.borrow_mut().last_pos = Some(pointer_xy);

        let drag_items_snapshot = self.inner.drag_items.borrow().clone();
        (store.update_node_positions)(&drag_items_snapshot, true);
    }

    fn on_drag_end_internal(&self, event: &PointerEventLike) {
        let drag_started;
        let abort_drag;
        {
            let g = self.inner.gesture.borrow();
            drag_started = g.drag_started;
            abort_drag = g.abort_drag;
        }
        if !drag_started || abort_drag {
            return;
        }

        {
            let mut g = self.inner.gesture.borrow_mut();
            g.auto_pan_started = false;
            g.drag_started = false;
        }

        let drag_items = self.inner.drag_items.borrow().clone();
        if drag_items.is_empty() {
            return;
        }

        let store = (self.inner.params.get_store_items)();
        let node_positions_changed = self.inner.gesture.borrow().node_positions_changed;
        if node_positions_changed {
            (store.update_node_positions)(&drag_items, false);
            self.inner.gesture.borrow_mut().node_positions_changed = false;
        }

        let update_params = self.inner.update_params.borrow().clone();
        let node_id = update_params.node_id.clone();
        let params = get_event_handler_params(node_id.as_deref(), &drag_items, &store.node_lookup, false);
        self.fire_drag_callback(
            CallbackKind::Stop,
            event,
            &drag_items,
            &params,
            node_id.as_deref(),
        );
    }

    fn fire_drag_callback(
        &self,
        kind: CallbackKind,
        event: &PointerEventLike,
        drag_items: &HashMap<String, NodeDragItem>,
        params: &DragEventHandlerParams<D>,
        node_id: Option<&str>,
    ) {
        let cb = match kind {
            CallbackKind::Start => self.inner.params.on_drag_start.as_ref(),
            CallbackKind::Drag => self.inner.params.on_drag.as_ref(),
            CallbackKind::Stop => self.inner.params.on_drag_stop.as_ref(),
        };
        if let Some(c) = cb {
            c(event, drag_items, &params.primary, &params.all);
        }

        let node_cb = match kind {
            CallbackKind::Start => self.inner.params.on_node_drag_start.as_ref(),
            CallbackKind::Drag => self.inner.params.on_node_drag.as_ref(),
            CallbackKind::Stop => self.inner.params.on_node_drag_stop.as_ref(),
        };
        if let Some(c) = node_cb {
            c(event, drag_items, &params.primary, &params.all);
        }

        if node_id.is_none() {
            let sel_cb = match kind {
                CallbackKind::Start => self.inner.params.on_selection_drag_start.as_ref(),
                CallbackKind::Drag => self.inner.params.on_selection_drag.as_ref(),
                CallbackKind::Stop => self.inner.params.on_selection_drag_stop.as_ref(),
            };
            if let Some(c) = sel_cb {
                c(event, &params.all);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum CallbackKind {
    Start,
    Drag,
    Stop,
}

// Helper: TS `getInternalNodesBounds(dragItems)` accepts a Map of any
// shape with `internals.positionAbsolute` + `measured`; the Rust
// equivalent only works on `NodeLookup`. We bridge by synthesising a
// throw-away lookup of `InternalNode<()>` clones.
fn as_internal_node_lookup(
    drag_items: &HashMap<String, NodeDragItem>,
) -> NodeLookup<()> {
    let mut lookup = NodeLookup::<()>::with_capacity(drag_items.len());
    for (id, item) in drag_items {
        let mut user = Node::<()>::minimal(id, item.position.x, item.position.y);
        user.measured = Some(crate::types::nodes::MeasuredDimensions {
            width: Some(item.measured.width),
            height: Some(item.measured.height),
        });
        let mut internal = InternalNode::from_user(user);
        internal.measured = crate::types::nodes::MeasuredDimensions {
            width: Some(item.measured.width),
            height: Some(item.measured.height),
        };
        internal.internals.position_absolute = item.position_absolute;
        lookup.insert(id.clone(), internal);
    }
    lookup
}

// Suppress unused import on DragEvent from rgraph_drag — we don't
// emit DragEvents directly but the import keeps the API surface
// discoverable for downstream debugging.
const _: fn() = || {
    let _: Option<DragEvent<()>> = None;
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::geometry::Dimensions;
    use crate::types::nodes::{InternalNode, MeasuredDimensions, Node};

    fn measured_internal(id: &str, x: f64, y: f64, w: f64, h: f64) -> InternalNode<()> {
        let mut user: Node<()> = Node::minimal(id, x, y);
        user.measured = Some(MeasuredDimensions {
            width: Some(w),
            height: Some(h),
        });
        let mut internal = InternalNode::from_user(user);
        internal.measured = MeasuredDimensions {
            width: Some(w),
            height: Some(h),
        };
        internal.internals.position_absolute = XYPosition::new(x, y);
        internal
    }

    fn store_snapshot<D: Clone>(
        lookup: NodeLookup<D>,
        node_drag_threshold: f64,
        position_log: Rc<RefCell<Vec<HashMap<String, NodeDragItem>>>>,
    ) -> StoreSnapshot<D> {
        let log = Rc::clone(&position_log);
        StoreSnapshot {
            node_lookup: Rc::new(lookup),
            node_extent: [
                [f64::NEG_INFINITY, f64::NEG_INFINITY],
                [f64::INFINITY, f64::INFINITY],
            ],
            snap_grid: (1.0, 1.0),
            snap_to_grid: false,
            node_origin: (0.0, 0.0),
            multi_selection_active: false,
            transform: Transform::IDENTITY,
            auto_pan_on_node_drag: false,
            nodes_draggable: true,
            select_nodes_on_drag: true,
            node_drag_threshold,
            auto_pan_speed: 15.0,
            pan_by: Rc::new(|_| Promise::resolved(false)),
            update_node_positions: Rc::new(move |items, _dragging| {
                log.borrow_mut().push(items.clone());
            }),
            unselect_nodes_and_edges: Rc::new(|| {}),
            on_error: None,
        }
    }

    fn empty_params<D: Clone + 'static>(
        store_factory: Rc<dyn Fn() -> StoreSnapshot<D>>,
    ) -> XYDragParams<D> {
        XYDragParams {
            get_store_items: store_factory,
            on_drag_start: None,
            on_drag: None,
            on_drag_stop: None,
            on_node_drag_start: None,
            on_node_drag: None,
            on_node_drag_stop: None,
            on_selection_drag_start: None,
            on_selection_drag: None,
            on_selection_drag_stop: None,
            on_node_mouse_down: None,
        }
    }

    #[test]
    fn new_then_destroy_is_safe() {
        let store_log: Rc<RefCell<Vec<HashMap<String, NodeDragItem>>>> = Rc::new(RefCell::new(Vec::new()));
        let log_clone = Rc::clone(&store_log);
        let factory: Rc<dyn Fn() -> StoreSnapshot<()>> = Rc::new(move || {
            store_snapshot(NodeLookup::new(), 0.0, Rc::clone(&log_clone))
        });
        let xd = XYDrag::new(empty_params(factory));
        xd.destroy();
        assert!(*xd.inner.destroyed.borrow());
    }

    #[test]
    fn single_node_drag_threshold_zero_fires_start_immediately() {
        // Build a lookup with a single draggable node "a".
        let mut a = measured_internal("a", 0.0, 0.0, 50.0, 50.0);
        a.user.draggable = Some(true);
        let mut lookup = NodeLookup::new();
        lookup.insert("a".into(), a);

        let log: Rc<RefCell<Vec<HashMap<String, NodeDragItem>>>> = Rc::new(RefCell::new(Vec::new()));
        let log_for_factory = Rc::clone(&log);
        let lookup_rc = Rc::new(lookup);
        let lookup_clone = Rc::clone(&lookup_rc);
        let factory: Rc<dyn Fn() -> StoreSnapshot<()>> = Rc::new(move || {
            let mut snap = store_snapshot::<()>(NodeLookup::new(), 0.0, Rc::clone(&log_for_factory));
            // Replace lookup with shared one so all calls see the same nodes.
            snap.node_lookup = Rc::clone(&lookup_clone);
            snap
        });

        let xd = XYDrag::new(empty_params(factory));
        xd.set_container_bounds(Some(Rect::new(0.0, 0.0, 800.0, 600.0)));
        xd.update(DragUpdateParams {
            node_id: Some("a".into()),
            is_selectable: true,
            ..Default::default()
        });

        let down = xd.handle_pointer_down(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 100.0,
                client_y: 100.0,
                ..Default::default()
            },
            0,
            false,
        );
        assert!(down);
        assert!(xd.is_drag_started());
        assert!(xd.drag_items().contains_key("a"));
    }

    #[test]
    fn drag_threshold_delays_start_until_movement() {
        let mut a = measured_internal("a", 0.0, 0.0, 50.0, 50.0);
        a.user.draggable = Some(true);
        let mut lookup = NodeLookup::new();
        lookup.insert("a".into(), a);

        let log: Rc<RefCell<Vec<HashMap<String, NodeDragItem>>>> = Rc::new(RefCell::new(Vec::new()));
        let log_for_factory = Rc::clone(&log);
        let lookup_rc = Rc::new(lookup);
        let lookup_clone = Rc::clone(&lookup_rc);
        let factory: Rc<dyn Fn() -> StoreSnapshot<()>> = Rc::new(move || {
            let mut snap = store_snapshot::<()>(NodeLookup::new(), 5.0, Rc::clone(&log_for_factory));
            snap.node_lookup = Rc::clone(&lookup_clone);
            snap
        });

        let xd = XYDrag::new(empty_params(factory));
        xd.set_container_bounds(Some(Rect::new(0.0, 0.0, 800.0, 600.0)));
        xd.update(DragUpdateParams {
            node_id: Some("a".into()),
            is_selectable: true,
            ..Default::default()
        });

        let _ = xd.handle_pointer_down(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 100.0,
                client_y: 100.0,
                ..Default::default()
            },
            0,
            false,
        );
        // Threshold > 0, so drag isn't started yet.
        assert!(!xd.is_drag_started());
        // Tiny move (1px) → still not started.
        xd.handle_pointer_move(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 101.0,
                client_y: 100.0,
                ..Default::default()
            },
        );
        assert!(!xd.is_drag_started());
        // Move past 5px threshold → started.
        xd.handle_pointer_move(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 110.0,
                client_y: 100.0,
                ..Default::default()
            },
        );
        assert!(xd.is_drag_started());
    }

    #[test]
    fn end_resets_drag_started() {
        let mut a = measured_internal("a", 0.0, 0.0, 50.0, 50.0);
        a.user.draggable = Some(true);
        let mut lookup = NodeLookup::new();
        lookup.insert("a".into(), a);

        let log: Rc<RefCell<Vec<HashMap<String, NodeDragItem>>>> = Rc::new(RefCell::new(Vec::new()));
        let log_for_factory = Rc::clone(&log);
        let lookup_rc = Rc::new(lookup);
        let lookup_clone = Rc::clone(&lookup_rc);
        let factory: Rc<dyn Fn() -> StoreSnapshot<()>> = Rc::new(move || {
            let mut snap = store_snapshot::<()>(NodeLookup::new(), 0.0, Rc::clone(&log_for_factory));
            snap.node_lookup = Rc::clone(&lookup_clone);
            snap
        });

        let xd = XYDrag::new(empty_params(factory));
        xd.set_container_bounds(Some(Rect::new(0.0, 0.0, 800.0, 600.0)));
        xd.update(DragUpdateParams {
            node_id: Some("a".into()),
            is_selectable: true,
            ..Default::default()
        });
        xd.handle_pointer_down(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 100.0,
                client_y: 100.0,
                ..Default::default()
            },
            0,
            false,
        );
        assert!(xd.is_drag_started());
        xd.handle_pointer_up(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 100.0,
                client_y: 100.0,
                ..Default::default()
            },
        );
        assert!(!xd.is_drag_started());
    }

    #[test]
    fn drag_callbacks_fire() {
        let mut a = measured_internal("a", 0.0, 0.0, 50.0, 50.0);
        a.user.draggable = Some(true);
        let mut lookup = NodeLookup::new();
        lookup.insert("a".into(), a);

        let log: Rc<RefCell<Vec<HashMap<String, NodeDragItem>>>> = Rc::new(RefCell::new(Vec::new()));
        let log_for_factory = Rc::clone(&log);
        let lookup_rc = Rc::new(lookup);
        let lookup_clone = Rc::clone(&lookup_rc);
        let factory: Rc<dyn Fn() -> StoreSnapshot<()>> = Rc::new(move || {
            let mut snap = store_snapshot::<()>(NodeLookup::new(), 0.0, Rc::clone(&log_for_factory));
            snap.node_lookup = Rc::clone(&lookup_clone);
            snap
        });

        let starts = Rc::new(RefCell::new(0u32));
        let stops = Rc::new(RefCell::new(0u32));
        let s_start = Rc::clone(&starts);
        let s_stop = Rc::clone(&stops);

        let mut params = empty_params(factory);
        params.on_drag_start = Some(Rc::new(move |_, _, _, _| {
            *s_start.borrow_mut() += 1;
        }));
        params.on_drag_stop = Some(Rc::new(move |_, _, _, _| {
            *s_stop.borrow_mut() += 1;
        }));

        let xd = XYDrag::new(params);
        xd.set_container_bounds(Some(Rect::new(0.0, 0.0, 800.0, 600.0)));
        xd.update(DragUpdateParams {
            node_id: Some("a".into()),
            is_selectable: true,
            ..Default::default()
        });
        xd.handle_pointer_down(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 100.0,
                client_y: 100.0,
                ..Default::default()
            },
            0,
            false,
        );
        xd.handle_pointer_up(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 100.0,
                client_y: 100.0,
                ..Default::default()
            },
        );
        assert_eq!(*starts.borrow(), 1);
        assert_eq!(*stops.borrow(), 1);
    }

    #[test]
    fn pointer_move_translates_node_position() {
        let mut a = measured_internal("a", 50.0, 50.0, 50.0, 50.0);
        a.user.draggable = Some(true);
        let mut lookup = NodeLookup::new();
        lookup.insert("a".into(), a);

        let log: Rc<RefCell<Vec<HashMap<String, NodeDragItem>>>> = Rc::new(RefCell::new(Vec::new()));
        let log_for_factory = Rc::clone(&log);
        let lookup_rc = Rc::new(lookup);
        let lookup_clone = Rc::clone(&lookup_rc);
        let factory: Rc<dyn Fn() -> StoreSnapshot<()>> = Rc::new(move || {
            let mut snap = store_snapshot::<()>(NodeLookup::new(), 0.0, Rc::clone(&log_for_factory));
            snap.node_lookup = Rc::clone(&lookup_clone);
            snap
        });

        let xd = XYDrag::new(empty_params(factory));
        xd.set_container_bounds(Some(Rect::new(0.0, 0.0, 800.0, 600.0)));
        xd.update(DragUpdateParams {
            node_id: Some("a".into()),
            is_selectable: true,
            ..Default::default()
        });

        // Down at (100, 100) — distance from node (50, 50) absolute is (50, 50).
        xd.handle_pointer_down(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 100.0,
                client_y: 100.0,
                ..Default::default()
            },
            0,
            false,
        );
        // Move by 30 — position should shift to (80, 80).
        xd.handle_pointer_move(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 130.0,
                client_y: 130.0,
                ..Default::default()
            },
        );
        let items = xd.drag_items();
        let item = &items["a"];
        assert!((item.position.x - 80.0).abs() < 1e-9);
        assert!((item.position.y - 80.0).abs() < 1e-9);
        // Position log should have at least one entry from the move.
        assert!(!log.borrow().is_empty());
    }

    #[test]
    fn accept_event_blocks_no_drag_class() {
        let factory: Rc<dyn Fn() -> StoreSnapshot<()>> = Rc::new(|| {
            store_snapshot(NodeLookup::new(), 0.0, Rc::new(RefCell::new(Vec::new())))
        });
        let xd = XYDrag::new(empty_params(factory));
        xd.update(DragUpdateParams {
            no_drag_class_name: Some("nodrag".into()),
            ..Default::default()
        });
        assert!(xd.accept_event(0, &[]));
        assert!(!xd.accept_event(0, &[vec!["nodrag".to_string()]]));
        assert!(!xd.accept_event(2, &[])); // right-click rejected
    }

    #[test]
    fn accept_event_requires_handle_selector() {
        let factory: Rc<dyn Fn() -> StoreSnapshot<()>> = Rc::new(|| {
            store_snapshot(NodeLookup::new(), 0.0, Rc::new(RefCell::new(Vec::new())))
        });
        let xd = XYDrag::new(empty_params(factory));
        xd.update(DragUpdateParams {
            handle_selector: Some("custom-drag-handle".into()),
            ..Default::default()
        });
        // No matching ancestor → rejected.
        assert!(!xd.accept_event(0, &[]));
        // Matching ancestor → accepted.
        assert!(xd.accept_event(
            0,
            &[vec!["custom-drag-handle".to_string()]],
        ));
    }

    #[test]
    fn auto_pan_tick_no_op_when_disabled() {
        let factory: Rc<dyn Fn() -> StoreSnapshot<()>> = Rc::new(|| {
            // auto_pan_on_node_drag is false by default in our helper.
            store_snapshot(NodeLookup::new(), 0.0, Rc::new(RefCell::new(Vec::new())))
        });
        let xd = XYDrag::new(empty_params(factory));
        xd.set_container_bounds(Some(Rect::new(0.0, 0.0, 100.0, 100.0)));
        let p = xd.auto_pan_tick();
        assert_eq!(p.try_take(), Some(false));
    }

    #[test]
    fn helper_lookup_synthesizes_internal_nodes() {
        let mut items = HashMap::new();
        items.insert(
            "x".into(),
            NodeDragItem {
                id: "x".into(),
                position: XYPosition::new(1.0, 2.0),
                distance: XYPosition::ZERO,
                measured: Dimensions::new(10.0, 10.0),
                position_absolute: XYPosition::new(3.0, 4.0),
                extent: crate::types::nodes::NodeExtent::Unbounded,
                parent_id: None,
                origin: None,
                expand_parent: None,
                dragging: None,
            },
        );
        let lookup = as_internal_node_lookup(&items);
        let n = lookup.get("x").unwrap();
        assert_eq!(n.measured.width, Some(10.0));
        assert_eq!(n.internals.position_absolute, XYPosition::new(3.0, 4.0));
    }
}
