//! Port of `xyflow-core/src/xyhandle/XYHandle.ts` — connection-line
//! state machine.
//!
//! Status: implemented (phase 6).
//!
//! The TS source registers `mousemove` / `mouseup` (and the touch
//! equivalents) directly on the document inside `onPointerDown`. In
//! Rust the Dioxus consumer wires those events at the component
//! level and feeds them into [`XYHandle::handle_pointer_move`] /
//! [`XYHandle::handle_pointer_up`].
//!
//! `XYHandle` is *single-instance* — it tracks at most one in-progress
//! connection. Apps with multiple simultaneous connections can
//! construct a separate `XYHandle` per source handle.

#![allow(clippy::module_name_repetitions)]

pub mod types;
pub mod utils;

use std::cell::RefCell;
use std::rc::Rc;

use crate::promise::Promise;
use crate::types::connection::{
    Connection, ConnectionInProgress, ConnectionMode, ConnectionState, EdgeOrConnection,
    FinalConnectionState,
};
use crate::types::geometry::{Position, Transform, XYPosition};
use crate::types::handles::{Handle, HandleType};
use crate::types::nodes::{InternalNode, NodeLookup, PointerEventLike};
use crate::utils::dom::get_event_position;
use crate::utils::edges::positions::get_handle_position;
use crate::utils::general::{calc_auto_pan, point_to_renderer_point, renderer_point_to_point};
use crate::xyhandle::types::{
    HandleSnapshot, IsValidParams, OnConnectStartArgs, StartConnectionParams, ValidHandleResult,
};
use crate::xyhandle::utils::{get_closest_handle, get_handle, is_connection_valid};

// ---------------------------------------------------------------------------
// Connection-state update callback
// ---------------------------------------------------------------------------

/// Closure that consumes a fresh [`ConnectionState`] each tick. The
/// Dioxus consumer typically pushes this into a `Signal`.
///
/// Generic over `D` because the consumer's node-data type flows
/// through `ConnectionState::InProgress.from_node`.
pub type UpdateConnectionFn<D> = Rc<dyn Fn(ConnectionState<InternalNode<D>>)>;

/// Closure that pans the viewport. Mirrors the `PanBy` callback used
/// by the rest of the workspace.
pub type PanByFn = Rc<dyn Fn(XYPosition) -> Promise<bool>>;

/// Provides on-demand access to the latest viewport transform.
pub type GetTransformFn = Rc<dyn Fn() -> Transform>;

/// Provides on-demand access to the latest handle the gesture began
/// from. Returns `None` if the originating handle has been removed
/// (e.g. the user deleted the source node mid-drag).
pub type GetFromHandleFn = Rc<dyn Fn() -> Option<Handle>>;

/// Provides on-demand access to the latest node lookup. The
/// connection logic reads this every move event so the consumer's
/// store mutations are picked up live.
pub type GetNodeLookupFn<D> = Rc<dyn Fn() -> Rc<NodeLookup<D>>>;

// ---------------------------------------------------------------------------
// XYHandle
// ---------------------------------------------------------------------------

/// Connection-line state machine.
pub struct XYHandle<D: Clone + 'static> {
    inner: Rc<XYHandleInner<D>>,
}

impl<D: Clone + 'static> Clone for XYHandle<D> {
    fn clone(&self) -> Self {
        XYHandle {
            inner: Rc::clone(&self.inner),
        }
    }
}

struct XYHandleInner<D: Clone + 'static> {
    /// Store accessors.
    update_connection: RefCell<Option<UpdateConnectionFn<D>>>,
    cancel_connection: RefCell<Option<Rc<dyn Fn()>>>,
    pan_by: RefCell<Option<PanByFn>>,
    get_transform: RefCell<Option<GetTransformFn>>,
    get_from_handle: RefCell<Option<GetFromHandleFn>>,
    get_node_lookup: RefCell<Option<GetNodeLookupFn<D>>>,

    /// Active gesture state, or `None` when no connection is in
    /// progress.
    gesture: RefCell<Option<GestureState<D>>>,
}

struct GestureState<D: Clone + 'static> {
    params: StartConnectionParams<D>,
    /// Container-relative pointer position at the moment of the
    /// originating pointerdown (used for the drag-threshold check).
    down_event_pos: XYPosition,
    /// Pointer position in container-relative pixels (used by
    /// auto-pan).
    pointer_container: XYPosition,
    /// Last-emitted in-progress state.
    last_in_progress: ConnectionInProgress<InternalNode<D>>,
    /// `true` after the drag threshold was crossed (or immediately if
    /// `drag_threshold == 0`).
    connection_started: bool,
    /// Whether the auto-pan loop has been requested by the consumer.
    auto_pan_started: bool,
    /// Cached resolved connection produced by the most recent
    /// is_valid call.
    candidate_connection: Option<Connection>,
    candidate_is_valid: Option<bool>,
    /// `True` while the gesture is in edge-updater mode.
    edge_updater: bool,
    /// True when the closest-handle search returned something this
    /// tick. Used to know whether to fire `on_connect`.
    has_close_handle: bool,
}

