//! Port of `xyflow-react/src/additional-components/MiniMap/MiniMapNodes.tsx`.
//!
//! Status: Phase 8 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;
use dioxus::events::MouseEvent;
use dioxus_signals::ReadableExt;
use rgraph_core::utils::general::{get_node_dimensions, node_has_dimensions};

use crate::context::use_rgraph_store;
use crate::store::RGraphStore;

use super::minimap_node::MiniMapNode;
use super::types::MiniMapNodeAttr;

#[derive(Props, Clone)]
pub struct MiniMapNodesProps<N: Clone + PartialEq + 'static = ()> {
    #[props(default)]
    pub node_color: Option<MiniMapNodeAttr<N>>,
    #[props(default)]
    pub node_stroke_color: Option<MiniMapNodeAttr<N>>,
    #[props(default)]
    pub node_class_name: Option<MiniMapNodeAttr<N>>,
    #[props(default = 5.0)]
    pub node_border_radius: f64,
    #[props(default)]
    pub node_stroke_width: Option<f64>,
    #[props(default)]
    pub on_click: Option<EventHandler<(MouseEvent, String)>>,
    /// Anti-aliasing hint; on Chrome we set `crispEdges`, elsewhere
    /// `geometricPrecision`. Desktop webview is Chromium, so default
    /// to `crispEdges`.
    #[props(default = "crispEdges".to_string())]
    pub shape_rendering: String,
}

impl<N: Clone + PartialEq + 'static> PartialEq for MiniMapNodesProps<N> {
    fn eq(&self, other: &Self) -> bool {
        self.node_color == other.node_color
            && self.node_stroke_color == other.node_stroke_color
            && self.node_class_name == other.node_class_name
            && self.node_border_radius == other.node_border_radius
            && self.node_stroke_width == other.node_stroke_width
            && self.shape_rendering == other.shape_rendering
    }
}

#[component]
pub fn MiniMapNodes<N: Clone + PartialEq + 'static>(props: MiniMapNodesProps<N>) -> Element {
    let store: RGraphStore<N, ()> = use_rgraph_store::<N, ()>();
    let lookup = store.node_lookup.read();

    let nodes = lookup
        .values()
        .filter(|n| !n.user.hidden.unwrap_or(false) && node_has_dimensions(&n.user))
        .cloned()
        .collect::<Vec<_>>();
    drop(lookup);

    let render = nodes.into_iter().map(|n| {
        let dims = get_node_dimensions(&n.user);
        let pos = n.internals.position_absolute;
        let class_name = props
            .node_class_name
            .as_ref()
            .map(|a| a.resolve(&n))
            .unwrap_or_default();
        let color = props.node_color.as_ref().map(|a| a.resolve(&n));
        let stroke_color = props.node_stroke_color.as_ref().map(|a| a.resolve(&n));
        let on_click = props.on_click;
        rsx! {
            MiniMapNode {
                key: "{n.user.id}",
                id: n.user.id.clone(),
                x: pos.x,
                y: pos.y,
                width: dims.width,
                height: dims.height,
                border_radius: props.node_border_radius,
                class_name,
                color,
                stroke_color,
                stroke_width: props.node_stroke_width,
                style: Option::<String>::None,
                selected: n.user.selected.unwrap_or(false),
                shape_rendering: props.shape_rendering.clone(),
                on_click,
            }
        }
    });

    rsx! {
        for el in render { {el} }
    }
}
