//! Port of `xyflow-react/src/types/component-props.ts`.
//!
//! Status: Phase 1 — implemented (field-shape only; the actual
//! `#[derive(dioxus::prelude::Props)]` derive is deferred to Phase 7
//! once the top-level `<RGraph>` component lands and we settle on a
//! concrete `Node<NodeData, …>` default for the public API).
//!
//! `RGraphProps<N, E>` mirrors the TS `ReactFlowProps`. The struct
//! aggregates **every** public prop accepted by `<ReactFlow>`. For
//! Phase 1 it is a plain `Clone + Default` struct documenting the
//! contract; Phase 2 (store) and Phase 7 (top-level component) will
//! consume these fields directly.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::{Callback, Event};

use rgraph_core::types::connection::{Connection, ConnectionMode, FinalConnectionState, OnConnectStartParams};
use rgraph_core::types::edges::ConnectionLineType;
use rgraph_core::types::geometry::{CoordinateExtent, XYPosition};
use rgraph_core::types::nodes::NodeOrigin;
use rgraph_core::types::panzoom::PanOnDrag;
use rgraph_core::types::viewport::{
    ColorMode, KeyCode, PanOnScrollMode, PanelPosition, ProOptions, SelectionMode, SnapGrid, Viewport,
    ZIndexMode,
};
use rgraph_core::AriaLabelConfig;

use crate::types::edges::{
    ConnectionLineComponent, DefaultEdgeOptions, EdgeMouseHandler, OnReconnect, OnReconnectEnd, OnReconnectStart,
};
use crate::types::general::{
    EdgeTypes, FitViewOptions, IsValidConnection, NodeTypes, OnBeforeDelete, OnDelete, OnEdgesChange, OnEdgesDelete,
    OnInit, OnNodesChange, OnNodesDelete, OnSelectionChangeFunc,
};
use crate::types::instance::RGraphInstance;
use crate::types::nodes::{InternalNode, MouseData, NodeMouseHandler, OnNodeDrag, SelectionDragHandler};

// ---------------------------------------------------------------------------
// Callback aliases that the React port adds on top of `rgraph_core`.
// ---------------------------------------------------------------------------

/// Fired with a successfully-completed connection (TS `OnConnect`).
pub type OnConnect = Callback<Connection>;

/// Fired when the user begins dragging a connection line (TS `OnConnectStart`).
pub type OnConnectStart = Callback<OnConnectStartCallbackArgs>;

#[derive(Debug, Clone)]
pub struct OnConnectStartCallbackArgs {
    pub event: std::rc::Rc<Event<MouseData>>,
    pub params: OnConnectStartParams,
}

/// Fired when the user releases the pointer regardless of whether a
/// connection was made (TS `OnConnectEnd`).
pub type OnConnectEnd<N = ()> = Callback<OnConnectEndCallbackArgs<N>>;

#[derive(Debug, Clone)]
pub struct OnConnectEndCallbackArgs<N: Clone = ()> {
    pub event: std::rc::Rc<Event<MouseData>>,
    pub connection_state: FinalConnectionState<InternalNode<N>>,
}

/// Fired while the user pans or zooms the viewport (TS `OnMove`).
pub type OnMove = Callback<OnMoveCallbackArgs>;

/// Fired when the user begins to pan or zoom (TS `OnMoveStart`).
pub type OnMoveStart = Callback<OnMoveCallbackArgs>;

/// Fired when panning/zooming stops (TS `OnMoveEnd`). `event` is `None`
/// for programmatic viewport changes.
pub type OnMoveEnd = Callback<OnMoveCallbackArgs>;

#[derive(Debug, Clone)]
pub struct OnMoveCallbackArgs {
    pub event: Option<std::rc::Rc<Event<MouseData>>>,
    pub viewport: Viewport,
}

/// Fired by the store when the user controls the viewport prop
/// (TS `onViewportChange`).
pub type OnViewportChange = Callback<Viewport>;

/// Wheel-event handler for the pane.
pub type OnPaneScroll = Callback<std::rc::Rc<Event<dioxus::events::WheelData>>>;

/// Click/context-menu/mouse handler for the pane.
pub type PaneMouseHandler = Callback<std::rc::Rc<Event<MouseData>>>;

