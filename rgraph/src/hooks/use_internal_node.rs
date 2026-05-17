//! Port of `xyflow-react/src/hooks/useInternalNode.ts`.
//!
//! Status: Phase 3 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::ReadableExt;

use crate::context::use_rgraph_store;
use crate::types::nodes::InternalNode;

/// Returns the [`InternalNode`] for the given id, or `None` when it is
/// not in the lookup.
///
/// Mirrors the TS `useInternalNode(id)`. Components that call this
/// hook re-render whenever any node changes (the underlying
/// `Signal<NodeLookup<N>>` cannot do per-key equality, so reads
/// subscribe to the whole lookup — same as the TS selector with
/// `shallow`).
#[must_use]
pub fn use_internal_node<N, E>(id: &str) -> Option<InternalNode<N>>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();
    store.node_lookup.read().get(id).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn use_internal_node_returns_internal_for_existing_id() {
        thread_local! { static FOUND: Cell<bool> = const { Cell::new(false) }; }

        #[component]
        fn Probe() -> Element {
            let n = use_internal_node::<(), ()>("a");
            FOUND.with(|c| c.set(n.is_some()));
            rsx! { div {} }
        }
        fn Root() -> Element {
            rsx! {
                RGraphProvider::<(), ()> {
                    initial_nodes: vec![crate::types::nodes::Node::<()>::minimal("a", 0.0, 0.0)],
                    Probe {}
                }
            }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(FOUND.with(|c| c.get()));
    }

    #[test]
    fn use_internal_node_returns_none_for_unknown_id() {
        thread_local! { static FOUND: Cell<bool> = const { Cell::new(true) }; }

        #[component]
        fn Probe() -> Element {
            let n = use_internal_node::<(), ()>("missing");
            FOUND.with(|c| c.set(n.is_some()));
            rsx! { div {} }
        }
        fn Root() -> Element {
            rsx! { RGraphProvider::<(), ()> { Probe {} } }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(!FOUND.with(|c| c.get()));
    }
}
