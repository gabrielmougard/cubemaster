//! Port of `xyflow-react/src/types/instance.ts`.
//!
//! Status: Phase 1 — implemented.
//!
//! Defines:
//!
//! * [`RGraphJsonObject`]      — serialisable snapshot of nodes/edges/viewport.
//! * [`DeleteElementsOptions`] — argument bundle for `delete_elements`.
//! * [`GeneralHelpers`]        — node/edge manipulation helpers.
//! * [`RGraphInstance`]        — the imperative handle returned by
//!   `use_rgraph()`; composes [`GeneralHelpers`] +
//!   [`crate::types::general::ViewportHelperFunctions`] + a
//!   `viewport_initialized` flag.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::Callback;

use rgraph_core::types::connection::{HandleConnection, NodeConnection};
use rgraph_core::types::geometry::Rect;
use rgraph_core::types::handles::HandleType;
use rgraph_core::types::viewport::Viewport;

use crate::types::edges::Edge;
use crate::types::general::{FitView, ViewportHelperFunctions};
use crate::types::nodes::{InternalNode, Node};

// ---------------------------------------------------------------------------
// RGraphJsonObject — serialisable snapshot.
// ---------------------------------------------------------------------------

/// Snapshot of an `<RGraph>` state, returned by
/// [`GeneralHelpers::to_object`].
///
/// Mirrors the TS `ReactFlowJsonObject<NodeType, EdgeType>`.
#[derive(Debug, Clone, PartialEq)]
pub struct RGraphJsonObject<N: Clone = (), E: Clone = ()> {
    pub nodes: Vec<Node<N>>,
    pub edges: Vec<Edge<E>>,
    pub viewport: Viewport,
}

// ---------------------------------------------------------------------------
// DeleteElementsOptions.
// ---------------------------------------------------------------------------

/// A reference to a node or edge by id only, or by full object. Used by
/// [`DeleteElementsOptions`]. Mirrors the TS `Node | { id: Node['id'] }`
/// / `Edge | { id: Edge['id'] }` unions.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeRef<D: Clone = ()> {
    /// Full node value.
    Full(Node<D>),
    /// Reference by id only.
    Id(String),
}

impl<D: Clone> NodeRef<D> {
    /// The id of the referenced node.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            NodeRef::Full(n) => &n.id,
            NodeRef::Id(id) => id.as_str(),
        }
    }
}

impl<D: Clone> From<Node<D>> for NodeRef<D> {
    fn from(n: Node<D>) -> Self {
        NodeRef::Full(n)
    }
}

impl<D: Clone> From<String> for NodeRef<D> {
    fn from(id: String) -> Self {
        NodeRef::Id(id)
    }
}

impl<D: Clone> From<&str> for NodeRef<D> {
    fn from(id: &str) -> Self {
        NodeRef::Id(id.to_string())
    }
}

/// Edge counterpart of [`NodeRef`].
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeRef<D: Clone = ()> {
    /// Full edge value.
    Full(Edge<D>),
    /// Reference by id only.
    Id(String),
}

impl<D: Clone> EdgeRef<D> {
    /// The id of the referenced edge.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            EdgeRef::Full(e) => &e.id,
            EdgeRef::Id(id) => id.as_str(),
        }
    }
}

impl<D: Clone> From<Edge<D>> for EdgeRef<D> {
    fn from(e: Edge<D>) -> Self {
        EdgeRef::Full(e)
    }
}

impl<D: Clone> From<String> for EdgeRef<D> {
    fn from(id: String) -> Self {
        EdgeRef::Id(id)
    }
}

impl<D: Clone> From<&str> for EdgeRef<D> {
    fn from(id: &str) -> Self {
        EdgeRef::Id(id.to_string())
    }
}

/// Argument bundle for [`GeneralHelpers::delete_elements`].
///
/// Mirrors the TS `DeleteElementsOptions`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeleteElementsOptions<N: Clone = (), E: Clone = ()> {
    pub nodes: Option<Vec<NodeRef<N>>>,
    pub edges: Option<Vec<EdgeRef<E>>>,
}

