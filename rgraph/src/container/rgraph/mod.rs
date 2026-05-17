//! Port of `xyflow-react/src/container/ReactFlow/index.tsx`.
//!
//! Status: Phase 7 — implemented.
//!
//! `<RGraph>` is the public top-level component — the Dioxus
//! equivalent of `<ReactFlow>`. It composes:
//!
//! * [`crate::container::rgraph::wrapper::Wrapper`] — auto-injects
//!   an `<RGraphProvider>` when none is mounted above.
//! * [`crate::components::store_updater::StoreUpdater`] — syncs the
//!   prop bag into the store on every render.
//! * [`crate::container::graph_view::GraphView`] — the renderer.
//! * [`crate::components::selection_listener::SelectionListener`] —
//!   fires `on_selection_change`.
//! * [`crate::components::attribution::Attribution`] — the bottom
//!   corner attribution link.
//! * [`crate::components::a11y_descriptions::A11yDescriptions`] —
//!   screen-reader description divs.

#![allow(clippy::module_name_repetitions)]

pub mod init_values;
pub mod wrapper;

use dioxus::prelude::*;

use rgraph_core::constants::AriaLabelConfig;
use rgraph_core::types::connection::ConnectionMode;
use rgraph_core::types::edges::ConnectionLineType;
use rgraph_core::types::geometry::CoordinateExtent;
use rgraph_core::types::nodes::NodeOrigin;
use rgraph_core::types::panzoom::PanOnDrag;
use rgraph_core::types::viewport::{
    ColorMode, KeyCode, PanOnScrollMode, PanelPosition, ProOptions, SelectionMode, SnapGrid,
    Viewport, ZIndexMode,
};

use crate::components::a11y_descriptions::A11yDescriptions;
use crate::components::attribution::Attribution;
use crate::components::selection_listener::SelectionListener;
use crate::components::store_updater::StoreUpdater;
use crate::container::graph_view::GraphView;
use crate::container::rgraph::init_values::{DEFAULT_NODE_ORIGIN, DEFAULT_VIEWPORT};
use crate::container::rgraph::wrapper::Wrapper;
use crate::hooks::use_color_mode_class::use_color_mode_class;
use crate::hooks::use_rgraph::RGraphHandle;
use crate::types::component_props::{
    OnConnect, OnConnectEnd, OnConnectStart, OnError, OnMove, OnMoveEnd, OnMoveStart,
    OnPaneScroll, OnViewportChange, PaneMouseHandler,
};
use crate::types::edges::{DefaultEdgeOptions, EdgeMouseHandler, OnReconnect, OnReconnectEnd, OnReconnectStart};
use crate::types::general::{
    FitViewOptions, IsValidConnection, OnBeforeDelete, OnDelete, OnEdgesChange, OnEdgesDelete,
    OnInit, OnNodesChange, OnNodesDelete, OnSelectionChangeFunc,
};
use crate::types::nodes::{
    BuiltInNodeData, Node, NodeMouseHandler, OnNodeDrag, SelectionDragHandler,
};
use crate::types::edges::Edge;
use crate::utils::general::PtrEq;

/// Props for [`RGraph`]. Mirrors the TS `ReactFlowProps`. Every field
/// is optional except `id` (required so the resize-observer shim has
/// a deterministic selector).
#[derive(Props, Clone, PartialEq)]
pub struct RGraphProps<
    N: Clone + PartialEq + 'static = BuiltInNodeData,
    E: Clone + PartialEq + 'static = (),
