//! Port of `xyflow-react/src/store/index.ts`.
//!
//! Status: Phase 2 — implemented.
//!
//! Exposes:
//!
//! * [`RGraphStore`]   — `Copy`-cheap handle to the per-graph reactive
//!   state. Internally every field is a [`dioxus::prelude::Signal<T>`]
//!   so independent reads only re-run when their slice changes — the
//!   Rust analogue of Zustand's selector-with-equality pattern.
//! * [`RGraphStore::new`] — builds a fresh store from
//!   [`initial_state`].
//! * Action methods on `RGraphStore` (in [`actions`]):
//!   `set_nodes`, `set_edges`, `pan_by`, …
//!
//! The TS source pairs `getInitialState` and the methods in a single
//! Zustand factory; we split them into [`initial_state::initial_state`]
//! and [`actions`].

#![allow(clippy::module_name_repetitions)]

pub mod actions;
pub mod initial_state;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::Signal;

use rgraph_core::constants::AriaLabelConfig;
use rgraph_core::types::changes::{EdgeChange, NodeChange};
use rgraph_core::types::connection::{ConnectionLookup, ConnectionMode, ConnectionState};
use rgraph_core::types::edges::EdgeLookup;
use rgraph_core::types::geometry::{CoordinateExtent, Transform};
use rgraph_core::types::nodes::{
    InternalNode, NodeLookup, NodeOrigin, ParentLookup,
};
use rgraph_core::types::panzoom::PanZoomInstance;
use rgraph_core::types::viewport::{SelectionRect, SnapGrid, Viewport, ZIndexMode};
use rgraph_core::Promise;

use crate::types::component_props::{
    OnConnect, OnConnectEnd, OnConnectStart, OnError, OnMove, OnMoveEnd, OnMoveStart, OnViewportChange,
};
use crate::types::edges::{DefaultEdgeOptions, Edge};
use crate::types::general::{
    FitViewOptions, IsValidConnection, OnBeforeDelete, OnDelete, OnEdgesChange, OnEdgesDelete,
    OnNodesChange, OnNodesDelete, OnSelectionChangeFunc,
};
use crate::types::nodes::{Node, OnNodeDrag, SelectionDragHandler};
use crate::types::store::{
    ConnectionClickStartHandle, EdgeChangeMiddleware, NodeChangeMiddleware,
};
use crate::utils::general::PtrEq;

pub use initial_state::{initial_state, InitialStateParams};

/// Wrapper around the live `Box<dyn PanZoomInstance>`. We can't store a
/// `Box<dyn PanZoomInstance>` directly inside a `Signal` because the
/// trait's `&mut self` methods require interior mutability and the slot
/// must be cloneable (Dioxus `Signal::read()` clones when the storage
/// uses `Clone` semantics). `Rc<RefCell<…>>` satisfies both.
pub type SharedPanZoom = Rc<RefCell<Box<dyn PanZoomInstance>>>;

/// `Rc`-wrapped [`Promise<bool>`] for the deferred `fit_view` resolution.
/// `Promise<T>` is not Clone, so wrapping in `Rc` lets us share it
/// across multiple `Signal::read()` borrows.
pub type SharedFitViewResolver = Rc<Promise<bool>>;

// ---------------------------------------------------------------------------
// RGraphStore — live, signal-backed store.
// ---------------------------------------------------------------------------

