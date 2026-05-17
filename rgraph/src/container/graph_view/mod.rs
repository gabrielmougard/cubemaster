//! Port of `xyflow-react/src/container/GraphView/`.
//!
//! Status: Phase 7 — implemented.

#![allow(clippy::module_name_repetitions)]

pub mod use_node_or_edge_types_warning;
pub mod use_styles_loaded_warning;

use dioxus::prelude::*;

use rgraph_core::types::edges::ConnectionLineType;
use rgraph_core::types::panzoom::PanOnDrag;
use rgraph_core::types::viewport::{KeyCode, PanOnScrollMode, SelectionMode, Viewport};

use crate::components::connection_line::ConnectionLineWrapper;
use crate::container::edge_renderer::EdgeRenderer;
use crate::container::flow_renderer::FlowRenderer;
use crate::container::node_renderer::NodeRenderer;
use crate::container::viewport::Viewport as ViewportComponent;
use crate::hooks::use_on_init_handler::use_on_init_handler;
use crate::hooks::use_viewport_sync::use_viewport_sync;
use crate::types::component_props::{OnViewportChange, OnPaneScroll, PaneMouseHandler};
use crate::types::edges::EdgeMouseHandler;
use crate::types::general::OnInit;
use crate::types::nodes::{BuiltInNodeData, NodeMouseHandler};
use crate::hooks::use_rgraph::RGraphHandle;

#[derive(Props, Clone, PartialEq)]
pub struct GraphViewProps<
    N: Clone + PartialEq + 'static = BuiltInNodeData,
    E: Clone + PartialEq + 'static = (),
