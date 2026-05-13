//! Port of `xyflow-core/src/xyhandle/utils.ts`.
//!
//! Status: implemented (phase 6).

#![allow(clippy::module_name_repetitions)]

use crate::types::connection::ConnectionMode;
use crate::types::geometry::{Rect, XYPosition};
use crate::types::handles::{Handle, HandleType};
use crate::types::nodes::{InternalNode, NodeLookup};
use crate::utils::edges::positions::get_handle_position;
use crate::utils::general::{get_overlapping_area, internal_node_to_rect};

/// Extra padding around the connection-radius search box. Matches
/// the TS `ADDITIONAL_DISTANCE = 250`.
const ADDITIONAL_DISTANCE: f64 = 250.0;

/// Return every node whose rect overlaps a `distance`-radius box
/// centred at `position`.
///
/// Mirrors the TS private `getNodesWithinDistance`.
#[must_use]
pub fn get_nodes_within_distance<'a, D: Clone>(
    position: XYPosition,
    node_lookup: &'a NodeLookup<D>,
    distance: f64,
) -> Vec<&'a InternalNode<D>> {
    let rect = Rect {
        x: position.x - distance,
        y: position.y - distance,
        width: distance * 2.0,
        height: distance * 2.0,
    };
    node_lookup
        .values()
        .filter(|node| get_overlapping_area(rect, internal_node_to_rect(node)) > 0.0)
        .collect()
}

/// Returns the closest handle (by Euclidean distance) to `position`
/// that lies within `connection_radius` and is not the `from_handle`
/// itself.
///
/// Mirrors the TS `getClosestHandle`. When two handles tie at the
/// same distance, the one with the opposite [`HandleType`] is
/// preferred.
#[must_use]
pub fn get_closest_handle<D: Clone>(
    position: XYPosition,
    connection_radius: f64,
    node_lookup: &NodeLookup<D>,
    from_node_id: &str,
    from_handle_id: Option<&str>,
    from_type: HandleType,
) -> Option<Handle> {
    let close_nodes = get_nodes_within_distance(
        position,
        node_lookup,
        connection_radius + ADDITIONAL_DISTANCE,
    );

    let mut closest_handles: Vec<Handle> = Vec::new();
    let mut min_distance = f64::INFINITY;

    for node in close_nodes {
        let Some(bounds) = node.internals.handle_bounds.as_ref() else {
            continue;
        };
        let empty: Vec<Handle> = Vec::new();
        let source = bounds.source.as_deref().unwrap_or(&empty);
        let target = bounds.target.as_deref().unwrap_or(&empty);
        let all_handles = source.iter().chain(target.iter());

        for handle in all_handles {
            // Skip the originating handle itself.
            if handle.node_id == from_node_id
                && handle.type_ == from_type
                && handle.id.as_deref() == from_handle_id
            {
                continue;
            }
            let absolute = get_handle_position(node, Some(handle), handle.position, true);
            let dx = absolute.x - position.x;
            let dy = absolute.y - position.y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > connection_radius {
                continue;
            }
            let resolved = Handle {
                id: handle.id.clone(),
                node_id: handle.node_id.clone(),
                x: absolute.x,
                y: absolute.y,
                position: handle.position,
                type_: handle.type_,
                width: handle.width,
                height: handle.height,
            };
            if distance < min_distance {
                closest_handles.clear();
                closest_handles.push(resolved);
                min_distance = distance;
            } else if (distance - min_distance).abs() < f64::EPSILON {
                closest_handles.push(resolved);
            }
        }
    }

    if closest_handles.is_empty() {
        return None;
    }
    if closest_handles.len() > 1 {
        let opposite = from_type.opposite();
        // Prefer the opposite-type handle when multiple tie.
        if let Some(found) = closest_handles.iter().find(|h| h.type_ == opposite) {
            return Some(found.clone());
        }
    }
    Some(closest_handles.into_iter().next().unwrap())
}

/// Resolve a handle by node id + type + handle id from a node lookup.
///
/// Mirrors the TS `getHandle` helper. With `connection_mode = Loose`
/// we search both source and target lists; with `Strict` only the
/// list matching `handle_type`. `with_absolute_position` re-computes
/// the handle's flow-space position.
#[must_use]
pub fn get_handle<D: Clone>(
    node_id: &str,
    handle_type: HandleType,
    handle_id: Option<&str>,
    node_lookup: &NodeLookup<D>,
    connection_mode: ConnectionMode,
    with_absolute_position: bool,
) -> Option<Handle> {
    let node = node_lookup.get(node_id)?;
    let bounds = node.internals.handle_bounds.as_ref()?;
    let empty: Vec<Handle> = Vec::new();
    let source = bounds.source.as_deref().unwrap_or(&empty);
    let target = bounds.target.as_deref().unwrap_or(&empty);

    let handle: Handle = if connection_mode == ConnectionMode::Strict {
        let list = match handle_type {
            HandleType::Source => source,
            HandleType::Target => target,
        };
        match handle_id {
            Some(id) => list.iter().find(|h| h.id.as_deref() == Some(id))?.clone(),
            None => list.first()?.clone(),
        }
    } else {
        let mut combined: Vec<&Handle> = Vec::with_capacity(source.len() + target.len());
        combined.extend(source.iter());
        combined.extend(target.iter());
        match handle_id {
            Some(id) => combined
                .into_iter()
                .find(|h| h.id.as_deref() == Some(id))?
                .clone(),
            None => combined.into_iter().next()?.clone(),
        }
    };

    if !with_absolute_position {
        return Some(handle);
    }
    let pos = get_handle_position(node, Some(&handle), handle.position, true);
    let mut resolved = handle;
    resolved.x = pos.x;
    resolved.y = pos.y;
    Some(resolved)
}