/// Resolved result of `delete_elements`.
///
/// Mirrors the TS return type
/// `{ deletedNodes: Node[]; deletedEdges: Edge[] }`.
#[derive(Debug, Clone, PartialEq)]
pub struct DeletedElements<N: Clone = (), E: Clone = ()> {
    pub deleted_nodes: Vec<Node<N>>,
    pub deleted_edges: Vec<Edge<E>>,
}

// ---------------------------------------------------------------------------
// `set_nodes` / `set_edges` / `update_xxx` setter unions.
// ---------------------------------------------------------------------------

/// Setter argument for [`GeneralHelpers::set_nodes`] —
/// either a new array or a function from the current array to a new
/// one.
///
/// Mirrors the TS `NodeType[] | ((nodes: NodeType[]) => NodeType[])`.
pub enum SetNodesArg<D: Clone = ()> {
    /// Replace the array directly.
    Array(Vec<Node<D>>),
    /// Compute the new array from the current one.
    ///
    /// The callback receives `&[Node<D>]` and must return a new owned
    /// `Vec<Node<D>>` (the TS doc warns against mutating the array in
    /// place, which the Rust signature enforces statically).
    Fn(Box<dyn FnOnce(&[Node<D>]) -> Vec<Node<D>>>),
}

/// Edge counterpart of [`SetNodesArg`].
pub enum SetEdgesArg<D: Clone = ()> {
    Array(Vec<Edge<D>>),
    Fn(Box<dyn FnOnce(&[Edge<D>]) -> Vec<Edge<D>>>),
}

/// `add_nodes` accepts either a single node or a list.
pub enum AddNodesArg<D: Clone = ()> {
    Single(Node<D>),
    Many(Vec<Node<D>>),
}

/// `add_edges` accepts either a single edge or a list.
pub enum AddEdgesArg<D: Clone = ()> {
    Single(Edge<D>),
    Many(Vec<Edge<D>>),
}

/// `update_node(id, update, options)` setter argument.
pub enum NodeUpdater<D: Clone = ()> {
    /// Partial overlay applied with merge or replace semantics.
    Partial(NodePartial<D>),
    /// Compute the partial overlay from the current node.
    Fn(Box<dyn FnOnce(&Node<D>) -> NodePartial<D>>),
}

/// Edge counterpart of [`NodeUpdater`].
pub enum EdgeUpdater<D: Clone = ()> {
    Partial(EdgePartial<D>),
    Fn(Box<dyn FnOnce(&Edge<D>) -> EdgePartial<D>>),
}

/// Subset of [`Node<D>`] fields that can be overlaid by `update_node`.
///
/// We mirror the TS `Partial<NodeType>` shape; only fields exposed on
/// the canonical [`Node`] are listed. To replace a node entirely,
/// callers should use `update_node` with `replace = true`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NodePartial<D: Clone = ()> {
    pub position: Option<rgraph_core::types::geometry::XYPosition>,
    pub data: Option<D>,
    pub selected: Option<bool>,
    pub hidden: Option<bool>,
    pub dragging: Option<bool>,
    pub draggable: Option<bool>,
    pub selectable: Option<bool>,
    pub connectable: Option<bool>,
    pub deletable: Option<bool>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub parent_id: Option<String>,
    pub z_index: Option<i32>,
    pub aria_label: Option<String>,
    pub type_: Option<String>,
}

/// Edge counterpart of [`NodePartial`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EdgePartial<D: Clone = ()> {
    pub source: Option<String>,
    pub target: Option<String>,
    pub source_handle: Option<String>,
    pub target_handle: Option<String>,
    pub animated: Option<bool>,
    pub hidden: Option<bool>,
    pub deletable: Option<bool>,
    pub selectable: Option<bool>,
    pub data: Option<D>,
    pub selected: Option<bool>,
    pub z_index: Option<i32>,
    pub aria_label: Option<String>,
    pub interaction_width: Option<f64>,
    pub type_: Option<String>,
}

