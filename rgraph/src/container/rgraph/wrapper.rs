//! Port of `xyflow-react/src/container/ReactFlow/Wrapper.tsx`.
//!
//! Status: Phase 7 — implemented.
//!
//! Auto-wraps the children in [`crate::components::rgraph_provider::RGraphProvider`]
//! **only** if there isn't one already in context (so users can choose
//! to mount `<RGraphProvider>` themselves to share a store across
//! sibling `<RGraph>`s).

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use crate::components::rgraph_provider::RGraphProvider;
use crate::context::try_use_rgraph_store;
use crate::store::RGraphStore;
use crate::types::edges::Edge;
use crate::types::general::FitViewOptions;
use crate::types::nodes::Node;
use crate::utils::general::PtrEq;

/// Props for [`Wrapper`]. Mirrors the TS `Wrapper`.
#[derive(Props, Clone, PartialEq)]
pub struct WrapperProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    #[props(default)]
    pub nodes: Option<Vec<Node<N>>>,
    #[props(default)]
    pub edges: Option<Vec<Edge<E>>>,
    #[props(default)]
    pub default_nodes: Option<Vec<Node<N>>>,
    #[props(default)]
    pub default_edges: Option<Vec<Edge<E>>>,
    #[props(default)]
    pub width: Option<f64>,
    #[props(default)]
    pub height: Option<f64>,
    #[props(default)]
    pub fit_view: Option<bool>,
    #[props(default)]
    pub fit_view_options: Option<PtrEq<FitViewOptions>>,
    #[props(default)]
    pub min_zoom: Option<f64>,
    #[props(default)]
    pub max_zoom: Option<f64>,
    #[props(default)]
    pub node_origin: Option<rgraph_core::types::nodes::NodeOrigin>,
    #[props(default)]
    pub node_extent: Option<rgraph_core::types::geometry::CoordinateExtent>,
    #[props(default)]
    pub z_index_mode: Option<rgraph_core::types::viewport::ZIndexMode>,
    pub children: Element,
}

/// `<Wrapper>` — auto-injects an [`RGraphProvider`] when one isn't
/// already mounted above the caller, otherwise renders children as-is.
#[component]
pub fn Wrapper<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: WrapperProps<N, E>,
) -> Element {
    // Check for an existing store in context. The TS source reads the
    // `StoreContext` value; in Dioxus we call `try_use_rgraph_store`
    // (the `try_*` variant returns `None` when no provider is mounted).
    let is_wrapped: Option<RGraphStore<N, E>> = try_use_rgraph_store::<N, E>();

    if is_wrapped.is_some() {
        return rsx! { {props.children} };
    }

    rsx! {
        RGraphProvider::<N, E> {
            initial_nodes: props.nodes.clone(),
            initial_edges: props.edges.clone(),
            default_nodes: props.default_nodes.clone(),
            default_edges: props.default_edges.clone(),
            initial_width: props.width,
            initial_height: props.height,
            fit_view: props.fit_view,
            initial_fit_view_options: props.fit_view_options.clone(),
            initial_min_zoom: props.min_zoom,
            initial_max_zoom: props.max_zoom,
            node_origin: props.node_origin,
            node_extent: props.node_extent,
            z_index_mode: props.z_index_mode,
            {props.children}
        }
    }
}
