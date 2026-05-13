//! Port of `xyflow-core/src/xydrag/utils.ts`.
//!
//! Status: implemented (phase 5).

#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;

use crate::types::geometry::{Dimensions, XYPosition};
use crate::types::nodes::{
    InternalNode, Node, NodeDragItem, NodeExtent, NodeLookup,
};
use crate::types::viewport::SnapGrid;
use crate::utils::general::snap_position;

// ---------------------------------------------------------------------------
// is_parent_selected
// ---------------------------------------------------------------------------

/// Walk up the parent chain to determine whether any ancestor is
/// `selected`.
///
/// Mirrors the TS `isParentSelected` (recursive). Returns `false` when
/// `node` has no parent or the parent chain is broken.
#[must_use]
pub fn is_parent_selected<D: Clone>(
    node: &InternalNode<D>,
    node_lookup: &NodeLookup<D>,
) -> bool {
    let Some(parent_id) = node.user.parent_id.as_deref() else {
        return false;
    };
    let Some(parent) = node_lookup.get(parent_id) else {
        return false;
    };
    if parent.user.selected.unwrap_or(false) {
        return true;
    }
    is_parent_selected(parent, node_lookup)
}

// ---------------------------------------------------------------------------
// has_selector — replaces the TS DOM walk
// ---------------------------------------------------------------------------

/// Mirrors the TS `hasSelector(target, selector, domNode)` which walks
/// up from the event target until either an element matches `selector`
/// or the container `domNode` is reached.
///
/// In Rust the consumer pre-collects, in target-to-root order, the set
/// of CSS classes attached to each ancestor (one `Vec<String>` per
/// ancestor up to and including the container). The function checks
/// whether `selector` (a single class name *without* the leading dot)
/// matches any of them.
///
/// `selector` is a class name. The TS variant supports general CSS
/// selectors via `Element.matches`; for the xyflow use cases that
/// matters (`.${noDragClassName}`, `.handleSelector`) it's always a
/// class.
///
/// Returns `true` as soon as a match is found, `false` when the
/// ancestor list is exhausted.
#[must_use]
pub fn has_selector(ancestor_class_lists: &[Vec<String>], selector: &str) -> bool {
    if selector.is_empty() {
        return false;
    }
    // Strip a leading '.' if present, since the TS variant takes
    // `.foo` and our consumers may forward it as-is.
    let class = selector.strip_prefix('.').unwrap_or(selector);
    ancestor_class_lists
        .iter()
        .any(|classes| classes.iter().any(|c| c == class))
}

// ---------------------------------------------------------------------------
// get_drag_items
// ---------------------------------------------------------------------------

/// Build the per-node drag items for a fresh gesture.
///
/// Mirrors the TS `getDragItems`. Returns a `HashMap<id, NodeDragItem>`
/// containing every node that should move on this drag:
///
/// * the explicit `node_id` if provided, plus
/// * any selected node whose parent is *not* selected (selected
///   parents already move their children).
///
/// `nodes_draggable` reflects the React-Flow-level toggle; an
/// individual node's `draggable: Some(false)` always wins.
#[must_use]
pub fn get_drag_items<D: Clone>(
    node_lookup: &NodeLookup<D>,
    nodes_draggable: bool,
    mouse_pos: XYPosition,
    node_id: Option<&str>,
) -> HashMap<String, NodeDragItem> {
    let mut drag_items: HashMap<String, NodeDragItem> = HashMap::new();
    for (id, node) in node_lookup {
        let is_selected = node.user.selected.unwrap_or(false);
        let is_target = node_id.is_some_and(|target| target == node.user.id.as_str());
        if !(is_selected || is_target) {
            continue;
        }
        if node.user.parent_id.is_some() && is_parent_selected(node, node_lookup) {
            continue;
        }
        let draggable = match node.user.draggable {
            Some(d) => d,
            // TS `nodesDraggable && typeof draggable === 'undefined'`
            None => nodes_draggable,
        };
        if !draggable {
            continue;
        }

        let position_absolute = node.internals.position_absolute;
        let measured = Dimensions {
            width: node.measured.width.unwrap_or(0.0),
            height: node.measured.height.unwrap_or(0.0),
        };
        drag_items.insert(
            id.clone(),
            NodeDragItem {
                id: id.clone(),
                position: node.user.position,
                distance: XYPosition {
                    x: mouse_pos.x - position_absolute.x,
                    y: mouse_pos.y - position_absolute.y,
                },
                measured,
                position_absolute,
                extent: node.user.extent,
                parent_id: node.user.parent_id.clone(),
                origin: node.user.origin,
                expand_parent: node.user.expand_parent,
                dragging: None,
            },
        );
    }
    drag_items
}

