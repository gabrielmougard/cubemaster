//! Port of `xyflow-core/src/utils/graph.ts` — graph algorithms over
//! node lookups.
//!
//! Status: implemented (phase 3).

#![allow(clippy::module_name_repetitions)]

use std::collections::{HashMap, HashSet};

use crate::constants::{error_005, error_015};
use crate::promise::Promise;
use crate::types::edges::Edge;
use crate::types::geometry::{
    Box2d, CoordinateExtent, Dimensions, Rect, Transform, XYPosition,
};
use crate::types::nodes::{
    InternalNode, Node, NodeExtent, NodeLookup, NodeOrigin,
};
use crate::types::panzoom::{PanZoomInstance, PanZoomTransformOptions};
use crate::types::viewport::{
    FitViewOptionsBase, InterpolationKind, Padding, PaddingWithUnit,
};
use crate::utils::general::{
    box_to_rect, clamp_position, get_bounds_of_boxes, get_node_dimensions,
    get_overlapping_area, internal_node_to_box, internal_node_to_rect, is_coordinate_extent,
    point_to_renderer_point, user_node_to_box,
};

// ---------------------------------------------------------------------------
// Type guards
// ---------------------------------------------------------------------------

// `isEdgeBase`, `isNodeBase`, `isInternalNodeBase` in TS act as
// `unknown -> boolean` type guards. Rust's typed model already knows
// the distinction at compile time, so the equivalents are reserved
// for cases where a value's shape really is unknown (e.g. when
// importing JSON). Such helpers can be added later if needed; we omit
// them here.

// ---------------------------------------------------------------------------
// Outgoers / incomers / connected edges
// ---------------------------------------------------------------------------

/// Returns the nodes that are *targets* of edges originating at
/// `node`.
///
/// Mirrors the TS `getOutgoers`. Returns an empty `Vec` when `node.id`
/// is empty.
#[must_use]
pub fn get_outgoers<'a, D: Clone, E: Clone>(
    node_id: &str,
    nodes: &'a [Node<D>],
    edges: &[Edge<E>],
) -> Vec<&'a Node<D>> {
    if node_id.is_empty() {
        return Vec::new();
    }
    let outgoer_ids: HashSet<&str> = edges
        .iter()
        .filter(|e| e.source == node_id)
        .map(|e| e.target.as_str())
        .collect();
    nodes
        .iter()
        .filter(|n| outgoer_ids.contains(n.id.as_str()))
        .collect()
}

/// Returns the nodes that are *sources* of edges terminating at
/// `node`.
///
/// Mirrors the TS `getIncomers`.
#[must_use]
pub fn get_incomers<'a, D: Clone, E: Clone>(
    node_id: &str,
    nodes: &'a [Node<D>],
    edges: &[Edge<E>],
) -> Vec<&'a Node<D>> {
    if node_id.is_empty() {
        return Vec::new();
    }
    let incomer_ids: HashSet<&str> = edges
        .iter()
        .filter(|e| e.target == node_id)
        .map(|e| e.source.as_str())
        .collect();
    nodes
        .iter()
        .filter(|n| incomer_ids.contains(n.id.as_str()))
        .collect()
}

/// Filter the given edges, keeping only those with at least one
/// endpoint in `nodes`.
///
/// Mirrors the TS `getConnectedEdges`.
#[must_use]
pub fn get_connected_edges<'a, D: Clone, E: Clone>(
    nodes: &[Node<D>],
    edges: &'a [Edge<E>],
) -> Vec<&'a Edge<E>> {
    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    edges
        .iter()
        .filter(|e| node_ids.contains(e.source.as_str()) || node_ids.contains(e.target.as_str()))
        .collect()
}

// ---------------------------------------------------------------------------
// Bounds / nodesInside / fit view
// ---------------------------------------------------------------------------

