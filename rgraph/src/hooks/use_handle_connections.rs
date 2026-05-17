//! Port of `xyflow-react/src/hooks/useHandleConnections.ts`.
//!
//! Status: Phase 3 — implemented.
//!
//! TS-deprecated alias of [`super::use_node_connections::use_node_connections`].
//! Kept for parity; new code should use `use_node_connections`.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::ReadableExt;

use rgraph_core::types::connection::HandleConnection;
use rgraph_core::types::handles::HandleType;

use crate::context::use_rgraph_store;
use crate::contexts::node_id::use_node_id;

/// Parameters accepted by [`use_handle_connections`].
///
/// Mirrors the TS `UseHandleConnectionsParams`. `type` is required.
#[derive(Clone)]
pub struct UseHandleConnectionsParams {
    pub type_: HandleType,
    pub id: Option<String>,
    pub node_id: Option<String>,
}

/// **Deprecated** — use [`super::use_node_connections::use_node_connections`].
///
/// Returns the `HandleConnection`s for a specific handle on a node.
/// Mirrors the TS `useHandleConnections`. Emits a `tracing::warn!`
/// to mirror the TS `console.warn` deprecation notice on first call.
#[must_use]
pub fn use_handle_connections<N, E>(params: UseHandleConnectionsParams) -> Vec<HandleConnection>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    tracing::warn!(
        target: "rgraph::hooks::use_handle_connections",
        "[DEPRECATED] `use_handle_connections` is deprecated. Use \
         `use_node_connections` instead — \
         https://reactflow.dev/api-reference/hooks/useNodeConnections"
    );

    let context_id = use_node_id();
    let node_id = params
        .node_id
        .clone()
        .or(context_id)
        .expect("rgraph: useHandleConnections needs a node id");

    let key = match (&params.type_, &params.id) {
        (t, Some(id)) => format!(
            "{node_id}-{}-{id}",
            match t {
                HandleType::Source => "source",
                HandleType::Target => "target",
            }
        ),
        (t, None) => format!(
            "{node_id}-{}",
            match t {
                HandleType::Source => "source",
                HandleType::Target => "target",
            }
        ),
    };

    let store = use_rgraph_store::<N, E>();
    let lookup = store.connection_lookup.read();
    lookup
        .get(&key)
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default()
}