/// The live store handle, mirror of the Zustand store in
/// `xyflow-react/src/store/index.ts`.
///
/// Each field is a [`dioxus::prelude::Signal<T>`]. The handle itself
/// is `Copy + Clone + PartialEq`, so it can be cheaply put into Dioxus
/// context, cloned into props, captured by `Callback`s, etc.
///
/// ## Reactivity model
///
/// Reading a single field (e.g. `store.nodes.read()`) subscribes the
/// current Dioxus scope only to changes of that signal — equivalent to
/// the TS pattern `useStore(s => s.nodes, shallow)` with one selected
/// slice. This is the Rust translation of Zustand's selector
/// equality-based memoisation, but with compile-time granularity
/// instead of runtime shallow comparison.
///
/// ## Generics
///
/// `N` and `E` are the node and edge data types (TS `NodeData` and
/// `EdgeData`). Both must be `Clone + PartialEq + 'static`; the
/// `PartialEq` bound comes from `adopt_user_nodes` (it diffs against
/// the cached node lookup) and the `Clone` bound from how the store
/// hands out node/edge snapshots to hooks.
pub struct RGraphStore<N: Clone + PartialEq + 'static = (), E: Clone + PartialEq + 'static = ()> {
    // -- Identity & sizing ----------------------------------------------------
    pub rf_id: Signal<String>,
    pub width: Signal<f64>,
    pub height: Signal<f64>,
    pub transform: Signal<Transform>,

    // -- Nodes & edges --------------------------------------------------------
    pub nodes: Signal<Vec<Node<N>>>,
    pub nodes_initialized: Signal<bool>,
    pub node_lookup: Signal<NodeLookup<N>>,
    pub parent_lookup: Signal<ParentLookup<N>>,
    pub edges: Signal<Vec<Edge<E>>>,
    pub edge_lookup: Signal<EdgeLookup<E>>,
    pub connection_lookup: Signal<ConnectionLookup>,

    pub on_nodes_change: Signal<Option<OnNodesChange<N>>>,
    pub on_edges_change: Signal<Option<OnEdgesChange<E>>>,
    pub has_default_nodes: Signal<bool>,
    pub has_default_edges: Signal<bool>,

    // -- DOM hookup ----------------------------------------------------------
    pub dom_node_id: Signal<Option<String>>,
    pub pane_dragging: Signal<bool>,
    pub no_pan_class_name: Signal<String>,
    pub pan_zoom: Signal<Option<SharedPanZoom>>,

    // -- Zoom / extents ------------------------------------------------------
    pub min_zoom: Signal<f64>,
    pub max_zoom: Signal<f64>,
    pub translate_extent: Signal<CoordinateExtent>,
    pub node_extent: Signal<CoordinateExtent>,
    pub node_origin: Signal<NodeOrigin>,
    pub node_drag_threshold: Signal<f64>,
    pub connection_drag_threshold: Signal<f64>,

    // -- Selection ------------------------------------------------------------
    pub nodes_selection_active: Signal<bool>,
    pub user_selection_active: Signal<bool>,
    pub user_selection_rect: Signal<Option<SelectionRect>>,

    // -- In-flight connection -----------------------------------------------
    pub connection: Signal<ConnectionState<InternalNode<N>>>,
    pub connection_mode: Signal<ConnectionMode>,
    pub connection_click_start_handle: Signal<Option<ConnectionClickStartHandle>>,

    // -- Snap-to-grid -------------------------------------------------------
    pub snap_to_grid: Signal<bool>,
    pub snap_grid: Signal<SnapGrid>,

    // -- Per-element flags --------------------------------------------------
    pub nodes_draggable: Signal<bool>,
    pub auto_pan_on_node_focus: Signal<bool>,
    pub nodes_connectable: Signal<bool>,
    pub nodes_focusable: Signal<bool>,
    pub edges_focusable: Signal<bool>,
    pub edges_reconnectable: Signal<bool>,
    pub elements_selectable: Signal<bool>,
    pub elevate_nodes_on_select: Signal<bool>,
    pub elevate_edges_on_select: Signal<bool>,
    pub select_nodes_on_drag: Signal<bool>,

    pub multi_selection_active: Signal<bool>,

    // -- Node-drag handlers -------------------------------------------------
    pub on_node_drag_start: Signal<Option<OnNodeDrag<N>>>,
    pub on_node_drag: Signal<Option<OnNodeDrag<N>>>,
    pub on_node_drag_stop: Signal<Option<OnNodeDrag<N>>>,

    // -- Selection-drag handlers --------------------------------------------
    pub on_selection_drag_start: Signal<Option<SelectionDragHandler<N>>>,
    pub on_selection_drag: Signal<Option<SelectionDragHandler<N>>>,
    pub on_selection_drag_stop: Signal<Option<SelectionDragHandler<N>>>,

