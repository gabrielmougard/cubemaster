//! Port of `xyflow-core/src/utils/edges/general.ts`.
//!
//! Status: implemented (phase 2).

#![allow(clippy::module_name_repetitions)]

use crate::types::connection::Connection;
use crate::types::edges::Edge;
use crate::types::geometry::{Rect, Transform};
use crate::types::nodes::InternalNode;
use crate::types::viewport::ZIndexMode;
use crate::utils::general::{
    box_to_rect, get_bounds_of_boxes, get_overlapping_area, internal_node_to_box,
};

// ---------------------------------------------------------------------------
// Edge geometry helpers
// ---------------------------------------------------------------------------

/// Returns the midpoint of a straight edge plus the absolute deltas
/// from the source to that midpoint.
///
/// Used by [`crate::utils::edges::straight::get_straight_path`] and the
/// smoothstep router as a fallback offset for label placement.
///
/// Mirrors the TS `getEdgeCenter`. Returns
/// `(centerX, centerY, offsetX, offsetY)`.
#[must_use]
pub fn get_edge_center(
    source_x: f64,
    source_y: f64,
    target_x: f64,
    target_y: f64,
) -> (f64, f64, f64, f64) {
    let x_offset = (target_x - source_x).abs() / 2.0;
    let center_x = if target_x < source_x {
        target_x + x_offset
    } else {
        target_x - x_offset
    };
    let y_offset = (target_y - source_y).abs() / 2.0;
    let center_y = if target_y < source_y {
        target_y + y_offset
    } else {
        target_y - y_offset
    };
    (center_x, center_y, x_offset, y_offset)
}

/// Parameters of [`get_elevated_edge_z_index`].
pub struct GetEdgeZIndexParams<'a, D: Clone = ()> {
    pub source_node: &'a InternalNode<D>,
    pub target_node: &'a InternalNode<D>,
    pub selected: bool,
    pub z_index: f64,
    pub elevate_on_select: bool,
    pub z_index_mode: ZIndexMode,
}

/// Returns the z-index for an edge based on the nodes it connects and
/// whether it is selected.
///
/// Mirrors `getElevatedEdgeZIndex`.
#[must_use]
pub fn get_elevated_edge_z_index<D: Clone>(params: GetEdgeZIndexParams<'_, D>) -> f64 {
    if params.z_index_mode == ZIndexMode::Manual {
        return params.z_index;
    }

    let edge_z = if params.elevate_on_select && params.selected {
        params.z_index + 1000.0
    } else {
        params.z_index
    };

    let source_contribution = if params.source_node.user.parent_id.is_some()
        || (params.elevate_on_select && params.source_node.user.selected.unwrap_or(false))
    {
        params.source_node.internals.z
    } else {
        0.0
    };
    let target_contribution = if params.target_node.user.parent_id.is_some()
        || (params.elevate_on_select && params.target_node.user.selected.unwrap_or(false))
    {
        params.target_node.internals.z
    } else {
        0.0
    };
    let node_z = source_contribution.max(target_contribution);
    edge_z + node_z
}

/// Parameters of [`is_edge_visible`].
pub struct IsEdgeVisibleParams<'a, D: Clone = ()> {
    pub source_node: &'a InternalNode<D>,
    pub target_node: &'a InternalNode<D>,
    pub width: f64,
    pub height: f64,
    pub transform: Transform,
}

/// Returns `true` when the edge between source and target is at least
/// partially inside the visible viewport.
///
/// Mirrors the TS `isEdgeVisible`.
#[must_use]
pub fn is_edge_visible<D: Clone>(params: IsEdgeVisibleParams<'_, D>) -> bool {
    let mut edge_box = get_bounds_of_boxes(
        internal_node_to_box(params.source_node),
        internal_node_to_box(params.target_node),
    );

    if edge_box.x == edge_box.x2 {
        edge_box.x2 += 1.0;
    }
    if edge_box.y == edge_box.y2 {
        edge_box.y2 += 1.0;
    }

    let Transform(tx, ty, scale) = params.transform;
    let view_rect = Rect {
        x: -tx / scale,
        y: -ty / scale,
        width: params.width / scale,
        height: params.height / scale,
    };

    get_overlapping_area(view_rect, box_to_rect(edge_box)) > 0.0
}

