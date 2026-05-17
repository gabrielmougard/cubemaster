//! Port of `xyflow-react/src/hooks/useNodeConnections.ts`.
//!
//! Status: Phase 3 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::ReadableExt;

use rgraph_core::types::connection::{HandleConnection, NodeConnection};
use rgraph_core::types::handles::HandleType;

use crate::context::use_rgraph_store;
use crate::contexts::node_id::use_node_id;

/// Parameters accepted by [`use_node_connections`].
///
/// Mirrors the TS `UseNodeConnectionsParams`. The `on_connect` /
/// `on_disconnect` callbacks are deferred to Phase 4 because they
/// require a stable diff observer over the connection map.
#[derive(Default, Clone)]
pub struct UseNodeConnectionsParams {
    /// ID of the node. When `None`, the id is read from
    /// [`crate::contexts::node_id::use_node_id`] (i.e. the enclosing
    /// custom node).
    pub id: Option<String>,
    /// Filter by handle type.
    pub handle_type: Option<HandleType>,
    /// Filter by handle id.
    pub handle_id: Option<String>,
}

fn type_str(t: HandleType) -> &'static str {
    match t {
        HandleType::Source => "source",
        HandleType::Target => "target",
    }
}

fn lookup_key(node_id: &str, handle_type: Option<HandleType>, handle_id: Option<&str>) -> String {
    match (handle_type, handle_id) {
        (Some(t), Some(id)) => format!("{node_id}-{}-{id}", type_str(t)),
        (Some(t), None) => format!("{node_id}-{}", type_str(t)),
        (None, _) => node_id.to_string(),
    }
}

/// Convert a [`HandleConnection`] to a [`NodeConnection`]. They are
/// shape-equivalent in `rgraph-core`; the conversion exists so callers
/// get the type they expect from this hook (matching the TS surface).
fn handle_to_node(c: &HandleConnection) -> NodeConnection {
    NodeConnection {
        connection: c.connection.clone(),
        edge_id: c.edge_id.clone(),
    }
}

/// Returns an array of [`NodeConnection`]s for a given node, optionally
/// filtered by handle type / handle id.
///
/// Mirrors the TS `useNodeConnections({ id, handleType, handleId })`.
/// Panics when no node id is supplied via param or context (TS throws
/// `error014`).
#[must_use]
pub fn use_node_connections<N, E>(params: UseNodeConnectionsParams) -> Vec<NodeConnection>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let context_id = use_node_id();
    let node_id = params
        .id
        .clone()
        .or(context_id)
        .expect("rgraph: useNodeConnections needs a node id (either via params.id or NodeIdContext)");

    let store = use_rgraph_store::<N, E>();
    let key = lookup_key(&node_id, params.handle_type, params.handle_id.as_deref());
    let lookup = store.connection_lookup.read();
    lookup
        .get(&key)
        .map(|map| map.values().map(handle_to_node).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_key_format_matches_ts() {
        assert_eq!(lookup_key("a", None, None), "a");
        assert_eq!(
            lookup_key("a", Some(HandleType::Source), None),
            "a-source"
        );
        assert_eq!(
            lookup_key("a", Some(HandleType::Target), Some("h1")),
            "a-target-h1"
        );
        // When `handle_type` is None, `handle_id` is ignored — same
        // as TS lines 70 (the conditional pattern only includes
        // `handleId` when `handleType` is truthy).
        assert_eq!(
            lookup_key("a", None, Some("h1")),
            "a"
        );
    }
}