/// Either a borrowed user [`Node`] or its id, accepted by
/// [`get_nodes_bounds`].
///
/// Mirrors the TS `(NodeType | InternalNodeBase | string)[]` union.
pub enum NodeOrId<'a, D: Clone = ()> {
    Node(&'a Node<D>),
    Internal(&'a InternalNode<D>),
    Id(&'a str),
}

impl<'a, D: Clone> From<&'a Node<D>> for NodeOrId<'a, D> {
    fn from(n: &'a Node<D>) -> Self {
        NodeOrId::Node(n)
    }
}

impl<'a, D: Clone> From<&'a InternalNode<D>> for NodeOrId<'a, D> {
    fn from(n: &'a InternalNode<D>) -> Self {
        NodeOrId::Internal(n)
    }
}

impl<'a, D: Clone> From<&'a str> for NodeOrId<'a, D> {
    fn from(s: &'a str) -> Self {
        NodeOrId::Id(s)
    }
}

/// Options for [`get_nodes_bounds`].
#[derive(Default)]
pub struct GetNodesBoundsParams<'a, D: Clone = ()> {
    /// Origin to apply to user nodes (those without internals). Default
    /// `(0.0, 0.0)`.
    pub node_origin: NodeOrigin,
    /// Lookup used to resolve string ids and to upgrade `Node` refs to
    /// `InternalNode` (so absolute positions of children are honoured).
    pub node_lookup: Option<&'a NodeLookup<D>>,
}

/// Return the rectangle that encloses every node in `nodes`.
///
/// Mirrors the TS `getNodesBounds`. The TS source warns at runtime
/// when no `nodeLookup` is provided; we keep that signal as a
/// `tracing::warn!` if the `tracing` crate is available — currently we
/// just silently accept it for parity, since `tracing` isn't a
/// dependency of this crate.
#[must_use]
pub fn get_nodes_bounds<'a, D: Clone>(
    nodes: impl IntoIterator<Item = NodeOrId<'a, D>>,
    params: GetNodesBoundsParams<'a, D>,
) -> Rect {
    let mut box_acc = Box2d {
        x: f64::INFINITY,
        y: f64::INFINITY,
        x2: f64::NEG_INFINITY,
        y2: f64::NEG_INFINITY,
    };
    let mut count = 0usize;

    for entry in nodes {
        let node_box = match entry {
            NodeOrId::Internal(n) => internal_node_to_box(n),
            NodeOrId::Node(n) => {
                if let Some(lookup) = params.node_lookup {
                    if let Some(internal) = lookup.get(&n.id) {
                        internal_node_to_box(internal)
                    } else {
                        user_node_to_box(n, params.node_origin)
                    }
                } else {
                    user_node_to_box(n, params.node_origin)
                }
            }
            NodeOrId::Id(id) => match params.node_lookup.and_then(|m| m.get(id)) {
                Some(internal) => internal_node_to_box(internal),
                None => Box2d::new(0.0, 0.0, 0.0, 0.0),
            },
        };
        box_acc = get_bounds_of_boxes(box_acc, node_box);
        count += 1;
    }

    if count == 0 || !box_acc.x.is_finite() {
        Rect::ZERO
    } else {
        box_to_rect(box_acc)
    }
}

/// Options for [`get_internal_nodes_bounds`].
#[derive(Default)]
pub struct GetInternalNodesBoundsParams<'a, D: Clone = ()> {
    /// Optional predicate; nodes for which it returns `false` are
    /// excluded from the bounds.
    pub filter: Option<Box<dyn Fn(&InternalNode<D>) -> bool + 'a>>,
}

/// Internal-node variant of [`get_nodes_bounds`] used by `fit_view`
/// and `update_node_internals`.
///
/// Mirrors the TS `getInternalNodesBounds`.
#[must_use]
pub fn get_internal_nodes_bounds<D: Clone>(
    node_lookup: &NodeLookup<D>,
    params: GetInternalNodesBoundsParams<'_, D>,
) -> Rect {
    let mut box_acc = Box2d {
        x: f64::INFINITY,
        y: f64::INFINITY,
        x2: f64::NEG_INFINITY,
        y2: f64::NEG_INFINITY,
    };
    let mut has_visible = false;
    for n in node_lookup.values() {
        if let Some(f) = &params.filter {
            if !f(n) {
                continue;
            }
        }
        box_acc = get_bounds_of_boxes(box_acc, internal_node_to_box(n));
        has_visible = true;
    }
    if has_visible {
        box_to_rect(box_acc)
    } else {
        Rect::ZERO
    }
}

