//! Port of `xyflow-core/src/utils/edges/positions.ts`.
//!
//! Status: implemented (phase 2).

#![allow(clippy::module_name_repetitions)]

use crate::types::connection::ConnectionMode;
use crate::types::edges::EdgePosition;
use crate::types::geometry::{Position, XYPosition};
use crate::types::handles::{Handle, HandleType};
use crate::types::nodes::{InternalNode, NodeHandle, NodeHandleBounds};
use crate::utils::general::get_node_dimensions;

/// Optional error reporter, mirroring the TS `OnError = (id, message) => void`.
pub type OnErrorFn = Box<dyn Fn(&str, &str) + Send + Sync>;

/// Parameters for [`get_edge_position`].
pub struct GetEdgePositionParams<'a, D: Clone = ()> {
    pub id: &'a str,
    pub source_node: &'a InternalNode<D>,
    pub source_handle: Option<&'a str>,
    pub target_node: &'a InternalNode<D>,
    pub target_handle: Option<&'a str>,
    pub connection_mode: ConnectionMode,
    pub on_error: Option<&'a OnErrorFn>,
}

fn is_node_initialized<D: Clone>(node: &InternalNode<D>) -> bool {
    let has_handles = node.internals.handle_bounds.is_some()
        || node.user.handles.as_ref().is_some_and(|h| !h.is_empty());
    let has_dim = node.measured.width.is_some()
        || node.user.width.is_some()
        || node.user.initial_width.is_some();
    has_handles && has_dim
}

fn to_handle_bounds(handles: Option<&Vec<NodeHandle>>, node_id: &str) -> Option<NodeHandleBounds> {
    let handles = handles?;
    let mut source: Vec<Handle> = Vec::new();
    let mut target: Vec<Handle> = Vec::new();
    for h in handles {
        let resolved = Handle {
            id: h.id.clone(),
            node_id: node_id.to_string(),
            x: h.x,
            y: h.y,
            position: h.position,
            type_: h.type_,
            // TS defaults width/height to 1 when absent.
            width: h.width.unwrap_or(1.0),
            height: h.height.unwrap_or(1.0),
        };
        match h.type_ {
            HandleType::Source => source.push(resolved),
            HandleType::Target => target.push(resolved),
        }
    }
    Some(NodeHandleBounds {
        source: Some(source),
        target: Some(target),
    })
}

fn get_handle<'a>(bounds: &'a [Handle], handle_id: Option<&str>) -> Option<&'a Handle> {
    if bounds.is_empty() {
        return None;
    }
    match handle_id {
        None => bounds.first(),
        Some(id) => bounds.iter().find(|h| h.id.as_deref() == Some(id)),
    }
}

/// Resolve the absolute screen-space position of a handle.
///
/// Mirrors the TS `getHandlePosition`. When `handle` is `None` the
/// node's bounds are used and `fallback_position` decides which side.
#[must_use]
pub fn get_handle_position<D: Clone>(
    node: &InternalNode<D>,
    handle: Option<&Handle>,
    fallback_position: Position,
    center: bool,
) -> XYPosition {
    let x = handle.map(|h| h.x).unwrap_or(0.0) + node.internals.position_absolute.x;
    let y = handle.map(|h| h.y).unwrap_or(0.0) + node.internals.position_absolute.y;
    let (width, height) = match handle {
        Some(h) => (h.width, h.height),
        None => {
            let d = get_node_dimensions(node);
            (d.width, d.height)
        }
    };
    if center {
        return XYPosition {
            x: x + width / 2.0,
            y: y + height / 2.0,
        };
    }
    let position = handle.map(|h| h.position).unwrap_or(fallback_position);
    match position {
        Position::Top => XYPosition { x: x + width / 2.0, y },
        Position::Right => XYPosition { x: x + width, y: y + height / 2.0 },
        Position::Bottom => XYPosition { x: x + width / 2.0, y: y + height },
        Position::Left => XYPosition { x, y: y + height / 2.0 },
    }
}

