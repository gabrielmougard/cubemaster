//! End-to-end integration smoke test for `rgraph-core`.
//!
//! Exercises a tiny but realistic flow:
//!   1. Build a `NodeLookup` with two measured nodes.
//!   2. Adopt them via `adopt_user_nodes`.
//!   3. Compute edge geometry between them via `get_edge_position`.
//!   4. Render the SVG path with `get_bezier_path` / `get_straight_path`.
//!   5. Drive an `XYPanZoom` instance to zoom in.
//!   6. Tear everything down.
//!
//! This isn't a deep correctness test — the per-module unit tests
//! cover that. It exists to catch *integration-level* breakage:
//! re-exports going stale, generics not lining up, etc.

use rgraph_core::{
    constants::INFINITE_EXTENT,
    types::{
        connection::ConnectionMode,
        edges::Edge,
        geometry::{Position, Rect},
        handles::HandleType,
        nodes::{InternalNode, MeasuredDimensions, Node, NodeHandle, NodeLookup},
        panzoom::PanZoomParams,
        viewport::Viewport,
    },
    utils::edges::{
        bezier::{get_bezier_path, GetBezierPathParams},
        general::{add_edge_in_place, get_edge_id, AddEdgeOptions},
        positions::{get_edge_position, GetEdgePositionParams},
        straight::{get_straight_path, GetStraightPathParams},
    },
    utils::store::{adopt_user_nodes, AdoptUserNodesOptions},
    xypanzoom::XYPanZoom,
};
use std::collections::HashMap;

fn build_test_node(id: &str, x: f64, y: f64, w: f64, h: f64) -> Node<()> {
    let mut node = Node::<()>::minimal(id, x, y);
    node.measured = Some(MeasuredDimensions {
        width: Some(w),
        height: Some(h),
    });
    // Single source handle on the right and target handle on the left.
    node.handles = Some(vec![
        NodeHandle {
            id: None,
            x: w,
            y: h / 2.0,
            position: Position::Right,
            type_: HandleType::Source,
            width: Some(1.0),
            height: Some(1.0),
        },
        NodeHandle {
            id: None,
            x: 0.0,
            y: h / 2.0,
            position: Position::Left,
            type_: HandleType::Target,
            width: Some(1.0),
            height: Some(1.0),
        },
    ]);
    node
}

#[test]
fn adopt_two_nodes_and_render_bezier_edge() {
    let mut node_lookup: NodeLookup<()> = HashMap::new();
    let mut parent_lookup = HashMap::new();
    let nodes = vec![
        build_test_node("source", 0.0, 0.0, 100.0, 50.0),
        build_test_node("target", 200.0, 100.0, 100.0, 50.0),
    ];

    let result = adopt_user_nodes(
        &nodes,
        &mut node_lookup,
        &mut parent_lookup,
        &AdoptUserNodesOptions::default(),
    );
    assert!(result.nodes_initialized);
    assert!(!result.has_selected_nodes);
    assert_eq!(node_lookup.len(), 2);

    // Build an edge.
    let mut edges: Vec<Edge<()>> = Vec::new();
    let added = add_edge_in_place(
        Edge::<()>::minimal("e1", "source", "target").into(),
        &mut edges,
        AddEdgeOptions::default(),
    );
    assert!(added);
    assert_eq!(edges.len(), 1);

    // Compute edge position. The `get_edge_position` helper looks at
    // `internals.handle_bounds` which `adopt_user_nodes` populates
    // from the user-supplied `handles` array.
    let source = node_lookup.get("source").unwrap();
    let target = node_lookup.get("target").unwrap();
    let position = get_edge_position(GetEdgePositionParams {
        id: "e1",
        source_node: source,
        source_handle: None,
        target_node: target,
        target_handle: None,
        connection_mode: ConnectionMode::Strict,
        on_error: None,
    })
    .expect("edge position should resolve");
    assert_eq!(position.source_position, Position::Right);
    assert_eq!(position.target_position, Position::Left);

    // Render bezier + straight paths.
    let (bezier_path, _, _, _, _) = get_bezier_path(GetBezierPathParams {
        source_x: position.source_x,
        source_y: position.source_y,
        source_position: position.source_position,
        target_x: position.target_x,
        target_y: position.target_y,
        target_position: position.target_position,
        curvature: 0.25,
    });
    assert!(bezier_path.starts_with('M'));
    assert!(bezier_path.contains('C'));

    let (straight_path, _, _, _, _) = get_straight_path(GetStraightPathParams {
        source_x: position.source_x,
        source_y: position.source_y,
        target_x: position.target_x,
        target_y: position.target_y,
    });
    assert!(straight_path.starts_with("M "));
    assert!(straight_path.contains("L "));
}

