//! Port of `xyflow-react/src/hooks/useConnection.ts`.
//!
//! Status: Phase 3 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::ReadableExt;

use rgraph_core::types::connection::ConnectionState;
use rgraph_core::utils::general::point_to_renderer_point;

use crate::context::use_rgraph_store;
use crate::types::nodes::InternalNode;

/// Returns the current [`ConnectionState`].
///
/// Mirrors the TS `useConnection`. When a connection is in progress,
/// the `to` field is converted from screen-space to flow-space using
/// the current viewport transform — same as TS lines 8–11.
///
/// Components that call this hook re-render whenever the connection
/// state or the viewport transform changes.
#[must_use]
pub fn use_connection<N, E>() -> ConnectionState<InternalNode<N>>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();
    let connection = store.connection.read().clone();
    match connection {
        ConnectionState::NoConnection => ConnectionState::NoConnection,
        ConnectionState::InProgress(mut p) => {
            let transform = *store.transform.read();
            // `to` is in screen-space while a connection is being
            // drawn; convert to renderer (flow) space.
            p.to = point_to_renderer_point(p.to, transform, false, (1.0, 1.0));
            ConnectionState::InProgress(p)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn returns_no_connection_by_default() {
        thread_local! { static IS_NO_CONN: Cell<bool> = const { Cell::new(false) }; }

        #[component]
        fn Probe() -> Element {
            let c = use_connection::<(), ()>();
            IS_NO_CONN.with(|x| x.set(matches!(c, ConnectionState::NoConnection)));
            rsx! { div {} }
        }
        fn Root() -> Element {
            rsx! { RGraphProvider::<(), ()> { Probe {} } }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(IS_NO_CONN.with(|c| c.get()));
    }
}