    // -- Move handlers ------------------------------------------------------
    pub on_move_start: Signal<Option<OnMoveStart>>,
    pub on_move: Signal<Option<OnMove>>,
    pub on_move_end: Signal<Option<OnMoveEnd>>,

    // -- Connect handlers --------------------------------------------------
    pub on_connect: Signal<Option<OnConnect>>,
    pub on_connect_start: Signal<Option<OnConnectStart>>,
    pub on_connect_end: Signal<Option<OnConnectEnd<N>>>,
    pub on_click_connect_start: Signal<Option<OnConnectStart>>,
    pub on_click_connect_end: Signal<Option<OnConnectEnd<N>>>,

    pub connect_on_click: Signal<bool>,
    pub default_edge_options: Signal<Option<DefaultEdgeOptions>>,

    // -- Fit-view -----------------------------------------------------------
    pub fit_view_queued: Signal<bool>,
    pub fit_view_options: Signal<Option<PtrEq<FitViewOptions>>>,
    pub fit_view_resolver: Signal<Option<SharedFitViewResolver>>,

    // -- Delete handlers ----------------------------------------------------
    pub on_nodes_delete: Signal<Option<OnNodesDelete<N>>>,
    pub on_edges_delete: Signal<Option<OnEdgesDelete<E>>>,
    pub on_delete: Signal<Option<OnDelete<N, E>>>,
    pub on_error: Signal<Option<OnError>>,

    // -- Viewport change handlers ------------------------------------------
    pub on_viewport_change_start: Signal<Option<OnViewportChange>>,
    pub on_viewport_change: Signal<Option<OnViewportChange>>,
    pub on_viewport_change_end: Signal<Option<OnViewportChange>>,
    pub on_before_delete: Signal<Option<OnBeforeDelete<N, E>>>,

    pub on_selection_change_handlers: Signal<Vec<OnSelectionChangeFunc<N, E>>>,

    // -- Misc state ---------------------------------------------------------
    pub aria_live_message: Signal<String>,
    pub auto_pan_on_connect: Signal<bool>,
    pub auto_pan_on_node_drag: Signal<bool>,
    pub auto_pan_speed: Signal<f64>,
    pub connection_radius: Signal<f64>,

    pub is_valid_connection: Signal<Option<IsValidConnection<E>>>,

    pub lib: Signal<String>,
    pub debug: Signal<bool>,
    pub aria_label_config: Signal<AriaLabelConfig>,

    pub z_index_mode: Signal<ZIndexMode>,

    // Middleware maps. `u64` keys mirror the TS `symbol`-keyed maps —
    // each `experimental_useOnNodesChangeMiddleware` hook reserves a
    // fresh id at mount time.
    pub on_nodes_change_middleware_map: Signal<HashMap<u64, NodeChangeMiddleware<N>>>,
    pub on_edges_change_middleware_map: Signal<HashMap<u64, EdgeChangeMiddleware<E>>>,
}

