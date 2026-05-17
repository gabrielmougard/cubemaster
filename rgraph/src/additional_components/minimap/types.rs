//! Port of `xyflow-react/src/additional-components/MiniMap/types.ts`.
//!
//! Status: Phase 8 — implemented.

#![allow(clippy::module_name_repetitions)]

use std::rc::Rc;

use rgraph_core::types::nodes::InternalNode;

/// Either a static string or a function `(node) -> String`. Mirrors
/// `string | GetMiniMapNodeAttribute<NodeType>` in TS.
pub enum MiniMapNodeAttr<N: Clone + 'static> {
    Static(String),
    Dynamic(Rc<dyn Fn(&InternalNode<N>) -> String>),
}

impl<N: Clone + 'static> Clone for MiniMapNodeAttr<N> {
    fn clone(&self) -> Self {
        match self {
            MiniMapNodeAttr::Static(s) => MiniMapNodeAttr::Static(s.clone()),
            MiniMapNodeAttr::Dynamic(f) => MiniMapNodeAttr::Dynamic(Rc::clone(f)),
        }
    }
}

impl<N: Clone + 'static> PartialEq for MiniMapNodeAttr<N> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MiniMapNodeAttr::Static(a), MiniMapNodeAttr::Static(b)) => a == b,
            (MiniMapNodeAttr::Dynamic(a), MiniMapNodeAttr::Dynamic(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl<N: Clone + 'static> MiniMapNodeAttr<N> {
    pub fn resolve(&self, node: &InternalNode<N>) -> String {
        match self {
            MiniMapNodeAttr::Static(s) => s.clone(),
            MiniMapNodeAttr::Dynamic(f) => f(node),
        }
    }
}

impl<N: Clone + 'static> From<&str> for MiniMapNodeAttr<N> {
    fn from(s: &str) -> Self {
        MiniMapNodeAttr::Static(s.to_string())
    }
}

impl<N: Clone + 'static> From<String> for MiniMapNodeAttr<N> {
    fn from(s: String) -> Self {
        MiniMapNodeAttr::Static(s)
    }
}