impl<D: Clone + 'static> XYHandle<D> {
    /// Construct a new, idle handle manager.
    ///
    /// Consumers call [`Self::configure`] once with their store
    /// accessors, then [`Self::start`] on every `pointerdown` over a
    /// handle.
    pub fn new() -> Self {
        XYHandle {
            inner: Rc::new(XYHandleInner {
                update_connection: RefCell::new(None),
                cancel_connection: RefCell::new(None),
                pan_by: RefCell::new(None),
                get_transform: RefCell::new(None),
                get_from_handle: RefCell::new(None),
                get_node_lookup: RefCell::new(None),
                gesture: RefCell::new(None),
            }),
        }
    }

    /// Configure (or re-configure) the store accessors. Must be called
    /// at least once before [`Self::start`].
    pub fn configure(&self, cfg: XYHandleConfig<D>) {
        *self.inner.update_connection.borrow_mut() = Some(cfg.update_connection);
        *self.inner.cancel_connection.borrow_mut() = Some(cfg.cancel_connection);
        *self.inner.pan_by.borrow_mut() = Some(cfg.pan_by);
        *self.inner.get_transform.borrow_mut() = Some(cfg.get_transform);
        *self.inner.get_from_handle.borrow_mut() = Some(cfg.get_from_handle);
        *self.inner.get_node_lookup.borrow_mut() = Some(cfg.get_node_lookup);
    }

    /// `true` when a connection is currently being drawn.
    #[must_use]
    pub fn is_in_progress(&self) -> bool {
        self.inner.gesture.borrow().is_some()
    }

    /// Begin a new connection from `event` at the given handle.
    ///
    /// Mirrors the TS `XYHandle.onPointerDown`. The TS source attaches
    /// `mousemove` / `mouseup` listeners to the document; here the
    /// consumer is responsible for that wiring and forwards each event
    /// via [`Self::handle_pointer_move`] / [`Self::handle_pointer_up`].
    ///
    /// Returns `false` if the gesture couldn't start (e.g. missing
    /// from-handle, missing container bounds).
    pub fn start(&self, event: &PointerEventLike, params: StartConnectionParams<D>) -> bool {
        let Some(get_lookup) = self.inner.get_node_lookup.borrow().clone() else {
            return false;
        };
        let node_lookup = get_lookup();
        let Some(from_internal) = node_lookup.get(&params.node_id).cloned() else {
            return false;
        };

        let Some(container) = params.container_bounds else {
            return false;
        };

        // Resolve the from-handle through the node lookup.
        let from_handle_resolved = match get_handle(
            &params.node_id,
            if params.is_target {
                HandleType::Target
            } else {
                HandleType::Source
            },
            params.handle_id.as_deref(),
            &node_lookup,
            params.connection_mode,
            true,
        ) {
            Some(h) => h,
            None => return false,
        };

        let position = get_event_position(event, Some(container));
        let from = get_handle_position(&from_internal, Some(&from_handle_resolved), Position::Left, true);
        let pointer = XYPosition::new(position.x, position.y);

        let in_progress = ConnectionInProgress {
            is_valid: None,
            from,
            from_handle: from_handle_resolved.clone(),
            from_position: from_handle_resolved.position,
            from_node: from_internal,
            to: pointer,
            to_handle: None,
            to_position: from_handle_resolved.position.opposite(),
            to_node: None,
            pointer,
        };

        let drag_threshold = params.drag_threshold;
        let edge_updater = params.edge_updater_type.is_some();
        let mut state = GestureState {
            params,
            down_event_pos: XYPosition::new(event.client_x, event.client_y),
            pointer_container: pointer,
            last_in_progress: in_progress.clone(),
            connection_started: false,
            auto_pan_started: false,
            candidate_connection: None,
            candidate_is_valid: None,
            edge_updater,
            has_close_handle: false,
        };

        // Threshold == 0 → emit start state immediately.
        if drag_threshold == 0.0 {
            self.fire_start(&mut state, event, in_progress);
        }
        *self.inner.gesture.borrow_mut() = Some(state);
        true
    }

    /// Forward a `pointermove` event. Returns `true` if the gesture
    /// is still active afterwards (consumers can stop forwarding once
    /// this returns `false`).
    ///
    /// `handle_below_pointer` is the result of the consumer's
    /// pointer-hit test against its own component tree (e.g. by
    /// walking `event.target` for a `data-handlepos` attribute). Pass
    /// `None` when no handle is under the pointer.
    pub fn handle_pointer_move(
        &self,
        event: &PointerEventLike,
        handle_below_pointer: Option<HandleSnapshot>,
    ) -> bool {
        if !self.is_in_progress() {
            return false;
        }

        let cancel_now = {
            let mut g = self.inner.gesture.borrow_mut();
            let Some(state) = g.as_mut() else {
                return false;
            };

            // Threshold gating.
            if !state.connection_started {
                let dx = event.client_x - state.down_event_pos.x;
                let dy = event.client_y - state.down_event_pos.y;
                let crossed = dx * dx + dy * dy
                    > state.params.drag_threshold * state.params.drag_threshold;
                if !crossed {
                    return true;
                }
                drop(g);
                let mut g2 = self.inner.gesture.borrow_mut();
                let state2 = g2.as_mut().unwrap();
                let snapshot = state2.last_in_progress.clone();
                self.fire_start(state2, event, snapshot);
                return true;
            }
            false
        };
        let _ = cancel_now;

        let get_from_handle = self.inner.get_from_handle.borrow().clone();
        let get_transform = self.inner.get_transform.borrow().clone();
        let get_lookup = self.inner.get_node_lookup.borrow().clone();
        let update_connection = self.inner.update_connection.borrow().clone();
        let pan_by = self.inner.pan_by.borrow().clone();

        let (Some(from_handle_provider), Some(transform_provider), Some(lookup_provider)) =
            (get_from_handle, get_transform, get_lookup)
        else {
            // Configuration missing → cancel the gesture safely.
            self.cancel();
            return false;
        };

        if from_handle_provider().is_none() {
            // Source handle deleted mid-drag → fall through to cleanup.
            self.handle_pointer_up(event);
            return false;
        }

        let transform = transform_provider();
        let node_lookup = lookup_provider();

        // Update container-relative pointer position.
        let container = {
            let g = self.inner.gesture.borrow();
            g.as_ref().and_then(|s| s.params.container_bounds)
        };
        let pos = get_event_position(event, container);
        let pointer = XYPosition::new(pos.x, pos.y);

        let renderer_point = point_to_renderer_point(pointer, transform, false, (1.0, 1.0));

        let (from_node_id, from_handle_id, from_type, connection_radius, connection_mode) = {
            let g = self.inner.gesture.borrow();
            let s = g.as_ref().unwrap();
            (
                s.params.node_id.clone(),
                s.params.handle_id.clone(),
                if s.params.is_target {
                    HandleType::Target
                } else {
                    HandleType::Source
                },
                s.params.connection_radius,
                s.params.connection_mode,
            )
        };

        let closest = get_closest_handle(
            renderer_point,
            connection_radius,
            &node_lookup,
            &from_node_id,
            from_handle_id.as_deref(),
            from_type,
        );

        // Kick off auto-pan if requested by the consumer.
        let should_pan = {
            let g = self.inner.gesture.borrow();
            let s = g.as_ref().unwrap();
            s.params.auto_pan_on_connect && !s.auto_pan_started
        };
        if should_pan {
            if let Some(state) = self.inner.gesture.borrow_mut().as_mut() {
                state.auto_pan_started = true;
                state.pointer_container = pointer;
            }
            self.tick_auto_pan(pan_by.as_ref());
        }

        // Build is-valid params.
        let is_valid_connection = {
            let g = self.inner.gesture.borrow();
            g.as_ref().and_then(|s| s.params.is_valid_connection.as_ref().map(|f| {
                // We can't move the Box out — pass an owning reference
                // through the Rc clone instead.
                let _ = f;
                ()
            }));
            // The actual closure is consulted via `validate_connection`
            // below since we need access by `&` not by clone.
            ()
        };
        let _ = is_valid_connection;

        let candidate_snapshot = closest.as_ref().map(|h| HandleSnapshot {
            node_id: h.node_id.clone(),
            id: h.id.clone(),
            type_: h.type_,
            connectable: true,
            connectable_end: true,
        });
        let result = self.is_valid(
            event,
            IsValidParams {
                handle: candidate_snapshot,
                connection_mode,
                from_node_id: &from_node_id,
                from_handle_id: from_handle_id.as_deref(),
                from_type,
                is_valid_connection: None,
                handle_below_pointer: handle_below_pointer.clone(),
            },
            &node_lookup,
        );

        // Run the user's is_valid_connection predicate (if any) after
        // the bookkeeping check.
        let mut final_is_valid = result.is_valid;
        if final_is_valid {
            if let Some(conn) = result.connection.as_ref() {
                let user_predicate = {
                    let g = self.inner.gesture.borrow();
                    g.as_ref()
                        .and_then(|s| s.params.is_valid_connection.as_ref().map(|cb| cb as *const _))
                };
                if let Some(_ptr) = user_predicate {
                    // SAFETY: the borrow above kept the ref alive for
                    // this scope; we re-borrow through the RefCell.
                    let g = self.inner.gesture.borrow();
                    if let Some(state) = g.as_ref() {
                        if let Some(predicate) = state.params.is_valid_connection.as_ref() {
                            final_is_valid = predicate(EdgeOrConnection::Connection(conn));
                        }
                    }
                }
            }
        }
        let is_valid_tri = is_connection_valid(closest.is_some(), final_is_valid);

        // Update the in-progress connection snapshot.
        let (from_internal_opt, from_handle_now) = {
            let g = self.inner.gesture.borrow();
            let s = g.as_ref().unwrap();
            (
                node_lookup.get(&s.params.node_id).cloned(),
                s.last_in_progress.from_handle.clone(),
            )
        };
        let from = match from_internal_opt.as_ref() {
            Some(internal) => {
                get_handle_position(internal, Some(&from_handle_now), Position::Left, true)
            }
            None => self.inner.gesture.borrow().as_ref().unwrap().last_in_progress.from,
        };

        let next = {
            let g = self.inner.gesture.borrow();
            let s = g.as_ref().unwrap();
            let mut next = s.last_in_progress.clone();
            next.from = from;
            next.is_valid = is_valid_tri;
            let to_handle = result.to_handle.clone();
            next.to = if final_is_valid && to_handle.is_some() {
                let h = to_handle.as_ref().unwrap();
                renderer_point_to_point(XYPosition::new(h.x, h.y), transform)
            } else {
                pointer
            };
            next.to_position = match (&to_handle, final_is_valid) {
                (Some(h), true) => h.position,
                _ => s.last_in_progress.from_handle.position.opposite(),
            };
            next.to_node = to_handle
                .as_ref()
                .and_then(|h| node_lookup.get(&h.node_id).cloned());
            next.to_handle = to_handle;
            next.pointer = pointer;
            next
        };

        // Persist mutations.
        {
            let mut g = self.inner.gesture.borrow_mut();
            let s = g.as_mut().unwrap();
            s.candidate_connection = result.connection.clone();
            s.candidate_is_valid = is_valid_tri;
            s.has_close_handle = closest.is_some() || handle_below_pointer.is_some();
            s.last_in_progress = next.clone();
            s.pointer_container = pointer;
        }

        if let Some(cb) = update_connection {
            cb(ConnectionState::InProgress(next));
        }
        true
    }

    /// Forward a `pointerup` (or `touchend`) event. Fires `on_connect`
    /// + `on_connect_end` as appropriate and clears the gesture.
    pub fn handle_pointer_up(&self, event: &PointerEventLike) {
        let snapshot = match self.inner.gesture.borrow().as_ref() {
            Some(s) => (
                s.connection_started,
                s.candidate_connection.clone(),
                s.candidate_is_valid,
                s.has_close_handle,
                s.last_in_progress.clone(),
                s.edge_updater,
                s.params.on_connect.clone(),
                s.params.on_connect_end.clone(),
                s.params.on_reconnect_end.clone(),
            ),
            None => return,
        };

        let (
            connection_started,
            candidate_connection,
            candidate_is_valid,
            has_close_handle,
            last_in_progress,
            edge_updater,
            on_connect,
            on_connect_end,
            on_reconnect_end,
        ) = snapshot;

        if connection_started {
            if has_close_handle && candidate_is_valid == Some(true) {
                if let (Some(cb), Some(conn)) = (on_connect, candidate_connection) {
                    cb(&conn);
                }
            }

            // TS sets `toPosition = null` when `toHandle` is None; our
            // `Position` enum has no null variant. Consumers can read
            // `to_handle.is_some()` to distinguish — the encoded
            // position remains the last-known opposite.
            let final_state: FinalConnectionState<InternalNode<D>> =
                ConnectionState::InProgress(last_in_progress);
            if let Some(cb) = on_connect_end {
                cb(event, &final_state);
            }
            if edge_updater {
                if let Some(cb) = on_reconnect_end {
                    cb(event, &final_state);
                }
            }
        }

        // Cancel store-side and clear local gesture.
        if let Some(cancel) = self.inner.cancel_connection.borrow().clone() {
            cancel();
        }
        *self.inner.gesture.borrow_mut() = None;
    }

    /// Force-cancel the gesture without firing `on_connect` / `on_connect_end`.
    /// Used by hosts when a node carrying the from-handle is deleted.
    pub fn cancel(&self) {
        *self.inner.gesture.borrow_mut() = None;
        if let Some(cancel) = self.inner.cancel_connection.borrow().clone() {
            cancel();
        }
    }

    /// Pure validity check. Mirrors the TS `XYHandle.isValid` — useful
    /// for tests and for consumers that want to query without
    /// running a full move tick.
    #[must_use]
    pub fn is_valid<'a>(
        &self,
        _event: &PointerEventLike,
        params: IsValidParams<'a>,
        node_lookup: &NodeLookup<D>,
    ) -> ValidHandleResult {
        let mut result = ValidHandleResult::default();

        // TS prioritises the handle *under the cursor* over the
        // closest-distance handle; emulate that here.
        let handle_to_check = params.handle_below_pointer.clone().or(params.handle);
        result.considered_handle = handle_to_check.clone();
        let Some(handle_to_check) = handle_to_check else {
            return result;
        };

        let is_target = params.from_type == HandleType::Target;
        let other_node_id = handle_to_check.node_id.clone();
        let other_handle_id = handle_to_check.id.clone();
        let other_type = handle_to_check.type_;

        let connection = Connection {
            source: if is_target {
                other_node_id.clone()
            } else {
                params.from_node_id.to_string()
            },
            source_handle: if is_target {
                other_handle_id.clone()
            } else {
                params.from_handle_id.map(str::to_string)
            },
            target: if is_target {
                params.from_node_id.to_string()
            } else {
                other_node_id.clone()
            },
            target_handle: if is_target {
                params.from_handle_id.map(str::to_string)
            } else {
                other_handle_id.clone()
            },
        };
        result.connection = Some(connection.clone());

        let is_connectable = handle_to_check.connectable && handle_to_check.connectable_end;
        let valid_under_mode = match params.connection_mode {
            ConnectionMode::Strict => match (is_target, other_type) {
                (true, HandleType::Source) => true,
                (false, HandleType::Target) => true,
                _ => false,
            },
            ConnectionMode::Loose => {
                other_node_id != params.from_node_id
                    || other_handle_id.as_deref() != params.from_handle_id
            }
        };
        let valid_under_predicate = match params.is_valid_connection {
            Some(predicate) => predicate(EdgeOrConnection::Connection(&connection)),
            None => true,
        };
        result.is_valid = is_connectable && valid_under_mode && valid_under_predicate;
        result.to_handle = get_handle(
            &other_node_id,
            other_type,
            other_handle_id.as_deref(),
            node_lookup,
            params.connection_mode,
            true,
        );
        result
    }

    /// Consumer-driven auto-pan tick. Mirrors the TS
    /// `requestAnimationFrame(autoPan)` loop. Returns `false` once the
    /// gesture has ended.
    pub fn auto_pan_tick(&self) -> bool {
        if !self.is_in_progress() {
            return false;
        }
        let pan_by = self.inner.pan_by.borrow().clone();
        self.tick_auto_pan(pan_by.as_ref());
        true
    }

    // -----------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------

    fn tick_auto_pan(&self, pan_by: Option<&PanByFn>) {
        let Some(pan_by) = pan_by else {
            return;
        };
        let (auto_pan, pointer, container, speed) = {
            let g = self.inner.gesture.borrow();
            let Some(s) = g.as_ref() else {
                return;
            };
            (
                s.params.auto_pan_on_connect,
                s.pointer_container,
                s.params.container_bounds,
                s.params.auto_pan_speed,
            )
        };
        if !auto_pan {
            return;
        }
        let Some(container) = container else {
            return;
        };
        let (dx, dy) = calc_auto_pan(
            pointer,
            crate::types::geometry::Dimensions {
                width: container.width,
                height: container.height,
            },
            speed,
            40.0,
        );
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        let _ = pan_by(XYPosition::new(dx, dy));
    }

    fn fire_start(
        &self,
        state: &mut GestureState<D>,
        event: &PointerEventLike,
        in_progress: ConnectionInProgress<InternalNode<D>>,
    ) {
        state.connection_started = true;
        if let Some(cb) = self.inner.update_connection.borrow().clone() {
            cb(ConnectionState::InProgress(in_progress));
        }
        if let Some(cb) = state.params.on_connect_start.clone() {
            let args = OnConnectStartArgs {
                node_id: state.params.node_id.clone(),
                handle_id: state.params.handle_id.clone(),
                handle_type: if state.params.is_target {
                    HandleType::Target
                } else {
                    HandleType::Source
                },
            };
            cb(event, args);
        }
    }
}