/// Options for [`get_nodes_inside`].
#[derive(Debug, Clone, Copy)]
pub struct GetNodesInsideParams {
    pub partially: bool,
    /// When `true`, nodes whose `selectable` flag is `false` are
    /// skipped.
    pub exclude_non_selectable_nodes: bool,
}

impl Default for GetNodesInsideParams {
    fn default() -> Self {
        Self {
            partially: false,
            exclude_non_selectable_nodes: false,
        }
    }
}

/// Returns every internal node overlapping the given screen-space
/// `rect` (in screen pixels), under the active viewport `transform`.
///
/// Mirrors the TS `getNodesInside`.
#[must_use]
pub fn get_nodes_inside<'a, D: Clone>(
    node_lookup: &'a NodeLookup<D>,
    rect: Rect,
    transform: Transform,
    params: GetNodesInsideParams,
) -> Vec<&'a InternalNode<D>> {
    let pane_origin = point_to_renderer_point(
        XYPosition {
            x: rect.x,
            y: rect.y,
        },
        transform,
        false,
        (1.0, 1.0),
    );
    let scale = transform.scale();
    let pane_rect = Rect {
        x: pane_origin.x,
        y: pane_origin.y,
        width: rect.width / scale,
        height: rect.height / scale,
    };

    let mut visible: Vec<&InternalNode<D>> = Vec::new();
    for node in node_lookup.values() {
        let selectable = node.user.selectable.unwrap_or(true);
        let hidden = node.user.hidden.unwrap_or(false);
        if (params.exclude_non_selectable_nodes && !selectable) || hidden {
            continue;
        }

        let dim = get_node_dimensions(node);
        let area = dim.width * dim.height;
        let overlap = get_overlapping_area(pane_rect, internal_node_to_rect(node));
        let partially_visible = params.partially && overlap > 0.0;
        let force_initial_render = node.internals.handle_bounds.is_none();
        let is_visible = force_initial_render || partially_visible || overlap >= area;

        if is_visible || node.user.dragging.unwrap_or(false) {
            visible.push(node);
        }
    }
    visible
}

/// Filter `node_lookup` to only those nodes that should participate in
/// `fit_view` (visible + measured + matching `options.nodes` if
/// given).
///
/// Equivalent to TS `getFitViewNodes`.
fn get_fit_view_nodes<'a, D: Clone>(
    node_lookup: &'a NodeLookup<D>,
    options: Option<&FitViewOptionsBase>,
) -> Vec<&'a InternalNode<D>> {
    let id_set: Option<HashSet<&str>> = options
        .and_then(|o| o.nodes.as_ref())
        .map(|ids| ids.iter().map(String::as_str).collect());
    let include_hidden = options.map(|o| o.include_hidden_nodes).unwrap_or(false);

    node_lookup
        .values()
        .filter(|n| {
            let has_dim = n.measured.width.is_some() && n.measured.height.is_some();
            let visible = has_dim && (include_hidden || !n.user.hidden.unwrap_or(false));
            let matches_filter = id_set.as_ref().map(|s| s.contains(n.user.id.as_str())).unwrap_or(true);
            visible && matches_filter
        })
        .collect()
}

/// Parameters for [`fit_viewport`] mirroring the TS `FitViewParamsBase`.
pub struct FitViewportParams<'a, D: Clone = (), P: PanZoomInstance + ?Sized = dyn PanZoomInstance> {
    pub nodes: &'a NodeLookup<D>,
    pub width: f64,
    pub height: f64,
    pub pan_zoom: &'a mut P,
    pub min_zoom: f64,
    pub max_zoom: f64,
}