/// Selection-start / selection-end handler (no node payload, just the event).
pub type SelectionRectHandler = Callback<std::rc::Rc<Event<MouseData>>>;

/// Right-click on a selection (TS `onSelectionContextMenu`).
pub type SelectionContextMenuHandler<N = ()> = Callback<SelectionContextMenuArgs<N>>;

#[derive(Debug, Clone)]
pub struct SelectionContextMenuArgs<N: Clone = ()> {
    pub event: std::rc::Rc<Event<MouseData>>,
    pub nodes: Vec<crate::types::nodes::Node<N>>,
}

/// Fired by the store when a non-fatal internal error occurs.
///
/// Mirrors the TS `OnError = (id: string, message: string) => void`.
pub type OnError = Callback<OnErrorArgs>;

#[derive(Debug, Clone, PartialEq)]
pub struct OnErrorArgs {
    pub id: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// RGraphProps — the (very large) prop bag.
// ---------------------------------------------------------------------------

/// `RGraphProps` — every prop accepted by the top-level [`crate::container::rgraph::mod_RGraph`]
/// component.
///
/// Mirrors the TS `ReactFlowProps<NodeType, EdgeType>`. Field names use
/// `snake_case` (idiomatic Rust). The `Omit<HTMLAttributes<HTMLDivElement>, 'onError'>`
/// pass-through is modelled as the `dom_attributes` + `class_name` +
/// `style` + `id` + `on_scroll` quintet exposed directly here. Per-event
/// listeners (`on_mouse_down` on the wrapper etc.) can be added later
/// via the same `dom_attributes` escape hatch.
///
/// All optional fields default to `None`; required defaults from the TS
/// source (e.g. `minZoom = 0.5`) are applied by the consuming component
/// rather than baked into this struct — that mirrors what `<ReactFlow>`
/// does in `xyflow-react/src/container/ReactFlow/index.tsx`.
#[derive(Clone, PartialEq)]
pub struct RGraphProps<N: Clone + PartialEq + 'static = (), E: Clone + 'static = ()> {
    // -- Nodes and edges -------------------------------------------------------
    pub nodes: Option<Vec<crate::types::nodes::Node<N>>>,
    pub edges: Option<Vec<crate::types::edges::Edge<E>>>,
    pub default_nodes: Option<Vec<crate::types::nodes::Node<N>>>,
    pub default_edges: Option<Vec<crate::types::edges::Edge<E>>>,
    pub default_edge_options: Option<DefaultEdgeOptions>,

    // -- Node mouse handlers ---------------------------------------------------
    pub on_node_click: Option<NodeMouseHandler<N>>,
    pub on_node_double_click: Option<NodeMouseHandler<N>>,
    pub on_node_mouse_enter: Option<NodeMouseHandler<N>>,
    pub on_node_mouse_move: Option<NodeMouseHandler<N>>,
    pub on_node_mouse_leave: Option<NodeMouseHandler<N>>,
    pub on_node_context_menu: Option<NodeMouseHandler<N>>,
    pub on_node_drag_start: Option<OnNodeDrag<N>>,
    pub on_node_drag: Option<OnNodeDrag<N>>,
    pub on_node_drag_stop: Option<OnNodeDrag<N>>,

    // -- Edge mouse handlers ---------------------------------------------------
    pub on_edge_click: Option<EdgeMouseHandler<E>>,
    pub on_edge_context_menu: Option<EdgeMouseHandler<E>>,
    pub on_edge_mouse_enter: Option<EdgeMouseHandler<E>>,
    pub on_edge_mouse_move: Option<EdgeMouseHandler<E>>,
    pub on_edge_mouse_leave: Option<EdgeMouseHandler<E>>,
    pub on_edge_double_click: Option<EdgeMouseHandler<E>>,
    pub on_reconnect: Option<OnReconnect<E>>,
    pub on_reconnect_start: Option<OnReconnectStart<E>>,
    pub on_reconnect_end: Option<OnReconnectEnd<E, N>>,