// ---------------------------------------------------------------------------
// Edge id generator + addEdge/reconnectEdge
// ---------------------------------------------------------------------------

/// Custom edge id generator type — receives a borrowed [`Connection`]
/// and returns a `String` id.
///
/// Mirrors `GetEdgeId = (params: Connection | EdgeBase) => string`. To
/// keep the Rust API simple we always project to a [`Connection`]
/// view; helpers that need to feed an `EdgeBase` can call
/// [`edge_to_connection_view`] first.
pub type GetEdgeIdFn = Box<dyn Fn(&Connection) -> String + Send + Sync>;

/// Default edge id generator: `xy-edge__{source}{sourceHandle}-{target}{targetHandle}`.
#[must_use]
pub fn get_edge_id(c: &Connection) -> String {
    format!(
        "xy-edge__{}{}-{}{}",
        c.source,
        c.source_handle.as_deref().unwrap_or(""),
        c.target,
        c.target_handle.as_deref().unwrap_or(""),
    )
}

/// Project an [`Edge`] into a borrowed [`Connection`]-shaped temporary.
///
/// Used by [`add_edge`]'s id generator path.
#[must_use]
pub fn edge_to_connection_view<D: Clone>(e: &Edge<D>) -> Connection {
    Connection {
        source: e.source.clone(),
        target: e.target.clone(),
        source_handle: e.source_handle.clone(),
        target_handle: e.target_handle.clone(),
    }
}

fn connection_exists<D: Clone>(edge: &Edge<D>, edges: &[Edge<D>]) -> bool {
    edges.iter().any(|el| {
        el.source == edge.source
            && el.target == edge.target
            && handles_match(&el.source_handle, &edge.source_handle)
            && handles_match(&el.target_handle, &edge.target_handle)
    })
}

fn handles_match(a: &Option<String>, b: &Option<String>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => x == y,
        (None, None) => true,
        // Mirrors the TS `(!a && !b)` clause: empty string and `null`
        // are both considered "no handle". We don't have empty strings
        // here, so just None-None is enough.
        _ => false,
    }
}

/// Either a full [`Edge`] or a bare [`Connection`] — the input shape
/// accepted by [`add_edge`].
///
/// Mirrors the TS overload `addEdge<EdgeType>(edge: EdgeType | Connection, edges, options)`.
pub enum AddEdgeInput<D: Clone> {
    /// Full edge (must have a non-empty id).
    Edge(Edge<D>),
    /// Bare connection — the id will be generated.
    Connection(Connection),
}

impl<D: Clone> From<Edge<D>> for AddEdgeInput<D> {
    fn from(e: Edge<D>) -> Self {
        AddEdgeInput::Edge(e)
    }
}

impl<D: Clone> From<Connection> for AddEdgeInput<D> {
    fn from(c: Connection) -> Self {
        AddEdgeInput::Connection(c)
    }
}

/// Options for [`add_edge`].
#[derive(Default)]
pub struct AddEdgeOptions {
    /// Custom function to generate edge IDs. If `None`, [`get_edge_id`]
    /// is used.
    pub get_edge_id: Option<GetEdgeIdFn>,
}