/// Animate the viewport to fit the requested nodes.
///
/// Mirrors the TS `fitViewport(params, options?)` (which returns
/// `Promise<boolean>`). The Rust port returns a [`Promise<bool>`].
///
/// When there are no nodes to fit, returns an already-resolved promise
/// (true). Otherwise the underlying `pan_zoom.set_viewport(...)` may be
/// animated; the returned promise resolves once that completes.
pub fn fit_viewport<D: Clone>(
    params: FitViewportParams<'_, D>,
    options: Option<&FitViewOptionsBase>,
) -> Promise<bool> {
    if params.nodes.is_empty() {
        return Promise::resolved(true);
    }

    let nodes_to_fit = get_fit_view_nodes(params.nodes, options);
    if nodes_to_fit.is_empty() {
        return Promise::resolved(true);
    }

    // Build a lookup containing only the matching internal nodes.
    let mut filtered: NodeLookup<D> = HashMap::with_capacity(nodes_to_fit.len());
    for n in nodes_to_fit {
        filtered.insert(n.user.id.clone(), n.clone());
    }

    let bounds = get_internal_nodes_bounds(
        &filtered,
        GetInternalNodesBoundsParams { filter: None },
    );

    let min_zoom = options.and_then(|o| o.min_zoom).unwrap_or(params.min_zoom);
    let max_zoom = options.and_then(|o| o.max_zoom).unwrap_or(params.max_zoom);
    let padding = options
        .and_then(|o| o.padding)
        .unwrap_or(Padding::Single(PaddingWithUnit::Number(0.1)));

    let viewport = crate::utils::general::get_viewport_for_bounds(
        bounds,
        params.width,
        params.height,
        min_zoom,
        max_zoom,
        padding,
    );

    let opts = options.map(|o| PanZoomTransformOptions {
        duration: o.duration,
        ease: None, // EaseFn isn't Clone; consumers needing animation pass options directly to set_viewport.
        interpolate: o.interpolate.map(|i| match i {
            // FitViewOptionsBase.interpolate is currently `Option<&'static str>` per TS
            // ('smooth' | 'linear'). InterpolationKind has variants; map directly.
            InterpolationKind::Smooth => InterpolationKind::Smooth,
            InterpolationKind::Linear => InterpolationKind::Linear,
        }),
    });

    // Forward to the pan/zoom instance and propagate its completion.
    params.pan_zoom.set_viewport(viewport, opts)
}

// ---------------------------------------------------------------------------
// calculateNodePosition
// ---------------------------------------------------------------------------

/// Result of [`calculate_node_position`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalculatedNodePosition {
    /// Position relative to the node's parent (or world origin).
    pub position: XYPosition,
    /// Absolute position in flow coordinates.
    pub position_absolute: XYPosition,
}

/// Optional error reporter — same shape as in `utils::edges::positions`.
///
/// Renamed from `OnErrorFn` (TS `OnError`) to avoid a glob-import
/// clash with [`crate::utils::edges::positions::OnErrorFn`].
pub type GraphOnErrorFn = Box<dyn Fn(&str, &str) + Send + Sync>;

/// Parameters for [`calculate_node_position`].
pub struct CalculateNodePositionParams<'a, D: Clone = ()> {
    pub node_id: &'a str,
    pub next_position: XYPosition,
    pub node_lookup: &'a NodeLookup<D>,
    pub node_origin: NodeOrigin,
    pub node_extent: Option<CoordinateExtent>,
    pub on_error: Option<&'a GraphOnErrorFn>,
}