// ---------------------------------------------------------------------------
// get_event_handler_params
// ---------------------------------------------------------------------------

/// Output of [`get_event_handler_params`].
///
/// Carries the "primary" node (the one being dragged or the first of a
/// selection) plus every node currently in the drag.
///
/// Mirrors the TS `[NodeBase, NodeBase[]]` tuple.
#[derive(Debug, Clone)]
pub struct DragEventHandlerParams<D: Clone> {
    pub primary: Option<Node<D>>,
    pub all: Vec<Node<D>>,
}

/// Synthesize the user-facing snapshot returned to `on_drag*`
/// callbacks.
///
/// Mirrors the TS `getEventHandlerParams({ nodeId, dragItems,
/// nodeLookup, dragging = true })`.
///
/// Each entry in `all` is the underlying user node updated to the
/// drag-item's transient `position`, with `dragging` flipped on/off
/// according to `dragging`.
#[must_use]
pub fn get_event_handler_params<D: Clone>(
    node_id: Option<&str>,
    drag_items: &HashMap<String, NodeDragItem>,
    node_lookup: &NodeLookup<D>,
    dragging: bool,
) -> DragEventHandlerParams<D> {
    let mut all: Vec<Node<D>> = Vec::with_capacity(drag_items.len());
    for (id, drag_item) in drag_items {
        let Some(internal) = node_lookup.get(id) else {
            continue;
        };
        let mut user = internal.user.clone();
        user.position = drag_item.position;
        user.dragging = Some(dragging);
        all.push(user);
    }

    if let Some(target_id) = node_id {
        if let Some(internal) = node_lookup.get(target_id) {
            let mut primary = internal.user.clone();
            if let Some(item) = drag_items.get(target_id) {
                primary.position = item.position;
            }
            primary.dragging = Some(dragging);
            return DragEventHandlerParams {
                primary: Some(primary),
                all,
            };
        }
        // TS falls back to the first node in `nodesFromDragItems` if
        // the target is no longer in the lookup.
        return DragEventHandlerParams {
            primary: all.first().cloned(),
            all,
        };
    }

    // No node id → primary is the first drag item (TS `nodesFromDragItems[0]`).
    DragEventHandlerParams {
        primary: all.first().cloned(),
        all,
    }
}

// ---------------------------------------------------------------------------
// calculate_snap_offset
// ---------------------------------------------------------------------------

/// Returned by [`calculate_snap_offset`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapOffset {
    pub x: f64,
    pub y: f64,
}

/// When dragging a multi-selection with snap-to-grid, all selected
/// nodes share the same snap offset (computed from the first item).
///
/// Mirrors the TS `calculateSnapOffset({ dragItems, snapGrid, x, y })`.
/// Returns `None` if `drag_items` is empty.
#[must_use]
pub fn calculate_snap_offset(
    drag_items: &HashMap<String, NodeDragItem>,
    snap_grid: SnapGrid,
    x: f64,
    y: f64,
) -> Option<SnapOffset> {
    // HashMap iteration order is unspecified — for parity with the TS
    // (which uses `dragItems.values().next().value`), we use whichever
    // entry arrives first; the resulting snap offset is deterministic
    // because every item already has the same `distance` modulo the
    // snap grid, so any item produces the same offset.
    let ref_item = drag_items.values().next()?;
    let ref_pos = XYPosition {
        x: x - ref_item.distance.x,
        y: y - ref_item.distance.y,
    };
    let snapped = snap_position(ref_pos, snap_grid);
    Some(SnapOffset {
        x: snapped.x - ref_pos.x,
        y: snapped.y - ref_pos.y,
    })
}

