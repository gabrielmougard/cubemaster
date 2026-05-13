//! Port of `xyflow-core/src/xyresizer/XYResizer.ts` — node resize
//! manager.
//!
//! Status: implemented (phase 7).
//!
//! Wraps [`rgraph_drag::DragBehavior`] for the drag input + applies
//! the resize math from [`crate::xyresizer::utils`] on every tick.
//!
//! ## Differences vs TS
//!
//! * TS attaches d3-drag to a DOM node and reads
//!   `getBoundingClientRect()` on `start`. The Rust port takes the
//!   container bounds via [`XYResizer::set_container_bounds`]
//!   (caller pre-measures it via `MountedData::get_client_rect()`).
//! * Callbacks are `Rc<dyn Fn>` so the Dioxus consumer can capture
//!   `Signal` writers without lifetime gymnastics.

#![allow(clippy::module_name_repetitions)]
#![allow(clippy::too_many_arguments)]

pub mod types;
pub mod utils;

use std::cell::RefCell;
use std::rc::Rc;

use rgraph_drag::{DragBehavior, PointerId, PointerInput};

use crate::types::geometry::{CoordinateExtent, Rect, Transform, XYPosition};
use crate::types::nodes::{InternalNode, NodeExtent, NodeLookup, NodeOrigin, PointerEventLike};
use crate::types::viewport::SnapGrid;
use crate::utils::dom::{get_pointer_position, ContainerBounds, GetPointerPositionParams};
use crate::xyresizer::types::{
    ControlPosition, OnResizeEndFn, OnResizeFn, OnResizeStartFn, PrevValues, ResizeBoundaries,
    ResizeControlDirection, ResizeParams, ResizeParamsWithDirection, ResizerChange,
    ResizerChildChange, ShouldResizeFn, StartValues,
};
use crate::xyresizer::utils::{
    get_control_direction, get_dimensions_after_resize, get_resize_direction,
    GetResizeDirectionParams,
};

// ---------------------------------------------------------------------------
// Store snapshot
// ---------------------------------------------------------------------------

/// Snapshot of the consumer store needed by `XYResizer`. Mirrors the
/// TS `getStoreItems()` return shape.
pub struct ResizerStoreSnapshot<D: Clone> {
    pub node_lookup: Rc<NodeLookup<D>>,
    pub transform: Transform,
    pub snap_grid: SnapGrid,
    pub snap_to_grid: bool,
    pub node_origin: NodeOrigin,
}

/// Fetcher closure that produces a fresh [`ResizerStoreSnapshot`].
pub type GetResizerStoreFn<D> = Rc<dyn Fn() -> ResizerStoreSnapshot<D>>;

/// Callback that receives both the per-tick [`ResizerChange`] and any
/// child-node position corrections. Mirrors the TS `onChange`.
pub type OnResizerChangeFn = Rc<dyn Fn(&ResizerChange, &[ResizerChildChange])>;

/// Callback fired once the resize completes. Mirrors the TS `onEnd`
/// (called with the *final* prev-values).
pub type OnResizerEndStoreFn = Rc<dyn Fn(&ResizerChange)>;

// ---------------------------------------------------------------------------
// Construction parameters
// ---------------------------------------------------------------------------

/// Construction parameters for [`XYResizer::new`].
pub struct XYResizerParams<D: Clone> {
    pub node_id: String,
    pub get_store_items: GetResizerStoreFn<D>,
    pub on_change: OnResizerChangeFn,
    pub on_end: Option<OnResizerEndStoreFn>,
}

/// Per-update reconfiguration. Mirrors TS `XYResizerUpdateParams`.
pub struct XYResizerUpdateParams {
    pub control_position: ControlPosition,
    pub boundaries: ResizeBoundaries,
    pub keep_aspect_ratio: bool,
    pub resize_direction: Option<ResizeControlDirection>,
    pub on_resize_start: Option<OnResizeStartFn>,
    pub on_resize: Option<OnResizeFn>,
    pub on_resize_end: Option<OnResizeEndFn>,
    pub should_resize: Option<ShouldResizeFn>,
}

// ---------------------------------------------------------------------------
// XYResizer
// ---------------------------------------------------------------------------

/// Pure-state node resize manager.
pub struct XYResizer<D: Clone + 'static> {
    inner: Rc<XYResizerInner<D>>,
}

