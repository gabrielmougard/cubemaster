//! Port of `xyflow-react/src/container/NodeRenderer/index.tsx`.
//!
//! Status: Phase 5 — implemented.
//!
//! `<NodeRenderer>` enumerates visible node ids and renders one
//! `<NodeWrapper>` per id inside a `react-flow__nodes` container.
//! The TS source splits responsibilities between this renderer and
//! `NodeWrapper` to minimise re-renders during drag (per-id wrappers
//! re-render only when *their* node changes).

#![allow(clippy::module_name_repetitions)]

pub mod use_resize_observer;

use dioxus::prelude::*;

use crate::components::node_wrapper::{NodeWrapper, NodeWrapperProps};
use crate::container::node_renderer::use_resize_observer::use_resize_observer;
use crate::hooks::use_visible_node_ids::use_visible_node_ids;
use crate::types::nodes::{BuiltInNodeData, NodeMouseHandler};

/// Props for [`NodeRenderer`]. Mirrors TS `NodeRendererProps` (a
/// subset of `GraphViewProps`).
#[derive(Props, Clone, PartialEq)]
pub struct NodeRendererProps<N: Clone + PartialEq + 'static = BuiltInNodeData> {
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
    pub only_render_visible_elements: bool,
    #[props(default = "nopan".to_string())]
    pub no_pan_class_name: String,
    #[props(default = "nodrag".to_string())]
    pub no_drag_class_name: String,
    pub rf_id: String,
    #[props(default)]
    pub disable_keyboard_a11y: bool,
    #[props(default = 0.0)]
    pub node_click_distance: f64,
    #[props(default)]
    pub _types: std::marker::PhantomData<N>,
}

#[component]
pub fn NodeRenderer<N: Clone + PartialEq + 'static>(
    props: NodeRendererProps<N>,
) -> Element {
    // Install the shared resize-observer shim. Each `<NodeWrapper>`
    // polls it through its own observer hook.
    use_resize_observer();

    let node_ids = use_visible_node_ids::<N, ()>(props.only_render_visible_elements);
    let style = "position:absolute;width:100%;height:100%;top:0;left:0;";

    rsx! {
        div {
            class: "react-flow__nodes",
            style: "{style}",
            for nid in node_ids {
                NodeWrapper::<N> {
                    key: "{nid}",
                    id: nid.clone(),
                    no_drag_class_name: props.no_drag_class_name.clone(),
                    no_pan_class_name: props.no_pan_class_name.clone(),
                    rf_id: props.rf_id.clone(),
                    disable_keyboard_a11y: props.disable_keyboard_a11y,
                    on_click: props.on_node_click,
                    on_double_click: props.on_node_double_click,
                    on_mouse_enter: props.on_node_mouse_enter,
                    on_mouse_move: props.on_node_mouse_move,
                    on_mouse_leave: props.on_node_mouse_leave,
                    on_context_menu: props.on_node_context_menu,
                    node_click_distance: props.node_click_distance,
                }
            }
        }
    }
}