/// Compute the next position of a node, taking into account its
/// extent, parent, and origin.
///
/// Mirrors the TS `calculateNodePosition`. Returns a tuple-equivalent
/// [`CalculatedNodePosition`] holding `{ position, positionAbsolute }`.
///
/// Returns the input position unchanged if the node id is not found
/// in `node_lookup` (the TS source unwraps with `!` and would crash —
/// we are slightly more defensive here, returning an absolute
/// position equal to `next_position`).
#[must_use]
pub fn calculate_node_position<D: Clone>(
    params: CalculateNodePositionParams<'_, D>,
) -> CalculatedNodePosition {
    let CalculateNodePositionParams {
        node_id,
        next_position,
        node_lookup,
        node_origin,
        node_extent,
        on_error,
    } = params;

    let Some(node) = node_lookup.get(node_id) else {
        return CalculatedNodePosition {
            position: next_position,
            position_absolute: next_position,
        };
    };

    let parent_node = node.user.parent_id.as_deref().and_then(|p| node_lookup.get(p));
    let (parent_x, parent_y) = parent_node
        .map(|p| (p.internals.position_absolute.x, p.internals.position_absolute.y))
        .unwrap_or((0.0, 0.0));

    let origin = node.user.origin.unwrap_or(node_origin);

    // Build the active extent. Order:
    //   * node.extent == Parent && !expandParent → parent rect
    //   * parent + custom extent on node.extent  → translated extent
    //   * otherwise: node.extent if Custom, else nodeExtent fallback.
    let mut extent: Option<CoordinateExtent> = match node.user.extent {
        NodeExtent::Custom(c) => Some(c),
        _ => node_extent,
    };

    let expand_parent = node.user.expand_parent.unwrap_or(false);
    match (node.user.extent, &parent_node) {
        (NodeExtent::Parent, parent_opt) if !expand_parent => {
            match parent_opt {
                None => {
                    if let Some(cb) = on_error {
                        cb("005", &error_005());
                    }
                }
                Some(parent) => {
                    if let (Some(pw), Some(ph)) = (parent.measured.width, parent.measured.height) {
                        extent = Some([
                            [parent_x, parent_y],
                            [parent_x + pw, parent_y + ph],
                        ]);
                    }
                }
            }
        }
        (NodeExtent::Custom(c), Some(_)) => {
            extent = Some([
                [c[0][0] + parent_x, c[0][1] + parent_y],
                [c[1][0] + parent_x, c[1][1] + parent_y],
            ]);
        }
        _ => {}
    }

    let dim = Dimensions {
        width: node.measured.width.unwrap_or(0.0),
        height: node.measured.height.unwrap_or(0.0),
    };

    let position_absolute = match extent {
        Some(ex) if is_coordinate_extent(&NodeExtent::Custom(ex)) => {
            clamp_position(next_position, ex, dim)
        }
        _ => next_position,
    };

    if node.measured.width.is_none() || node.measured.height.is_none() {
        if let Some(cb) = on_error {
            cb("015", &error_015());
        }
    }

    CalculatedNodePosition {
        position: XYPosition {
            x: position_absolute.x - parent_x + dim.width * origin.0,
            y: position_absolute.y - parent_y + dim.height * origin.1,
        },
        position_absolute,
    }
}

// ---------------------------------------------------------------------------
// getElementsToRemove
// ---------------------------------------------------------------------------

/// Async predicate that, given a deletion candidate `{ nodes, edges }`,
/// returns either a boolean confirming or rejecting the deletion, or
/// a refined `{ nodes, edges }` to delete.
///
/// Mirrors the TS `OnBeforeDeleteBase`. Because Rust does not have a
/// promise type built in, callers supply a regular `Fn` returning a
/// [`OnBeforeDeleteResult`] synchronously. If true async deletion
/// (e.g. user-confirm dialog) is needed, downstream Dioxus code can
/// resolve the dialog before invoking [`get_elements_to_remove`].
pub enum OnBeforeDeleteResult<D: Clone, E: Clone> {
    /// Delete the candidate as-is.
    Confirm,
    /// Reject the deletion.
    Reject,
    /// Replace the deletion candidate with this list.
    Refined { nodes: Vec<Node<D>>, edges: Vec<Edge<E>> },
}

/// Output of [`get_elements_to_remove`].
#[derive(Debug, Clone, PartialEq)]
pub struct ElementsToRemove<D: Clone, E: Clone> {
    pub nodes: Vec<Node<D>>,
    pub edges: Vec<Edge<E>>,
}

/// Parameters for [`get_elements_to_remove`].
pub struct GetElementsToRemoveParams<'a, D: Clone, E: Clone> {
    pub nodes_to_remove: &'a [&'a str],
    pub edges_to_remove: &'a [&'a str],
    pub nodes: &'a [Node<D>],
    pub edges: &'a [Edge<E>],
    pub on_before_delete:
        Option<Box<dyn Fn(&[Node<D>], &[Edge<E>]) -> OnBeforeDeleteResult<D, E>>>,
}