/// Optional flag accepted by `update_node` / `update_edge` /
/// `update_node_data` / `update_edge_data`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UpdateOptions {
    /// When `true`, the update replaces the existing node/edge instead
    /// of merging.
    pub replace: bool,
}

/// Setter argument for `update_node_data` / `update_edge_data`.
pub enum DataUpdater<D: Clone> {
    /// Partial overlay applied with merge or replace semantics.
    Partial(D),
    /// Compute the partial overlay from the current node/edge.
    NodeFn(Box<dyn FnOnce(&Node<D>) -> D>),
    /// Compute the partial overlay from the current edge.
    EdgeFn(Box<dyn FnOnce(&Edge<D>) -> D>),
}

// ---------------------------------------------------------------------------
// `getIntersectingNodes` argument.
// ---------------------------------------------------------------------------

/// Argument accepted by [`GeneralHelpers::get_intersecting_nodes`] and
/// [`GeneralHelpers::is_node_intersecting`]. Mirrors the TS
/// `NodeType | { id: Node['id'] } | Rect`.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeOrRect<D: Clone = ()> {
    Node(Node<D>),
    Id(String),
    Rect(Rect),
}

impl<D: Clone> From<Rect> for NodeOrRect<D> {
    fn from(r: Rect) -> Self {
        NodeOrRect::Rect(r)
    }
}

impl<D: Clone> From<Node<D>> for NodeOrRect<D> {
    fn from(n: Node<D>) -> Self {
        NodeOrRect::Node(n)
    }
}

// ---------------------------------------------------------------------------
// HandleConnection / NodeConnection query payloads.
// ---------------------------------------------------------------------------

/// Argument for [`GeneralHelpers::get_handle_connections`].
#[derive(Debug, Clone, PartialEq)]
pub struct GetHandleConnectionsArgs {
    pub type_: HandleType,
    pub node_id: String,
    pub id: Option<String>,
}

/// Argument for [`GeneralHelpers::get_node_connections`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GetNodeConnectionsArgs {
    pub type_: Option<HandleType>,
    pub node_id: String,
    pub handle_id: Option<String>,
}

// ---------------------------------------------------------------------------
// GeneralHelpers — bundle of node/edge manipulation callbacks.
// ---------------------------------------------------------------------------

/// Bundle of imperative node/edge helpers exposed through `use_rgraph()`.
///
/// Mirrors the TS `GeneralHelpers<NodeType, EdgeType>`. Each field is a
/// [`Callback`] so the bundle is cheap to clone and `PartialEq`.
///
/// Note: callbacks that take ownership of a function-style argument
/// (e.g. [`SetNodesArg::Fn`]) wrap the closure in a `Box<dyn FnOnce>`,
/// which is **not** `Clone`. That's fine for the call site — the
/// callback consumes the argument exactly once.
#[derive(Clone, PartialEq)]
pub struct GeneralHelpers<N: Clone + 'static = (), E: Clone + 'static = ()> {
    pub get_nodes: Callback<(), Vec<Node<N>>>,
    pub set_nodes: Callback<SetNodesArg<N>>,
    pub add_nodes: Callback<AddNodesArg<N>>,
    pub get_node: Callback<String, Option<Node<N>>>,
    pub get_internal_node: Callback<String, Option<InternalNode<N>>>,
    pub get_edges: Callback<(), Vec<Edge<E>>>,
    pub set_edges: Callback<SetEdgesArg<E>>,
    pub add_edges: Callback<AddEdgesArg<E>>,
    pub get_edge: Callback<String, Option<Edge<E>>>,
    pub to_object: Callback<(), RGraphJsonObject<N, E>>,
    pub delete_elements: Callback<DeleteElementsOptions<N, E>, DeletedElements<N, E>>,
    pub get_intersecting_nodes: Callback<GetIntersectingNodesArgs<N>, Vec<Node<N>>>,
    pub is_node_intersecting: Callback<IsNodeIntersectingArgs<N>, bool>,
    pub update_node: Callback<UpdateNodeArgs<N>>,
    pub update_node_data: Callback<UpdateNodeDataArgs<N>>,
    pub update_edge: Callback<UpdateEdgeArgs<E>>,
    pub update_edge_data: Callback<UpdateEdgeDataArgs<E>>,
    pub get_nodes_bounds: Callback<Vec<NodeOrIdOrInternal<N>>, Rect>,
    pub get_handle_connections: Callback<GetHandleConnectionsArgs, Vec<HandleConnection>>,
    pub get_node_connections: Callback<GetNodeConnectionsArgs, Vec<NodeConnection>>,
    pub fit_view: FitView,
}

