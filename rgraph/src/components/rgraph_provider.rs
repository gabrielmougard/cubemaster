//! Port of `xyflow-react/src/components/ReactFlowProvider/index.tsx`.
//!
//! Status: Phase 2 — implemented.
//!
//! The `RGraphProvider<N, E>` component creates an [`RGraphStore`] and
//! injects it into Dioxus context so descendants (including
//! `<RGraph>`) can use the hooks. Equivalent of `<ReactFlowProvider>`.
//!
//! ## Differences from the TS source
//!
//! * `<BatchProvider>` is part of Phase 6 (queue + frame-flush). For
//!   now the provider renders its children directly.
//! * In TS the `useState(() => createStore({...}))` runs once;
//!   we use [`dioxus::prelude::use_hook`] for the same one-shot
//!   semantics.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use crate::context::provide_rgraph_store;
use crate::store::{InitialStateParams, RGraphStore};
use crate::types::edges::Edge;
use crate::types::nodes::Node;
use crate::utils::general::PtrEq;

/// Props for [`RGraphProvider`]. Mirrors the TS `ReactFlowProviderProps`.
///
/// Every field is optional except `children`. Defaults mirror the TS
/// source.
#[derive(Props, Clone, PartialEq)]
pub struct RGraphProviderProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    /// Initial controlled nodes (TS `initialNodes`).
    #[props(default)]
    pub initial_nodes: Option<Vec<Node<N>>>,
    /// Initial controlled edges (TS `initialEdges`).
    #[props(default)]
    pub initial_edges: Option<Vec<Edge<E>>>,
    /// Initial uncontrolled nodes (TS `defaultNodes`).
    #[props(default)]
    pub default_nodes: Option<Vec<Node<N>>>,
    /// Initial uncontrolled edges (TS `defaultEdges`).
    #[props(default)]
    pub default_edges: Option<Vec<Edge<E>>>,
    /// Initial viewport width (TS `initialWidth`).
    #[props(default)]
    pub initial_width: Option<f64>,
    /// Initial viewport height (TS `initialHeight`).
    #[props(default)]
    pub initial_height: Option<f64>,
    /// When `true`, the flow will be zoomed and panned to fit all the
    /// nodes initially provided.
    #[props(default)]
    pub fit_view: Option<bool>,
    /// Options customising the initial `fit_view`. Wrapped in `PtrEq`
    /// because `FitViewOptions` carries a non-`Clone`/`PartialEq`
    /// boxed `EaseFn`.
    #[props(default)]
    pub initial_fit_view_options: Option<PtrEq<crate::types::general::FitViewOptions>>,
    /// Initial minimum zoom level (TS `initialMinZoom`).
    #[props(default)]
    pub initial_min_zoom: Option<f64>,
    /// Initial maximum zoom level (TS `initialMaxZoom`).
    #[props(default)]
    pub initial_max_zoom: Option<f64>,
    /// Node origin. Default `(0.0, 0.0)`.
    #[props(default)]
    pub node_origin: Option<rgraph_core::types::nodes::NodeOrigin>,
    /// Node extent. Default infinite.
    #[props(default)]
    pub node_extent: Option<rgraph_core::types::geometry::CoordinateExtent>,
    /// `ZIndexMode`. Default `Basic`.
    #[props(default)]
    pub z_index_mode: Option<rgraph_core::types::viewport::ZIndexMode>,
    /// Children rendered inside the provider's subtree.
    pub children: Element,
}

/// The `<RGraphProvider />` component is a context provider that makes
/// it possible to access a flow's internal state outside of the
/// [`crate::container::rgraph::mod_RGraph`] component. Many of the
/// hooks rely on this component to work.
///
/// Mirrors the TS `ReactFlowProvider`. The store is created once on
/// first render via `use_hook` (the analogue of React's
/// `useState(() => createStore(...))`).
///
/// # Example
/// ```ignore
/// use rgraph::prelude::*;
/// use rgraph::components::rgraph_provider::RGraphProvider;
/// use dioxus::prelude::*;
///
/// fn App() -> Element {
///     rsx! {
///         RGraphProvider::<(), ()> {
///             initial_nodes: vec![Node::<()>::minimal("n1", 0.0, 0.0)],
///             // <RGraph /> and any other consumer hooks go here:
///             div { "graph mounts here" }
///         }
///     }
/// }
/// ```
#[component]
pub fn RGraphProvider<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: RGraphProviderProps<N, E>,
) -> Element {
    // Build the store exactly once for this provider's lifetime. The
    // TS source uses `useState(() => createStore(...))`; `use_hook`
    // has the same "run on first render only" semantics.
    let store: RGraphStore<N, E> = use_hook(|| {
        RGraphStore::new(InitialStateParams {
            nodes: props.initial_nodes.clone(),
            edges: props.initial_edges.clone(),
            default_nodes: props.default_nodes.clone(),
            default_edges: props.default_edges.clone(),
            width: props.initial_width,
            height: props.initial_height,
            fit_view: props.fit_view,
            fit_view_options: props.initial_fit_view_options.clone(),
            min_zoom: props.initial_min_zoom,
            max_zoom: props.initial_max_zoom,
            node_origin: props.node_origin,
            node_extent: props.node_extent,
            z_index_mode: props.z_index_mode,
        })
    });

    provide_rgraph_store(store);

    // TODO(rgraph/phase6): wrap children in `<BatchProvider>` to coalesce
    // sub-render store writes. For Phase 2 we render children directly.
    rsx! { {props.children} }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::use_rgraph_store;
    use std::cell::Cell;

    /// Smoke test: mount `<RGraphProvider>` with one initial node, read
    /// the store from a child, and confirm the node made it through.
    #[test]
    fn provider_injects_store_with_initial_nodes() {
        thread_local! {
            static OBSERVED_COUNT: Cell<usize> = const { Cell::new(usize::MAX) };
        }

        #[component]
        fn Probe() -> Element {
            use dioxus_signals::ReadableExt;
            let store: RGraphStore<(), ()> = use_rgraph_store();
            OBSERVED_COUNT.with(|c| c.set(store.nodes.peek().len()));
            rsx! { div {} }
        }

        fn Root() -> Element {
            rsx! {
                RGraphProvider::<(), ()> {
                    initial_nodes: vec![
                        Node::<()>::minimal("n1", 0.0, 0.0),
                        Node::<()>::minimal("n2", 10.0, 10.0),
                    ],
                    Probe {}
                }
            }
        }

        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert_eq!(OBSERVED_COUNT.with(|c| c.get()), 2);
    }
}