> {
    /// HTML id of the outer wrapper. Defaults to `"rgraph"`.
    #[props(default = "rgraph".to_string())]
    pub id: String,
    #[props(default)]
    pub class_name: Option<String>,
    #[props(default)]
    pub style: Option<String>,

    // Controlled / uncontrolled.
    #[props(default)]
    pub nodes: Option<Vec<Node<N>>>,
    #[props(default)]
    pub edges: Option<Vec<Edge<E>>>,
    #[props(default)]
    pub default_nodes: Option<Vec<Node<N>>>,
    #[props(default)]
    pub default_edges: Option<Vec<Edge<E>>>,

    // Node handlers.
    #[props(default)]
    pub on_node_click: Option<NodeMouseHandler<N>>,
    #[props(default)]
    pub on_node_double_click: Option<NodeMouseHandler<N>>,
    #[props(default)]
    pub on_node_mouse_enter: Option<NodeMouseHandler<N>>,
    #[props(default)]
    pub on_node_mouse_move: Option<NodeMouseHandler<N>>,
    #[props(default)]
    pub on_node_mouse_leave: Option<NodeMouseHandler<N>>,
    #[props(default)]
    pub on_node_context_menu: Option<NodeMouseHandler<N>>,
    #[props(default)]
    pub on_node_drag_start: Option<OnNodeDrag<N>>,
    #[props(default)]
    pub on_node_drag: Option<OnNodeDrag<N>>,
    #[props(default)]
    pub on_node_drag_stop: Option<OnNodeDrag<N>>,

    // Edge handlers.
    #[props(default)]
    pub on_edge_click: Option<EdgeMouseHandler<E>>,
    #[props(default)]
    pub on_edge_double_click: Option<EdgeMouseHandler<E>>,
    #[props(default)]
    pub on_edge_mouse_enter: Option<EdgeMouseHandler<E>>,
    #[props(default)]
    pub on_edge_mouse_move: Option<EdgeMouseHandler<E>>,
    #[props(default)]
    pub on_edge_mouse_leave: Option<EdgeMouseHandler<E>>,
    #[props(default)]
    pub on_edge_context_menu: Option<EdgeMouseHandler<E>>,

    // Reconnect.
    #[props(default)]
    pub on_reconnect: Option<OnReconnect<E>>,
    #[props(default)]
    pub on_reconnect_start: Option<OnReconnectStart<E>>,
    #[props(default)]
    pub on_reconnect_end: Option<OnReconnectEnd<E, N>>,
    #[props(default = 10.0)]
    pub reconnect_radius: f64,

    // Change handlers.
    #[props(default)]
    pub on_nodes_change: Option<OnNodesChange<N>>,
    #[props(default)]
    pub on_edges_change: Option<OnEdgesChange<E>>,
    #[props(default)]
    pub on_nodes_delete: Option<OnNodesDelete<N>>,
    #[props(default)]
    pub on_edges_delete: Option<OnEdgesDelete<E>>,
    #[props(default)]
    pub on_delete: Option<OnDelete<N, E>>,
    #[props(default)]
    pub on_before_delete: Option<OnBeforeDelete<N, E>>,

    // Selection handlers.
    #[props(default)]
    pub on_selection_change: Option<OnSelectionChangeFunc<N, E>>,
    #[props(default)]
    pub on_selection_drag_start: Option<SelectionDragHandler<N>>,
    #[props(default)]
    pub on_selection_drag: Option<SelectionDragHandler<N>>,
    #[props(default)]
    pub on_selection_drag_stop: Option<SelectionDragHandler<N>>,

    // Connection handlers.
    #[props(default)]
    pub on_connect: Option<OnConnect>,
    #[props(default)]
    pub on_connect_start: Option<OnConnectStart>,
    #[props(default)]
    pub on_connect_end: Option<OnConnectEnd<N>>,
    #[props(default)]
    pub on_click_connect_start: Option<OnConnectStart>,
    #[props(default)]
    pub on_click_connect_end: Option<OnConnectEnd<N>>,

    // Init / viewport.
    #[props(default)]
    pub on_init: Option<OnInit<RGraphHandle<N, E>>>,
    #[props(default)]
    pub on_move: Option<OnMove>,
    #[props(default)]
    pub on_move_start: Option<OnMoveStart>,
    #[props(default)]
    pub on_move_end: Option<OnMoveEnd>,
    #[props(default)]
    pub on_viewport_change: Option<OnViewportChange>,
    #[props(default)]
    pub viewport: Option<Viewport>,
    #[props(default = DEFAULT_VIEWPORT)]
    pub default_viewport: Viewport,

    // Pane handlers.
    #[props(default)]
    pub on_pane_click: Option<PaneMouseHandler>,
    #[props(default)]
    pub on_pane_mouse_enter: Option<PaneMouseHandler>,
    #[props(default)]
    pub on_pane_mouse_move: Option<PaneMouseHandler>,
    #[props(default)]
    pub on_pane_mouse_leave: Option<PaneMouseHandler>,
    #[props(default)]
    pub on_pane_context_menu: Option<PaneMouseHandler>,
    #[props(default)]
    pub on_pane_scroll: Option<OnPaneScroll>,
    #[props(default = 1.0)]
    pub pane_click_distance: f64,
    #[props(default = 0.0)]
    pub node_click_distance: f64,

    // Connection line.
    #[props(default = ConnectionLineType::Bezier)]
    pub connection_line_type: ConnectionLineType,
    #[props(default)]
    pub connection_line_style: Option<String>,
    #[props(default)]
    pub connection_line_container_style: Option<String>,
    #[props(default)]
    pub connection_mode: Option<ConnectionMode>,

    // Keyboard.
    #[props(default = KeyCode::from("Backspace"))]
    pub delete_key_code: KeyCode,
    #[props(default = KeyCode::from("Shift"))]
    pub selection_key_code: KeyCode,
    #[props(default)]
    pub selection_on_drag: bool,
    #[props(default = SelectionMode::Full)]
    pub selection_mode: SelectionMode,
    #[props(default = KeyCode::from("Space"))]
    pub pan_activation_key_code: KeyCode,
    /// Defaults to "Control" — TS uses `isMacOs() ? 'Meta' : 'Control'`.
    /// Hosts should override this on macOS.
    #[props(default = KeyCode::from("Control"))]
    pub multi_selection_key_code: KeyCode,
    #[props(default = KeyCode::from("Control"))]
    pub zoom_activation_key_code: KeyCode,

    // Snap-to-grid.
    #[props(default)]
    pub snap_to_grid: bool,
    #[props(default = (15.0, 15.0))]
    pub snap_grid: SnapGrid,

    // Rendering.
    #[props(default)]
    pub only_render_visible_elements: bool,

    // Per-element flags.
    #[props(default)]
    pub select_nodes_on_drag: Option<bool>,
    #[props(default)]
    pub nodes_draggable: Option<bool>,
    #[props(default)]
    pub auto_pan_on_node_focus: Option<bool>,
    #[props(default)]
    pub nodes_connectable: Option<bool>,
    #[props(default)]
    pub nodes_focusable: Option<bool>,
    #[props(default = DEFAULT_NODE_ORIGIN)]
    pub node_origin: NodeOrigin,
    #[props(default)]
    pub edges_focusable: Option<bool>,
    #[props(default)]
    pub edges_reconnectable: Option<bool>,
    #[props(default = true)]
    pub elements_selectable: bool,

    // Zoom / pan.
    #[props(default = 0.5)]
    pub min_zoom: f64,
    #[props(default = 2.0)]
    pub max_zoom: f64,
    #[props(default)]
    pub translate_extent: Option<CoordinateExtent>,
    #[props(default = true)]
    pub prevent_scrolling: bool,
    #[props(default)]
    pub node_extent: Option<CoordinateExtent>,
    #[props(default = "#b1b1b7".to_string())]
    pub default_marker_color: String,
    #[props(default = true)]
    pub zoom_on_scroll: bool,
    #[props(default = true)]
    pub zoom_on_pinch: bool,
    #[props(default)]
    pub pan_on_scroll: bool,
    #[props(default = 0.5)]
    pub pan_on_scroll_speed: f64,
    #[props(default)]
    pub pan_on_scroll_mode: PanOnScrollMode,
    #[props(default = true)]
    pub zoom_on_double_click: bool,
    #[props(default = PanOnDrag::On)]
    pub pan_on_drag: PanOnDrag,

    // Class names.
    #[props(default = "nodrag".to_string())]
    pub no_drag_class_name: String,
    #[props(default = "nowheel".to_string())]
    pub no_wheel_class_name: String,
    #[props(default = "nopan".to_string())]
    pub no_pan_class_name: String,

    // Fit-view.
    #[props(default)]
    pub fit_view: Option<bool>,
    #[props(default)]
    pub fit_view_options: Option<PtrEq<FitViewOptions>>,

    // Connect-on-click.
    #[props(default)]
    pub connect_on_click: Option<bool>,

    // Attribution / pro.
    #[props(default = PanelPosition::BottomRight)]
    pub attribution_position: PanelPosition,
    #[props(default)]
    pub pro_options: Option<ProOptions>,
    #[props(default)]
    pub default_edge_options: Option<DefaultEdgeOptions>,

    // Z-index.
    #[props(default = true)]
    pub elevate_nodes_on_select: bool,
    #[props(default)]
    pub elevate_edges_on_select: bool,
    #[props(default = ZIndexMode::Basic)]
    pub z_index_mode: ZIndexMode,

    // Disable keyboard a11y.
    #[props(default)]
    pub disable_keyboard_a11y: bool,

    // Auto-pan.
    #[props(default)]
    pub auto_pan_on_connect: Option<bool>,
    #[props(default)]
    pub auto_pan_on_node_drag: Option<bool>,
    #[props(default = true)]
    pub auto_pan_on_selection: bool,
    #[props(default)]
    pub auto_pan_speed: Option<f64>,

    // Connection.
    #[props(default)]
    pub connection_radius: Option<f64>,
    #[props(default)]
    pub is_valid_connection: Option<IsValidConnection<E>>,

    // Error.
    #[props(default)]
    pub on_error: Option<OnError>,

    // Thresholds.
    #[props(default)]
    pub node_drag_threshold: Option<f64>,
    #[props(default)]
    pub connection_drag_threshold: Option<f64>,

    // Sizing (controlled).
    #[props(default)]
    pub width: Option<f64>,
    #[props(default)]
    pub height: Option<f64>,

    // Color mode.
    #[props(default = ColorMode::Light)]
    pub color_mode: ColorMode,

    // Misc.
    #[props(default)]
    pub debug: bool,
    #[props(default)]
    pub aria_label_config: Option<AriaLabelConfig>,
    #[props(default)]
    pub selection_drag_threshold: Option<f64>,

    // Children rendered inside the viewport.
    #[props(default)]
    pub children: Element,

    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

/// The `<RGraph />` component is the heart of an rgraph application
/// — the Dioxus equivalent of `<ReactFlow>`. It renders nodes and
/// edges, handles user interaction, and emits change events.
///
/// # Example
///
/// ```ignore
/// use dioxus::prelude::*;
/// use rgraph::prelude::*;
///
/// fn App() -> Element {
///     let nodes = use_signal(|| vec![Node::<BuiltInNodeData>::minimal("n1", 0.0, 0.0)]);
///     rsx! {
///         RGraph::<BuiltInNodeData, ()> {
///             nodes: nodes.read().clone(),
///             fit_view: Some(true),
///         }
///     }
/// }
/// ```
#[component]
pub fn RGraph<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: RGraphProps<N, E>,
) -> Element {
    let rf_id = props.id.clone();
    let color_mode = props.color_mode;
    let color_class_signal = use_color_mode_class(color_mode);
    let color_class = match *color_class_signal.read() {
        rgraph_core::types::viewport::ColorModeClass::Light => "light",
        rgraph_core::types::viewport::ColorModeClass::Dark => "dark",
    };

    let mut classes = String::from("react-flow ");
    classes.push_str(color_class);
    if let Some(extra) = &props.class_name {
        classes.push(' ');
        classes.push_str(extra);
    }

    let mut style_str = String::from(
        "width:100%;height:100%;overflow:hidden;position:relative;z-index:0;",
    );
    if let Some(extra) = &props.style {
        style_str.push_str(extra);
    }

    rsx! {
        div {
            "data-testid": "rg__wrapper",
            id: "{rf_id}",
            class: "{classes}",
            style: "{style_str}",
            role: "application",
            Wrapper::<N, E> {
                nodes: props.nodes.clone(),
                edges: props.edges.clone(),
                default_nodes: props.default_nodes.clone(),
                default_edges: props.default_edges.clone(),
                width: props.width,
                height: props.height,
                fit_view: props.fit_view,
                fit_view_options: props.fit_view_options.clone(),
                min_zoom: Some(props.min_zoom),
                max_zoom: Some(props.max_zoom),
                node_origin: Some(props.node_origin),
                node_extent: props.node_extent,
                z_index_mode: Some(props.z_index_mode),
                StoreUpdater::<N, E> {
                    rf_id: rf_id.clone(),
                    nodes: props.nodes.clone(),
                    edges: props.edges.clone(),
                    default_nodes: props.default_nodes.clone(),
                    default_edges: props.default_edges.clone(),
                    on_connect: props.on_connect,
                    on_connect_start: props.on_connect_start,
                    on_connect_end: props.on_connect_end,
                    on_click_connect_start: props.on_click_connect_start,
                    on_click_connect_end: props.on_click_connect_end,
                    nodes_draggable: props.nodes_draggable,
                    auto_pan_on_node_focus: props.auto_pan_on_node_focus,
                    nodes_connectable: props.nodes_connectable,
                    nodes_focusable: props.nodes_focusable,
                    edges_focusable: props.edges_focusable,
                    edges_reconnectable: props.edges_reconnectable,
                    elevate_nodes_on_select: Some(props.elevate_nodes_on_select),
                    elevate_edges_on_select: Some(props.elevate_edges_on_select),
                    min_zoom: Some(props.min_zoom),
                    max_zoom: Some(props.max_zoom),
                    node_extent: props.node_extent,
                    on_nodes_change: props.on_nodes_change,
                    on_edges_change: props.on_edges_change,
                    snap_to_grid: Some(props.snap_to_grid),
                    snap_grid: Some(props.snap_grid),
                    connection_mode: props.connection_mode,
                    translate_extent: props.translate_extent,
                    connect_on_click: props.connect_on_click,
                    default_edge_options: props.default_edge_options.clone(),
                    fit_view: props.fit_view,
                    fit_view_options: props.fit_view_options.clone(),
                    on_nodes_delete: props.on_nodes_delete,
                    on_edges_delete: props.on_edges_delete,
                    on_delete: props.on_delete,
                    on_node_drag: props.on_node_drag,
                    on_node_drag_start: props.on_node_drag_start,
                    on_node_drag_stop: props.on_node_drag_stop,
                    on_selection_drag: props.on_selection_drag,
                    on_selection_drag_start: props.on_selection_drag_start,
                    on_selection_drag_stop: props.on_selection_drag_stop,
                    on_move: props.on_move,
                    on_move_start: props.on_move_start,
                    on_move_end: props.on_move_end,
                    no_pan_class_name: Some(props.no_pan_class_name.clone()),
                    node_origin: Some(props.node_origin),
                    auto_pan_on_connect: props.auto_pan_on_connect,
                    auto_pan_on_node_drag: props.auto_pan_on_node_drag,
                    auto_pan_speed: props.auto_pan_speed,
                    on_error: props.on_error,
                    connection_radius: props.connection_radius,
                    is_valid_connection: props.is_valid_connection,
                    select_nodes_on_drag: props.select_nodes_on_drag,
                    node_drag_threshold: props.node_drag_threshold,
                    connection_drag_threshold: props.connection_drag_threshold,
                    on_before_delete: props.on_before_delete,
                    debug: Some(props.debug),
                    aria_label_config: props.aria_label_config.clone(),
                    z_index_mode: Some(props.z_index_mode),
                    elements_selectable: Some(props.elements_selectable),
                }
                GraphView::<N, E> {
                    id: format!("{rf_id}__zoompane"),
                    on_init: props.on_init,
                    on_node_click: props.on_node_click,
                    on_node_double_click: props.on_node_double_click,
                    on_node_mouse_enter: props.on_node_mouse_enter,
                    on_node_mouse_move: props.on_node_mouse_move,
                    on_node_mouse_leave: props.on_node_mouse_leave,
                    on_node_context_menu: props.on_node_context_menu,
                    on_edge_click: props.on_edge_click,
                    on_edge_double_click: props.on_edge_double_click,
                    on_edge_mouse_enter: props.on_edge_mouse_enter,
                    on_edge_mouse_move: props.on_edge_mouse_move,
                    on_edge_mouse_leave: props.on_edge_mouse_leave,
                    on_edge_context_menu: props.on_edge_context_menu,
                    on_pane_click: props.on_pane_click,
                    on_pane_mouse_enter: props.on_pane_mouse_enter,
                    on_pane_mouse_move: props.on_pane_mouse_move,
                    on_pane_mouse_leave: props.on_pane_mouse_leave,
                    on_pane_context_menu: props.on_pane_context_menu,
                    on_pane_scroll: props.on_pane_scroll,
                    on_viewport_change: props.on_viewport_change,
                    viewport: props.viewport,
                    connection_line_type: props.connection_line_type,
                    connection_line_style: props.connection_line_style.clone(),
                    connection_line_container_style: props.connection_line_container_style.clone(),
                    selection_key_code: Some(props.selection_key_code.clone()),
                    delete_key_code: Some(props.delete_key_code.clone()),
                    multi_selection_key_code: Some(props.multi_selection_key_code.clone()),
                    pan_activation_key_code: Some(props.pan_activation_key_code.clone()),
                    zoom_activation_key_code: Some(props.zoom_activation_key_code.clone()),
                    selection_on_drag: props.selection_on_drag,
                    selection_mode: props.selection_mode,
                    only_render_visible_elements: props.only_render_visible_elements,
                    default_viewport: props.default_viewport,
                    min_zoom: props.min_zoom,
                    max_zoom: props.max_zoom,
                    prevent_scrolling: props.prevent_scrolling,
                    default_marker_color: props.default_marker_color.clone(),
                    zoom_on_scroll: props.zoom_on_scroll,
                    zoom_on_pinch: props.zoom_on_pinch,
                    pan_on_scroll: props.pan_on_scroll,
                    pan_on_scroll_speed: props.pan_on_scroll_speed,
                    pan_on_scroll_mode: props.pan_on_scroll_mode,
                    zoom_on_double_click: props.zoom_on_double_click,
                    pan_on_drag: props.pan_on_drag.clone(),
                    auto_pan_on_selection: props.auto_pan_on_selection,
                    pane_click_distance: props.pane_click_distance,
                    node_click_distance: props.node_click_distance,
                    reconnect_radius: props.reconnect_radius,
                    no_drag_class_name: props.no_drag_class_name.clone(),
                    no_wheel_class_name: props.no_wheel_class_name.clone(),
                    no_pan_class_name: props.no_pan_class_name.clone(),
                    disable_keyboard_a11y: props.disable_keyboard_a11y,
                }
                SelectionListener::<N, E> {
                    on_selection_change: props.on_selection_change,
                }
                {props.children}
                Attribution {
                    pro_options: props.pro_options.clone(),
                    position: Some(props.attribution_position),
                }
                A11yDescriptions::<N, E> {
                    rf_id: rf_id.clone(),
                    disable_keyboard_a11y: props.disable_keyboard_a11y,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::nodes::{BuiltInNodeData, Node};

    /// Smoke test: mount `<RGraph>` with a couple of built-in nodes
    /// and let the virtual dom render through the full component tree
    /// (RGraph → Wrapper → RGraphProvider → BatchProvider →
    /// StoreUpdater + GraphView + SelectionListener + Attribution +
    /// A11yDescriptions).
    #[test]
    fn rgraph_renders_without_panic() {
        fn Root() -> Element {
            rsx! {
                RGraph::<BuiltInNodeData, ()> {
                    nodes: Some(vec![
                        Node::<BuiltInNodeData>::minimal("a", 0.0, 0.0),
                        Node::<BuiltInNodeData>::minimal("b", 100.0, 100.0),
                    ]),
                }
            }
        }
        let mut vdom = VirtualDom::new(Root);
        let _ = vdom.rebuild_to_vec();
        // We don't assert on the rendered output (which depends on
        // Dioxus internals); reaching this line means every Phase 7
        // component compiles, registers its hooks, and produces a
        // valid VNode tree.
    }

    /// Inner wiring: `nodes` prop reaches the store via `<StoreUpdater>`.
    #[test]
    fn rgraph_routes_nodes_into_store() {
        use std::cell::Cell;
        thread_local! { static COUNT: Cell<usize> = const { Cell::new(0) }; }

        #[component]
        fn Probe() -> Element {
            use dioxus::prelude::ReadableExt;
            let store = crate::context::use_rgraph_store::<BuiltInNodeData, ()>();
            COUNT.with(|c| c.set(store.nodes.peek().len()));
            rsx! { div {} }
        }

        fn Root() -> Element {
            rsx! {
                RGraph::<BuiltInNodeData, ()> {
                    nodes: Some(vec![
                        Node::<BuiltInNodeData>::minimal("a", 0.0, 0.0),
                        Node::<BuiltInNodeData>::minimal("b", 1.0, 1.0),
                        Node::<BuiltInNodeData>::minimal("c", 2.0, 2.0),
                    ]),
                    Probe {}
                }
            }
        }
        let mut vdom = VirtualDom::new(Root);
        let _ = vdom.rebuild_to_vec();
        assert_eq!(COUNT.with(|c| c.get()), 3);
    }
}