    // -- Change handlers -------------------------------------------------------
    pub on_nodes_change: Option<OnNodesChange<N>>,
    pub on_edges_change: Option<OnEdgesChange<E>>,
    pub on_nodes_delete: Option<OnNodesDelete<N>>,
    pub on_edges_delete: Option<OnEdgesDelete<E>>,
    pub on_delete: Option<OnDelete<N, E>>,
    pub on_before_delete: Option<OnBeforeDelete<N, E>>,

    // -- Selection handlers ----------------------------------------------------
    pub on_selection_drag_start: Option<SelectionDragHandler<N>>,
    pub on_selection_drag: Option<SelectionDragHandler<N>>,
    pub on_selection_drag_stop: Option<SelectionDragHandler<N>>,
    pub on_selection_start: Option<SelectionRectHandler>,
    pub on_selection_end: Option<SelectionRectHandler>,
    pub on_selection_context_menu: Option<SelectionContextMenuHandler<N>>,
    pub on_selection_change: Option<OnSelectionChangeFunc<N, E>>,

    // -- Connection handlers ---------------------------------------------------
    pub on_connect: Option<OnConnect>,
    pub on_connect_start: Option<OnConnectStart>,
    pub on_connect_end: Option<OnConnectEnd<N>>,
    pub on_click_connect_start: Option<OnConnectStart>,
    pub on_click_connect_end: Option<OnConnectEnd<N>>,

    // -- Init / viewport handlers ---------------------------------------------
    pub on_init: Option<OnInit<RGraphInstance<N, E>>>,
    pub on_move: Option<OnMove>,
    pub on_move_start: Option<OnMoveStart>,
    pub on_move_end: Option<OnMoveEnd>,
    pub on_viewport_change: Option<OnViewportChange>,

    // -- Pane handlers ---------------------------------------------------------
    pub on_pane_scroll: Option<OnPaneScroll>,
    pub on_pane_click: Option<PaneMouseHandler>,
    pub on_pane_context_menu: Option<PaneMouseHandler>,
    pub on_pane_mouse_enter: Option<PaneMouseHandler>,
    pub on_pane_mouse_move: Option<PaneMouseHandler>,
    pub on_pane_mouse_leave: Option<PaneMouseHandler>,
    pub pane_click_distance: Option<f64>,
    pub node_click_distance: Option<f64>,

    // -- Component registries --------------------------------------------------
    pub node_types: Option<NodeTypes<N>>,
    pub edge_types: Option<EdgeTypes<E>>,

    // -- Connection line ------------------------------------------------------
    pub connection_line_type: Option<ConnectionLineType>,
    pub connection_line_style: Option<String>,
    pub connection_line_component: Option<ConnectionLineComponent<N>>,
    pub connection_line_container_style: Option<String>,
    pub connection_mode: Option<ConnectionMode>,

    // -- Keyboard --------------------------------------------------------------
    pub delete_key_code: Option<KeyCode>,
    pub selection_key_code: Option<KeyCode>,
    pub selection_on_drag: Option<bool>,
    pub selection_mode: Option<SelectionMode>,
    pub pan_activation_key_code: Option<KeyCode>,
    pub multi_selection_key_code: Option<KeyCode>,
    pub zoom_activation_key_code: Option<KeyCode>,

    // -- Snap-to-grid ---------------------------------------------------------
    pub snap_to_grid: Option<bool>,
    pub snap_grid: Option<SnapGrid>,

    // -- Rendering optimisation -----------------------------------------------
    pub only_render_visible_elements: Option<bool>,

    // -- Per-element flags ----------------------------------------------------
    pub nodes_draggable: Option<bool>,
    pub auto_pan_on_node_focus: Option<bool>,
    pub nodes_connectable: Option<bool>,
    pub nodes_focusable: Option<bool>,
    pub node_origin: Option<NodeOrigin>,
    pub edges_focusable: Option<bool>,
    pub edges_reconnectable: Option<bool>,
    pub elements_selectable: Option<bool>,
    pub select_nodes_on_drag: Option<bool>,
    pub pan_on_drag: Option<PanOnDrag>,

