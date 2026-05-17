//! Port of `xyflow-react/src/hooks/useOnNodesChangeMiddleware.ts`.
//!
//! Status: Phase 3 — implemented.

#![allow(clippy::module_name_repetitions)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use dioxus::prelude::{use_drop, use_hook, ReadableExt, WritableExt};

use rgraph_core::types::changes::NodeChange;

use crate::context::use_rgraph_store;

static NEXT_MIDDLEWARE_ID: AtomicU64 = AtomicU64::new(1);

/// Registers a node-change middleware function that transforms every
/// `Vec<NodeChange<N>>` before it reaches `on_nodes_change`.
///
/// Mirrors the TS `experimental_useOnNodesChangeMiddleware`. The
/// middleware id is reserved at mount time (a per-process atomic
/// `u64`, mirroring the TS `Symbol()` reservation in `useState`),
/// installed/replaced on every render, and removed on unmount.
///
/// `fn_` should be a `'static` closure, ideally cheap to construct on
/// every render — re-installation is a `HashMap::insert` so the cost
/// is constant.
pub fn experimental_use_on_nodes_change_middleware<N, E, F>(fn_: F)
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
    F: Fn(Vec<NodeChange<N>>) -> Vec<NodeChange<N>> + 'static,
{
    let store = use_rgraph_store::<N, E>();
    let id = use_hook(|| NEXT_MIDDLEWARE_ID.fetch_add(1, Ordering::Relaxed));

    let arc: Arc<dyn Fn(Vec<NodeChange<N>>) -> Vec<NodeChange<N>>> = Arc::new(fn_);
    store
        .on_nodes_change_middleware_map
        .clone()
        .write()
        .insert(id, arc);

    // On unmount, drop the middleware registration.
    use_drop(move || {
        store
            .on_nodes_change_middleware_map
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
            experimental_use_on_nodes_change_middleware::<(), (), _>(|c| c);
            let store = use_rgraph_store::<(), ()>();
            SIZE.with(|c| c.set(store.on_nodes_change_middleware_map.read().len()));
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