/// Compute which nodes / edges may actually be deleted.
///
/// Mirrors the TS `getElementsToRemove`. Cascades:
/// * children of a removed parent are also removed,
/// * edges with a removed endpoint are also removed,
/// * `deletable: false` items are kept,
/// * `on_before_delete` may further narrow / cancel the list.
#[must_use]
pub fn get_elements_to_remove<D: Clone, E: Clone>(
    params: GetElementsToRemoveParams<'_, D, E>,
) -> ElementsToRemove<D, E> {
    let GetElementsToRemoveParams {
        nodes_to_remove,
        edges_to_remove,
        nodes,
        edges,
        on_before_delete,
    } = params;

    let node_ids: HashSet<&str> = nodes_to_remove.iter().copied().collect();
    let mut matching_nodes: Vec<Node<D>> = Vec::new();

    for node in nodes {
        if node.deletable == Some(false) {
            continue;
        }
        let is_included = node_ids.contains(node.id.as_str());
        let parent_hit = !is_included
            && node
                .parent_id
                .as_deref()
                .map(|pid| matching_nodes.iter().any(|m| m.id == pid))
                .unwrap_or(false);
        if is_included || parent_hit {
            matching_nodes.push(node.clone());
        }
    }

    let edge_ids: HashSet<&str> = edges_to_remove.iter().copied().collect();
    let deletable_edges: Vec<&Edge<E>> = edges
        .iter()
        .filter(|e| e.deletable != Some(false))
        .collect();
    // Connected edges = edges where source/target id is in matching_nodes.
    let matching_node_id_set: HashSet<&str> =
        matching_nodes.iter().map(|n| n.id.as_str()).collect();
    let mut matching_edges: Vec<Edge<E>> = deletable_edges
        .iter()
        .filter(|e| {
            matching_node_id_set.contains(e.source.as_str())
                || matching_node_id_set.contains(e.target.as_str())
        })
        .map(|e| (*e).clone())
        .collect();

    for edge in &deletable_edges {
        let is_included = edge_ids.contains(edge.id.as_str());
        if is_included && !matching_edges.iter().any(|e| e.id == edge.id) {
            matching_edges.push((*edge).clone());
        }
    }

    let Some(cb) = on_before_delete else {
        return ElementsToRemove {
            nodes: matching_nodes,
            edges: matching_edges,
        };
    };

    match cb(&matching_nodes, &matching_edges) {
        OnBeforeDeleteResult::Confirm => ElementsToRemove {
            nodes: matching_nodes,
            edges: matching_edges,
        },
        OnBeforeDeleteResult::Reject => ElementsToRemove {
            nodes: Vec::new(),
            edges: Vec::new(),
        },
        OnBeforeDeleteResult::Refined { nodes, edges } => ElementsToRemove { nodes, edges },
    }
}

