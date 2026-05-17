//! Port of `xyflow-react/src/types/store.ts`.
//!
//! Status: Phase 1 — implemented (POD contract).
//!
//! Defines:
//!
//! * [`RGraphStoreState`]   — the data half (every field of the React
//!   `ReactFlowStore` interface). In Phase 2 each of these fields will
//!   be wrapped in a [`dioxus::prelude::Signal`] inside the live
//!   [`crate::store::RGraphStore`] struct so reads are granularly
//!   reactive.
//! * [`RGraphStoreActions`] — the action half (every method attached to
//!   the store in `store/index.ts`). The action fields are
//!   [`dioxus::prelude::Callback`]s for cheap `Clone + PartialEq`.
//! * [`RGraphState`]        — the combined "state + actions" view that
//!   matches the TS `ReactFlowState`.
//!
//! Phase 2's `store/mod.rs` will provide the actual runtime store and
//! wire each callback to a method on it.

#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;

use dioxus::prelude::Callback;

use rgraph_core::constants::AriaLabelConfig;
use rgraph_core::types::changes::{EdgeChange, NodeChange};
use rgraph_core::types::connection::{
    ConnectionLookup, ConnectionMode, ConnectionState,
};
use rgraph_core::types::edges::EdgeLookup;
use rgraph_core::types::geometry::{CoordinateExtent, Transform, XYPosition};
use rgraph_core::types::handles::{Handle, HandleType};
use rgraph_core::types::nodes::{
    InternalNodeUpdate, NodeLookup, NodeOrigin, ParentLookup,
};
use rgraph_core::types::panzoom::PanZoomInstance;
use rgraph_core::types::viewport::SnapGrid;
use rgraph_core::types::viewport::Viewport;

use crate::types::edges::{DefaultEdgeOptions, Edge};
use crate::types::component_props::{
    OnConnect, OnConnectEnd, OnConnectStart, OnError, OnMove, OnMoveEnd, OnMoveStart,
    OnViewportChange,
};
use crate::types::general::{
    FitViewOptions, IsValidConnection, OnDelete, OnEdgesChange, OnEdgesDelete, OnNodesChange,
    OnNodesDelete, OnSelectionChangeFunc, UnselectNodesAndEdgesParams,
};
use crate::types::instance::{NodePartial, SetEdgesArg, SetNodesArg};
use crate::types::nodes::{InternalNode, Node, OnNodeDrag, SelectionDragHandler};

// ---------------------------------------------------------------------------
// `ReactFlowStore` — data half.
// ---------------------------------------------------------------------------

/// Pre-handle-click handle (TS
/// `(Pick<Handle, 'nodeId' | 'id'> & Required<Pick<Handle, 'type'>>) | null`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionClickStartHandle {
    pub node_id: String,
    pub id: Option<String>,
    pub type_: HandleType,
}

/// Trait object for the [`PanZoomInstance`] held in the store. We use
/// `dyn` to keep `RGraphStoreState` generic-free, mirroring the TS
/// `PanZoomInstance | null`.
pub type DynPanZoom = Box<dyn PanZoomInstance>;

/// Middleware function applied to a batch of node changes before they
/// are committed.
///
/// TS `(changes: NodeChange<NodeType>[]) => NodeChange<NodeType>[]`.
/// Stored boxed for `Send + Sync` parity with the store maps; can be
/// upgraded to a typed callback once Phase 2 lands.
pub type NodeChangeMiddleware<D = ()> = std::sync::Arc<dyn Fn(Vec<NodeChange<D>>) -> Vec<NodeChange<D>>>;

/// Edge counterpart of [`NodeChangeMiddleware`].
pub type EdgeChangeMiddleware<D = ()> = std::sync::Arc<dyn Fn(Vec<EdgeChange<D>>) -> Vec<EdgeChange<D>>>;

/// `RGraphStoreState<N, E>` — the data half of the store.
///
/// Mirrors the TS `ReactFlowStore<NodeType, EdgeType>` (lines 54–160 of
/// `types/store.ts`). Every field is a plain owned value; in Phase 2,
/// `crate::store::RGraphStore<N, E>` will wrap each field in a
/// `dioxus::prelude::Signal<T>` so individual reads are granularly
/// reactive. We keep this POD struct as the documentation contract so
/// downstream code (hooks, components) can refer to a single canonical
/// schema.
pub struct RGraphStoreState<N: Clone + 'static = (), E: Clone + 'static = ()> {
    pub rf_id: String,
    pub width: f64,
    pub height: f64,
    pub transform: Transform,