impl<D: Clone + 'static> Clone for XYResizer<D> {
    fn clone(&self) -> Self {
        XYResizer {
            inner: Rc::clone(&self.inner),
        }
    }
}

struct XYResizerInner<D: Clone + 'static> {
    drag: DragBehavior<()>,
    node_id: String,
    get_store_items: GetResizerStoreFn<D>,
    on_change: OnResizerChangeFn,
    on_end: Option<OnResizerEndStoreFn>,
    update: RefCell<Option<XYResizerUpdateParams>>,
    container_bounds: RefCell<ContainerBounds>,
    state: RefCell<GestureState>,
    destroyed: RefCell<bool>,
}

#[derive(Debug, Clone, Default)]
struct GestureState {
    prev_values: PrevValues,
    start_values: StartValues,
    parent_extent: Option<CoordinateExtent>,
    child_extent: Option<CoordinateExtent>,
    /// Children of the node being resized, with their pre-resize
    /// positions (used to correct relative positions when top/left
    /// changes).
    child_nodes: Vec<ResizerChildChange>,
    /// True after we've fired `on_change` at least once during this
    /// gesture (controls whether `on_resize_end` fires on `end`).
    resize_detected: bool,
    /// True iff a parent exists *and* the node has expand-parent on.
    has_expand_parent_parent: bool,
    /// True iff the gesture is currently active.
    active: bool,
}

