//! Port of `xyflow-react/src/hooks/useVisibleEdgeIds.ts`.
//!
//! Status: Phase 3 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::ReadableExt;

use rgraph_core::utils::edges::general::{is_edge_visible, IsEdgeVisibleParams};

use crate::context::use_rgraph_store;

/// Returns the ids of edges that are currently visible (when
/// `only_render_visible` is `true`), or every edge id in the array
/// otherwise.
///
/// Mirrors the TS `useVisibleEdgeIds`.
#[must_use]
pub fn use_visible_edge_ids<N, E>(only_render_visible: bool) -> Vec<String>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();
    let edges = store.edges.read();

    if !only_render_visible {
        return edges.iter().map(|e| e.id.clone()).collect();
    }

    let width = *store.width.read();
    let height = *store.height.read();
    if width == 0.0 || height == 0.0 {
        return Vec::new();
    }

    let lookup = store.node_lookup.read();
    let transform = *store.transform.read();
    let mut visible = Vec::with_capacity(edges.len());
    for edge in edges.iter() {
        let Some(source) = lookup.get(&edge.source) else { continue; };
        let Some(target) = lookup.get(&edge.target) else { continue; };
        if is_edge_visible(IsEdgeVisibleParams {
            source_node: source,
            target_node: target,
            width,
            height,
            transform,
        }) {
            visible.push(edge.id.clone());
        }
    }
    visible
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use crate::types::edges::Edge;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn returns_all_ids_when_only_render_visible_is_false() {
        thread_local! { static COUNT: Cell<usize> = const { Cell::new(0) }; }

        #[component]
        fn Probe() -> Element {
            let ids = use_visible_edge_ids::<(), ()>(false);
            COUNT.with(|c| c.set(ids.len()));
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
