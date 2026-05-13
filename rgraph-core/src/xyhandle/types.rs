//! Port of `xyflow-core/src/xyhandle/types.ts`.
//!
//! Status: implemented (phase 6).

#![allow(clippy::module_name_repetitions)]

use std::rc::Rc;

use crate::types::connection::{Connection, ConnectionMode, IsValidConnection};
use crate::types::handles::{Handle, HandleType};
use crate::types::nodes::PointerEventLike;

/// Result of [`crate::xyhandle::XYHandle::is_valid`] — mirrors the TS
/// `Result` type.
#[derive(Debug, Clone, Default)]
pub struct ValidHandleResult {
    /// Whether the candidate produces a valid connection.
    pub is_valid: bool,
    /// The resolved [`Connection`] (source/target/handle ids) when a
    /// valid drop target is hovered, otherwise `None`.
    pub connection: Option<Connection>,
    /// The target [`Handle`] under the pointer, with its absolute
    /// position resolved. `None` when no handle is hovered.
    pub to_handle: Option<Handle>,
    /// Echoes whichever candidate handle was actually inspected (for
    /// the consumer to compare with the closest-handle hit).
    pub considered_handle: Option<HandleSnapshot>,
}

/// Minimal handle descriptor used both as input to
/// [`crate::xyhandle::XYHandle::is_valid`] and as the
/// [`ValidHandleResult::considered_handle`] output. Mirrors the TS
/// `Pick<Handle, 'nodeId' | 'id' | 'type'>` but also carries
/// `connectable` / `connectable_end` flags the consumer reads from
/// the candidate's CSS classes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandleSnapshot {
    pub node_id: String,
    pub id: Option<String>,
    pub type_: HandleType,
    /// `true` if the candidate carries the `connectable` class.
    pub connectable: bool,
    /// `true` if the candidate carries the `connectableend` class.
    pub connectable_end: bool,
}

impl HandleSnapshot {
    /// Convenience: build a snapshot from a [`Handle`] with both
    /// connectable flags set to `true`.
    #[must_use]
    pub fn fully_connectable(h: &Handle) -> Self {
        HandleSnapshot {
            node_id: h.node_id.clone(),
            id: h.id.clone(),
            type_: h.type_,
            connectable: true,
            connectable_end: true,
        }
    }
}

/// Parameters for [`crate::xyhandle::XYHandle::is_valid`].
///
/// Mirrors the TS `IsValidParams`. The TS source includes `doc`,
/// `lib`, and `flowId` — used to compute a `data-id` selector for
/// `doc.querySelector` lookups. We drop all three because the Rust
/// port doesn't reach into the DOM; instead `handle_below_pointer`
/// is passed in by the consumer after its own hit-test (e.g. walking
/// the Dioxus VirtualDom for a matching handle attribute).
pub struct IsValidParams<'a> {
    /// The closest handle (by Euclidean distance, from the connection
    /// radius search) — corresponds to the TS `handle` param.
    pub handle: Option<HandleSnapshot>,
    pub connection_mode: ConnectionMode,
    /// Source node of the in-progress connection.
    pub from_node_id: &'a str,
    /// Source handle id (when the source node has multiple handles).
    pub from_handle_id: Option<&'a str>,
    pub from_type: HandleType,
    /// Optional user-supplied validator predicate. When `None` we
    /// accept all candidates (TS `alwaysValid`).
    pub is_valid_connection: Option<&'a IsValidConnection>,
    /// Handle directly under the pointer, if any. Replaces TS's
    /// `doc.elementFromPoint(x, y)` lookup. The consumer hit-tests
    /// its own component tree and supplies the result.
    pub handle_below_pointer: Option<HandleSnapshot>,
}

/// Construction parameters for [`crate::xyhandle::XYHandle::start`].
///
/// Stores all the state needed across the gesture (consumer callbacks,
/// store accessors, etc.). The `dom_node` / `flow_id` / `lib`
/// fields from TS are dropped — they only matter for the
/// `querySelector` DOM lookups that have been replaced by
/// [`IsValidParams::handle_below_pointer`].
///
/// Generic over the user-data type `D` so callbacks receive
/// `FinalConnectionState<InternalNode<D>>` rather than the type-erased
/// `FinalConnectionState<()>`.
pub struct StartConnectionParams<D: Clone + 'static = ()> {
    pub auto_pan_on_connect: bool,
    pub connection_mode: ConnectionMode,
    pub connection_radius: f64,
    /// Container bounds in viewport coords. The consumer supplies the
    /// pre-measured `MountedData::get_client_rect()`.
    pub container_bounds: Option<crate::types::geometry::Rect>,
    pub handle_id: Option<String>,
    pub node_id: String,
    pub is_target: bool,
    /// `Some(HandleType)` puts the gesture into edge-updater mode,
    /// where the resulting events flow to the `on_reconnect_end`
    /// callback as well as `on_connect_end`.
    pub edge_updater_type: Option<HandleType>,
    pub auto_pan_speed: f64,
    /// Pixel distance the pointer has to travel before a connection
    /// is considered started. Default in TS is 1.
    pub drag_threshold: f64,
    /// Optional validator predicate.
    pub is_valid_connection: Option<IsValidConnection>,
    pub on_connect_start: Option<Rc<dyn Fn(&PointerEventLike, OnConnectStartArgs)>>,
    pub on_connect: Option<Rc<dyn Fn(&Connection)>>,
    pub on_connect_end: Option<
        Rc<
            dyn Fn(
                &PointerEventLike,
                &crate::types::connection::FinalConnectionState<
                    crate::types::nodes::InternalNode<D>,
                >,
            ),
        >,
    >,
    pub on_reconnect_end: Option<
        Rc<
            dyn Fn(
                &PointerEventLike,
                &crate::types::connection::FinalConnectionState<
                    crate::types::nodes::InternalNode<D>,
                >,
            ),
        >,
    >,
}

/// Payload for the `on_connect_start` callback. Mirrors the TS
/// `OnConnectStartParams`.
#[derive(Debug, Clone)]
pub struct OnConnectStartArgs {
    pub node_id: String,
    pub handle_id: Option<String>,
    pub handle_type: HandleType,
}
