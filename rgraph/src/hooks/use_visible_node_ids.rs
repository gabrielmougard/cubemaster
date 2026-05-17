//! Port of `xyflow-react/src/hooks/useVisibleNodeIds.ts`.
//!
//! Status: Phase 3 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::ReadableExt;

use rgraph_core::types::geometry::Rect;
use rgraph_core::utils::graph::{get_nodes_inside, GetNodesInsideParams};

use crate::context::use_rgraph_store;

/// Returns the ids of nodes that are currently visible inside the
/// viewport (when `only_render_visible` is `true`), or every node id
/// in the lookup otherwise.
///
/// Mirrors the TS `useVisibleNodeIds`.
#[must_use]
pub fn use_visible_node_ids<N, E>(only_render_visible: bool) -> Vec<String>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();
    let lookup = store.node_lookup.read();

    if !only_render_visible {
        return lookup.keys().cloned().collect();
    }

    let width = *store.width.read();
    let height = *store.height.read();
    let transform = *store.transform.read();
    let visible = get_nodes_inside(
        &lookup,
        Rect::new(0.0, 0.0, width, height),
        transform,
        GetNodesInsideParams {
            partially: true,
            ..GetNodesInsideParams::default()
        },
    );
    visible.into_iter().map(|n| n.user.id.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use crate::types::nodes::Node;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn returns_all_ids_when_only_render_visible_is_false() {
        thread_local! { static COUNT: Cell<usize> = const { Cell::new(0) }; }

        #[component]
        fn Probe() -> Element {
            let ids = use_visible_node_ids::<(), ()>(false);
            COUNT.with(|c| c.set(ids.len()));
            rsx! { div {} }
        }
        fn Root() -> Element {
            rsx! {
                RGraphProvider::<(), ()> {
                    initial_nodes: vec![
                        Node::<()>::minimal("a", 0.0, 0.0),
                        Node::<()>::minimal("b", 1.0, 1.0),
                    ],
                    Probe {}
                }
            }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert_eq!(COUNT.with(|c| c.get()), 2);
    }
}
