//! Port of `xyflow-react/src/contexts/NodeIdContext.ts`.
//!
//! Status: Phase 2 — implemented.
//!
//! Provides the `NodeIdCtx` Dioxus context (an `Option<String>` wrapped
//! in a newtype to disambiguate from other `String`-shaped contexts)
//! and the `use_node_id()` hook used by `Handle`, `NodeToolbar`, etc.
//! to learn which node they belong to.
//!
//! TS reference: a tiny `createContext<string | null>(null)` plus
//! `useNodeId()`. The Rust port wraps the value in a newtype so
//! Dioxus' type-based context dispatch can find it unambiguously
//! (multiple plain-`String` contexts in the same tree would collide).

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::{try_consume_context, use_context_provider};

/// Newtype around `Option<String>` carrying the id of the nearest
/// enclosing node. `None` means "not inside a node".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeIdCtx(pub Option<String>);

impl NodeIdCtx {
    /// Convenience: borrow the id as a `&str` (returns `None` when not
    /// inside a node).
    #[inline]
    #[must_use]
    pub fn as_deref(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

impl From<String> for NodeIdCtx {
    fn from(id: String) -> Self {
        NodeIdCtx(Some(id))
    }
}

impl From<&str> for NodeIdCtx {
    fn from(id: &str) -> Self {
        NodeIdCtx(Some(id.to_string()))
    }
}

impl From<Option<String>> for NodeIdCtx {
    fn from(id: Option<String>) -> Self {
        NodeIdCtx(id)
    }
}

/// Inject a node id into Dioxus context for descendant components to
/// pick up via [`use_node_id`].
///
/// Called by `<NodeWrapper>` (Phase 5) immediately around each node's
/// rendered subtree.
pub fn provide_node_id(id: impl Into<NodeIdCtx>) -> NodeIdCtx {
    use_context_provider(|| id.into())
}

/// Returns the id of the nearest enclosing node, or `None` if the
/// caller isn't inside a node subtree.
///
/// Mirrors the TS `useNodeId()`. Implementation note: we use
/// `try_consume_context` (the non-hook variant) so this helper is safe
/// to call from non-component contexts as well — same as
/// `xyflow-react/src/contexts/NodeIdContext.ts:35` where the original
/// `useContext` happens to be safe in any consumer.
#[must_use]
pub fn use_node_id() -> Option<String> {
    try_consume_context::<NodeIdCtx>().and_then(|ctx| ctx.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn child_reads_id_from_ancestor_provider() {
        thread_local! {
            static OBSERVED: Cell<Option<&'static str>> = const { Cell::new(None) };
        }

        #[component]
        fn Child() -> Element {
            let id = use_node_id();
            if let Some(id_str) = id {
                // Lazy leak to convert to &'static for the test sink.
                let leaked: &'static str = Box::leak(id_str.into_boxed_str());
                OBSERVED.with(|c| c.set(Some(leaked)));
            }
            rsx! { div {} }
        }

        fn Root() -> Element {
            provide_node_id("n1");
            rsx! { Child {} }
        }

        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert_eq!(OBSERVED.with(|c| c.get()), Some("n1"));
    }

    #[test]
    fn use_node_id_returns_none_outside_provider() {
        thread_local! {
            static SAW_NONE: Cell<bool> = const { Cell::new(false) };
        }
        fn Root() -> Element {
            let id = use_node_id();
            SAW_NONE.with(|c| c.set(id.is_none()));
            rsx! { div {} }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(SAW_NONE.with(|c| c.get()));
    }

    #[test]
    fn newtype_conversions_work() {
        let from_str: NodeIdCtx = "abc".into();
        assert_eq!(from_str.as_deref(), Some("abc"));
        let from_string: NodeIdCtx = String::from("xyz").into();
        assert_eq!(from_string.as_deref(), Some("xyz"));
        let from_none: NodeIdCtx = Option::<String>::None.into();
        assert_eq!(from_none.as_deref(), None);
    }
}
