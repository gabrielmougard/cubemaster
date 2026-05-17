//! Port of `xyflow-react/src/hooks/useEdges.ts`.
//!
//! Status: Phase 3 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::ReadableExt;

use crate::context::use_rgraph_store;
use crate::types::edges::Edge;

/// Returns the current array of edges. Mirrors the TS `useEdges`.
///
/// Components that call this hook re-render whenever any edge
/// changes. The shallow-equality short-circuit is applied
/// automatically by the underlying `Signal<Vec<Edge<E>>>`.
#[must_use]
pub fn use_edges<N, E>() -> Vec<Edge<E>>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();
    store.edges.read().clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn use_edges_reads_initial_edges() {
        thread_local! { static COUNT: Cell<usize> = const { Cell::new(0) }; }

        #[component]
        fn Probe() -> Element {
            let e = use_edges::<(), ()>();
            COUNT.with(|c| c.set(e.len()));
            rsx! { div {} }
        }
        fn Root() -> Element {
            rsx! {
                RGraphProvider::<(), ()> {
                    initial_edges: vec![Edge::<()>::minimal("e1", "a", "b")],
                    Probe {}
                }
            }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert_eq!(COUNT.with(|c| c.get()), 1);
    }
}
