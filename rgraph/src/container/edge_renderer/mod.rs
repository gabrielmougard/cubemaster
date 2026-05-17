//! Port of `xyflow-react/src/container/EdgeRenderer/`.
//!
//! Status: Phase 6 — implemented.

#![allow(clippy::module_name_repetitions)]

pub mod marker_definitions;
pub mod marker_symbols;

use dioxus::prelude::*;

use crate::components::edge_wrapper::{EdgeWrapper, EdgeWrapperComponentProps};
use crate::container::edge_renderer::marker_definitions::MarkerDefinitions;
use crate::hooks::use_visible_edge_ids::use_visible_edge_ids;
use crate::types::edges::EdgeMouseHandler;

#[derive(Props, Clone, PartialEq)]
pub struct EdgeRendererProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
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

    #[props(default)]
    pub default_marker_color: Option<String>,
    #[props(default)]
    pub only_render_visible_elements: bool,
    pub rf_id: String,
    #[props(default = "nopan".to_string())]
    pub no_pan_class_name: String,
    #[props(default)]
    pub disable_keyboard_a11y: bool,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn EdgeRenderer<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: EdgeRendererProps<N, E>,
) -> Element {
    let edge_ids = use_visible_edge_ids::<N, E>(props.only_render_visible_elements);

    rsx! {
        div {
            class: "react-flow__edges",
            MarkerDefinitions::<N, E> {
                default_color: props.default_marker_color.clone(),
                rf_id: Some(props.rf_id.clone()),
            }
            for id in edge_ids {
                EdgeWrapper::<N, E> {
                    key: "{id}",
                    id: id.clone(),
                    rf_id: props.rf_id.clone(),
                    no_pan_class_name: props.no_pan_class_name.clone(),
                    disable_keyboard_a11y: props.disable_keyboard_a11y,
                    on_click: props.on_edge_click,
                    on_double_click: props.on_edge_double_click,
                    on_mouse_enter: props.on_edge_mouse_enter,
                    on_mouse_move: props.on_edge_mouse_move,
                    on_mouse_leave: props.on_edge_mouse_leave,
                    on_context_menu: props.on_edge_context_menu,
                }
            }
        }
    }
}

// `EdgeWrapperComponentProps` re-exported through the prelude.
#[allow(dead_code)]
type _EWP = EdgeWrapperComponentProps;