    pub nodes: Vec<Node<N>>,
    pub nodes_initialized: bool,
    pub node_lookup: NodeLookup<N>,
    pub parent_lookup: ParentLookup<N>,
    pub edges: Vec<Edge<E>>,
    pub edge_lookup: EdgeLookup<E>,
    pub connection_lookup: ConnectionLookup,

    pub on_nodes_change: Option<OnNodesChange<N>>,
    pub on_edges_change: Option<OnEdgesChange<E>>,
    pub has_default_nodes: bool,
    pub has_default_edges: bool,

    /// The HTML id of the host `<div>`. Mirrors TS `domNode: HTMLDivElement | null`.
    /// In Dioxus the wrapper carries an `id="…"`; we store that id so
    /// hooks can resolve the DOM element via `use_eval` when needed.
    pub dom_node_id: Option<String>,

    pub pane_dragging: bool,
    pub no_pan_class_name: String,
    pub pan_zoom: Option<DynPanZoom>,

    pub min_zoom: f64,
    pub max_zoom: f64,
    pub translate_extent: CoordinateExtent,
    pub node_extent: CoordinateExtent,
    pub node_origin: NodeOrigin,
    pub node_drag_threshold: f64,
    pub connection_drag_threshold: f64,

    pub nodes_selection_active: bool,
    pub user_selection_active: bool,
    pub user_selection_rect: Option<rgraph_core::types::viewport::SelectionRect>,

    pub connection: ConnectionState<InternalNode<N>>,
    pub connection_mode: ConnectionMode,
    pub connection_click_start_handle: Option<ConnectionClickStartHandle>,

    pub snap_to_grid: bool,
    pub snap_grid: SnapGrid,

    pub nodes_draggable: bool,
    pub auto_pan_on_node_focus: bool,
    pub nodes_connectable: bool,
    pub nodes_focusable: bool,
    pub edges_focusable: bool,
    pub edges_reconnectable: bool,
    pub elements_selectable: bool,
    pub elevate_nodes_on_select: bool,
    pub elevate_edges_on_select: bool,
    pub select_nodes_on_drag: bool,

    pub multi_selection_active: bool,

    pub on_node_drag_start: Option<OnNodeDrag<N>>,
    pub on_node_drag: Option<OnNodeDrag<N>>,
    pub on_node_drag_stop: Option<OnNodeDrag<N>>,

    pub on_selection_drag_start: Option<SelectionDragHandler<N>>,
    pub on_selection_drag: Option<SelectionDragHandler<N>>,
    pub on_selection_drag_stop: Option<SelectionDragHandler<N>>,

    pub on_move_start: Option<OnMoveStart>,
    pub on_move: Option<OnMove>,
    pub on_move_end: Option<OnMoveEnd>,

    pub on_connect: Option<OnConnect>,
    pub on_connect_start: Option<OnConnectStart>,
    pub on_connect_end: Option<OnConnectEnd<N>>,
    pub on_click_connect_start: Option<OnConnectStart>,
    pub on_click_connect_end: Option<OnConnectEnd<N>>,

    pub connect_on_click: bool,
    pub default_edge_options: Option<DefaultEdgeOptions>,

    pub fit_view_queued: bool,
    /// `Arc`-wrapped because [`FitViewOptions`] contains a non-Clone
    /// callback. The store updates this slot when `fit_view_queued`
    /// flips on.
    pub fit_view_options: Option<crate::utils::general::PtrEq<FitViewOptions>>,
    /// In TS this is `ReturnType<typeof withResolvers<boolean>>`. The
    /// Rust port uses [`rgraph_core::Promise`] for the same purpose
    /// (phase 4 will materialise it).
    pub fit_view_resolver: Option<rgraph_core::Promise<bool>>,

    pub on_nodes_delete: Option<OnNodesDelete<N>>,
    pub on_edges_delete: Option<OnEdgesDelete<E>>,
    pub on_delete: Option<OnDelete<N, E>>,
    pub on_error: Option<OnError>,

    pub on_viewport_change_start: Option<OnViewportChange>,
    pub on_viewport_change: Option<OnViewportChange>,
    pub on_viewport_change_end: Option<OnViewportChange>,
    pub on_before_delete: Option<crate::types::general::OnBeforeDelete<N, E>>,

    pub on_selection_change_handlers: Vec<OnSelectionChangeFunc<N, E>>,

    pub aria_live_message: String,
    pub auto_pan_on_connect: bool,
    pub auto_pan_on_node_drag: bool,
    pub auto_pan_speed: f64,
    pub connection_radius: f64,

    pub is_valid_connection: Option<IsValidConnection<E>>,

    /// Library identifier — `"react"` in the JS source; we use
    /// `"dioxus"` here.
    pub lib: String,
    pub debug: bool,
    pub aria_label_config: AriaLabelConfig,