// Re-export `get_node_position_with_origin` from `utils::general` —
// the TS source defines it here under `graph.ts`, but it's already
// implemented in `general.rs` of this crate so we just re-export.
pub use crate::utils::general::get_node_position_with_origin;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::nodes::MeasuredDimensions;

    fn n(id: &str, x: f64, y: f64) -> Node<()> {
        Node::minimal(id, x, y)
    }

    fn e(id: &str, src: &str, tgt: &str) -> Edge<()> {
        Edge::<()>::minimal(id, src, tgt)
    }

    fn measured_internal(id: &str, x: f64, y: f64, w: f64, h: f64) -> InternalNode<()> {
        let mut user: Node<()> = Node::minimal(id, x, y);
        user.measured = Some(MeasuredDimensions {
            width: Some(w),
            height: Some(h),
        });
        let mut internal = InternalNode::from_user(user);
        internal.measured = MeasuredDimensions {
            width: Some(w),
            height: Some(h),
        };
        internal
    }

    #[test]
    fn outgoers_and_incomers() {
        let nodes = vec![n("a", 0.0, 0.0), n("b", 0.0, 0.0), n("c", 0.0, 0.0)];
        let edges = vec![e("ab", "a", "b"), e("ac", "a", "c"), e("cb", "c", "b")];
        let out: Vec<_> = get_outgoers("a", &nodes, &edges).iter().map(|n| n.id.clone()).collect();
        let mut out_sorted = out.clone();
        out_sorted.sort();
        assert_eq!(out_sorted, vec!["b", "c"]);
        let inc: Vec<_> = get_incomers("b", &nodes, &edges).iter().map(|n| n.id.clone()).collect();
        let mut inc_sorted = inc.clone();
        inc_sorted.sort();
        assert_eq!(inc_sorted, vec!["a", "c"]);
        assert!(get_outgoers::<(), ()>("", &nodes, &edges).is_empty());
    }

    #[test]
    fn connected_edges_filters_correctly() {
        let nodes = vec![n("a", 0.0, 0.0), n("b", 0.0, 0.0)];
        let edges = vec![
            e("ab", "a", "b"),
            e("xy", "x", "y"),       // both endpoints unrelated
            e("ax", "a", "x"),       // one endpoint matches
        ];
        let result = get_connected_edges(&nodes, &edges);
        let ids: Vec<&str> = result.iter().map(|e| e.id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(sorted, vec!["ab", "ax"]);
    }

    #[test]
    fn get_nodes_bounds_empty_returns_zero() {
        let r = get_nodes_bounds::<()>(std::iter::empty(), GetNodesBoundsParams::default());
        assert_eq!(r, Rect::ZERO);
    }

    #[test]
    fn get_nodes_bounds_envelopes() {
        let mut a = n("a", 0.0, 0.0);
        a.width = Some(50.0);
        a.height = Some(50.0);
        let mut b = n("b", 100.0, 100.0);
        b.width = Some(50.0);
        b.height = Some(50.0);
        let nodes = vec![a, b];
        let refs: Vec<NodeOrId<'_, ()>> = nodes.iter().map(|n| NodeOrId::Node(n)).collect();
        let r = get_nodes_bounds(refs, GetNodesBoundsParams::default());
        assert_eq!(r, Rect::new(0.0, 0.0, 150.0, 150.0));
    }

    #[test]
    fn get_internal_nodes_bounds_picks_visible() {
        let mut lookup: NodeLookup<()> = NodeLookup::new();
        lookup.insert("a".into(), measured_internal("a", 0.0, 0.0, 50.0, 50.0));
        lookup.insert("b".into(), measured_internal("b", 100.0, 100.0, 50.0, 50.0));
        let r = get_internal_nodes_bounds(&lookup, GetInternalNodesBoundsParams::default());
        assert_eq!(r, Rect::new(0.0, 0.0, 150.0, 150.0));
    }

    #[test]
    fn get_internal_nodes_bounds_filters() {
        let mut lookup: NodeLookup<()> = NodeLookup::new();
        lookup.insert("a".into(), measured_internal("a", 0.0, 0.0, 50.0, 50.0));
        lookup.insert("b".into(), measured_internal("b", 100.0, 100.0, 50.0, 50.0));
        let r = get_internal_nodes_bounds(
            &lookup,
            GetInternalNodesBoundsParams {
                filter: Some(Box::new(|n| n.user.id == "a")),
            },
        );
        assert_eq!(r, Rect::new(0.0, 0.0, 50.0, 50.0));
    }

    #[test]
    fn get_nodes_inside_returns_overlapping() {
        let mut lookup: NodeLookup<()> = NodeLookup::new();
        lookup.insert("a".into(), measured_internal("a", 0.0, 0.0, 50.0, 50.0));
        lookup.insert("b".into(), measured_internal("b", 200.0, 200.0, 50.0, 50.0));
        // Mark them as initialized so the force_initial_render path doesn't
        // include everything.
        for n in lookup.values_mut() {
            n.internals.handle_bounds = Some(crate::types::nodes::NodeHandleBounds::default());
        }

        let visible = get_nodes_inside(
            &lookup,
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Transform::IDENTITY,
            GetNodesInsideParams::default(),
        );
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].user.id, "a");
    }

    #[test]
    fn calculate_node_position_clamps_to_extent() {
        let mut lookup: NodeLookup<()> = NodeLookup::new();
        lookup.insert("a".into(), measured_internal("a", 50.0, 50.0, 20.0, 20.0));
        let result = calculate_node_position(CalculateNodePositionParams {
            node_id: "a",
            next_position: XYPosition::new(-100.0, -100.0),
            node_lookup: &lookup,
            node_origin: (0.0, 0.0),
            node_extent: Some([[0.0, 0.0], [100.0, 100.0]]),
            on_error: None,
        });
        // Clamped to [0,0] (node's measured 20x20 still fits)
        assert_eq!(result.position_absolute, XYPosition::new(0.0, 0.0));
        assert_eq!(result.position, XYPosition::new(0.0, 0.0));
    }

    #[test]
    fn calculate_node_position_unknown_returns_input() {
        let lookup: NodeLookup<()> = NodeLookup::new();
        let result = calculate_node_position(CalculateNodePositionParams {
            node_id: "missing",
            next_position: XYPosition::new(7.0, 8.0),
            node_lookup: &lookup,
            node_origin: (0.0, 0.0),
            node_extent: None,
            on_error: None,
        });
        assert_eq!(result.position_absolute, XYPosition::new(7.0, 8.0));
        assert_eq!(result.position, XYPosition::new(7.0, 8.0));
    }

    #[test]
    fn calculate_node_position_with_parent_extent() {
        let mut lookup: NodeLookup<()> = NodeLookup::new();
        let mut parent = measured_internal("p", 10.0, 10.0, 100.0, 100.0);
        parent.internals.position_absolute = XYPosition::new(10.0, 10.0);
        lookup.insert("p".into(), parent);

        let mut child = measured_internal("c", 5.0, 5.0, 20.0, 20.0);
        child.user.parent_id = Some("p".into());
        child.user.extent = NodeExtent::Parent;
        lookup.insert("c".into(), child);

        // Try to place child at absolute (-50, -50) — clamped to parent (10,10).
        let result = calculate_node_position(CalculateNodePositionParams {
            node_id: "c",
            next_position: XYPosition::new(-50.0, -50.0),
            node_lookup: &lookup,
            node_origin: (0.0, 0.0),
            node_extent: None,
            on_error: None,
        });
        assert_eq!(result.position_absolute, XYPosition::new(10.0, 10.0));
        // Position relative to parent (which is also at 10,10).
        assert_eq!(result.position, XYPosition::new(0.0, 0.0));
    }

    #[test]
    fn elements_to_remove_cascades_children_and_edges() {
        let mut parent = n("p", 0.0, 0.0);
        parent.deletable = Some(true);
        let mut child = n("c", 0.0, 0.0);
        child.parent_id = Some("p".into());
        let other = n("o", 0.0, 0.0);
        let nodes = vec![parent, child, other];

        let edges = vec![
            e("p-o", "p", "o"),
            e("c-o", "c", "o"),
            e("o-other", "o", "other"),
        ];

        let result = get_elements_to_remove(GetElementsToRemoveParams {
            nodes_to_remove: &["p"],
            edges_to_remove: &[],
            nodes: &nodes,
            edges: &edges,
            on_before_delete: None,
        });

        let node_ids: HashSet<&str> = result.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(node_ids.contains("p"));
        assert!(node_ids.contains("c"));
        assert!(!node_ids.contains("o"));

        // Edges connected to either removed node.
        let edge_ids: HashSet<&str> = result.edges.iter().map(|e| e.id.as_str()).collect();
        assert!(edge_ids.contains("p-o"));
        assert!(edge_ids.contains("c-o"));
        assert!(!edge_ids.contains("o-other"));
    }

    #[test]
    fn elements_to_remove_keeps_non_deletable() {
        let mut parent = n("p", 0.0, 0.0);
        parent.deletable = Some(false);
        let nodes = vec![parent];
        let edges: Vec<Edge<()>> = Vec::new();
        let result = get_elements_to_remove(GetElementsToRemoveParams {
            nodes_to_remove: &["p"],
            edges_to_remove: &[],
            nodes: &nodes,
            edges: &edges,
            on_before_delete: None,
        });
        assert!(result.nodes.is_empty());
    }

    #[test]
    fn elements_to_remove_on_before_delete_can_reject() {
        let nodes = vec![n("p", 0.0, 0.0)];
        let edges: Vec<Edge<()>> = Vec::new();
        let result = get_elements_to_remove(GetElementsToRemoveParams {
            nodes_to_remove: &["p"],
            edges_to_remove: &[],
            nodes: &nodes,
            edges: &edges,
            on_before_delete: Some(Box::new(|_, _| OnBeforeDeleteResult::<(), ()>::Reject)),
        });
        assert!(result.nodes.is_empty());
        assert!(result.edges.is_empty());
    }
}