impl<D: Clone + 'static> Default for XYHandle<D> {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Configuration struct
// ---------------------------------------------------------------------------

/// Bundle of store-accessor closures passed to [`XYHandle::configure`].
pub struct XYHandleConfig<D: Clone + 'static> {
    pub update_connection: UpdateConnectionFn<D>,
    pub cancel_connection: Rc<dyn Fn()>,
    pub pan_by: PanByFn,
    pub get_transform: GetTransformFn,
    pub get_from_handle: GetFromHandleFn,
    pub get_node_lookup: GetNodeLookupFn<D>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::geometry::{Position, Rect};
    use crate::types::handles::Handle;
    use crate::types::nodes::{MeasuredDimensions, Node, NodeHandleBounds};

    fn build_node(
        id: &str,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        handles: Vec<Handle>,
    ) -> InternalNode<()> {
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
        let mut source = Vec::new();
        let mut target = Vec::new();
        for handle in handles {
            match handle.type_ {
                HandleType::Source => source.push(handle),
                HandleType::Target => target.push(handle),
            }
        }
        internal.internals.handle_bounds = Some(NodeHandleBounds {
            source: Some(source),
            target: Some(target),
        });
        internal
    }

    fn handle(node_id: &str, id: Option<&str>, type_: HandleType, x: f64, y: f64) -> Handle {
        Handle {
            id: id.map(str::to_string),
            node_id: node_id.to_string(),
            x,
            y,
            position: match type_ {
                HandleType::Source => Position::Right,
                HandleType::Target => Position::Left,
            },
            type_,
            width: 1.0,
            height: 1.0,
        }
    }

    fn make_xyhandle(
        lookup: Rc<NodeLookup<()>>,
        captured: Rc<RefCell<Vec<ConnectionState<InternalNode<()>>>>>,
    ) -> XYHandle<()> {
        let xy: XYHandle<()> = XYHandle::new();
        let lookup_for_closures = Rc::clone(&lookup);
        let cap = Rc::clone(&captured);
        xy.configure(XYHandleConfig {
            update_connection: Rc::new(move |state| {
                cap.borrow_mut().push(state);
            }),
            cancel_connection: Rc::new(|| {}),
            pan_by: Rc::new(|_| Promise::resolved(false)),
            get_transform: Rc::new(|| Transform::IDENTITY),
            get_from_handle: Rc::new(|| {
                Some(Handle {
                    id: None,
                    node_id: "a".into(),
                    x: 0.0,
                    y: 0.0,
                    position: Position::Right,
                    type_: HandleType::Source,
                    width: 1.0,
                    height: 1.0,
                })
            }),
            get_node_lookup: Rc::new(move || Rc::clone(&lookup_for_closures)),
        });
        xy
    }

    #[test]
    fn start_requires_existing_from_handle() {
        let lookup = Rc::new(NodeLookup::<()>::new());
        let captured: Rc<RefCell<Vec<ConnectionState<InternalNode<()>>>>> =
            Rc::new(RefCell::new(Vec::new()));
        let xy = make_xyhandle(Rc::clone(&lookup), Rc::clone(&captured));

        let started = xy.start(
            &PointerEventLike::default(),
            StartConnectionParams {
                auto_pan_on_connect: false,
                connection_mode: ConnectionMode::Strict,
                connection_radius: 25.0,
                container_bounds: Some(Rect::new(0.0, 0.0, 100.0, 100.0)),
                handle_id: None,
                node_id: "missing".into(),
                is_target: false,
                edge_updater_type: None,
                auto_pan_speed: 15.0,
                drag_threshold: 1.0,
                is_valid_connection: None,
                on_connect_start: None,
                on_connect: None,
                on_connect_end: None,
                on_reconnect_end: None,
            },
        );
        assert!(!started);
        assert!(!xy.is_in_progress());
    }

    #[test]
    fn start_with_zero_threshold_fires_start_immediately() {
        let mut lookup_inner = NodeLookup::<()>::new();
        let h = handle("a", Some("s1"), HandleType::Source, 5.0, 5.0);
        lookup_inner.insert("a".into(), build_node("a", 0.0, 0.0, 10.0, 10.0, vec![h]));
        let lookup = Rc::new(lookup_inner);
        let captured: Rc<RefCell<Vec<ConnectionState<InternalNode<()>>>>> =
            Rc::new(RefCell::new(Vec::new()));
        let xy = make_xyhandle(Rc::clone(&lookup), Rc::clone(&captured));

        let started = xy.start(
            &PointerEventLike {
                client_x: 100.0,
                client_y: 100.0,
                ..Default::default()
            },
            StartConnectionParams {
                auto_pan_on_connect: false,
                connection_mode: ConnectionMode::Strict,
                connection_radius: 25.0,
                container_bounds: Some(Rect::new(0.0, 0.0, 800.0, 600.0)),
                handle_id: Some("s1".into()),
                node_id: "a".into(),
                is_target: false,
                edge_updater_type: None,
                auto_pan_speed: 15.0,
                drag_threshold: 0.0,
                is_valid_connection: None,
                on_connect_start: None,
                on_connect: None,
                on_connect_end: None,
                on_reconnect_end: None,
            },
        );
        assert!(started);
        assert!(xy.is_in_progress());
        // Initial update_connection should have been pushed.
        assert!(!captured.borrow().is_empty());
        match captured.borrow().last().unwrap() {
            ConnectionState::InProgress(_) => {}
            _ => panic!("expected InProgress"),
        }
    }

    #[test]
    fn drag_threshold_delays_start() {
        let mut lookup_inner = NodeLookup::<()>::new();
        let h = handle("a", Some("s1"), HandleType::Source, 5.0, 5.0);
        lookup_inner.insert("a".into(), build_node("a", 0.0, 0.0, 10.0, 10.0, vec![h]));
        let lookup = Rc::new(lookup_inner);
        let captured: Rc<RefCell<Vec<ConnectionState<InternalNode<()>>>>> =
            Rc::new(RefCell::new(Vec::new()));
        let xy = make_xyhandle(Rc::clone(&lookup), Rc::clone(&captured));

        xy.start(
            &PointerEventLike {
                client_x: 100.0,
                client_y: 100.0,
                ..Default::default()
            },
            StartConnectionParams {
                auto_pan_on_connect: false,
                connection_mode: ConnectionMode::Strict,
                connection_radius: 25.0,
                container_bounds: Some(Rect::new(0.0, 0.0, 800.0, 600.0)),
                handle_id: Some("s1".into()),
                node_id: "a".into(),
                is_target: false,
                edge_updater_type: None,
                auto_pan_speed: 15.0,
                drag_threshold: 10.0,
                is_valid_connection: None,
                on_connect_start: None,
                on_connect: None,
                on_connect_end: None,
                on_reconnect_end: None,
            },
        );
        // Threshold > 0 → no initial update yet (the start hasn't fired).
        let before = captured.borrow().len();
        // Move 5px → below threshold.
        xy.handle_pointer_move(
            &PointerEventLike {
                client_x: 105.0,
                client_y: 100.0,
                ..Default::default()
            },
            None,
        );
        let after_small = captured.borrow().len();
        assert_eq!(after_small, before);
        // Move 50px → crosses threshold, fires start.
        xy.handle_pointer_move(
            &PointerEventLike {
                client_x: 150.0,
                client_y: 100.0,
                ..Default::default()
            },
            None,
        );
        assert!(captured.borrow().len() > before);
    }

    #[test]
    fn pointer_up_without_match_resets_state() {
        let mut lookup_inner = NodeLookup::<()>::new();
        let h = handle("a", Some("s1"), HandleType::Source, 5.0, 5.0);
        lookup_inner.insert("a".into(), build_node("a", 0.0, 0.0, 10.0, 10.0, vec![h]));
        let lookup = Rc::new(lookup_inner);
        let captured: Rc<RefCell<Vec<ConnectionState<InternalNode<()>>>>> =
            Rc::new(RefCell::new(Vec::new()));
        let xy = make_xyhandle(Rc::clone(&lookup), Rc::clone(&captured));

        let connect_called = Rc::new(RefCell::new(false));
        let end_called = Rc::new(RefCell::new(false));
        let cc = Rc::clone(&connect_called);
        let ec = Rc::clone(&end_called);

        xy.start(
            &PointerEventLike {
                client_x: 100.0,
                client_y: 100.0,
                ..Default::default()
            },
            StartConnectionParams {
                auto_pan_on_connect: false,
                connection_mode: ConnectionMode::Strict,
                connection_radius: 25.0,
                container_bounds: Some(Rect::new(0.0, 0.0, 800.0, 600.0)),
                handle_id: Some("s1".into()),
                node_id: "a".into(),
                is_target: false,
                edge_updater_type: None,
                auto_pan_speed: 15.0,
                drag_threshold: 0.0,
                is_valid_connection: None,
                on_connect_start: None,
                on_connect: Some(Rc::new(move |_| {
                    *cc.borrow_mut() = true;
                })),
                on_connect_end: Some(Rc::new(move |_, _| {
                    *ec.borrow_mut() = true;
                })),
                on_reconnect_end: None,
            },
        );
        // No close handle in the lookup near (100,100) → on_connect NOT fired.
        xy.handle_pointer_up(&PointerEventLike {
            client_x: 100.0,
            client_y: 100.0,
            ..Default::default()
        });
        assert!(!*connect_called.borrow());
        // But on_connect_end always fires for a started gesture.
        assert!(*end_called.borrow());
        assert!(!xy.is_in_progress());
    }

    #[test]
    fn cancel_clears_gesture_without_callbacks() {
        let mut lookup_inner = NodeLookup::<()>::new();
        let h = handle("a", Some("s1"), HandleType::Source, 5.0, 5.0);
        lookup_inner.insert("a".into(), build_node("a", 0.0, 0.0, 10.0, 10.0, vec![h]));
        let lookup = Rc::new(lookup_inner);
        let captured: Rc<RefCell<Vec<ConnectionState<InternalNode<()>>>>> =
            Rc::new(RefCell::new(Vec::new()));
        let xy = make_xyhandle(Rc::clone(&lookup), Rc::clone(&captured));

        xy.start(
            &PointerEventLike::default(),
            StartConnectionParams {
                auto_pan_on_connect: false,
                connection_mode: ConnectionMode::Strict,
                connection_radius: 25.0,
                container_bounds: Some(Rect::new(0.0, 0.0, 800.0, 600.0)),
                handle_id: Some("s1".into()),
                node_id: "a".into(),
                is_target: false,
                edge_updater_type: None,
                auto_pan_speed: 15.0,
                drag_threshold: 0.0,
                is_valid_connection: None,
                on_connect_start: None,
                on_connect: None,
                on_connect_end: None,
                on_reconnect_end: None,
            },
        );
        assert!(xy.is_in_progress());
        xy.cancel();
        assert!(!xy.is_in_progress());
    }

    #[test]
    fn is_valid_strict_rejects_source_to_source() {
        let lookup = NodeLookup::<()>::new();
        let xy: XYHandle<()> = XYHandle::new();
        let result = xy.is_valid(
            &PointerEventLike::default(),
            IsValidParams {
                handle: Some(HandleSnapshot {
                    node_id: "b".into(),
                    id: None,
                    type_: HandleType::Source,
                    connectable: true,
                    connectable_end: true,
                }),
                connection_mode: ConnectionMode::Strict,
                from_node_id: "a",
                from_handle_id: None,
                from_type: HandleType::Source,
                is_valid_connection: None,
                handle_below_pointer: None,
            },
            &lookup,
        );
        assert!(!result.is_valid);
        // A connection is still built (mirrors TS — `result.connection`
        // is populated even when invalid).
        assert!(result.connection.is_some());
    }

    #[test]
    fn is_valid_strict_accepts_source_to_target() {
        let mut lookup = NodeLookup::<()>::new();
        let h = handle("b", None, HandleType::Target, 5.0, 5.0);
        lookup.insert("b".into(), build_node("b", 50.0, 0.0, 10.0, 10.0, vec![h]));
        let xy: XYHandle<()> = XYHandle::new();
        let result = xy.is_valid(
            &PointerEventLike::default(),
            IsValidParams {
                handle: Some(HandleSnapshot {
                    node_id: "b".into(),
                    id: None,
                    type_: HandleType::Target,
                    connectable: true,
                    connectable_end: true,
                }),
                connection_mode: ConnectionMode::Strict,
                from_node_id: "a",
                from_handle_id: None,
                from_type: HandleType::Source,
                is_valid_connection: None,
                handle_below_pointer: None,
            },
            &lookup,
        );
        assert!(result.is_valid);
        assert!(result.to_handle.is_some());
        let conn = result.connection.unwrap();
        assert_eq!(conn.source, "a");
        assert_eq!(conn.target, "b");
    }

    #[test]
    fn is_valid_uses_handle_below_pointer_when_present() {
        let mut lookup = NodeLookup::<()>::new();
        let h_b = handle("b", None, HandleType::Target, 5.0, 5.0);
        let h_c = handle("c", None, HandleType::Target, 5.0, 5.0);
        lookup.insert(
            "b".into(),
            build_node("b", 50.0, 0.0, 10.0, 10.0, vec![h_b]),
        );
        lookup.insert(
            "c".into(),
            build_node("c", 100.0, 0.0, 10.0, 10.0, vec![h_c]),
        );

        let xy: XYHandle<()> = XYHandle::new();
        // closest-handle picked node B but the pointer is actually
        // hovering C — the result should be against C.
        let result = xy.is_valid(
            &PointerEventLike::default(),
            IsValidParams {
                handle: Some(HandleSnapshot {
                    node_id: "b".into(),
                    id: None,
                    type_: HandleType::Target,
                    connectable: true,
                    connectable_end: true,
                }),
                connection_mode: ConnectionMode::Strict,
                from_node_id: "a",
                from_handle_id: None,
                from_type: HandleType::Source,
                is_valid_connection: None,
                handle_below_pointer: Some(HandleSnapshot {
                    node_id: "c".into(),
                    id: None,
                    type_: HandleType::Target,
                    connectable: true,
                    connectable_end: true,
                }),
            },
            &lookup,
        );
        assert!(result.is_valid);
        let conn = result.connection.unwrap();
        assert_eq!(conn.target, "c");
    }

    #[test]
    fn is_valid_non_connectable_rejected() {
        let mut lookup = NodeLookup::<()>::new();
        let h = handle("b", None, HandleType::Target, 5.0, 5.0);
        lookup.insert("b".into(), build_node("b", 50.0, 0.0, 10.0, 10.0, vec![h]));
        let xy: XYHandle<()> = XYHandle::new();
        let result = xy.is_valid(
            &PointerEventLike::default(),
            IsValidParams {
                handle: Some(HandleSnapshot {
                    node_id: "b".into(),
                    id: None,
                    type_: HandleType::Target,
                    connectable: false, // <-- not connectable
                    connectable_end: true,
                }),
                connection_mode: ConnectionMode::Strict,
                from_node_id: "a",
                from_handle_id: None,
                from_type: HandleType::Source,
                is_valid_connection: None,
                handle_below_pointer: None,
            },
            &lookup,
        );
        assert!(!result.is_valid);
    }

    #[test]
    fn is_valid_user_predicate_can_reject() {
        let mut lookup = NodeLookup::<()>::new();
        let h = handle("b", None, HandleType::Target, 5.0, 5.0);
        lookup.insert("b".into(), build_node("b", 50.0, 0.0, 10.0, 10.0, vec![h]));
        let xy: XYHandle<()> = XYHandle::new();
        let predicate: crate::types::connection::IsValidConnection =
            Box::new(|_| false);
        let result = xy.is_valid(
            &PointerEventLike::default(),
            IsValidParams {
                handle: Some(HandleSnapshot {
                    node_id: "b".into(),
                    id: None,
                    type_: HandleType::Target,
                    connectable: true,
                    connectable_end: true,
                }),
                connection_mode: ConnectionMode::Strict,
                from_node_id: "a",
                from_handle_id: None,
                from_type: HandleType::Source,
                is_valid_connection: Some(&predicate),
                handle_below_pointer: None,
            },
            &lookup,
        );
        assert!(!result.is_valid);
    }
}
