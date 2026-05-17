//! Port of `xyflow-react/src/hooks/useNodesEdgesState.ts`.
//!
//! Status: Phase 3 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::{use_callback, use_signal, Signal, WritableExt};

use rgraph_core::types::changes::{EdgeChange, NodeChange};

use crate::types::edges::Edge;
use crate::types::general::{OnEdgesChange, OnNodesChange};
use crate::types::nodes::Node;
use crate::utils::changes::{apply_edge_changes, apply_node_changes};

/// `(nodes_signal, on_nodes_change)` pair returned by
/// [`use_nodes_state`]. The TS hook returns three values
/// `(nodes, setNodes, onNodesChange)`; in Dioxus the first two collapse
/// into a single `Signal<Vec<Node<N>>>` (which is read with `.read()`
/// and written with `.set(..)`).
pub struct UseNodesState<N: Clone + PartialEq + 'static> {
    /// Current nodes signal. Read with `nodes.read()`, write with
    /// `nodes.set(..)`. Equivalent to TS `[nodes, setNodes]`.
    pub nodes: Signal<Vec<Node<N>>>,
    /// `on_nodes_change` callback ready to be plugged into
    /// `<RGraph on_nodes_change=…>`. Equivalent to TS
    /// `[, , onNodesChange]`.
    pub on_nodes_change: OnNodesChange<N>,
}

/// Returns a controlled-flow node signal plus a default
/// `on_nodes_change` handler that applies incoming changes via
/// [`apply_node_changes`].
///
/// Mirrors TS `useNodesState(initialNodes)`.
#[must_use]
pub fn use_nodes_state<N>(initial_nodes: Vec<Node<N>>) -> UseNodesState<N>
where
    N: Clone + PartialEq + 'static,
{
    let nodes = use_signal(|| initial_nodes);
    let on_nodes_change: OnNodesChange<N> =
        use_callback(move |changes: Vec<NodeChange<N>>| {
            let current = nodes.peek().clone();
            nodes.clone().set(apply_node_changes(changes, current));
        });
    UseNodesState { nodes, on_nodes_change }
}

/// Edge counterpart of [`UseNodesState`].
pub struct UseEdgesState<E: Clone + PartialEq + 'static> {
    pub edges: Signal<Vec<Edge<E>>>,
    pub on_edges_change: OnEdgesChange<E>,
}

/// Returns a controlled-flow edge signal plus a default
/// `on_edges_change` handler. Mirrors TS `useEdgesState(initialEdges)`.
#[must_use]
pub fn use_edges_state<E>(initial_edges: Vec<Edge<E>>) -> UseEdgesState<E>
where
    E: Clone + PartialEq + 'static,
{
    let edges = use_signal(|| initial_edges);
    let on_edges_change: OnEdgesChange<E> =
        use_callback(move |changes: Vec<EdgeChange<E>>| {
            let current = edges.peek().clone();
            edges.clone().set(apply_edge_changes(changes, current));
        });
    UseEdgesState { edges, on_edges_change }
}

#[allow(unused_imports)]
use dioxus::prelude::ReadableExt;

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn use_nodes_state_handles_apply_node_changes() {
        thread_local! { static FINAL_LEN: Cell<usize> = const { Cell::new(0) }; }

        fn Root() -> Element {
            let UseNodesState { nodes, on_nodes_change } =
                use_nodes_state::<()>(vec![Node::<()>::minimal("a", 0.0, 0.0)]);

            // Drive a change.
            on_nodes_change.call(vec![NodeChange::Add {
                item: Node::<()>::minimal("b", 1.0, 1.0),
                index: None,
            }]);
            FINAL_LEN.with(|c| c.set(nodes.peek().len()));
            rsx! { div {} }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert_eq!(FINAL_LEN.with(|c| c.get()), 2);
    }

    #[test]
    fn use_edges_state_handles_apply_edge_changes() {
        thread_local! { static FINAL_LEN: Cell<usize> = const { Cell::new(0) }; }

        fn Root() -> Element {
            let UseEdgesState { edges, on_edges_change } =
                use_edges_state::<()>(vec![Edge::<()>::minimal("e1", "a", "b")]);
            on_edges_change.call(vec![EdgeChange::Remove { id: "e1".into() }]);
            FINAL_LEN.with(|c| c.set(edges.peek().len()));
            rsx! { div {} }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert_eq!(FINAL_LEN.with(|c| c.get()), 0);
    }
}
