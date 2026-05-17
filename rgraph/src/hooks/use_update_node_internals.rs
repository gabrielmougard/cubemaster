//! Port of `xyflow-react/src/hooks/useUpdateNodeInternals.ts`.
//!
//! Status: Phase 5 — implemented.
//!
//! The TS hook uses `domNode.querySelector('.react-flow__node[data-id="…"]')`
//! plus `requestAnimationFrame` to schedule a re-measurement of the
//! affected nodes. We do the same via the
//! [`crate::dom::resize_observer`] shim: the shim has been observing
//! every mounted node wrapper since its `<NodeWrapper>` `onmounted`
//! callback fired, so we can fetch the latest dimensions by node id
//! and dispatch a synthetic measurement update through
//! [`crate::components::node_wrapper::use_node_observer::apply_dimension_update`].

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::spawn;

use rgraph_core::types::geometry::Dimensions;

use crate::components::node_wrapper::use_node_observer::apply_dimension_update;
use crate::context::use_rgraph_store;
use crate::dom::resize_observer;
use crate::store::RGraphStore;

/// Closure-like handle returned by [`use_update_node_internals`].
pub struct UpdateNodeInternals<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> {
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
    /// Asynchronously queries the JS-bridged ResizeObserver shim for
    /// each id's latest `(width, height)`, then writes the dimensions
    /// into the store via
    /// [`apply_dimension_update`]. Skipped ids
    /// (those without a recorded measurement) are silently ignored —
    /// they're typically nodes that haven't mounted yet.
    pub fn call(&self, ids: &[&str]) {
        let store = self.store;
        let owned_ids: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
        spawn(async move {
            for id in owned_ids {
                let Some(size) = resize_observer::get_size(&id).await else {
                    continue;
                };
                if size.width <= 0.0 || size.height <= 0.0 {
                    continue;
                }
                apply_dimension_update(
                    store,
                    &id,
                    Dimensions {
                        width: size.width,
                        height: size.height,
                    },
                );
            }
        });
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