impl<D: Clone + 'static> XYResizer<D> {
    /// Build a new manager.
    pub fn new(params: XYResizerParams<D>) -> Self {
        let drag = DragBehavior::<()>::new();
        drag.subject(|ctx| {
            Some(rgraph_drag::Subject {
                x: ctx.x,
                y: ctx.y,
                datum: (),
            })
        });
        XYResizer {
            inner: Rc::new(XYResizerInner {
                drag,
                node_id: params.node_id,
                get_store_items: params.get_store_items,
                on_change: params.on_change,
                on_end: params.on_end,
                update: RefCell::new(None),
                container_bounds: RefCell::new(None),
                state: RefCell::new(GestureState::default()),
                destroyed: RefCell::new(false),
            }),
        }
    }

    /// Replace the cached container bounds (consumer pre-measures it
    /// from the *flow pane*, not the node).
    pub fn set_container_bounds(&self, bounds: Option<Rect>) {
        *self.inner.container_bounds.borrow_mut() = bounds;
    }

    /// Reconfigure the resizer. Mirrors the TS `update()`.
    pub fn update(&self, params: XYResizerUpdateParams) {
        if *self.inner.destroyed.borrow() {
            return;
        }
        *self.inner.update.borrow_mut() = Some(params);
        *self.inner.state.borrow_mut() = GestureState::default();
    }

    /// Detach the manager. Subsequent calls are no-ops.
    pub fn destroy(&self) {
        *self.inner.destroyed.borrow_mut() = true;
        self.inner.drag.on("start", None);
        self.inner.drag.on("drag", None);
        self.inner.drag.on("end", None);
    }

    /// Returns `true` while a resize gesture is in progress.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.inner.state.borrow().active
    }

    // -----------------------------------------------------------------
    // Pointer forwarders
    // -----------------------------------------------------------------

    /// Forward a `pointerdown` over a resize control.
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
        if consumed {
            self.on_resize_start(event);
        }
        consumed
    }

    /// Forward a `pointermove` event.
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
            self.on_resize_drag(event);
        }
        consumed
    }

    /// Forward a `pointerup` event.
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
            self.on_resize_end(event);
        }
        consumed
    }

    /// Forward a `pointercancel` event. Behaves like `pointerup`.
    pub fn handle_pointer_cancel(&self, id: PointerId, event: &PointerEventLike) -> bool {
        if *self.inner.destroyed.borrow() {
            return false;
        }
        let consumed = self.inner.drag.handle(PointerInput::Cancel { id });
        if consumed {
            self.on_resize_end(event);
        }
        consumed
    }

    // -----------------------------------------------------------------
    // Internal handlers
    // -----------------------------------------------------------------

    fn on_resize_start(&self, event: &PointerEventLike) {
        let Some(update_params) = self.inner.update.borrow().as_ref().map(|u| {
            (
                u.control_position,
                u.on_resize_start.clone(),
            )
        }) else {
            return;
        };
        let (_control_pos, on_resize_start) = update_params;

        let store = (self.inner.get_store_items)();
        let Some(node) = store.node_lookup.get(&self.inner.node_id).cloned() else {
            return;
        };

        let container_bounds = *self.inner.container_bounds.borrow();
        let pointer = get_pointer_position(
            event,
            GetPointerPositionParams {
                transform: store.transform,
                snap_grid: store.snap_grid,
                snap_to_grid: store.snap_to_grid,
                container_bounds,
            },
        );

        let width = node.measured.width.unwrap_or(0.0);
        let height = node.measured.height.unwrap_or(0.0);
        let prev_values = PrevValues {
            width,
            height,
            x: node.user.position.x,
            y: node.user.position.y,
        };
        let aspect_ratio = if height != 0.0 {
            width / height
        } else {
            1.0
        };
        let start_values = StartValues {
            width: prev_values.width,
            height: prev_values.height,
            x: prev_values.x,
            y: prev_values.y,
            pointer_x: pointer.x_snapped,
            pointer_y: pointer.y_snapped,
            aspect_ratio,
        };

        // Parent + extent bookkeeping.
        let mut parent_extent: Option<CoordinateExtent> = None;
        let mut has_expand_parent_parent = false;
        if let Some(parent_id) = node.user.parent_id.as_deref() {
            let extent_is_parent = matches!(node.user.extent, NodeExtent::Parent);
            let expand_parent = node.user.expand_parent.unwrap_or(false);
            if extent_is_parent || expand_parent {
                if let Some(parent) = store.node_lookup.get(parent_id) {
                    has_expand_parent_parent = true;
                    if extent_is_parent {
                        if let (Some(pw), Some(ph)) =
                            (parent.measured.width, parent.measured.height)
                        {
                            parent_extent = Some([[0.0, 0.0], [pw, ph]]);
                        }
                    }
                }
            }
        }

        // Collect children + their largest constraining extent.
        let mut child_nodes: Vec<ResizerChildChange> = Vec::new();
        let mut child_extent: Option<CoordinateExtent> = None;
        for (child_id, child) in store.node_lookup.iter() {
            if child.user.parent_id.as_deref() != Some(&self.inner.node_id) {
                continue;
            }
            child_nodes.push(ResizerChildChange {
                id: child_id.clone(),
                position: child.user.position,
                extent: child.user.extent,
            });
            if matches!(child.user.extent, NodeExtent::Parent)
                || child.user.expand_parent.unwrap_or(false)
            {
                let origin = child.user.origin.unwrap_or(store.node_origin);
                let extent = node_to_child_extent(child, &node, origin);
                child_extent = Some(match child_extent {
                    Some(prev) => [
                        [prev[0][0].min(extent[0][0]), prev[0][1].min(extent[0][1])],
                        [prev[1][0].max(extent[1][0]), prev[1][1].max(extent[1][1])],
                    ],
                    None => extent,
                });
            }
        }

        {
            let mut state = self.inner.state.borrow_mut();
            state.prev_values = prev_values;
            state.start_values = start_values;
            state.parent_extent = parent_extent;
            state.child_extent = child_extent;
            state.child_nodes = child_nodes;
            state.resize_detected = false;
            state.has_expand_parent_parent = has_expand_parent_parent;
            state.active = true;
        }

        if let Some(cb) = on_resize_start {
            let params = ResizeParams {
                x: prev_values.x,
                y: prev_values.y,
                width: prev_values.width,
                height: prev_values.height,
            };
            cb(event, &params);
        }
    }

    fn on_resize_drag(&self, event: &PointerEventLike) {
        let Some(update_params) = self.inner.update.borrow().as_ref().map(|u| {
            (
                u.control_position,
                u.boundaries,
                u.keep_aspect_ratio,
                u.resize_direction,
                u.should_resize.clone(),
                u.on_resize.clone(),
            )
        }) else {
            return;
        };
        let (control_pos, boundaries, keep_aspect_ratio, resize_dir, should_resize, on_resize) =
            update_params;

        if !self.inner.state.borrow().active {
            return;
        }

        let store = (self.inner.get_store_items)();
        let Some(node) = store.node_lookup.get(&self.inner.node_id).cloned() else {
            return;
        };

        let pointer = get_pointer_position(
            event,
            GetPointerPositionParams {
                transform: store.transform,
                snap_grid: store.snap_grid,
                snap_to_grid: store.snap_to_grid,
                container_bounds: *self.inner.container_bounds.borrow(),
            },
        );

        let (start_values, prev_values, parent_extent, child_extent) = {
            let s = self.inner.state.borrow();
            (
                s.start_values,
                s.prev_values,
                s.parent_extent,
                s.child_extent,
            )
        };
        let node_origin = node.user.origin.unwrap_or(store.node_origin);
        let control_direction = get_control_direction(control_pos);

        let dims = get_dimensions_after_resize(
            start_values,
            control_direction,
            pointer,
            boundaries,
            keep_aspect_ratio,
            node_origin,
            parent_extent,
            child_extent,
        );

        let is_width_change = dims.width != prev_values.width;
        let is_height_change = dims.height != prev_values.height;
        let is_x_pos_change = dims.x != prev_values.x && is_width_change;
        let is_y_pos_change = dims.y != prev_values.y && is_height_change;

        if !is_x_pos_change && !is_y_pos_change && !is_width_change && !is_height_change {
            return;
        }

        let mut change = ResizerChange::default();
        let mut next_prev = prev_values;
        let mut child_changes: Vec<ResizerChildChange> = Vec::new();

        if is_x_pos_change || is_y_pos_change || node_origin.0 == 1.0 || node_origin.1 == 1.0 {
            change.x = Some(if is_x_pos_change { dims.x } else { prev_values.x });
            change.y = Some(if is_y_pos_change { dims.y } else { prev_values.y });
            next_prev.x = change.x.unwrap();
            next_prev.y = change.y.unwrap();

            let x_change = dims.x - prev_values.x;
            let y_change = dims.y - prev_values.y;
            let child_nodes_snapshot = self.inner.state.borrow().child_nodes.clone();
            for child in child_nodes_snapshot {
                let new_pos = XYPosition::new(
                    child.position.x - x_change + node_origin.0 * (dims.width - prev_values.width),
                    child.position.y - y_change + node_origin.1 * (dims.height - prev_values.height),
                );
                child_changes.push(ResizerChildChange {
                    id: child.id,
                    position: new_pos,
                    extent: child.extent,
                });
            }
        }

        if is_width_change || is_height_change {
            change.width = Some(
                if is_width_change
                    && (resize_dir.is_none() || resize_dir == Some(ResizeControlDirection::Horizontal))
                {
                    dims.width
                } else {
                    prev_values.width
                },
            );
            change.height = Some(
                if is_height_change
                    && (resize_dir.is_none() || resize_dir == Some(ResizeControlDirection::Vertical))
                {
                    dims.height
                } else {
                    prev_values.height
                },
            );
            next_prev.width = change.width.unwrap();
            next_prev.height = change.height.unwrap();
        }

        // expand-parent correction for top/left dragging.
        let mut updated_start_values = self.inner.state.borrow().start_values;
        if self.inner.state.borrow().has_expand_parent_parent
            && node.user.expand_parent.unwrap_or(false)
        {
            let x_limit = node_origin.0 * change.width.unwrap_or(prev_values.width);
            if let Some(cx) = change.x {
                if cx < x_limit {
                    next_prev.x = x_limit;
                    updated_start_values.x -= cx - x_limit;
                    change.x = Some(x_limit);
                }
            }
            let y_limit = node_origin.1 * change.height.unwrap_or(prev_values.height);
            if let Some(cy) = change.y {
                if cy < y_limit {
                    next_prev.y = y_limit;
                    updated_start_values.y -= cy - y_limit;
                    change.y = Some(y_limit);
                }
            }
        }

        let direction = get_resize_direction(GetResizeDirectionParams {
            width: next_prev.width,
            prev_width: prev_values.width,
            height: next_prev.height,
            prev_height: prev_values.height,
            affects_x: control_direction.affects_x,
            affects_y: control_direction.affects_y,
        });
        let next_values = ResizeParamsWithDirection {
            x: next_prev.x,
            y: next_prev.y,
            width: next_prev.width,
            height: next_prev.height,
            direction,
        };

        if let Some(predicate) = should_resize.as_ref() {
            if !predicate(event, &next_values) {
                return;
            }
        }

        {
            let mut state = self.inner.state.borrow_mut();
            state.prev_values = next_prev;
            state.start_values = updated_start_values;
            state.resize_detected = true;
        }

        if let Some(cb) = on_resize.as_ref() {
            cb(event, &next_values);
        }
        (self.inner.on_change)(&change, &child_changes);
    }

    fn on_resize_end(&self, event: &PointerEventLike) {
        let on_resize_end = self
            .inner
            .update
            .borrow()
            .as_ref()
            .and_then(|u| u.on_resize_end.clone());
        let state_snapshot = {
            let s = self.inner.state.borrow();
            (s.resize_detected, s.prev_values, s.active)
        };
        let (resize_detected, prev_values, _) = state_snapshot;

        // Always mark inactive on `end` so future updates can find a
        // clean slate.
        self.inner.state.borrow_mut().active = false;

        if !resize_detected {
            return;
        }

        let params = ResizeParams {
            x: prev_values.x,
            y: prev_values.y,
            width: prev_values.width,
            height: prev_values.height,
        };
        if let Some(cb) = on_resize_end {
            cb(event, &params);
        }
        if let Some(cb) = self.inner.on_end.as_ref() {
            cb(&ResizerChange {
                x: Some(prev_values.x),
                y: Some(prev_values.y),
                width: Some(prev_values.width),
                height: Some(prev_values.height),
            });
        }
        self.inner.state.borrow_mut().resize_detected = false;
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

fn node_to_child_extent<D: Clone>(
    child: &InternalNode<D>,
    parent: &InternalNode<D>,
    node_origin: NodeOrigin,
) -> CoordinateExtent {
    let x = parent.user.position.x + child.user.position.x;
    let y = parent.user.position.y + child.user.position.y;
    let width = child.measured.width.unwrap_or(0.0);
    let height = child.measured.height.unwrap_or(0.0);
    let origin_x = node_origin.0 * width;
    let origin_y = node_origin.1 * height;
    [
        [x - origin_x, y - origin_y],
        [x + width - origin_x, y + height - origin_y],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::nodes::{MeasuredDimensions, Node};
    use std::cell::Cell;

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

    fn store_factory(
        lookup: Rc<NodeLookup<()>>,
    ) -> GetResizerStoreFn<()> {
        Rc::new(move || ResizerStoreSnapshot {
            node_lookup: Rc::clone(&lookup),
            transform: Transform::IDENTITY,
            snap_grid: (1.0, 1.0),
            snap_to_grid: false,
            node_origin: (0.0, 0.0),
        })
    }

    fn no_op_change() -> OnResizerChangeFn {
        Rc::new(|_, _| {})
    }

    fn build_resizer(
        nodes: Vec<InternalNode<()>>,
        change_log: Rc<RefCell<Vec<(ResizerChange, Vec<ResizerChildChange>)>>>,
    ) -> XYResizer<()> {
        let mut lookup = NodeLookup::<()>::new();
        for n in nodes {
            lookup.insert(n.user.id.clone(), n);
        }
        let lookup = Rc::new(lookup);
        let log = Rc::clone(&change_log);
        XYResizer::new(XYResizerParams {
            node_id: "a".into(),
            get_store_items: store_factory(lookup),
            on_change: Rc::new(move |c, kids| {
                log.borrow_mut().push((*c, kids.to_vec()));
            }),
            on_end: None,
        })
    }

    fn default_update(control: ControlPosition) -> XYResizerUpdateParams {
        XYResizerUpdateParams {
            control_position: control,
            boundaries: ResizeBoundaries::default(),
            keep_aspect_ratio: false,
            resize_direction: None,
            on_resize_start: None,
            on_resize: None,
            on_resize_end: None,
            should_resize: None,
        }
    }

    #[test]
    fn new_then_destroy_is_safe() {
        let r = build_resizer(vec![], Rc::new(RefCell::new(Vec::new())));
        r.destroy();
        assert!(*r.inner.destroyed.borrow());
        assert!(!r.is_active());
    }

    #[test]
    fn pointer_down_initializes_state() {
        let node = measured_internal("a", 10.0, 20.0, 100.0, 50.0);
        let log = Rc::new(RefCell::new(Vec::new()));
        let r = build_resizer(vec![node], Rc::clone(&log));
        r.set_container_bounds(Some(Rect::new(0.0, 0.0, 800.0, 600.0)));
        r.update(default_update(ControlPosition::BottomRight));

        let down = r.handle_pointer_down(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 50.0,
                client_y: 50.0,
                ..Default::default()
            },
            0,
            false,
        );
        assert!(down);
        assert!(r.is_active());
        // start values match node dimensions
        let s = r.inner.state.borrow();
        assert_eq!(s.start_values.width, 100.0);
        assert_eq!(s.start_values.height, 50.0);
        assert_eq!(s.start_values.x, 10.0);
        assert_eq!(s.start_values.y, 20.0);
    }

    #[test]
    fn drag_bottom_right_grows_node_and_fires_on_change() {
        let node = measured_internal("a", 0.0, 0.0, 100.0, 100.0);
        let log = Rc::new(RefCell::new(Vec::new()));
        let r = build_resizer(vec![node], Rc::clone(&log));
        r.set_container_bounds(Some(Rect::new(0.0, 0.0, 800.0, 600.0)));
        r.update(default_update(ControlPosition::BottomRight));

        r.handle_pointer_down(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 100.0,
                client_y: 100.0,
                ..Default::default()
            },
            0,
            false,
        );
        r.handle_pointer_move(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 150.0,
                client_y: 150.0,
                ..Default::default()
            },
        );
        let entries = log.borrow();
        assert!(!entries.is_empty());
        let (change, _children) = entries.last().unwrap();
        assert_eq!(change.width, Some(150.0));
        assert_eq!(change.height, Some(150.0));
        // bottom-right doesn't move x/y; they should not be Some.
        assert!(change.x.is_none() || change.x == Some(0.0));
        assert!(change.y.is_none() || change.y == Some(0.0));
    }

    #[test]
    fn drag_top_left_emits_position_change() {
        let node = measured_internal("a", 0.0, 0.0, 100.0, 100.0);
        let log = Rc::new(RefCell::new(Vec::new()));
        let r = build_resizer(vec![node], Rc::clone(&log));
        r.set_container_bounds(Some(Rect::new(0.0, 0.0, 800.0, 600.0)));
        r.update(default_update(ControlPosition::TopLeft));

        r.handle_pointer_down(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 0.0,
                client_y: 0.0,
                ..Default::default()
            },
            0,
            false,
        );
        r.handle_pointer_move(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 10.0,
                client_y: 20.0,
                ..Default::default()
            },
        );
        let entries = log.borrow();
        assert!(!entries.is_empty());
        let (change, _) = entries.last().unwrap();
        // top-left drag inward by (10, 20) → x = 10, y = 20, w = 90, h = 80.
        assert_eq!(change.x, Some(10.0));
        assert_eq!(change.y, Some(20.0));
        assert_eq!(change.width, Some(90.0));
        assert_eq!(change.height, Some(80.0));
    }

    #[test]
    fn pointer_up_fires_on_end() {
        let node = measured_internal("a", 0.0, 0.0, 100.0, 100.0);
        let log = Rc::new(RefCell::new(Vec::new()));
        let end_fired = Rc::new(Cell::new(false));
        let ef = Rc::clone(&end_fired);

        let mut lookup = NodeLookup::<()>::new();
        lookup.insert("a".into(), node);
        let lookup = Rc::new(lookup);

        let r = XYResizer::new(XYResizerParams {
            node_id: "a".into(),
            get_store_items: store_factory(lookup),
            on_change: {
                let l = Rc::clone(&log);
                Rc::new(move |c, kids| {
                    l.borrow_mut().push((*c, kids.to_vec()));
                })
            },
            on_end: Some(Rc::new(move |_| {
                ef.set(true);
            })),
        });

        r.set_container_bounds(Some(Rect::new(0.0, 0.0, 800.0, 600.0)));
        r.update(default_update(ControlPosition::BottomRight));

        r.handle_pointer_down(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 100.0,
                client_y: 100.0,
                ..Default::default()
            },
            0,
            false,
        );
        r.handle_pointer_move(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 110.0,
                client_y: 110.0,
                ..Default::default()
            },
        );
        r.handle_pointer_up(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 110.0,
                client_y: 110.0,
                ..Default::default()
            },
        );
        assert!(end_fired.get());
        assert!(!r.is_active());
    }

    #[test]
    fn should_resize_predicate_can_cancel_tick() {
        let node = measured_internal("a", 0.0, 0.0, 100.0, 100.0);
        let log = Rc::new(RefCell::new(Vec::new()));
        let r = build_resizer(vec![node], Rc::clone(&log));
        r.set_container_bounds(Some(Rect::new(0.0, 0.0, 800.0, 600.0)));
        let mut params = default_update(ControlPosition::BottomRight);
        params.should_resize = Some(Rc::new(|_, _| false));
        r.update(params);

        r.handle_pointer_down(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 100.0,
                client_y: 100.0,
                ..Default::default()
            },
            0,
            false,
        );
        r.handle_pointer_move(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 200.0,
                client_y: 200.0,
                ..Default::default()
            },
        );
        // Predicate rejected → no on_change fired.
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn keeps_aspect_ratio_when_enabled() {
        let node = measured_internal("a", 0.0, 0.0, 100.0, 50.0);
        let log = Rc::new(RefCell::new(Vec::new()));
        let r = build_resizer(vec![node], Rc::clone(&log));
        r.set_container_bounds(Some(Rect::new(0.0, 0.0, 800.0, 600.0)));
        let mut params = default_update(ControlPosition::Right);
        params.keep_aspect_ratio = true;
        r.update(params);

        r.handle_pointer_down(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 100.0,
                client_y: 25.0,
                ..Default::default()
            },
            0,
            false,
        );
        r.handle_pointer_move(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 200.0,
                client_y: 25.0,
                ..Default::default()
            },
        );
        let entries = log.borrow();
        assert!(!entries.is_empty());
        let (change, _) = entries.last().unwrap();
        // Aspect ratio 2:1 — width grew to 200, height should track to 100.
        assert_eq!(change.width, Some(200.0));
        assert_eq!(change.height, Some(100.0));
    }

    #[test]
    fn children_relative_positions_updated_when_top_left_dragged() {
        let parent = measured_internal("a", 0.0, 0.0, 100.0, 100.0);
        let mut child = measured_internal("c", 20.0, 30.0, 10.0, 10.0);
        child.user.parent_id = Some("a".into());

        let log = Rc::new(RefCell::new(Vec::new()));
        let r = build_resizer(vec![parent, child], Rc::clone(&log));
        r.set_container_bounds(Some(Rect::new(0.0, 0.0, 800.0, 600.0)));
        r.update(default_update(ControlPosition::TopLeft));

        r.handle_pointer_down(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 0.0,
                client_y: 0.0,
                ..Default::default()
            },
            0,
            false,
        );
        r.handle_pointer_move(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 10.0,
                client_y: 20.0,
                ..Default::default()
            },
        );
        let entries = log.borrow();
        let (_, children) = entries.last().unwrap();
        assert_eq!(children.len(), 1);
        let c = &children[0];
        assert_eq!(c.id, "c");
        // x_change = 10, y_change = 20 → child shifts by (-10, -20)
        assert!((c.position.x - 10.0).abs() < 1e-9);
        assert!((c.position.y - 10.0).abs() < 1e-9);
    }

    #[test]
    fn resize_direction_horizontal_only_filters_height() {
        let node = measured_internal("a", 0.0, 0.0, 100.0, 100.0);
        let log = Rc::new(RefCell::new(Vec::new()));
        let r = build_resizer(vec![node], Rc::clone(&log));
        r.set_container_bounds(Some(Rect::new(0.0, 0.0, 800.0, 600.0)));
        let mut params = default_update(ControlPosition::BottomRight);
        params.resize_direction = Some(ResizeControlDirection::Horizontal);
        r.update(params);

        r.handle_pointer_down(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 100.0,
                client_y: 100.0,
                ..Default::default()
            },
            0,
            false,
        );
        r.handle_pointer_move(
            PointerId::Mouse,
            &PointerEventLike {
                client_x: 150.0,
                client_y: 200.0,
                ..Default::default()
            },
        );
        let entries = log.borrow();
        let (change, _) = entries.last().unwrap();
        // Horizontal mode: width grows to 150 but height stays at 100.
        assert_eq!(change.width, Some(150.0));
        // height was reported via change.height even if not changed
        // because the dim-change branch always populates it; for the
        // horizontal-only case the value is the previous (100).
        assert_eq!(change.height, Some(100.0));
    }
}
