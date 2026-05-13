//! Port of connection-related types from `xyflow-core/src/types/general.ts`.
//!
//! Status: implemented (phase 1).
//!
//! Covers `Connection`, `HandleConnection`, `NodeConnection`,
//! `ConnectionMode`, `OnConnectStartParams`, `IsValidConnection`, and
//! the [`ConnectionState`] / [`ConnectionInProgress`] pair, plus the
//! [`ConnectionLookup`] alias used by the store.

#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::types::geometry::{Position, XYPosition};
use crate::types::handles::{Handle, HandleType};

/// Minimal description of an edge between two nodes.
///
/// The `add_edge` utility upgrades a [`Connection`] into a full
/// [`crate::types::edges::Edge`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Connection {
    /// Id of the node this connection originates from.
    pub source: String,
    /// Id of the node this connection terminates at.
    pub target: String,
    /// When `Some`, the id of the handle on the source node that this
    /// connection originates from.
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_handle: Option<String>,
    /// When `Some`, the id of the handle on the target node that this
    /// connection terminates at.
    #[cfg_attr(feature = "serde", serde(default))]
    pub target_handle: Option<String>,
}

/// Extension of a basic [`Connection`] that includes the `edge_id`,
/// keyed by handle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HandleConnection {
    pub connection: Connection,
    pub edge_id: String,
}

/// Extension of a basic [`Connection`] that includes the `edge_id`,
/// keyed by node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeConnection {
    pub connection: Connection,
    pub edge_id: String,
}

/// Connection-mode policy.
///
/// * `Strict` — only allow `source → target` edges (default).
/// * `Loose`  — also allow `source → source` and `target → target`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum ConnectionMode {
    #[default]
    Strict,
    Loose,
}

/// Payload of the `on_connect_start` callback.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct OnConnectStartParams {
    pub node_id: Option<String>,
    pub handle_id: Option<String>,
    pub handle_type: Option<HandleType>,
}

/// Predicate type alias used by handles and the top-level ReactFlow
/// component to validate a candidate connection.
///
/// Mirrors the TS `IsValidConnection = (edge: EdgeBase | Connection) => boolean`.
/// Because Rust can't accept a heterogeneous `Edge | Connection`, we
/// take an [`EdgeOrConnection`] view enum that borrows the underlying
/// data without copying.
///
/// The function is `Send + Sync` so it can be stored on shared
/// component state.
pub type IsValidConnection =
    Box<dyn for<'a> Fn(EdgeOrConnection<'a>) -> bool + Send + Sync>;

/// Borrowed view passed to [`IsValidConnection`] predicates.
///
/// The full edge case carries source/target/handles plus an `id`; the
/// connection case is the in-progress `Connection` minus the `id`. We
/// pass the connection-shaped fields by reference so the predicate
/// can read them uniformly without an allocation.
#[derive(Debug, Clone, Copy)]
pub enum EdgeOrConnection<'a> {
    /// In-progress connection (no edge id yet).
    Connection(&'a Connection),
    /// Existing edge — only the connection fields and id are exposed.
    Edge {
        id: &'a str,
        source: &'a str,
        target: &'a str,
        source_handle: Option<&'a str>,
        target_handle: Option<&'a str>,
    },
}

/// Snapshot of the connection-line state machine.
///
/// Generic over the node type `N` so the connection machine in
/// `xyhandle` can hold strong references to the in-progress source
/// (and possibly target) node.
#[derive(Debug, Clone)]
pub enum ConnectionState<N> {
    /// No connection in progress. Equivalent to JS `inProgress: false`.
    NoConnection,
    /// A connection is currently being drawn.
    InProgress(ConnectionInProgress<N>),
}

impl<N> Default for ConnectionState<N> {
    fn default() -> Self {
        ConnectionState::NoConnection
    }
}

/// Active drawing connection.
///
/// Mirrors the TS `ConnectionInProgress<NodeType>`. All fields are
/// always present here; in JS many of these are `null` when no
/// connection is in progress, but in Rust we model that absence with
/// the [`ConnectionState::NoConnection`] variant instead.
#[derive(Debug, Clone)]
pub struct ConnectionInProgress<N> {
    /// `Some(true)` when valid, `Some(false)` when invalid, `None`
    /// while we have not yet decided (e.g. pointer between handles).
    pub is_valid: Option<bool>,
    /// Start position in flow coordinates.
    pub from: XYPosition,
    /// Start handle.
    pub from_handle: Handle,
    /// Side of the start handle.
    pub from_position: Position,
    /// Start node.
    pub from_node: N,
    /// Current pointer position in flow coordinates.
    pub to: XYPosition,
    /// End handle when the pointer is over a handle, otherwise `None`.
    pub to_handle: Option<Handle>,
    /// Side of the end handle (best-guess if `to_handle` is `None`).
    pub to_position: Position,
    /// End node when the pointer is over one, otherwise `None`.
    pub to_node: Option<N>,
    /// Pointer position in screen-space (x, y).
    pub pointer: XYPosition,
}

/// Returned by `on_connect_end`. In TS this is `Omit<ConnectionState, 'inProgress'>`,
/// i.e. the fields without the discriminator. We model it as
/// [`ConnectionState`] directly because dropping the `inProgress` flag
/// in Rust simply drops the `enum` discriminator — callers can match.
pub type FinalConnectionState<N> = ConnectionState<N>;

/// Initial value for [`ConnectionState`] — the no-connection variant.
///
/// Equivalent of JS `initialConnection`.
#[must_use]
#[inline]
pub fn initial_connection<N>() -> ConnectionState<N> {
    ConnectionState::NoConnection
}

/// `node_id -> handle_id -> HandleConnection` lookup used by the store
/// to track every handle's currently-connected edge.
pub type ConnectionLookup = HashMap<String, HashMap<String, HandleConnection>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_mode_default_is_strict() {
        assert_eq!(ConnectionMode::default(), ConnectionMode::Strict);
    }

    #[test]
    fn initial_connection_is_no_connection() {
        match initial_connection::<()>() {
            ConnectionState::NoConnection => (),
            ConnectionState::InProgress(_) => panic!("expected NoConnection"),
        }
    }

    #[test]
    fn is_valid_connection_predicate() {
        let f: IsValidConnection = Box::new(|view| match view {
            EdgeOrConnection::Connection(c) => c.source == "a",
            EdgeOrConnection::Edge { source, .. } => source == "a",
        });
        let c = Connection {
            source: "a".into(),
            target: "b".into(),
            source_handle: None,
            target_handle: None,
        };
        assert!(f(EdgeOrConnection::Connection(&c)));
        assert!(f(EdgeOrConnection::Edge {
            id: "e1",
            source: "a",
            target: "b",
            source_handle: None,
            target_handle: None
        }));
    }
}