    pub z_index_mode: rgraph_core::types::viewport::ZIndexMode,

    pub on_nodes_change_middleware_map: HashMap<u64, NodeChangeMiddleware<N>>,
    pub on_edges_change_middleware_map: HashMap<u64, EdgeChangeMiddleware<E>>,
}

// ---------------------------------------------------------------------------
// `ReactFlowActions` — method half.
// ---------------------------------------------------------------------------

/// Args for [`RGraphStoreActions::update_node_internals`].
pub struct UpdateNodeInternalsArgs {
    pub updates: HashMap<String, InternalNodeUpdate>,
    pub trigger_fit_view: bool,
}

/// Args for [`RGraphStoreActions::update_node_positions`]. Mirrors the
/// TS `UpdateNodePositions = (nodeDragItems, dragging?) => void`.
pub struct UpdateNodePositionsArgs {
    pub drag_items: HashMap<String, rgraph_core::types::nodes::NodeDragItem>,
    pub dragging: bool,
}

/// Args for [`RGraphStoreActions::set_center`]. Mirrors the TS
/// `SetCenter = (x, y, options?) => Promise<boolean>`.
pub struct SetCenterCallArgs {
    pub x: f64,
    pub y: f64,
    pub options: Option<rgraph_core::types::viewport::SetCenterOptions>,
}

/// Args for [`RGraphStoreActions::pan_by`]. Mirrors the TS `PanBy =
/// (delta) => Promise<boolean>`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PanByArgs {
    pub delta: XYPosition,
}

/// `RGraphStoreActions<N, E>` — the method half of the store.
///
/// Mirrors the TS `ReactFlowActions<NodeType, EdgeType>` (lines 162–183
/// of `types/store.ts`). Every method is a [`Callback`] so the bundle
/// is cheaply cloneable and `PartialEq`.
#[derive(Clone, PartialEq)]
pub struct RGraphStoreActions<N: Clone + 'static = (), E: Clone + 'static = ()> {
    pub set_nodes: Callback<Vec<Node<N>>>,
    pub set_edges: Callback<Vec<Edge<E>>>,
    pub set_default_nodes_and_edges: Callback<SetDefaultArgs<N, E>>,
    pub update_node_internals: Callback<UpdateNodeInternalsArgs>,
    pub update_node_positions: Callback<UpdateNodePositionsArgs>,
    pub reset_selected_elements: Callback<()>,
    pub unselect_nodes_and_edges: Callback<UnselectNodesAndEdgesParams<N, E>>,
    pub add_selected_nodes: Callback<Vec<String>>,
    pub add_selected_edges: Callback<Vec<String>>,
    pub set_min_zoom: Callback<f64>,
    pub set_max_zoom: Callback<f64>,
    pub set_translate_extent: Callback<CoordinateExtent>,
    pub set_node_extent: Callback<CoordinateExtent>,
    pub cancel_connection: Callback<()>,
    pub update_connection: Callback<ConnectionState<InternalNode<N>>>,
    pub reset: Callback<()>,
    pub trigger_node_changes: Callback<Vec<NodeChange<N>>>,
    pub trigger_edge_changes: Callback<Vec<EdgeChange<E>>>,
    pub pan_by: Callback<PanByArgs, bool>,
    pub set_center: Callback<SetCenterCallArgs, bool>,
}

/// Args for `set_default_nodes_and_edges`. Both fields are optional;
/// `None` means "leave the corresponding default slot untouched".
pub struct SetDefaultArgs<N: Clone = (), E: Clone = ()> {
    pub nodes: Option<Vec<Node<N>>>,
    pub edges: Option<Vec<Edge<E>>>,
}

// ---------------------------------------------------------------------------
// RGraphState — combined view.
// ---------------------------------------------------------------------------

/// Combined "state + actions" view (TS `ReactFlowState =
/// ReactFlowStore & ReactFlowActions`).
///
/// Phase 2 will produce one of these via `RGraphStore::snapshot()` or
/// equivalent; downstream `use_store()` hooks select fields off this
/// view.
pub struct RGraphState<N: Clone + 'static = (), E: Clone + 'static = ()> {
    pub state: RGraphStoreState<N, E>,
    pub actions: RGraphStoreActions<N, E>,
}

// ---------------------------------------------------------------------------
// Re-export the user-facing `NodePartial` so downstream `update_node`
// users have one canonical place to import from.
// ---------------------------------------------------------------------------

pub use crate::types::instance::NodePartial as StoreNodePartial;
pub use crate::types::instance::SetEdgesArg as StoreSetEdgesArg;
pub use crate::types::instance::SetNodesArg as StoreSetNodesArg;