> {
    /// HTML id of the `<ZoomPane>` wrapper. Required so the
    /// resize-observer shim can locate the element.
    pub id: String,

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

    // Init / viewport.
    #[props(default)]
    pub on_init: Option<OnInit<RGraphHandle<N, E>>>,
    #[props(default)]
    pub on_viewport_change: Option<OnViewportChange>,
    #[props(default)]
    pub viewport: Option<Viewport>,

    // Connection line.
    #[props(default = ConnectionLineType::Bezier)]
    pub connection_line_type: ConnectionLineType,
    #[props(default)]
    pub connection_line_style: Option<String>,
    #[props(default)]
    pub connection_line_container_style: Option<String>,

    // Keyboard / selection.
    #[props(default)]
    pub selection_key_code: Option<KeyCode>,
    #[props(default)]
    pub delete_key_code: Option<KeyCode>,
    #[props(default)]
    pub multi_selection_key_code: Option<KeyCode>,
    #[props(default)]
    pub pan_activation_key_code: Option<KeyCode>,
    #[props(default)]
    pub zoom_activation_key_code: Option<KeyCode>,
    #[props(default)]
    pub selection_on_drag: bool,
    #[props(default = SelectionMode::Full)]
    pub selection_mode: SelectionMode,

    // Pan / zoom.
    #[props(default)]
    pub only_render_visible_elements: bool,
    #[props(default)]
    pub default_viewport: Viewport,
    #[props(default = 0.5)]
    pub min_zoom: f64,
    #[props(default = 2.0)]
    pub max_zoom: f64,
    #[props(default = true)]
    pub prevent_scrolling: bool,
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
    #[props(default = true)]
    pub auto_pan_on_selection: bool,
    #[props(default = 1.0)]
    pub pane_click_distance: f64,
    #[props(default = 0.0)]
    pub node_click_distance: f64,
    #[props(default = 10.0)]
    pub reconnect_radius: f64,

    // Class names.
    #[props(default = "nodrag".to_string())]
    pub no_drag_class_name: String,
    #[props(default = "nowheel".to_string())]
    pub no_wheel_class_name: String,
    #[props(default = "nopan".to_string())]
    pub no_pan_class_name: String,

    #[props(default)]
    pub disable_keyboard_a11y: bool,

    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn GraphView<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: GraphViewProps<N, E>,
) -> Element {
    // Dev warnings.
    use_node_or_edge_types_warning::use_node_or_edge_types_warning::<N, E>(
        use_node_or_edge_types_warning::TypesKind::Node,
        Vec::new(),
    );
    use_node_or_edge_types_warning::use_node_or_edge_types_warning::<N, E>(
        use_node_or_edge_types_warning::TypesKind::Edge,
        Vec::new(),
    );
    use_styles_loaded_warning::use_styles_loaded_warning::<N, E>();

    use_on_init_handler::<N, E>(props.on_init);
    use_viewport_sync::<N, E>(props.viewport);

    let is_controlled_viewport = props.viewport.is_some();
    // The rf_id ferried to inner children — TS uses `rfId` from the
    // outer prop bag, which in our port comes from `props.id`.
    let rf_id = props.id.clone();

    rsx! {
        FlowRenderer::<N, E> {
            id: props.id.clone(),
            on_pane_click: props.on_pane_click,
            on_pane_mouse_enter: props.on_pane_mouse_enter,
            on_pane_mouse_move: props.on_pane_mouse_move,
            on_pane_mouse_leave: props.on_pane_mouse_leave,
            on_pane_context_menu: props.on_pane_context_menu,
            on_pane_scroll: props.on_pane_scroll,
            pane_click_distance: props.pane_click_distance,
            delete_key_code: props.delete_key_code.clone(),
            selection_key_code: props.selection_key_code.clone(),
            selection_on_drag: props.selection_on_drag,
            selection_mode: props.selection_mode,
            multi_selection_key_code: props.multi_selection_key_code.clone(),
            pan_activation_key_code: props.pan_activation_key_code.clone(),
            zoom_activation_key_code: props.zoom_activation_key_code.clone(),
            zoom_on_scroll: props.zoom_on_scroll,
            zoom_on_pinch: props.zoom_on_pinch,
            zoom_on_double_click: props.zoom_on_double_click,
            pan_on_scroll: props.pan_on_scroll,
            pan_on_scroll_speed: props.pan_on_scroll_speed,
            pan_on_scroll_mode: props.pan_on_scroll_mode,
            pan_on_drag: props.pan_on_drag.clone(),
            auto_pan_on_selection: props.auto_pan_on_selection,
            default_viewport: props.default_viewport,
            min_zoom: props.min_zoom,
            max_zoom: props.max_zoom,
            prevent_scrolling: props.prevent_scrolling,
            no_drag_class_name: props.no_drag_class_name.clone(),
            no_wheel_class_name: props.no_wheel_class_name.clone(),
            no_pan_class_name: props.no_pan_class_name.clone(),
            disable_keyboard_a11y: props.disable_keyboard_a11y,
            on_viewport_change: props.on_viewport_change,
            is_controlled_viewport,
            ViewportComponent::<N, E> {
                EdgeRenderer::<N, E> {
                    on_edge_click: props.on_edge_click,
                    on_edge_double_click: props.on_edge_double_click,
                    on_edge_mouse_enter: props.on_edge_mouse_enter,
                    on_edge_mouse_move: props.on_edge_mouse_move,
                    on_edge_mouse_leave: props.on_edge_mouse_leave,
                    on_edge_context_menu: props.on_edge_context_menu,
                    default_marker_color: Some(props.default_marker_color.clone()),
                    only_render_visible_elements: props.only_render_visible_elements,
                    rf_id: rf_id.clone(),
                    no_pan_class_name: props.no_pan_class_name.clone(),
                    disable_keyboard_a11y: props.disable_keyboard_a11y,
                }
                ConnectionLineWrapper::<N, E> {
                    type_: props.connection_line_type,
                    style: props.connection_line_style.clone(),
                    container_style: props.connection_line_container_style.clone(),
                }
                // The TS source emits an empty `<div class="react-flow__edgelabel-renderer" />`
                // inside the viewport so `<EdgeLabelRenderer>` portals
                // can attach their children. Our Phase 6 portal is
                // inline so the empty div isn't required, but we keep
                // it as a styling-hook for hosts that target it.
                div { class: "react-flow__edgelabel-renderer" }
                NodeRenderer::<N> {
                    on_node_click: props.on_node_click,
                    on_node_double_click: props.on_node_double_click,
                    on_node_mouse_enter: props.on_node_mouse_enter,
                    on_node_mouse_move: props.on_node_mouse_move,
                    on_node_mouse_leave: props.on_node_mouse_leave,
                    on_node_context_menu: props.on_node_context_menu,
                    node_click_distance: props.node_click_distance,
                    only_render_visible_elements: props.only_render_visible_elements,
                    no_pan_class_name: props.no_pan_class_name.clone(),
                    no_drag_class_name: props.no_drag_class_name.clone(),
                    disable_keyboard_a11y: props.disable_keyboard_a11y,
                    rf_id: rf_id.clone(),
                }
                // Same as the edge-label renderer: empty viewport-portal
                // div retained for stylesheet parity.
                div { class: "react-flow__viewport-portal" }
            }
        }
    }
}