impl std::fmt::Debug for AddEdgeOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddEdgeOptions")
            .field("get_edge_id", &self.get_edge_id.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

/// Adds an edge to the array, validating shape and skipping duplicates.
///
/// Mirrors the TS `addEdge`. Returns the new edges vec.
///
/// In keeping with the workspace decision to "mutate `&mut` directly",
/// callers may prefer [`add_edge_in_place`].
#[must_use]
pub fn add_edge<D: Clone + Default>(
    edge_params: AddEdgeInput<D>,
    edges: Vec<Edge<D>>,
    options: AddEdgeOptions,
) -> Vec<Edge<D>> {
    let mut edges = edges;
    add_edge_in_place(edge_params, &mut edges, options);
    edges
}

/// In-place variant of [`add_edge`] — appends to `edges` if not a
/// duplicate. Returns `true` iff an edge was actually added.
pub fn add_edge_in_place<D: Clone + Default>(
    edge_params: AddEdgeInput<D>,
    edges: &mut Vec<Edge<D>>,
    options: AddEdgeOptions,
) -> bool {
    let id_gen = options
        .get_edge_id
        .unwrap_or_else(|| Box::new(|c: &Connection| get_edge_id(c)));

    let candidate = match edge_params {
        AddEdgeInput::Edge(e) => {
            // TS rejects edges without source/target. In Rust the
            // fields are `String`, never absent, but they could still
            // be empty.
            if e.source.is_empty() || e.target.is_empty() {
                return false;
            }
            e
        }
        AddEdgeInput::Connection(c) => {
            if c.source.is_empty() || c.target.is_empty() {
                return false;
            }
            let id = id_gen(&c);
            Edge {
                id,
                type_: None,
                source: c.source,
                target: c.target,
                source_handle: c.source_handle,
                target_handle: c.target_handle,
                animated: None,
                hidden: None,
                deletable: None,
                selectable: None,
                data: D::default(),
                selected: None,
                marker_start: None,
                marker_end: None,
                z_index: None,
                aria_label: None,
                interaction_width: None,
            }
        }
    };

    if connection_exists(&candidate, edges) {
        return false;
    }

    edges.push(candidate);
    true
}

/// Options for [`reconnect_edge`].
pub struct ReconnectEdgeOptions {
    /// Whether the id of the old edge should be replaced by a freshly
    /// generated one based on the new connection. Defaults to `true`
    /// (matching the TS default).
    pub should_replace_id: bool,
    pub get_edge_id: Option<GetEdgeIdFn>,
}

impl Default for ReconnectEdgeOptions {
    fn default() -> Self {
        ReconnectEdgeOptions {
            should_replace_id: true,
            get_edge_id: None,
        }
    }
}

impl std::fmt::Debug for ReconnectEdgeOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReconnectEdgeOptions")
            .field("should_replace_id", &self.should_replace_id)
            .field("get_edge_id", &self.get_edge_id.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

/// Update an existing edge with new connection endpoints.
///
/// Mirrors the TS `reconnectEdge`.
#[must_use]
pub fn reconnect_edge<D: Clone>(
    old_edge: &Edge<D>,
    new_connection: Connection,
    edges: Vec<Edge<D>>,
    options: ReconnectEdgeOptions,
) -> Vec<Edge<D>> {
    if new_connection.source.is_empty() || new_connection.target.is_empty() {
        return edges;
    }

    let found = edges.iter().any(|e| e.id == old_edge.id);
    if !found {
        return edges;
    }

    let id_gen = options
        .get_edge_id
        .unwrap_or_else(|| Box::new(|c: &Connection| get_edge_id(c)));

    let new_id = if options.should_replace_id {
        id_gen(&new_connection)
    } else {
        old_edge.id.clone()
    };

    let new_edge = Edge {
        id: new_id,
        source: new_connection.source,
        target: new_connection.target,
        source_handle: new_connection.source_handle,
        target_handle: new_connection.target_handle,
        // copy the rest of the fields verbatim from the old edge
        type_: old_edge.type_.clone(),
        animated: old_edge.animated,
        hidden: old_edge.hidden,
        deletable: old_edge.deletable,
        selectable: old_edge.selectable,
        data: old_edge.data.clone(),
        selected: old_edge.selected,
        marker_start: old_edge.marker_start.clone(),
        marker_end: old_edge.marker_end.clone(),
        z_index: old_edge.z_index,
        aria_label: old_edge.aria_label.clone(),
        interaction_width: old_edge.interaction_width,
    };

    let old_id = old_edge.id.clone();
    edges
        .into_iter()
        .filter(|e| e.id != old_id)
        .chain(std::iter::once(new_edge))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_center_simple() {
        let (cx, cy, ox, oy) = get_edge_center(0.0, 0.0, 100.0, 200.0);
        assert!((cx - 50.0).abs() < 1e-9);
        assert!((cy - 100.0).abs() < 1e-9);
        assert!((ox - 50.0).abs() < 1e-9);
        assert!((oy - 100.0).abs() < 1e-9);
    }

    #[test]
    fn edge_center_when_target_left_of_source() {
        // sourceX=100, targetX=0 → centerX should be 50, offsetX=50.
        let (cx, _, ox, _) = get_edge_center(100.0, 0.0, 0.0, 0.0);
        assert!((cx - 50.0).abs() < 1e-9);
        assert!((ox - 50.0).abs() < 1e-9);
    }

    #[test]
    fn default_edge_id_format() {
        let id = get_edge_id(&Connection {
            source: "a".into(),
            target: "b".into(),
            source_handle: None,
            target_handle: None,
        });
        assert_eq!(id, "xy-edge__a-b");
        let id2 = get_edge_id(&Connection {
            source: "a".into(),
            target: "b".into(),
            source_handle: Some("h1".into()),
            target_handle: Some("h2".into()),
        });
        assert_eq!(id2, "xy-edge__ah1-bh2");
    }

    #[test]
    fn add_edge_skips_duplicates() {
        let mut edges: Vec<Edge<()>> = Vec::new();
        let a = Edge::<()>::minimal("e1", "a", "b");
        assert!(add_edge_in_place(a.clone().into(), &mut edges, AddEdgeOptions::default()));
        assert_eq!(edges.len(), 1);
        // Same source/target → duplicate → not added.
        let b = Edge::<()>::minimal("e2", "a", "b");
        assert!(!add_edge_in_place(b.into(), &mut edges, AddEdgeOptions::default()));
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn add_edge_from_connection_generates_id() {
        let mut edges: Vec<Edge<()>> = Vec::new();
        let c = Connection {
            source: "a".into(),
            target: "b".into(),
            source_handle: None,
            target_handle: None,
        };
        assert!(add_edge_in_place(c.into(), &mut edges, AddEdgeOptions::default()));
        assert_eq!(edges[0].id, "xy-edge__a-b");
    }

    #[test]
    fn add_edge_rejects_empty_endpoints() {
        let mut edges: Vec<Edge<()>> = Vec::new();
        let bad = Edge::<()>::minimal("e1", "", "b");
        assert!(!add_edge_in_place(bad.into(), &mut edges, AddEdgeOptions::default()));
        assert!(edges.is_empty());
    }

    #[test]
    fn reconnect_edge_replaces_endpoints() {
        let edges: Vec<Edge<()>> = vec![Edge::<()>::minimal("xy-edge__a-b", "a", "b")];
        let new_c = Connection {
            source: "a".into(),
            target: "c".into(),
            source_handle: None,
            target_handle: None,
        };
        let updated = reconnect_edge(&edges[0], new_c, edges.clone(), ReconnectEdgeOptions::default());
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].source, "a");
        assert_eq!(updated[0].target, "c");
        // shouldReplaceId defaults true → id is regenerated
        assert_eq!(updated[0].id, "xy-edge__a-c");
    }

    #[test]
    fn reconnect_edge_preserves_id_when_requested() {
        let edges: Vec<Edge<()>> = vec![Edge::<()>::minimal("custom-id", "a", "b")];
        let new_c = Connection {
            source: "a".into(),
            target: "c".into(),
            source_handle: None,
            target_handle: None,
        };
        let updated = reconnect_edge(
            &edges[0],
            new_c,
            edges.clone(),
            ReconnectEdgeOptions {
                should_replace_id: false,
                get_edge_id: None,
            },
        );
        assert_eq!(updated[0].id, "custom-id");
    }

    #[test]
    fn reconnect_edge_no_op_when_old_missing() {
        let stranger = Edge::<()>::minimal("not-in-list", "a", "b");
        let edges: Vec<Edge<()>> = vec![Edge::<()>::minimal("real", "x", "y")];
        let new_c = Connection {
            source: "a".into(),
            target: "c".into(),
            source_handle: None,
            target_handle: None,
        };
        let updated = reconnect_edge(&stranger, new_c, edges.clone(), ReconnectEdgeOptions::default());
        assert_eq!(updated, edges);
    }
}