/// Resolve the source/target endpoint geometry for an edge between
/// two adopted nodes.
///
/// Returns `None` and (optionally) calls `on_error` when one of the
/// handles cannot be resolved or the nodes are not yet initialized.
///
/// Mirrors the TS `getEdgePosition`.
#[must_use]
pub fn get_edge_position<D: Clone>(params: GetEdgePositionParams<'_, D>) -> Option<EdgePosition> {
    let GetEdgePositionParams {
        id,
        source_node,
        source_handle,
        target_node,
        target_handle,
        connection_mode,
        on_error,
    } = params;

    if !is_node_initialized(source_node) || !is_node_initialized(target_node) {
        return None;
    }

    let source_bounds = source_node
        .internals
        .handle_bounds
        .clone()
        .or_else(|| to_handle_bounds(source_node.user.handles.as_ref(), &source_node.user.id));
    let target_bounds = target_node
        .internals
        .handle_bounds
        .clone()
        .or_else(|| to_handle_bounds(target_node.user.handles.as_ref(), &target_node.user.id));

    let empty: Vec<Handle> = Vec::new();
    let source_list: &[Handle] = source_bounds
        .as_ref()
        .and_then(|b| b.source.as_deref())
        .unwrap_or(&empty);
    let source_handle_resolved = get_handle(source_list, source_handle);

    // For loose connection mode, target handles include both sides.
    let target_combined: Vec<Handle> = match connection_mode {
        ConnectionMode::Strict => target_bounds
            .as_ref()
            .and_then(|b| b.target.clone())
            .unwrap_or_default(),
        ConnectionMode::Loose => {
            let mut t = target_bounds
                .as_ref()
                .and_then(|b| b.target.clone())
                .unwrap_or_default();
            if let Some(s) = target_bounds.as_ref().and_then(|b| b.source.clone()) {
                t.extend(s);
            }
            t
        }
    };
    let target_handle_resolved = get_handle(&target_combined, target_handle);

    if source_handle_resolved.is_none() || target_handle_resolved.is_none() {
        if let Some(cb) = on_error {
            let msg = if source_handle_resolved.is_none() {
                crate::constants::error_008(HandleType::Source, id, source_handle, target_handle)
            } else {
                crate::constants::error_008(HandleType::Target, id, source_handle, target_handle)
            };
            cb("008", &msg);
        }
        return None;
    }

    let source_position = source_handle_resolved
        .map(|h| h.position)
        .unwrap_or(Position::Bottom);
    let target_position = target_handle_resolved
        .map(|h| h.position)
        .unwrap_or(Position::Top);
    let source = get_handle_position(source_node, source_handle_resolved, source_position, false);
    let target = get_handle_position(target_node, target_handle_resolved, target_position, false);

    Some(EdgePosition {
        source_x: source.x,
        source_y: source.y,
        target_x: target.x,
        target_y: target.y,
        source_position,
        target_position,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::nodes::{MeasuredDimensions, Node};

    fn make_node_with_handles(id: &str, x: f64, y: f64, w: f64, h: f64, handles: Vec<NodeHandle>) -> InternalNode<()> {
        let mut user: Node<()> = Node::minimal(id, x, y);
        user.handles = Some(handles);
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
    fn handle_position_center_disregards_position() {
        let mut user: Node<()> = Node::minimal("n", 0.0, 0.0);
        user.measured = Some(MeasuredDimensions {
            width: Some(100.0),
            height: Some(50.0),
        });
        let mut internal = InternalNode::from_user(user);
        internal.measured = MeasuredDimensions {
            width: Some(100.0),
            height: Some(50.0),
        };
        let pos = get_handle_position(&internal, None, Position::Right, true);
        assert_eq!(pos, XYPosition::new(50.0, 25.0));
    }

    #[test]
    fn handle_position_each_side() {
        let mut user: Node<()> = Node::minimal("n", 0.0, 0.0);
        user.measured = Some(MeasuredDimensions {
            width: Some(100.0),
            height: Some(50.0),
        });
        let mut internal = InternalNode::from_user(user);
        internal.measured = MeasuredDimensions {
            width: Some(100.0),
            height: Some(50.0),
        };
        assert_eq!(
            get_handle_position(&internal, None, Position::Top, false),
            XYPosition::new(50.0, 0.0)
        );
        assert_eq!(
            get_handle_position(&internal, None, Position::Right, false),
            XYPosition::new(100.0, 25.0)
        );
        assert_eq!(
            get_handle_position(&internal, None, Position::Bottom, false),
            XYPosition::new(50.0, 50.0)
        );
        assert_eq!(
            get_handle_position(&internal, None, Position::Left, false),
            XYPosition::new(0.0, 25.0)
        );
    }

    #[test]
    fn edge_position_returns_none_for_uninitialised_nodes() {
        let src = InternalNode::from_user(Node::<()>::minimal("a", 0.0, 0.0));
        let tgt = InternalNode::from_user(Node::<()>::minimal("b", 100.0, 100.0));
        let result = get_edge_position(GetEdgePositionParams {
            id: "e1",
            source_node: &src,
            source_handle: None,
            target_node: &tgt,
            target_handle: None,
            connection_mode: ConnectionMode::Strict,
            on_error: None,
        });
        assert!(result.is_none());
    }

    #[test]
    fn edge_position_basic_horizontal() {
        // Two nodes, source has a Right source handle, target has a Left target handle.
        let src = make_node_with_handles(
            "a",
            0.0,
            0.0,
            100.0,
            50.0,
            vec![NodeHandle {
                id: None,
                x: 100.0,
                y: 25.0,
                position: Position::Right,
                type_: HandleType::Source,
                width: Some(1.0),
                height: Some(1.0),
            }],
        );
        let tgt = make_node_with_handles(
            "b",
            200.0,
            0.0,
            100.0,
            50.0,
            vec![NodeHandle {
                id: None,
                x: 0.0,
                y: 25.0,
                position: Position::Left,
                type_: HandleType::Target,
                width: Some(1.0),
                height: Some(1.0),
            }],
        );
        let result = get_edge_position(GetEdgePositionParams {
            id: "e1",
            source_node: &src,
            source_handle: None,
            target_node: &tgt,
            target_handle: None,
            connection_mode: ConnectionMode::Strict,
            on_error: None,
        })
        .expect("edge position should resolve");

        assert_eq!(result.source_position, Position::Right);
        assert_eq!(result.target_position, Position::Left);
        // Source handle: x=100 (handle) + 0 (node) = 100, plus full width (1) → 101 (Right edge)
        assert!((result.source_x - 101.0).abs() < 1e-9);
        // Target handle: x=0 (handle) + 200 (node) = 200, Left → x stays
        assert!((result.target_x - 200.0).abs() < 1e-9);
    }

    #[test]
    fn edge_position_loose_mode_includes_source_handles_on_target() {
        // In loose mode, target_handle can resolve to a source-type
        // handle on the target node.
        let src = make_node_with_handles(
            "a",
            0.0,
            0.0,
            100.0,
            50.0,
            vec![NodeHandle {
                id: None,
                x: 100.0,
                y: 25.0,
                position: Position::Right,
                type_: HandleType::Source,
                width: Some(1.0),
                height: Some(1.0),
            }],
        );
        // Target has only a SOURCE handle, no target handle.
        let tgt = make_node_with_handles(
            "b",
            200.0,
            0.0,
            100.0,
            50.0,
            vec![NodeHandle {
                id: None,
                x: 0.0,
                y: 25.0,
                position: Position::Left,
                type_: HandleType::Source,
                width: Some(1.0),
                height: Some(1.0),
            }],
        );
        // Strict → fails.
        assert!(get_edge_position(GetEdgePositionParams {
            id: "e1",
            source_node: &src,
            source_handle: None,
            target_node: &tgt,
            target_handle: None,
            connection_mode: ConnectionMode::Strict,
            on_error: None,
        })
        .is_none());
        // Loose → succeeds.
        assert!(get_edge_position(GetEdgePositionParams {
            id: "e1",
            source_node: &src,
            source_handle: None,
            target_node: &tgt,
            target_handle: None,
            connection_mode: ConnectionMode::Loose,
            on_error: None,
        })
        .is_some());
    }
}