    // -- Zoom / pan -----------------------------------------------------------
    pub min_zoom: Option<f64>,
    pub max_zoom: Option<f64>,
    pub viewport: Option<Viewport>,
    pub default_viewport: Option<Viewport>,
    pub translate_extent: Option<CoordinateExtent>,
    pub prevent_scrolling: Option<bool>,
    pub node_extent: Option<CoordinateExtent>,
    pub default_marker_color: Option<String>,
    pub zoom_on_scroll: Option<bool>,
    pub zoom_on_pinch: Option<bool>,
    pub pan_on_scroll: Option<bool>,
    pub pan_on_scroll_speed: Option<f64>,
    pub pan_on_scroll_mode: Option<PanOnScrollMode>,
    pub zoom_on_double_click: Option<bool>,

    // -- Reconnect ------------------------------------------------------------
    pub reconnect_radius: Option<f64>,

    // -- Class names ----------------------------------------------------------
    pub no_drag_class_name: Option<String>,
    pub no_wheel_class_name: Option<String>,
    pub no_pan_class_name: Option<String>,

    // -- Fit-view ------------------------------------------------------------
    pub fit_view: Option<bool>,
    /// Wrapped in [`PtrEq`] because [`FitViewOptions`] contains a
    /// non-`Clone` boxed `EaseFn` callback; comparison is pointer-based.
    pub fit_view_options: Option<crate::utils::general::PtrEq<FitViewOptions>>,

    // -- Connect-on-click ----------------------------------------------------
    pub connect_on_click: Option<bool>,

    // -- Attribution / pro ---------------------------------------------------
    pub attribution_position: Option<PanelPosition>,
    pub pro_options: Option<ProOptions>,

    // -- Z-index handling ----------------------------------------------------
    pub elevate_nodes_on_select: Option<bool>,
    pub elevate_edges_on_select: Option<bool>,

    // -- A11y ----------------------------------------------------------------
    pub disable_keyboard_a11y: Option<bool>,
    pub aria_label_config: Option<AriaLabelConfig>,

    // -- Auto-pan ------------------------------------------------------------
    pub auto_pan_on_node_drag: Option<bool>,
    pub auto_pan_on_connect: Option<bool>,
    pub auto_pan_on_selection: Option<bool>,
    pub auto_pan_speed: Option<f64>,

    // -- Connection ----------------------------------------------------------
    pub connection_radius: Option<f64>,

    // -- Error handling ------------------------------------------------------
    pub on_error: Option<OnError>,

    // -- Connection validation ----------------------------------------------
    pub is_valid_connection: Option<IsValidConnection<E>>,

    // -- Thresholds ---------------------------------------------------------
    pub node_drag_threshold: Option<f64>,
    pub connection_drag_threshold: Option<f64>,

    // -- Sizing -------------------------------------------------------------
    pub width: Option<f64>,
    pub height: Option<f64>,

    // -- Color mode ---------------------------------------------------------
    pub color_mode: Option<ColorMode>,

    // -- Misc ---------------------------------------------------------------
    pub debug: Option<bool>,
    pub z_index_mode: Option<ZIndexMode>,

    // -- DOM pass-through --------------------------------------------------
    /// HTML `id` attribute for the wrapper `<div>`. Also doubles as the
    /// React Flow `rfId` when set, matching TS behaviour.
    pub id: Option<String>,
    /// `style="…"` snippet applied to the outer wrapper `<div>`.
    pub style: Option<String>,
    /// Class name(s) applied to the outer wrapper.
    pub class_name: Option<String>,
    /// `onScroll` handler for the wrapper (TS keeps this distinct from
    /// `on_pane_scroll` because it watches accidental focus-induced scrolls).
    pub on_scroll: Option<OnPaneScroll>,
    /// Free-form HTML attributes appended verbatim to the wrapper.
    pub dom_attributes: std::collections::HashMap<String, String>,