// `Signal<T>` is `Copy`, so the whole `RGraphStore` is `Copy` too,
// which lets us pass it around like a token. We can't auto-derive
// `Copy` because of generic-parameter quirks, so we implement the
// derives manually.

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Copy for RGraphStore<N, E> {}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Clone for RGraphStore<N, E> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> PartialEq for RGraphStore<N, E> {
    fn eq(&self, other: &Self) -> bool {
        // Two stores are equal iff every signal handle refers to the
        // same underlying generational slot. Comparing one slot is
        // enough to identify the store (every field is created in the
        // same scope).
        self.rf_id == other.rf_id
    }
}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> RGraphStore<N, E> {
    /// Create a fresh `RGraphStore`, populating every signal with the
    /// values returned by [`initial_state::initial_state`].
    ///
    /// Mirrors the TS `createStore({…})`. Must be called inside a
    /// Dioxus scope (i.e. from within a component or a hook) because
    /// `Signal::new` allocates in the current scope's generational
    /// arena.
    pub fn new(params: InitialStateParams<N, E>) -> Self {
        let state = initial_state(params);

        RGraphStore {
            rf_id: Signal::new(state.rf_id),
            width: Signal::new(state.width),
            height: Signal::new(state.height),
            transform: Signal::new(state.transform),

            nodes: Signal::new(state.nodes),
            nodes_initialized: Signal::new(state.nodes_initialized),
            node_lookup: Signal::new(state.node_lookup),
            parent_lookup: Signal::new(state.parent_lookup),
            edges: Signal::new(state.edges),
            edge_lookup: Signal::new(state.edge_lookup),
            connection_lookup: Signal::new(state.connection_lookup),

            on_nodes_change: Signal::new(state.on_nodes_change),
            on_edges_change: Signal::new(state.on_edges_change),
            has_default_nodes: Signal::new(state.has_default_nodes),
            has_default_edges: Signal::new(state.has_default_edges),

            dom_node_id: Signal::new(state.dom_node_id),
            pane_dragging: Signal::new(state.pane_dragging),
            no_pan_class_name: Signal::new(state.no_pan_class_name),
            pan_zoom: Signal::new(None),

            min_zoom: Signal::new(state.min_zoom),
            max_zoom: Signal::new(state.max_zoom),
            translate_extent: Signal::new(state.translate_extent),
            node_extent: Signal::new(state.node_extent),
            node_origin: Signal::new(state.node_origin),
            node_drag_threshold: Signal::new(state.node_drag_threshold),
            connection_drag_threshold: Signal::new(state.connection_drag_threshold),

            nodes_selection_active: Signal::new(state.nodes_selection_active),
            user_selection_active: Signal::new(state.user_selection_active),
            user_selection_rect: Signal::new(state.user_selection_rect),

            connection: Signal::new(state.connection),
            connection_mode: Signal::new(state.connection_mode),
            connection_click_start_handle: Signal::new(state.connection_click_start_handle),

            snap_to_grid: Signal::new(state.snap_to_grid),
            snap_grid: Signal::new(state.snap_grid),

            nodes_draggable: Signal::new(state.nodes_draggable),
            auto_pan_on_node_focus: Signal::new(state.auto_pan_on_node_focus),
            nodes_connectable: Signal::new(state.nodes_connectable),
            nodes_focusable: Signal::new(state.nodes_focusable),
            edges_focusable: Signal::new(state.edges_focusable),
            edges_reconnectable: Signal::new(state.edges_reconnectable),
            elements_selectable: Signal::new(state.elements_selectable),
            elevate_nodes_on_select: Signal::new(state.elevate_nodes_on_select),
            elevate_edges_on_select: Signal::new(state.elevate_edges_on_select),
            select_nodes_on_drag: Signal::new(state.select_nodes_on_drag),

            multi_selection_active: Signal::new(state.multi_selection_active),

            on_node_drag_start: Signal::new(state.on_node_drag_start),
            on_node_drag: Signal::new(state.on_node_drag),
            on_node_drag_stop: Signal::new(state.on_node_drag_stop),

            on_selection_drag_start: Signal::new(state.on_selection_drag_start),
            on_selection_drag: Signal::new(state.on_selection_drag),
            on_selection_drag_stop: Signal::new(state.on_selection_drag_stop),

            on_move_start: Signal::new(state.on_move_start),
            on_move: Signal::new(state.on_move),
            on_move_end: Signal::new(state.on_move_end),

            on_connect: Signal::new(state.on_connect),
            on_connect_start: Signal::new(state.on_connect_start),
            on_connect_end: Signal::new(state.on_connect_end),
            on_click_connect_start: Signal::new(state.on_click_connect_start),
            on_click_connect_end: Signal::new(state.on_click_connect_end),

            connect_on_click: Signal::new(state.connect_on_click),
            default_edge_options: Signal::new(state.default_edge_options),

            fit_view_queued: Signal::new(state.fit_view_queued),
            fit_view_options: Signal::new(state.fit_view_options),
            fit_view_resolver: Signal::new(None),

            on_nodes_delete: Signal::new(state.on_nodes_delete),
            on_edges_delete: Signal::new(state.on_edges_delete),
            on_delete: Signal::new(state.on_delete),
            on_error: Signal::new(state.on_error),

            on_viewport_change_start: Signal::new(state.on_viewport_change_start),
            on_viewport_change: Signal::new(state.on_viewport_change),
            on_viewport_change_end: Signal::new(state.on_viewport_change_end),
            on_before_delete: Signal::new(state.on_before_delete),

            on_selection_change_handlers: Signal::new(state.on_selection_change_handlers),

            aria_live_message: Signal::new(state.aria_live_message),
            auto_pan_on_connect: Signal::new(state.auto_pan_on_connect),
            auto_pan_on_node_drag: Signal::new(state.auto_pan_on_node_drag),
            auto_pan_speed: Signal::new(state.auto_pan_speed),
            connection_radius: Signal::new(state.connection_radius),

            is_valid_connection: Signal::new(state.is_valid_connection),

            lib: Signal::new(state.lib),
            debug: Signal::new(state.debug),
            aria_label_config: Signal::new(state.aria_label_config),

            z_index_mode: Signal::new(state.z_index_mode),

            on_nodes_change_middleware_map: Signal::new(state.on_nodes_change_middleware_map),
            on_edges_change_middleware_map: Signal::new(state.on_edges_change_middleware_map),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::*;

    /// Spin up a tiny VirtualDom, build an `RGraphStore` inside the
    /// root scope, and run a few sanity checks. Building `Signal`s
    /// requires an active runtime scope; this test confirms the
    /// constructor and the action methods work end-to-end.
    #[test]
    fn store_constructs_in_a_dioxus_scope() {
        use dioxus_signals::ReadableExt;

        fn root() -> Element {
            let store: RGraphStore<(), ()> =
                RGraphStore::new(InitialStateParams::default());
            // Default zoom is 0.5..2.0, default extent is (0,0)..
            assert_eq!(*store.min_zoom.peek(), 0.5);
            assert_eq!(*store.max_zoom.peek(), 2.0);
            assert!(!*store.elements_selectable.peek() == false);
            assert_eq!(store.nodes.peek().len(), 0);

            // Action smoke test: set a node, expect the lookup to grow.
            store.set_nodes(vec![Node::<()>::minimal("n1", 0.0, 0.0)]);
            assert_eq!(store.nodes.peek().len(), 1);
            assert!(store.node_lookup.peek().contains_key("n1"));

            // Add an edge.
            store.set_edges(vec![Edge::<()>::minimal("e1", "a", "b")]);
            assert_eq!(store.edges.peek().len(), 1);
            assert!(store.edge_lookup.peek().contains_key("e1"));

            // Zoom mutators.
            store.set_min_zoom(0.25);
            assert_eq!(*store.min_zoom.peek(), 0.25);
            store.set_max_zoom(4.0);
            assert_eq!(*store.max_zoom.peek(), 4.0);

            // Pan-by with no PanZoom returns false.
            assert!(!store.pan_by(rgraph_core::types::geometry::XYPosition::new(10.0, 0.0)));

            // Cancel the connection — the snapshot resets to NoConnection.
            store.cancel_connection();

            rsx! { div {} }
        }

        let mut vdom = VirtualDom::new(root);
        // Drive an initial render — root() must execute under a scope.
        let _muts = vdom.rebuild_to_vec();
    }

    /// Constructing two distinct stores in the same scope yields two
    /// distinct handles (different generational signal ids).
    #[test]
    fn two_stores_are_distinct() {
        fn root() -> Element {
            let a: RGraphStore<(), ()> = RGraphStore::new(InitialStateParams::default());
            let b: RGraphStore<(), ()> = RGraphStore::new(InitialStateParams::default());
            assert!(a != b);
            rsx! { div {} }
        }
        let mut vdom = VirtualDom::new(root);
        let _muts = vdom.rebuild_to_vec();
    }
}
