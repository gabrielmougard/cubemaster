//! Port of `xyflow-react/src/hooks/useOnEdgesChangeMiddleware.ts`.
//!
//! Status: Phase 3 — implemented. Mirrors
//! [`super::use_on_nodes_change_middleware::experimental_use_on_nodes_change_middleware`]
//! but for the edge change pipeline.

#![allow(clippy::module_name_repetitions)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use dioxus::prelude::{use_drop, use_hook, ReadableExt, WritableExt};

use rgraph_core::types::changes::EdgeChange;

use crate::context::use_rgraph_store;

static NEXT_MIDDLEWARE_ID: AtomicU64 = AtomicU64::new(1);

/// Registers an edge-change middleware. Mirrors the TS
/// `experimental_useOnEdgesChangeMiddleware`.
pub fn experimental_use_on_edges_change_middleware<N, E, F>(fn_: F)
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
    F: Fn(Vec<EdgeChange<E>>) -> Vec<EdgeChange<E>> + 'static,
{
    let store = use_rgraph_store::<N, E>();
    let id = use_hook(|| NEXT_MIDDLEWARE_ID.fetch_add(1, Ordering::Relaxed));

    let arc: Arc<dyn Fn(Vec<EdgeChange<E>>) -> Vec<EdgeChange<E>>> = Arc::new(fn_);
    store
        .on_edges_change_middleware_map
        .clone()
        .write()
        .insert(id, arc);

    use_drop(move || {
        store
            .on_edges_change_middleware_map
            .clone()
            .write()
            .remove(&id);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn middleware_registered_on_mount() {
        thread_local! { static SIZE: Cell<usize> = const { Cell::new(0) }; }

        #[component]
        fn Probe() -> Element {
            experimental_use_on_edges_change_middleware::<(), (), _>(|c| c);
            let store = use_rgraph_store::<(), ()>();
            SIZE.with(|c| c.set(store.on_edges_change_middleware_map.read().len()));
            rsx! { div {} }
        }
        fn Root() -> Element {
            rsx! { RGraphProvider::<(), ()> { Probe {} } }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(SIZE.with(|c| c.get()) >= 1);
    }
}