/// Args for `get_intersecting_nodes`.
pub struct GetIntersectingNodesArgs<D: Clone = ()> {
    pub node_or_rect: NodeOrRect<D>,
    pub partially: Option<bool>,
    pub nodes: Option<Vec<Node<D>>>,
}

/// Args for `is_node_intersecting`.
pub struct IsNodeIntersectingArgs<D: Clone = ()> {
    pub node_or_rect: NodeOrRect<D>,
    pub area: Rect,
    pub partially: Option<bool>,
}

/// Args for `update_node`.
pub struct UpdateNodeArgs<D: Clone = ()> {
    pub id: String,
    pub update: NodeUpdater<D>,
    pub options: UpdateOptions,
}

/// Args for `update_node_data`.
pub struct UpdateNodeDataArgs<D: Clone = ()> {
    pub id: String,
    pub update: DataUpdater<D>,
    pub options: UpdateOptions,
}

/// Args for `update_edge`.
pub struct UpdateEdgeArgs<D: Clone = ()> {
    pub id: String,
    pub update: EdgeUpdater<D>,
    pub options: UpdateOptions,
}

/// Args for `update_edge_data`.
pub struct UpdateEdgeDataArgs<D: Clone = ()> {
    pub id: String,
    pub update: DataUpdater<D>,
    pub options: UpdateOptions,
}

/// `Node | InternalNode | string` union accepted by `get_nodes_bounds`.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeOrIdOrInternal<D: Clone = ()> {
    Node(Node<D>),
    Internal(InternalNode<D>),
    Id(String),
}

// ---------------------------------------------------------------------------
// RGraphInstance — combined imperative handle.
// ---------------------------------------------------------------------------

/// The `RGraphInstance` provides a collection of methods to query and
/// manipulate the internal state of your flow. You can get an instance
/// by using the `use_rgraph()` hook or via the `on_init` callback.
///
/// Mirrors the TS `ReactFlowInstance`.
#[derive(Clone, PartialEq)]
pub struct RGraphInstance<N: Clone + 'static = (), E: Clone + 'static = ()> {
    /// Node/edge manipulation helpers.
    pub general: GeneralHelpers<N, E>,
    /// Viewport helpers (`zoom_in`, `set_center`, `fit_bounds`, …).
    pub viewport: ViewportHelperFunctions,
    /// `true` once the viewport has been mounted and its pan/zoom
    /// machine is initialised.
    pub viewport_initialized: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_ref_id_works_for_both_variants() {
        let r1: NodeRef<()> = NodeRef::Id("n1".into());
        assert_eq!(r1.id(), "n1");
        let r2: NodeRef<()> = Node::<()>::minimal("n2", 0.0, 0.0).into();
        assert_eq!(r2.id(), "n2");
    }

    #[test]
    fn edge_ref_id_works_for_both_variants() {
        let r1: EdgeRef<()> = EdgeRef::Id("e1".into());
        assert_eq!(r1.id(), "e1");
        let r2: EdgeRef<()> = Edge::<()>::minimal("e2", "a", "b").into();
        assert_eq!(r2.id(), "e2");
    }

    #[test]
    fn delete_elements_options_default_is_empty() {
        let d: DeleteElementsOptions = DeleteElementsOptions::default();
        assert!(d.nodes.is_none());
        assert!(d.edges.is_none());
    }
}
