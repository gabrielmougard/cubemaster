//! Port of `xyflow-react/src/components/BatchProvider/types.ts`.
//!
//! Status: Phase 6 — implemented.
//!
//! Defines the queue-payload shape used by [`crate::components::batch_provider::BatchProvider`].
//! Each consumer (e.g. `useReactFlow().setNodes`) pushes a closure or
//! a fresh array into the queue; the provider drains the queue once per
//! render and applies the changes through the store.

#![allow(clippy::module_name_repetitions)]

use std::rc::Rc;

use crate::types::edges::Edge;
use crate::types::nodes::Node;

/// Single queued node update. Mirrors the TS `Node[] | ((nodes) => Node[])`.
pub enum NodeQueueItem<N: Clone + 'static = ()> {
    /// Replace the current array wholesale.
    Replace(Vec<Node<N>>),
    /// Compute the new array from the current one.
    Fn(Rc<dyn Fn(&[Node<N>]) -> Vec<Node<N>>>),
}

/// Single queued edge update. Mirrors the TS `Edge[] | ((edges) => Edge[])`.
pub enum EdgeQueueItem<E: Clone + 'static = ()> {
    Replace(Vec<Edge<E>>),
    Fn(Rc<dyn Fn(&[Edge<E>]) -> Vec<Edge<E>>>),
}
