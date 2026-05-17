//! Port of `xyflow-react/src/additional-components/EdgeToolbar/EdgeToolbar.tsx`.
//!
//! Status: Phase 8 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;
use dioxus_signals::ReadableExt;
use rgraph_core::types::edges::{AlignX, AlignY};
use rgraph_core::utils::edge_toolbar::get_edge_toolbar_transform;

use crate::components::edge_label_renderer::EdgeLabelRenderer;
use crate::context::use_rgraph_store;
use crate::store::RGraphStore;

use super::types::{DEFAULT_EDGE_TOOLBAR_ALIGN_X, DEFAULT_EDGE_TOOLBAR_ALIGN_Y};

#[derive(Props, Clone, PartialEq)]
pub struct EdgeToolbarProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    pub edge_id: String,
    pub x: f64,
    pub y: f64,
    #[props(default)]
    pub class_name: Option<String>,
    #[props(default)]
    pub style: Option<String>,
    /// `None` → derived from the edge's `selected` flag.
    #[props(default)]
    pub is_visible: Option<bool>,
    #[props(default = DEFAULT_EDGE_TOOLBAR_ALIGN_X)]
    pub align_x: AlignX,
    #[props(default = DEFAULT_EDGE_TOOLBAR_ALIGN_Y)]
    pub align_y: AlignY,
    pub children: Element,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn EdgeToolbar<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static>(
    props: EdgeToolbarProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let lookup = store.edge_lookup.read();
    let edge = lookup.get(&props.edge_id).cloned();
    drop(lookup);

    let is_active = match props.is_visible {
        Some(v) => v,
        None => edge.as_ref().and_then(|e| e.selected).unwrap_or(false),
    };
    if !is_active {
        return rsx! {};
    }

    let zoom = store.transform.read().scale();
    let z_index = edge.as_ref().and_then(|e| e.z_index).unwrap_or(0) + 1;
    let transform = get_edge_toolbar_transform(
        props.x, props.y, zoom, props.align_x, props.align_y,
    );

    let mut style_str = format!(
        "position:absolute;transform:{transform};z-index:{z_index};pointer-events:all;transform-origin:0 0;"
    );
    if let Some(extra) = &props.style {
        style_str.push_str(extra);
    }
    let user_class = props.class_name.clone().unwrap_or_default();
    let class = format!("react-flow__edge-toolbar {user_class}");
    let data_id = edge.as_ref().map(|e| e.id.as_str()).unwrap_or("");

    rsx! {
        EdgeLabelRenderer {
            div {
                class: "{class}",
                style: "{style_str}",
                "data-id": "{data_id}",
                {props.children}
            }
        }
    }
}
