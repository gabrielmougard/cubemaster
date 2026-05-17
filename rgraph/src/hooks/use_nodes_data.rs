//! Port of `xyflow-react/src/hooks/useNodesData.ts`.
//!
//! Status: Phase 3 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::ReadableExt;

use crate::context::use_rgraph_store;
use crate::types::nodes::Node;

/// `{ id, type, data }` triple returned by [`use_nodes_data`].
///
/// Mirrors the TS `DistributivePick<NodeType, 'id' | 'type' | 'data'>`.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeDataView<D: Clone> {
    pub id: String,
    pub type_: Option<String>,
    pub data: D,
}

impl<D: Clone> NodeDataView<D> {
    fn from_node(node: &Node<D>) -> Self {
        NodeDataView {
            id: node.id.clone(),
            type_: node.type_.clone(),
            data: node.data.clone(),
        }
    }
}

/// Returns the `{ id, type, data }` view of a single node by id.
///
/// Mirrors the single-arg overload of TS `useNodesData(nodeId)`. The
/// hook re-renders whenever any node's data changes (the underlying
/// signal subscribes to the whole `node_lookup`).
#[must_use]
pub fn use_node_data<N, E>(node_id: &str) -> Option<NodeDataView<N>>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();
    store
        .node_lookup
        .read()
        .get(node_id)
        .map(|n| NodeDataView::from_node(&n.user))
}

/// Returns the `{ id, type, data }` views of multiple nodes by id.
///
/// Mirrors the multi-arg overload of TS `useNodesData(ids)`. Missing
/// ids are silently skipped (TS does the same: it iterates ids and
/// pushes only those it finds in the lookup).
#[must_use]
pub fn use_nodes_data<N, E>(node_ids: &[&str]) -> Vec<NodeDataView<N>>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();
    let lookup = store.node_lookup.read();
    node_ids
        .iter()
        .filter_map(|id| lookup.get(*id).map(|n| NodeDataView::from_node(&n.user)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn use_node_data_finds_existing_id() {
        thread_local! { static FOUND: Cell<bool> = const { Cell::new(false) }; }

        #[component]
        fn Probe() -> Element {
            let v = use_node_data::<(), ()>("a");
            FOUND.with(|c| c.set(v.is_some()));
            rsx! { div {} }
        }
        fn Root() -> Element {
            rsx! {
                RGraphProvider::<(), ()> {
                    initial_nodes: vec![Node::<()>::minimal("a", 0.0, 0.0)],
                    Probe {}
                }
            }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(FOUND.with(|c| c.get()));
    }

    #[test]
    fn use_nodes_data_skips_missing_ids() {
        thread_local! { static COUNT: Cell<usize> = const { Cell::new(usize::MAX) }; }

        #[component]
        fn Probe() -> Element {
            let v = use_nodes_data::<(), ()>(&["a", "missing", "b"]);
            COUNT.with(|c| c.set(v.len()));
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
