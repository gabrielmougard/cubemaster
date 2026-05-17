//! Port of `xyflow-react/src/hooks/useNodesInitialized.ts`.
//!
//! Status: Phase 3 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::ReadableExt;

use rgraph_core::utils::general::node_has_dimensions;

use crate::context::use_rgraph_store;

/// Options accepted by [`use_nodes_initialized`].
///
/// Mirrors the TS `UseNodesInitializedOptions`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UseNodesInitializedOptions {
    /// When `true`, hidden nodes are also required to have dimensions
    /// before the hook returns `true`. Defaults to `false`.
    pub include_hidden_nodes: bool,
}

/// Returns `true` once every node in the flow has been measured and
/// given a width and height.
///
/// Mirrors the TS `useNodesInitialized`. When
/// `include_hidden_nodes = false` (the default) the hook simply reads
/// `store.nodes_initialized`. When `true` it walks the whole
/// `node_lookup` and requires both `handle_bounds.is_some()` and
/// `node_has_dimensions(user)` for every node — same as TS lines
/// 11–27.
#[must_use]
pub fn use_nodes_initialized<N, E>(options: UseNodesInitializedOptions) -> bool
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();

    if !options.include_hidden_nodes {
        return *store.nodes_initialized.read();
    }

    let lookup = store.node_lookup.read();
    if lookup.is_empty() {
        return false;
    }

    for internal in lookup.values() {
        if internal.internals.handle_bounds.is_none() {
            return false;
        }
        if !node_has_dimensions(&internal.user) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use crate::types::nodes::Node;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn returns_false_when_no_nodes() {
        thread_local! { static V: Cell<bool> = const { Cell::new(true) }; }

        #[component]
        fn Probe() -> Element {
            let b = use_nodes_initialized::<(), ()>(UseNodesInitializedOptions::default());
            V.with(|c| c.set(b));
            rsx! { div {} }
        }
        fn Root() -> Element {
            rsx! { RGraphProvider::<(), ()> { Probe {} } }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(!V.with(|c| c.get()));
    }

    #[test]
    fn returns_true_when_all_nodes_measured() {
        thread_local! { static V: Cell<bool> = const { Cell::new(false) }; }

        #[component]
        fn Probe() -> Element {
            let b = use_nodes_initialized::<(), ()>(UseNodesInitializedOptions::default());
            V.with(|c| c.set(b));
            rsx! { div {} }
        }
        fn Root() -> Element {
            let mut n = Node::<()>::minimal("a", 0.0, 0.0);
            n.measured = Some(rgraph_core::types::nodes::MeasuredDimensions {
                width: Some(10.0),
                height: Some(10.0),
            });
            rsx! {
                RGraphProvider::<(), ()> {
                    initial_nodes: vec![n],
                    Probe {}
                }
            }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(V.with(|c| c.get()));
    }
}