/// Resolve a [`HandleType`] from either an explicit `edge_updater_type`
/// (set when the gesture is started from an edge endpoint) or from the
/// hovered handle's pre-collected classes.
///
/// Mirrors the TS `getHandleType`. The TS variant inspects
/// `handleDomNode.classList`; here we accept the snapshot directly.
#[must_use]
pub fn get_handle_type(
    edge_updater_type: Option<HandleType>,
    handle_below: Option<&crate::xyhandle::types::HandleSnapshot>,
) -> Option<HandleType> {
    if let Some(t) = edge_updater_type {
        return Some(t);
    }
    handle_below.map(|s| s.type_)
}

/// Tri-state validity classifier used to drive UI feedback.
///
/// Mirrors the TS `isConnectionValid`:
///
/// * `Some(true)`  — handle is valid (allow drop),
/// * `Some(false)` — pointer is inside the radius but the handle is
///   invalid (show invalid feedback),
/// * `None`        — pointer is outside the radius (no feedback).
#[must_use]
#[inline]
pub fn is_connection_valid(is_inside_radius: bool, is_handle_valid: bool) -> Option<bool> {
    if is_handle_valid {
        Some(true)
    } else if is_inside_radius {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::geometry::Position;
    use crate::types::nodes::{MeasuredDimensions, Node, NodeHandleBounds};

    fn measured_with_handles(
        id: &str,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        handles: Vec<Handle>,
    ) -> InternalNode<()> {
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
        internal.internals.position_absolute = XYPosition::new(x, y);
        let mut source = Vec::new();
        let mut target = Vec::new();
        for h in handles {
            match h.type_ {
                HandleType::Source => source.push(h),
                HandleType::Target => target.push(h),
            }
        }
        internal.internals.handle_bounds = Some(NodeHandleBounds {
            source: Some(source),
            target: Some(target),
        });
        internal
    }

    fn handle(node_id: &str, id: Option<&str>, type_: HandleType, x: f64, y: f64) -> Handle {
        Handle {
            id: id.map(str::to_string),
            node_id: node_id.to_string(),
            x,
            y,
            position: match type_ {
                HandleType::Source => Position::Right,
                HandleType::Target => Position::Left,
            },
            type_,
            width: 1.0,
            height: 1.0,
        }
    }

    #[test]
    fn nodes_within_distance_includes_only_overlapping() {
        let mut lookup = NodeLookup::<()>::new();
        lookup.insert(
            "near".into(),
            measured_with_handles("near", 0.0, 0.0, 50.0, 50.0, vec![]),
        );
        lookup.insert(
            "far".into(),
            measured_with_handles("far", 1000.0, 1000.0, 50.0, 50.0, vec![]),
        );
        let near = get_nodes_within_distance(XYPosition::new(25.0, 25.0), &lookup, 100.0);
        assert_eq!(near.len(), 1);
        assert_eq!(near[0].user.id, "near");
    }

    #[test]
    fn closest_handle_returns_none_when_no_match() {
        let mut lookup = NodeLookup::<()>::new();
        lookup.insert(
            "a".into(),
            measured_with_handles("a", 0.0, 0.0, 10.0, 10.0, vec![]),
        );
        let result = get_closest_handle(
            XYPosition::ZERO,
            10.0,
            &lookup,
            "from",
            None,
            HandleType::Source,
        );
        assert!(result.is_none());
    }

    #[test]
    fn closest_handle_finds_nearest() {
        let mut lookup = NodeLookup::<()>::new();
        // Node A at origin, has a target handle at the centre.
        let a = measured_with_handles(
            "a",
            0.0,
            0.0,
            10.0,
            10.0,
            vec![handle("a", None, HandleType::Target, 5.0, 5.0)],
        );
        lookup.insert("a".into(), a);
        // Node B further away.
        let b = measured_with_handles(
            "b",
            100.0,
            100.0,
            10.0,
            10.0,
            vec![handle("b", None, HandleType::Target, 5.0, 5.0)],
        );
        lookup.insert("b".into(), b);

        // Pointer near A's handle.
        let result = get_closest_handle(
            XYPosition::new(8.0, 8.0),
            10.0,
            &lookup,
            "from",
            None,
            HandleType::Source,
        );
        let h = result.expect("should find a handle");
        assert_eq!(h.node_id, "a");
    }

    #[test]
    fn closest_handle_skips_from_handle() {
        let mut lookup = NodeLookup::<()>::new();
        let a = measured_with_handles(
            "a",
            0.0,
            0.0,
            10.0,
            10.0,
            vec![handle("a", Some("h1"), HandleType::Source, 5.0, 5.0)],
        );
        lookup.insert("a".into(), a);
        // Searching from the same handle should return None.
        let result = get_closest_handle(
            XYPosition::new(8.0, 8.0),
            20.0,
            &lookup,
            "a",
            Some("h1"),
            HandleType::Source,
        );
        assert!(result.is_none());
    }

    #[test]
    fn closest_handle_prefers_opposite_type_on_tie() {
        let mut lookup = NodeLookup::<()>::new();
        // Two handles on the same node at the same point: one source,
        // one target. From a source gesture we should pick the target.
        let h_src = handle("a", Some("s"), HandleType::Source, 5.0, 5.0);
        let h_tgt = handle("a", Some("t"), HandleType::Target, 5.0, 5.0);
        let a = measured_with_handles("a", 0.0, 0.0, 10.0, 10.0, vec![h_src, h_tgt]);
        lookup.insert("a".into(), a);
        let result = get_closest_handle(
            XYPosition::new(8.0, 8.0),
            20.0,
            &lookup,
            "from",
            None,
            HandleType::Source,
        )
        .expect("a candidate should exist");
        assert_eq!(result.type_, HandleType::Target);
    }

    #[test]
    fn get_handle_resolves_by_id_in_strict_mode() {
        let mut lookup = NodeLookup::<()>::new();
        let h1 = handle("a", Some("h1"), HandleType::Source, 5.0, 5.0);
        let h2 = handle("a", Some("h2"), HandleType::Source, 6.0, 6.0);
        let a = measured_with_handles("a", 0.0, 0.0, 10.0, 10.0, vec![h1, h2]);
        lookup.insert("a".into(), a);

        let resolved = get_handle(
            "a",
            HandleType::Source,
            Some("h2"),
            &lookup,
            ConnectionMode::Strict,
            false,
        );
        let r = resolved.expect("should resolve");
        assert_eq!(r.id.as_deref(), Some("h2"));
    }

    #[test]
    fn get_handle_with_absolute_position_translates() {
        let mut lookup = NodeLookup::<()>::new();
        let h1 = handle("a", Some("h1"), HandleType::Source, 5.0, 5.0);
        let a = measured_with_handles("a", 100.0, 200.0, 10.0, 10.0, vec![h1]);
        lookup.insert("a".into(), a);

        let resolved = get_handle(
            "a",
            HandleType::Source,
            Some("h1"),
            &lookup,
            ConnectionMode::Strict,
            true,
        )
        .expect("should resolve");
        // get_handle_position with center=true:
        //   x = node.position_absolute.x + handle.x + width/2 = 100 + 5 + 0.5 = 105.5
        //   y = node.position_absolute.y + handle.y + height/2 = 200 + 5 + 0.5 = 205.5
        assert!((resolved.x - 105.5).abs() < 1e-9);
        assert!((resolved.y - 205.5).abs() < 1e-9);
    }

    #[test]
    fn get_handle_loose_mode_searches_both_sides() {
        let mut lookup = NodeLookup::<()>::new();
        let h_src = handle("a", Some("h1"), HandleType::Source, 5.0, 5.0);
        let a = measured_with_handles("a", 0.0, 0.0, 10.0, 10.0, vec![h_src]);
        lookup.insert("a".into(), a);

        // Asking for a target handle in Strict mode → None.
        let strict = get_handle(
            "a",
            HandleType::Target,
            Some("h1"),
            &lookup,
            ConnectionMode::Strict,
            false,
        );
        assert!(strict.is_none());
        // In Loose mode → finds the source-typed handle.
        let loose = get_handle(
            "a",
            HandleType::Target,
            Some("h1"),
            &lookup,
            ConnectionMode::Loose,
            false,
        );
        assert!(loose.is_some());
    }

    #[test]
    fn get_handle_type_prefers_edge_updater() {
        assert_eq!(
            get_handle_type(Some(HandleType::Source), None),
            Some(HandleType::Source)
        );
        let snap = crate::xyhandle::types::HandleSnapshot {
            node_id: "a".into(),
            id: None,
            type_: HandleType::Target,
            connectable: true,
            connectable_end: true,
        };
        assert_eq!(get_handle_type(None, Some(&snap)), Some(HandleType::Target));
        assert_eq!(get_handle_type(None, None), None);
    }

    #[test]
    fn connection_validity_tri_state() {
        assert_eq!(is_connection_valid(true, true), Some(true));
        assert_eq!(is_connection_valid(true, false), Some(false));
        assert_eq!(is_connection_valid(false, false), None);
        // Outside radius even when valid yields None (matches TS:
        // logic relies on isHandleValid implying inside radius).
        assert_eq!(is_connection_valid(false, true), Some(true));
    }
}