#[test]
fn panzoom_drives_full_viewport_lifecycle() {
    let panzoom = XYPanZoom::<()>::new_single(PanZoomParams {
        min_zoom: 0.25,
        max_zoom: 4.0,
        viewport: Viewport::IDENTITY,
        translate_extent: INFINITE_EXTENT,
        dom_bbox: Rect::new(0.0, 0.0, 1024.0, 768.0),
        on_dragging_change: Box::new(|_| {}),
        on_pan_zoom_start: None,
        on_pan_zoom: None,
        on_pan_zoom_end: None,
    });

    // Initial viewport is identity.
    assert_eq!(panzoom.get_viewport(), Viewport::IDENTITY);

    // scale_to clamps to extent.
    let _ = panzoom.scale_to(8.0, None);
    assert!((panzoom.get_viewport().zoom - 4.0).abs() < 1e-9);

    // scale_to back to 1 — synchronous via resolved promise.
    let promise = panzoom.scale_to(1.0, None);
    assert_eq!(promise.try_take(), Some(true));
    assert!((panzoom.get_viewport().zoom - 1.0).abs() < 1e-9);

    // set_viewport via synchronous path.
    let v = Viewport::new(50.0, 100.0, 2.0);
    let _ = panzoom.set_viewport(v, None);
    let now = panzoom.get_viewport();
    assert!((now.x - 50.0).abs() < 1e-9);
    assert!((now.y - 100.0).abs() < 1e-9);
    assert!((now.zoom - 2.0).abs() < 1e-9);

    panzoom.destroy();
}

#[test]
fn promise_round_trip() {
    let p = rgraph_core::promise::Promise::resolved(42i32);
    assert_eq!(p.try_take(), Some(42));

    let (promise, resolver) = rgraph_core::promise::channel::<bool>();
    resolver.resolve(true);
    assert_eq!(promise.try_take(), Some(true));
}

#[test]
fn styles_bundle_is_consumable() {
    use rgraph_core::styles::{ALL_CSS, BASE_CSS, INIT_CSS, NODE_RESIZER_CSS, STYLE_CSS};
    assert!(!BASE_CSS.is_empty());
    assert!(!INIT_CSS.is_empty());
    assert!(!NODE_RESIZER_CSS.is_empty());
    assert!(!STYLE_CSS.is_empty());
    assert!(ALL_CSS.contains(BASE_CSS.trim_end()) || ALL_CSS.contains(BASE_CSS));
}

#[test]
fn edge_id_deterministic() {
    let id_a = get_edge_id(&rgraph_core::types::connection::Connection {
        source: "a".into(),
        target: "b".into(),
        source_handle: None,
        target_handle: None,
    });
    let id_b = get_edge_id(&rgraph_core::types::connection::Connection {
        source: "a".into(),
        target: "b".into(),
        source_handle: None,
        target_handle: None,
    });
    assert_eq!(id_a, id_b);
    assert_eq!(id_a, "xy-edge__a-b");
}

#[test]
fn internal_node_round_trips_through_lookup() {
    let user = build_test_node("n1", 10.0, 20.0, 100.0, 50.0);
    let internal = InternalNode::from_user(user);
    let mut lookup: NodeLookup<()> = HashMap::new();
    lookup.insert("n1".into(), internal);
    assert!(lookup.contains_key("n1"));
    let n = lookup.get("n1").unwrap();
    assert_eq!(n.user.position.x, 10.0);
    assert_eq!(n.measured.width, Some(100.0));
}