// `NodeExtent` re-export for ergonomic `use`.
pub use crate::types::nodes::NodeExtent as ReExportedNodeExtent;
const _: fn() = || {
    let _ = NodeExtent::Unbounded;
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::nodes::{MeasuredDimensions, Node};

    fn measured(id: &str, x: f64, y: f64, w: f64, h: f64) -> InternalNode<()> {
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

    fn lookup_with(nodes: Vec<InternalNode<()>>) -> NodeLookup<()> {
        let mut lookup = NodeLookup::new();
        for n in nodes {
            lookup.insert(n.user.id.clone(), n);
        }
        lookup
    }

    #[test]
    fn is_parent_selected_walks_chain() {
        let mut grand = measured("g", 0.0, 0.0, 10.0, 10.0);
        grand.user.selected = Some(true);
        let mut parent = measured("p", 0.0, 0.0, 10.0, 10.0);
        parent.user.parent_id = Some("g".into());
        let mut child = measured("c", 0.0, 0.0, 10.0, 10.0);
        child.user.parent_id = Some("p".into());

        let lookup = lookup_with(vec![grand, parent, child.clone()]);
        assert!(is_parent_selected(&child, &lookup));
    }

    #[test]
    fn is_parent_selected_returns_false_for_orphan() {
        let n = measured("o", 0.0, 0.0, 10.0, 10.0);
        let lookup = lookup_with(vec![n.clone()]);
        assert!(!is_parent_selected(&n, &lookup));
    }

    #[test]
    fn has_selector_finds_ancestor_class() {
        let ancestors = vec![
            vec!["nodrag".to_string()],
            vec!["pane".to_string()],
        ];
        assert!(has_selector(&ancestors, "nodrag"));
        assert!(has_selector(&ancestors, ".nodrag"));
        assert!(!has_selector(&ancestors, "missing"));
        assert!(!has_selector(&ancestors, ""));
    }

    #[test]
    fn get_drag_items_picks_target_node() {
        let mut a = measured("a", 10.0, 10.0, 50.0, 50.0);
        a.user.draggable = Some(true);
        let lookup = lookup_with(vec![a]);

        let items = get_drag_items(&lookup, true, XYPosition::new(20.0, 30.0), Some("a"));
        assert_eq!(items.len(), 1);
        let item = &items["a"];
        // distance = mouse - positionAbsolute = (20-10, 30-10) = (10, 20)
        assert_eq!(item.distance, XYPosition::new(10.0, 20.0));
        assert_eq!(item.position_absolute, XYPosition::new(10.0, 10.0));
        assert_eq!(item.measured, Dimensions::new(50.0, 50.0));
    }

    #[test]
    fn get_drag_items_picks_selected_nodes() {
        let mut a = measured("a", 0.0, 0.0, 10.0, 10.0);
        a.user.selected = Some(true);
        let b = measured("b", 0.0, 0.0, 10.0, 10.0);
        let lookup = lookup_with(vec![a, b]);
        let items = get_drag_items(&lookup, true, XYPosition::ZERO, None);
        assert!(items.contains_key("a"));
        assert!(!items.contains_key("b"));
    }

    #[test]
    fn get_drag_items_skips_when_parent_selected() {
        let mut parent = measured("p", 0.0, 0.0, 10.0, 10.0);
        parent.user.selected = Some(true);
        let mut child = measured("c", 0.0, 0.0, 10.0, 10.0);
        child.user.selected = Some(true);
        child.user.parent_id = Some("p".into());
        let lookup = lookup_with(vec![parent, child]);
        let items = get_drag_items(&lookup, true, XYPosition::ZERO, None);
        // Only the parent should be in the drag items — the child is
        // skipped because its parent is already selected.
        assert!(items.contains_key("p"));
        assert!(!items.contains_key("c"));
    }

    #[test]
    fn get_drag_items_respects_node_draggable_false() {
        let mut a = measured("a", 0.0, 0.0, 10.0, 10.0);
        a.user.draggable = Some(false);
        let lookup = lookup_with(vec![a]);
        let items = get_drag_items(&lookup, true, XYPosition::ZERO, Some("a"));
        assert!(items.is_empty());
    }

    #[test]
    fn get_drag_items_respects_global_nodes_draggable() {
        let a = measured("a", 0.0, 0.0, 10.0, 10.0);
        let lookup = lookup_with(vec![a]);
        // nodes_draggable=false and node.draggable is None → not draggable.
        let items_off = get_drag_items(&lookup, false, XYPosition::ZERO, Some("a"));
        assert!(items_off.is_empty());
        let items_on = get_drag_items(&lookup, true, XYPosition::ZERO, Some("a"));
        assert!(!items_on.is_empty());
    }

    #[test]
    fn get_event_handler_params_node_id_path() {
        let internal = measured("a", 0.0, 0.0, 10.0, 10.0);
        let lookup = lookup_with(vec![internal]);
        let mut items = HashMap::new();
        items.insert(
            "a".into(),
            NodeDragItem {
                id: "a".into(),
                position: XYPosition::new(5.0, 7.0),
                distance: XYPosition::ZERO,
                measured: Dimensions::new(10.0, 10.0),
                position_absolute: XYPosition::new(5.0, 7.0),
                extent: NodeExtent::Unbounded,
                parent_id: None,
                origin: None,
                expand_parent: None,
                dragging: None,
            },
        );
        let params = get_event_handler_params(Some("a"), &items, &lookup, true);
        let primary = params.primary.expect("primary should resolve");
        assert_eq!(primary.id, "a");
        assert_eq!(primary.position, XYPosition::new(5.0, 7.0));
        assert_eq!(primary.dragging, Some(true));
    }

    #[test]
    fn get_event_handler_params_no_node_id_uses_first_drag_item() {
        let a = measured("a", 0.0, 0.0, 10.0, 10.0);
        let lookup = lookup_with(vec![a]);
        let mut items = HashMap::new();
        items.insert(
            "a".into(),
            NodeDragItem {
                id: "a".into(),
                position: XYPosition::new(1.0, 2.0),
                distance: XYPosition::ZERO,
                measured: Dimensions::new(10.0, 10.0),
                position_absolute: XYPosition::new(1.0, 2.0),
                extent: NodeExtent::Unbounded,
                parent_id: None,
                origin: None,
                expand_parent: None,
                dragging: None,
            },
        );
        let params = get_event_handler_params::<()>(None, &items, &lookup, false);
        assert!(params.primary.is_some());
        assert_eq!(params.primary.unwrap().dragging, Some(false));
    }

    #[test]
    fn calculate_snap_offset_applies_grid() {
        let mut items = HashMap::new();
        items.insert(
            "a".into(),
            NodeDragItem {
                id: "a".into(),
                position: XYPosition::ZERO,
                distance: XYPosition::new(2.0, 3.0),
                measured: Dimensions::ZERO,
                position_absolute: XYPosition::ZERO,
                extent: NodeExtent::Unbounded,
                parent_id: None,
                origin: None,
                expand_parent: None,
                dragging: None,
            },
        );
        // refPos = (mouse - distance) = (10-2, 10-3) = (8, 7)
        // snapped to (5,5) grid: round(8/5)*5 = 10, round(7/5)*5 = 5
        // offset = (10 - 8, 5 - 7) = (2, -2)
        let off = calculate_snap_offset(&items, (5.0, 5.0), 10.0, 10.0).unwrap();
        assert!((off.x - 2.0).abs() < 1e-9);
        assert!((off.y - (-2.0)).abs() < 1e-9);
    }

    #[test]
    fn calculate_snap_offset_empty_returns_none() {
        let items: HashMap<String, NodeDragItem> = HashMap::new();
        assert!(calculate_snap_offset(&items, (10.0, 10.0), 0.0, 0.0).is_none());
    }
}
