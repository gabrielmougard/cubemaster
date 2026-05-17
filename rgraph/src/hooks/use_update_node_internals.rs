//! Port of `xyflow-react/src/hooks/useUpdateNodeInternals.ts`.
//!
//! Status: Phase 3 — partial implementation.
//!
//! The TS hook uses `domNode.querySelector('.react-flow__node[data-id="…"]')`
//! plus `requestAnimationFrame` to schedule a re-measurement of the
//! affected nodes. Both pieces require the Phase 5 `dom::resize_observer`
//! bridge to function correctly.
//!
//! For Phase 3 we ship the public hook signature plus a no-op body
//! that emits a `tracing::debug!` call when invoked, so downstream
//! code can take a dependency without breaking. Phase 5 will implement
//! the real measurement pipeline by:
//!
//! 1. Resolving the node's DOM element via the `dom::resize_observer`
//!    handle map (keyed by node id).
//! 2. Building a `utils::store::InternalNodeUpdate` per id with the
//!    measured dimensions.
//! 3. Dispatching them through `RGraphStore::update_node_internals`.

#![allow(clippy::module_name_repetitions)]

use crate::context::use_rgraph_store;
use crate::store::RGraphStore;

/// Closure-like handle returned by [`use_update_node_internals`].
pub struct UpdateNodeInternals<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> {
    #[allow(dead_code)]
    store: RGraphStore<N, E>,
}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Copy for UpdateNodeInternals<N, E> {}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Clone for UpdateNodeInternals<N, E> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> UpdateNodeInternals<N, E> {
    /// Schedule a re-measurement of the supplied node ids.
    ///
    /// **Phase 3 caveat**: this is a no-op until Phase 5 wires the DOM
    /// measurement pipeline. The call is logged at the `debug` level
    /// for observability during the porting effort.
    pub fn call(&self, ids: &[&str]) {
        tracing::debug!(
            target: "rgraph::hooks::use_update_node_internals",
            ids = ?ids,
            "update_node_internals scheduled (no-op until Phase 5)"
        );
        // TODO(rgraph/phase5): build the InternalNodeUpdate batch and
        // call `self.store.update_node_internals(batch)` after the next
        // animation frame.
    }
}

/// Returns an [`UpdateNodeInternals`] handle bound to the current
/// store. Mirrors the TS `useUpdateNodeInternals`.
#[must_use]
pub fn use_update_node_internals<N, E>() -> UpdateNodeInternals<N, E>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();
    UpdateNodeInternals { store }
}
