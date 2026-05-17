//! Port of `xyflow-react/src/hooks/useNodes.ts`.
//!
//! Status: Phase 3 — implemented.
//!
//! Returns a clone of the current `Vec<Node<N>>` from the store.
//! Components that call this hook re-render whenever any node changes.
//!
//! ## Difference from the TS source
//!
//! The TS `useStore(state => state.nodes, shallow)` does a shallow
//! array equality check to skip re-renders when the references inside
//! `state.nodes` are unchanged. In our port the
//! `Signal<Vec<Node<N>>>` already short-circuits when the new value
//! `==` the old one; with `Node<N>: PartialEq` (a phase-1 invariant)
//! that gives us the same behaviour automatically.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::ReadableExt;

use crate::context::use_rgraph_store;
use crate::types::nodes::Node;

/// Returns the current array of nodes. Mirrors the TS `useNodes`.
///
/// # Example
/// ```ignore
/// use rgraph::prelude::*;
///
/// fn NodeCount() -> Element {
///     let nodes: Vec<Node<()>> = use_nodes();
///     rsx! { p { "There are currently {nodes.len()} nodes!" } }
/// }
/// ```
#[must_use]
pub fn use_nodes<N, E>() -> Vec<Node<N>>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();
    store.nodes.read().clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn use_nodes_reads_initial_nodes() {
        thread_local! { static COUNT: Cell<usize> = const { Cell::new(0) }; }

        #[component]
        fn Probe() -> Element {
            let n = use_nodes::<(), ()>();
            COUNT.with(|c| c.set(n.len()));
            rsx! { div {} }
        }

        fn Root() -> Element {
            rsx! {
                RGraphProvider::<(), ()> {
                    initial_nodes: vec![
                        Node::<()>::minimal("a", 0.0, 0.0),
                        Node::<()>::minimal("b", 1.0, 1.0),
                    ],
                    Probe {}
                }
            }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert_eq!(COUNT.with(|c| c.get()), 2);
    }
}