    // -- Children (Dioxus equivalent of TS `children: ReactNode`) -----------
    /// Children rendered inside the `<RGraph>` viewport (custom UI,
    /// `<Background>`, `<Controls>`, etc.). Optional because the TS
    /// `children` prop defaults to undefined.
    pub children: Option<dioxus::prelude::Element>,
}

impl<N: Clone + PartialEq + 'static, E: Clone + 'static> Default for RGraphProps<N, E> {
    fn default() -> Self {
        RGraphProps {
            nodes: None,
            edges: None,
            default_nodes: None,
            default_edges: None,
            default_edge_options: None,
            on_node_click: None,
            on_node_double_click: None,
            on_node_mouse_enter: None,
            on_node_mouse_move: None,
            on_node_mouse_leave: None,
            on_node_context_menu: None,
            on_node_drag_start: None,
            on_node_drag: None,
            on_node_drag_stop: None,
            on_edge_click: None,
            on_edge_context_menu: None,
            on_edge_mouse_enter: None,
            on_edge_mouse_move: None,
            on_edge_mouse_leave: None,
            on_edge_double_click: None,
            on_reconnect: None,
            on_reconnect_start: None,
            on_reconnect_end: None,
            on_nodes_change: None,
            on_edges_change: None,
            on_nodes_delete: None,
            on_edges_delete: None,
            on_delete: None,
            on_before_delete: None,
            on_selection_drag_start: None,
            on_selection_drag: None,
            on_selection_drag_stop: None,
            on_selection_start: None,
            on_selection_end: None,
            on_selection_context_menu: None,
            on_selection_change: None,
            on_connect: None,
            on_connect_start: None,
            on_connect_end: None,
            on_click_connect_start: None,
            on_click_connect_end: None,
            on_init: None,
            on_move: None,
            on_move_start: None,
            on_move_end: None,
            on_viewport_change: None,
            on_pane_scroll: None,
            on_pane_click: None,
            on_pane_context_menu: None,
            on_pane_mouse_enter: None,
            on_pane_mouse_move: None,
            on_pane_mouse_leave: None,
            pane_click_distance: None,
            node_click_distance: None,
            node_types: None,
            edge_types: None,
            connection_line_type: None,
            connection_line_style: None,
            connection_line_component: None,
            connection_line_container_style: None,
            connection_mode: None,
            delete_key_code: None,
            selection_key_code: None,
            selection_on_drag: None,
            selection_mode: None,
            pan_activation_key_code: None,
            multi_selection_key_code: None,
            zoom_activation_key_code: None,
            snap_to_grid: None,
            snap_grid: None,
            only_render_visible_elements: None,
            nodes_draggable: None,
            auto_pan_on_node_focus: None,
            nodes_connectable: None,
            nodes_focusable: None,
            node_origin: None,
            edges_focusable: None,
            edges_reconnectable: None,
            elements_selectable: None,
            select_nodes_on_drag: None,
            pan_on_drag: None,
            min_zoom: None,
            max_zoom: None,
            viewport: None,
            default_viewport: None,
            translate_extent: None,
            prevent_scrolling: None,
            node_extent: None,
            default_marker_color: None,
            zoom_on_scroll: None,
            zoom_on_pinch: None,
            pan_on_scroll: None,
            pan_on_scroll_speed: None,
            pan_on_scroll_mode: None,
            zoom_on_double_click: None,
            reconnect_radius: None,
            no_drag_class_name: None,
            no_wheel_class_name: None,
            no_pan_class_name: None,
            fit_view: None,
            fit_view_options: None,
            connect_on_click: None,
            attribution_position: None,
            pro_options: None,
            elevate_nodes_on_select: None,
            elevate_edges_on_select: None,
            disable_keyboard_a11y: None,
            aria_label_config: None,
            auto_pan_on_node_drag: None,
            auto_pan_on_connect: None,
            auto_pan_on_selection: None,
            auto_pan_speed: None,
            connection_radius: None,
            on_error: None,
            is_valid_connection: None,
            node_drag_threshold: None,
            connection_drag_threshold: None,
            width: None,
            height: None,
            color_mode: None,
            debug: None,
            z_index_mode: None,
            id: None,
            style: None,
            class_name: None,
            on_scroll: None,
            dom_attributes: std::collections::HashMap::new(),
            children: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgraph_props_default_has_all_none() {
        let p: RGraphProps = RGraphProps::default();
        assert!(p.nodes.is_none());
        assert!(p.edges.is_none());
        assert!(p.on_init.is_none());
        assert!(p.fit_view.is_none());
        assert!(p.dom_attributes.is_empty());
    }
}
