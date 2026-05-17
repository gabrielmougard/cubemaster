//! Port of `xyflow-react/src/additional-components/NodeToolbar/NodeToolbar.tsx`.
//!
//! Status: Phase 8 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;
use dioxus_signals::ReadableExt;
use rgraph_core::types::geometry::Position;
use rgraph_core::types::nodes::{Align, NodeLookup};
use rgraph_core::types::viewport::Viewport;
use rgraph_core::utils::graph::{get_internal_nodes_bounds, GetInternalNodesBoundsParams};
use rgraph_core::utils::node_toolbar::get_node_toolbar_transform;

use crate::context::use_rgraph_store;
use crate::contexts::node_id::use_node_id;
use crate::store::RGraphStore;

use super::node_toolbar_portal::NodeToolbarPortal;
use super::types::{
    NodeToolbarTarget, DEFAULT_TOOLBAR_ALIGN, DEFAULT_TOOLBAR_OFFSET, DEFAULT_TOOLBAR_POSITION,
};

#[derive(Props, Clone, PartialEq)]
pub struct NodeToolbarProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    #[props(default)]
    pub node_id: Option<NodeToolbarTarget>,
    #[props(default)]
    pub class_name: Option<String>,
    #[props(default)]
    pub style: Option<String>,
    /// `None` → auto: visible iff exactly one node is selected and it's
    /// the node identified by [`Self::node_id`].
    #[props(default)]
    pub is_visible: Option<bool>,
    #[props(default = DEFAULT_TOOLBAR_POSITION)]
    pub position: Position,
    #[props(default = DEFAULT_TOOLBAR_OFFSET)]
    pub offset: f64,
    #[props(default = DEFAULT_TOOLBAR_ALIGN)]
    pub align: Align,
    pub children: Element,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn NodeToolbar<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static>(
    props: NodeToolbarProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let context_node_id = use_node_id();

    let ids: Vec<String> = match &props.node_id {
        Some(t) => t.ids(),
        None => context_node_id.into_iter().collect(),
    };

    if ids.is_empty() {
        return rsx! {};
    }

    let lookup = store.node_lookup.read();
    let mut target: NodeLookup<N> = NodeLookup::default();
    for id in &ids {
        if let Some(n) = lookup.get(id) {
            target.insert(id.clone(), n.clone());
        }
    }
    drop(lookup);

    if target.is_empty() {
        return rsx! {};
    }

    let transform = *store.transform.read();
    let nodes = store.nodes.read();
    let selected_count = nodes.iter().filter(|n| n.selected.unwrap_or(false)).count();
    drop(nodes);

    let is_active = match props.is_visible {
        Some(b) => b,
        None => {
            target.len() == 1
                && target
                    .values()
                    .next()
                    .map(|n| n.user.selected.unwrap_or(false))
                    .unwrap_or(false)
                && selected_count == 1
        }
    };

    if !is_active {
        return rsx! {};
    }

    let node_rect = get_internal_nodes_bounds(
        &target,
        GetInternalNodesBoundsParams { filter: None },
    );
    let z_index = target
        .values()
        .map(|n| n.internals.z + 1.0)
        .fold(f64::NEG_INFINITY, f64::max)
        .max(1.0);
    let viewport = Viewport {
        x: transform.tx(),
        y: transform.ty(),
        zoom: transform.scale(),
    };
    let transform_str = get_node_toolbar_transform(
        node_rect,
        viewport,
        props.position,
        props.offset,
        props.align,
    );

    let mut style_str = format!(
        "position:absolute;transform:{transform_str};z-index:{z_index};"
    );
    if let Some(extra) = &props.style {
        style_str.push_str(extra);
    }

    let user_class = props.class_name.clone().unwrap_or_default();
    let class = format!("react-flow__node-toolbar {user_class}");
    let data_id: String = target
        .values()
        .map(|n| n.user.id.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    rsx! {
        NodeToolbarPortal {
            div {
                class: "{class}",
                style: "{style_str}",
                "data-id": "{data_id}",
                {props.children}
            }
        }
    }
}
