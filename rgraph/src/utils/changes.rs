//! Port of `xyflow-react/src/utils/changes.ts`.
//!
//! Status: Phase 1 — implemented.
//!
//! Functions ported:
//!
//! * [`apply_node_changes`]      — apply a `Vec<NodeChange<D>>` to a `Vec<Node<D>>`.
//! * [`apply_edge_changes`]      — apply a `Vec<EdgeChange<D>>` to a `Vec<Edge<D>>`.
//! * [`create_selection_change`] — build a single selection change.
//! * [`get_selection_changes_for_nodes`] — diff a node lookup against a
//!   target selection.
//! * [`get_selection_changes_for_edges`] — same for edges.
//! * [`get_elements_diff_changes_nodes`] / `_edges` — diff two slices.
//! * [`element_to_remove_change`] — id-only `remove` change builder.
//!
//! ## Differences from the TS source
//!
//! * Two `getSelectionChanges` variants instead of one TS function with
//!   a `mutateItem` toggle: nodes need to mutate `internal_node.selected`
//!   *during the iteration* (the TS hack), so they get a `&mut`-aware
//!   variant; edges go through the simpler immutable variant.
//! * `getElementsDiffChanges` is split into two functions for the same
//!   reason — TS uses overloads, Rust uses two named functions.
//! * Internally we follow the TS algorithm closely: build a map of
//!   per-id change lists with `Remove`/`Replace` short-circuiting, then
//!   walk the original elements once. New `Add`s are appended last.

#![allow(clippy::module_name_repetitions)]

use std::collections::{HashMap, HashSet};

use rgraph_core::types::changes::{EdgeChange, NodeChange};
use rgraph_core::types::edges::EdgeLookup;
use rgraph_core::types::nodes::{InternalNode, MeasuredDimensions, NodeLookup};

use crate::types::edges::Edge;
use crate::types::nodes::Node;

// ---------------------------------------------------------------------------
// Internal helper: per-id grouping of node/edge changes.
// ---------------------------------------------------------------------------

/// Outcome of grouping changes by id (see [`group_node_changes`]).
struct GroupedNodeChanges<D: Clone> {
    /// `id → ordered list of non-Add changes`.
    by_id: HashMap<String, Vec<NodeChange<D>>>,
    /// `Add` changes preserved in insertion order.
    adds: Vec<NodeChange<D>>,
}

fn group_node_changes<D: Clone>(changes: Vec<NodeChange<D>>) -> GroupedNodeChanges<D> {
    let mut by_id: HashMap<String, Vec<NodeChange<D>>> = HashMap::new();
    let mut adds: Vec<NodeChange<D>> = Vec::new();

    for change in changes {
        match &change {
            NodeChange::Add { .. } => adds.push(change),
            NodeChange::Remove { id } | NodeChange::Replace { id, .. } => {
                /*
                 * Remove / replace clobber any earlier per-id changes —
                 * the element is going away or being wholesale-replaced
                 * anyway.
                 */
                by_id.insert(id.clone(), vec![change]);
            }
            NodeChange::Select { id, .. }
            | NodeChange::Position { id, .. }
            | NodeChange::Dimensions { id, .. } => {
                by_id
                    .entry(id.clone())
                    .or_default()
                    .push(change);
            }
        }
    }

    GroupedNodeChanges { by_id, adds }
}

/// Edge counterpart of [`group_node_changes`].
struct GroupedEdgeChanges<D: Clone> {
    by_id: HashMap<String, Vec<EdgeChange<D>>>,
    adds: Vec<EdgeChange<D>>,
}

fn group_edge_changes<D: Clone>(changes: Vec<EdgeChange<D>>) -> GroupedEdgeChanges<D> {
    let mut by_id: HashMap<String, Vec<EdgeChange<D>>> = HashMap::new();
    let mut adds: Vec<EdgeChange<D>> = Vec::new();

    for change in changes {
        match &change {
            EdgeChange::Add { .. } => adds.push(change),
            EdgeChange::Remove { id } | EdgeChange::Replace { id, .. } => {
                by_id.insert(id.clone(), vec![change]);
            }
            EdgeChange::Select { id, .. } => {
                by_id.entry(id.clone()).or_default().push(change);
            }
        }
    }

    GroupedEdgeChanges { by_id, adds }
}

// ---------------------------------------------------------------------------
// Mutation helpers — apply a single change to an owned element.
// ---------------------------------------------------------------------------

fn apply_node_change_in_place<D: Clone>(change: NodeChange<D>, node: &mut Node<D>) {
    match change {
        NodeChange::Select { selected, .. } => {
            node.selected = Some(selected);
        }
        NodeChange::Position {
            position,
            dragging,
            position_absolute: _,
            ..
        } => {
            if let Some(p) = position {
                node.position = p;
            }
            if let Some(d) = dragging {
                node.dragging = Some(d);
            }
        }
        NodeChange::Dimensions {
            dimensions,
            resizing,
            set_attributes,
            ..
        } => {
            if let Some(d) = dimensions {
                node.measured = Some(MeasuredDimensions {
                    width: Some(d.width),
                    height: Some(d.height),
                });
                use rgraph_core::types::changes::SetAttributesMode;
                match set_attributes {
                    SetAttributesMode::None => {}
                    SetAttributesMode::All => {
                        node.width = Some(d.width);
                        node.height = Some(d.height);
                    }
                    SetAttributesMode::WidthOnly => {
                        node.width = Some(d.width);
                    }
                    SetAttributesMode::HeightOnly => {
                        node.height = Some(d.height);
                    }
                }
            }
            if let Some(_r) = resizing {
                /*
                 * `resizing` lives on the React-only `NodePresentation`,
                 * not on the canonical `Node<D>`. The store applies
                 * this flag to the presentation slice in Phase 2; here
                 * we silently drop it to keep `apply_node_changes` a
                 * pure data transform.
                 *
                 * (TS sets `element.resizing = change.resizing` but the
                 * field only exists because the TS `Node` union carries
                 * presentational fields. See `types/nodes.rs`.)
                 */
            }
        }
        NodeChange::Remove { .. } | NodeChange::Add { .. } | NodeChange::Replace { .. } => {
            // Handled at the outer level — never reach here.
        }
    }
}

fn apply_edge_change_in_place<D: Clone>(change: EdgeChange<D>, edge: &mut Edge<D>) {
    match change {
        EdgeChange::Select { selected, .. } => {
            edge.selected = Some(selected);
        }
        EdgeChange::Remove { .. } | EdgeChange::Add { .. } | EdgeChange::Replace { .. } => {
            // Handled at the outer level.
        }
    }
}

// ---------------------------------------------------------------------------
// Public API: apply_node_changes / apply_edge_changes.
// ---------------------------------------------------------------------------

/// Apply node changes to a `Vec<Node<D>>` and return the updated vector.
///
/// Mirrors the TS `applyNodeChanges`. The TS implementation is
/// described in the file-level comment.
///
/// # Example
/// ```
/// use rgraph::utils::changes::apply_node_changes;
/// use rgraph::types::nodes::Node;
/// use rgraph_core::types::changes::NodeChange;
///
/// let mut nodes = vec![Node::<()>::minimal("n1", 0.0, 0.0)];
/// let changes = vec![NodeChange::<()>::Select { id: "n1".into(), selected: true }];
/// nodes = apply_node_changes(changes, nodes);
/// assert_eq!(nodes[0].selected, Some(true));
/// ```
#[must_use]
pub fn apply_node_changes<D: Clone>(
    changes: Vec<NodeChange<D>>,
    nodes: Vec<Node<D>>,
) -> Vec<Node<D>> {
    let GroupedNodeChanges { mut by_id, adds } = group_node_changes(changes);
    let mut out: Vec<Node<D>> = Vec::with_capacity(nodes.len() + adds.len());

    for node in nodes.into_iter() {
        let Some(my_changes) = by_id.remove(&node.id) else {
            out.push(node);
            continue;
        };

        // Remove: skip this element entirely.
        if matches!(my_changes.first(), Some(NodeChange::Remove { .. })) {
            continue;
        }

        // Replace: substitute and move on.
        if let Some(NodeChange::Replace { item, .. }) = my_changes.first() {
            out.push(item.clone());
            continue;
        }

        // Else apply each change in order to a shallow copy.
        let mut updated = node.clone();
        for change in my_changes {
            apply_node_change_in_place(change, &mut updated);
        }
        out.push(updated);
    }

    // Apply Add changes at the very end (TS insertion semantics).
    for change in adds {
        if let NodeChange::Add { item, index } = change {
            if let Some(idx) = index {
                let clamped = idx.min(out.len());
                out.insert(clamped, item);
            } else {
                out.push(item);
            }
        }
    }

    out
}

/// Edge counterpart of [`apply_node_changes`]. Mirrors the TS
/// `applyEdgeChanges`.
#[must_use]
pub fn apply_edge_changes<D: Clone>(
    changes: Vec<EdgeChange<D>>,
    edges: Vec<Edge<D>>,
) -> Vec<Edge<D>> {
    let GroupedEdgeChanges { mut by_id, adds } = group_edge_changes(changes);
    let mut out: Vec<Edge<D>> = Vec::with_capacity(edges.len() + adds.len());

    for edge in edges.into_iter() {
        let Some(my_changes) = by_id.remove(&edge.id) else {
            out.push(edge);
            continue;
        };

        if matches!(my_changes.first(), Some(EdgeChange::Remove { .. })) {
            continue;
        }

        if let Some(EdgeChange::Replace { item, .. }) = my_changes.first() {
            out.push(item.clone());
            continue;
        }

        let mut updated = edge.clone();
        for change in my_changes {
            apply_edge_change_in_place(change, &mut updated);
        }
        out.push(updated);
    }

    for change in adds {
        if let EdgeChange::Add { item, index } = change {
            if let Some(idx) = index {
                let clamped = idx.min(out.len());
                out.insert(clamped, item);
            } else {
                out.push(item);
            }
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Selection-change builders.
// ---------------------------------------------------------------------------

/// Build a single `Select` change for nodes.
///
/// Mirrors the TS `createSelectionChange`. The TS function returns a
/// `NodeSelectionChange | EdgeSelectionChange` because both share the
/// same shape; in Rust they are distinct enum variants so we provide
/// two helpers (`create_node_selection_change` and
/// `create_edge_selection_change`).
#[must_use]
#[inline]
pub fn create_node_selection_change<D: Clone>(id: String, selected: bool) -> NodeChange<D> {
    NodeChange::Select { id, selected }
}

/// Edge counterpart of [`create_node_selection_change`].
#[must_use]
#[inline]
pub fn create_edge_selection_change<D: Clone>(id: String, selected: bool) -> EdgeChange<D> {
    EdgeChange::Select { id, selected }
}

/// TS-compatible alias of [`create_node_selection_change`].
///
/// Provided so the TS signature `createSelectionChange(id, selected)`
/// has a direct Rust counterpart with the same name. The TS function
/// returns the *node* variant by default (the only place edges call it
/// directly, `add_selected_edges` / `getSelectionChanges`, build the
/// edge variant explicitly).
#[must_use]
#[inline]
pub fn create_selection_change<D: Clone>(id: String, selected: bool) -> NodeChange<D> {
    create_node_selection_change(id, selected)
}

// ---------------------------------------------------------------------------
// getSelectionChanges — diff a lookup against a target selection set.
// ---------------------------------------------------------------------------

/// Compute the node selection changes needed to bring the
/// `node_lookup`'s `selected` flags in line with `selected_ids`.
///
/// Mirrors the TS `getSelectionChanges` with `mutateItem = true`:
/// internal nodes have their `selected` flag mutated *in place* so
/// concurrent drag actions see the right state before the
/// `on_nodes_change` callback runs.
#[must_use]
pub fn get_selection_changes_for_nodes<D: Clone>(
    node_lookup: &mut NodeLookup<D>,
    selected_ids: &HashSet<String>,
) -> Vec<NodeChange<D>> {
    let mut changes: Vec<NodeChange<D>> = Vec::new();

    for (id, internal) in node_lookup.iter_mut() {
        let will_be_selected = selected_ids.contains(id);
        let current = internal.user.selected;

        // Skip the first-selection edge case: `selected` is `None` and
        // we're not selecting it now — leave it alone.
        let first_selection = current.is_none() && !will_be_selected;
        let already_in_state = current == Some(will_be_selected);
        if first_selection || already_in_state {
            continue;
        }

        // Mutate the lookup so the next render reads the correct state.
        internal.user.selected = Some(will_be_selected);
        changes.push(create_node_selection_change(id.clone(), will_be_selected));
    }

    changes
}

/// Compute the edge selection changes needed to bring `edge_lookup`'s
/// `selected` flags in line with `selected_ids`.
///
/// Mirrors `getSelectionChanges` with `mutateItem = false` (the TS
/// default for edges). The edges in the lookup are not mutated; only
/// the change list is returned.
#[must_use]
pub fn get_selection_changes_for_edges<D: Clone>(
    edge_lookup: &EdgeLookup<D>,
    selected_ids: &HashSet<String>,
) -> Vec<EdgeChange<D>> {
    let mut changes: Vec<EdgeChange<D>> = Vec::new();

    for (id, edge) in edge_lookup.iter() {
        let will_be_selected = selected_ids.contains(id);
        let current = edge.selected;

        let first_selection = current.is_none() && !will_be_selected;
        let already_in_state = current == Some(will_be_selected);
        if first_selection || already_in_state {
            continue;
        }

        changes.push(create_edge_selection_change(id.clone(), will_be_selected));
    }

    changes
}

// ---------------------------------------------------------------------------
// getElementsDiffChanges — diff a slice against a lookup.
// ---------------------------------------------------------------------------

/// Compute the `NodeChange`s that turn the lookup into the given
/// `items` slice (Add / Replace / Remove).
///
/// Mirrors the first TS `getElementsDiffChanges` overload (the one for
/// nodes).
#[must_use]
pub fn get_elements_diff_changes_nodes<D: Clone + PartialEq>(
    items: &[Node<D>],
    lookup: &NodeLookup<D>,
) -> Vec<NodeChange<D>> {
    let mut changes: Vec<NodeChange<D>> = Vec::new();
    let mut items_lookup: HashMap<&str, &Node<D>> = HashMap::with_capacity(items.len());
    for item in items {
        items_lookup.insert(item.id.as_str(), item);
    }

    for (index, item) in items.iter().enumerate() {
        let lookup_item = lookup.get(&item.id);
        match lookup_item {
            None => {
                // Item is new — Add change.
                changes.push(NodeChange::Add {
                    item: item.clone(),
                    index: Some(index),
                });
            }
            Some(internal) => {
                // TS compares `storeItem !== item` by identity. Rust
                // has no notion of object identity, so we use the
                // deepest available signal of "the user has rebuilt
                // this node": structural inequality. This is consistent
                // with what the store does post-`adoptUserNodes`.
                if &internal.user != item {
                    changes.push(NodeChange::Replace {
                        id: item.id.clone(),
                        item: item.clone(),
                    });
                }
            }
        }
    }

    for id in lookup.keys() {
        if !items_lookup.contains_key(id.as_str()) {
            changes.push(NodeChange::Remove { id: id.clone() });
        }
    }

    changes
}

/// Compute the `EdgeChange`s that turn the lookup into the given
/// `items` slice. Mirrors the edge overload of TS
/// `getElementsDiffChanges`.
#[must_use]
pub fn get_elements_diff_changes_edges<D: Clone + PartialEq>(
    items: &[Edge<D>],
    lookup: &EdgeLookup<D>,
) -> Vec<EdgeChange<D>> {
    let mut changes: Vec<EdgeChange<D>> = Vec::new();
    let mut items_lookup: HashMap<&str, &Edge<D>> = HashMap::with_capacity(items.len());
    for item in items {
        items_lookup.insert(item.id.as_str(), item);
    }

    for (index, item) in items.iter().enumerate() {
        let lookup_item = lookup.get(&item.id);
        match lookup_item {
            None => {
                changes.push(EdgeChange::Add {
                    item: item.clone(),
                    index: Some(index),
                });
            }
            Some(store_edge) => {
                if store_edge != item {
                    changes.push(EdgeChange::Replace {
                        id: item.id.clone(),
                        item: item.clone(),
                    });
                }
            }
        }
    }

    for id in lookup.keys() {
        if !items_lookup.contains_key(id.as_str()) {
            changes.push(EdgeChange::Remove { id: id.clone() });
        }
    }

    changes
}

// ---------------------------------------------------------------------------
// elementToRemoveChange — id-only remove builder.
// ---------------------------------------------------------------------------

/// Build a `Remove` change for a node.
///
/// Mirrors the TS `elementToRemoveChange<T extends Node | Edge>(item: T)`.
/// The TS function is generic over the union; in Rust we provide two
/// dedicated helpers.
#[must_use]
#[inline]
pub fn node_to_remove_change<D: Clone>(node: &Node<D>) -> NodeChange<D> {
    NodeChange::Remove { id: node.id.clone() }
}

/// Build a `Remove` change for an edge.
#[must_use]
#[inline]
pub fn edge_to_remove_change<D: Clone>(edge: &Edge<D>) -> EdgeChange<D> {
    EdgeChange::Remove { id: edge.id.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgraph_core::types::changes::SetAttributesMode;
    use rgraph_core::types::geometry::{Dimensions, XYPosition};

    fn n(id: &str, x: f64, y: f64) -> Node<()> {
        Node::<()>::minimal(id, x, y)
    }

    fn e(id: &str, source: &str, target: &str) -> Edge<()> {
        Edge::<()>::minimal(id, source, target)
    }

    #[test]
    fn select_change_flips_selected_flag() {
        let nodes = vec![n("n1", 0.0, 0.0)];
        let changes = vec![NodeChange::<()>::Select {
            id: "n1".into(),
            selected: true,
        }];
        let out = apply_node_changes(changes, nodes);
        assert_eq!(out[0].selected, Some(true));
    }

    #[test]
    fn position_change_updates_position_and_dragging() {
        let nodes = vec![n("n1", 0.0, 0.0)];
        let changes = vec![NodeChange::<()>::Position {
            id: "n1".into(),
            position: Some(XYPosition::new(10.0, 20.0)),
            position_absolute: None,
            dragging: Some(true),
        }];
        let out = apply_node_changes(changes, nodes);
        assert_eq!(out[0].position, XYPosition::new(10.0, 20.0));
        assert_eq!(out[0].dragging, Some(true));
    }

    #[test]
    fn dimensions_change_writes_measured_and_optional_width() {
        let nodes = vec![n("n1", 0.0, 0.0)];
        let changes = vec![NodeChange::<()>::Dimensions {
            id: "n1".into(),
            dimensions: Some(Dimensions::new(100.0, 50.0)),
            resizing: None,
            set_attributes: SetAttributesMode::All,
        }];
        let out = apply_node_changes(changes, nodes);
        assert_eq!(out[0].width, Some(100.0));
        assert_eq!(out[0].height, Some(50.0));
        assert_eq!(out[0].measured.unwrap().width, Some(100.0));
    }

    #[test]
    fn remove_drops_element() {
        let nodes = vec![n("n1", 0.0, 0.0), n("n2", 1.0, 1.0)];
        let changes = vec![NodeChange::<()>::Remove { id: "n1".into() }];
        let out = apply_node_changes(changes, nodes);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "n2");
    }

    #[test]
    fn remove_short_circuits_other_changes_for_same_id() {
        let nodes = vec![n("n1", 0.0, 0.0)];
        let changes = vec![
            NodeChange::<()>::Position {
                id: "n1".into(),
                position: Some(XYPosition::new(99.0, 99.0)),
                position_absolute: None,
                dragging: None,
            },
            NodeChange::<()>::Remove { id: "n1".into() },
        ];
        let out = apply_node_changes(changes, nodes);
        // Remove wins, the prior position change is dropped.
        assert!(out.is_empty());
    }

    #[test]
    fn add_at_end_when_no_index() {
        let nodes = vec![n("n1", 0.0, 0.0)];
        let changes = vec![NodeChange::<()>::Add {
            item: n("n2", 1.0, 1.0),
            index: None,
        }];
        let out = apply_node_changes(changes, nodes);
        assert_eq!(out.len(), 2);
        assert_eq!(out[1].id, "n2");
    }

    #[test]
    fn add_at_index_inserts_in_place() {
        let nodes = vec![n("a", 0.0, 0.0), n("b", 1.0, 1.0)];
        let changes = vec![NodeChange::<()>::Add {
            item: n("z", 2.0, 2.0),
            index: Some(1),
        }];
        let out = apply_node_changes(changes, nodes);
        assert_eq!(out.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(), vec!["a", "z", "b"]);
    }

    #[test]
    fn add_clamps_out_of_range_index() {
        let nodes = vec![n("a", 0.0, 0.0)];
        let changes = vec![NodeChange::<()>::Add {
            item: n("z", 2.0, 2.0),
            index: Some(99),
        }];
        let out = apply_node_changes(changes, nodes);
        assert_eq!(out.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(), vec!["a", "z"]);
    }

    #[test]
    fn replace_swaps_element_wholesale() {
        let nodes = vec![n("n1", 0.0, 0.0)];
        let mut replacement = n("n1", 5.0, 5.0);
        replacement.aria_label = Some("replaced".into());
        let changes = vec![NodeChange::<()>::Replace {
            id: "n1".into(),
            item: replacement,
        }];
        let out = apply_node_changes(changes, nodes);
        assert_eq!(out[0].aria_label.as_deref(), Some("replaced"));
        assert_eq!(out[0].position, XYPosition::new(5.0, 5.0));
    }

    #[test]
    fn apply_edge_changes_select_remove_add() {
        let edges = vec![e("e1", "a", "b"), e("e2", "a", "c")];
        let changes = vec![
            EdgeChange::<()>::Select { id: "e1".into(), selected: true },
            EdgeChange::<()>::Remove { id: "e2".into() },
            EdgeChange::<()>::Add { item: e("e3", "b", "c"), index: None },
        ];
        let out = apply_edge_changes(changes, edges);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].id, "e1");
        assert_eq!(out[0].selected, Some(true));
        assert_eq!(out[1].id, "e3");
    }

    #[test]
    fn untouched_nodes_are_preserved_verbatim() {
        let n1 = n("n1", 0.0, 0.0);
        let n2 = n("n2", 1.0, 1.0);
        let out = apply_node_changes(
            vec![NodeChange::<()>::Select { id: "n1".into(), selected: true }],
            vec![n1.clone(), n2.clone()],
        );
        assert_eq!(out[1], n2); // unchanged
    }

    #[test]
    fn create_selection_change_node_variant() {
        let c = create_node_selection_change::<()>("n1".into(), true);
        match c {
            NodeChange::Select { id, selected } => {
                assert_eq!(id, "n1");
                assert!(selected);
            }
            _ => panic!("expected Select"),
        }
    }

    #[test]
    fn get_selection_changes_for_nodes_mutates_lookup() {
        let mut lookup: NodeLookup<()> = NodeLookup::default();
        lookup.insert("n1".into(), InternalNode::from_user(n("n1", 0.0, 0.0)));
        lookup.insert("n2".into(), {
            let mut int = InternalNode::from_user(n("n2", 0.0, 0.0));
            int.user.selected = Some(true);
            int
        });

        let mut selected: HashSet<String> = HashSet::new();
        selected.insert("n1".to_string());

        let changes = get_selection_changes_for_nodes(&mut lookup, &selected);

        // Expect: n1 -> selected=true (was None+target=true), n2 -> false.
        let n1_change = changes.iter().find(|c| matches!(c, NodeChange::Select { id, .. } if id == "n1"));
        let n2_change = changes.iter().find(|c| matches!(c, NodeChange::Select { id, .. } if id == "n2"));
        assert!(matches!(n1_change, Some(NodeChange::Select { selected: true, .. })));
        assert!(matches!(n2_change, Some(NodeChange::Select { selected: false, .. })));

        // Lookup was mutated.
        assert_eq!(lookup["n1"].user.selected, Some(true));
        assert_eq!(lookup["n2"].user.selected, Some(false));
    }

    #[test]
    fn get_selection_changes_for_nodes_skips_first_unselected() {
        let mut lookup: NodeLookup<()> = NodeLookup::default();
        lookup.insert("n1".into(), InternalNode::from_user(n("n1", 0.0, 0.0)));
        // node has `selected: None` and target is also unselected → skip.
        let changes = get_selection_changes_for_nodes(&mut lookup, &HashSet::new());
        assert!(changes.is_empty());
    }

    #[test]
    fn get_selection_changes_for_edges_immutable() {
        let mut lookup: EdgeLookup<()> = EdgeLookup::default();
        let mut e = e("e1", "a", "b");
        e.selected = Some(true);
        lookup.insert("e1".into(), e);
        let changes = get_selection_changes_for_edges(&lookup, &HashSet::new());
        assert_eq!(changes.len(), 1);
        match &changes[0] {
            EdgeChange::Select { id, selected } => {
                assert_eq!(id, "e1");
                assert!(!selected);
            }
            _ => panic!("expected Select"),
        }
        // Lookup is NOT mutated (mutateItem = false for edges).
        assert_eq!(lookup["e1"].selected, Some(true));
    }

    #[test]
    fn elements_diff_finds_add_replace_remove_nodes() {
        let mut lookup: NodeLookup<()> = NodeLookup::default();
        lookup.insert("a".into(), InternalNode::from_user(n("a", 0.0, 0.0)));
        lookup.insert("b".into(), InternalNode::from_user(n("b", 0.0, 0.0)));

        let items = vec![
            n("a", 0.0, 0.0),       // unchanged
            n("c", 0.0, 0.0),       // new
            {
                let mut b2 = n("b", 5.0, 5.0); // moved → replace
                b2.aria_label = Some("hi".into());
                b2
            },
        ];

        let changes = get_elements_diff_changes_nodes(&items, &lookup);

        let kinds: Vec<&'static str> = changes
            .iter()
            .map(|c| match c {
                NodeChange::Add { .. } => "add",
                NodeChange::Remove { .. } => "remove",
                NodeChange::Replace { .. } => "replace",
                _ => "other",
            })
            .collect();
        assert!(kinds.contains(&"add"));
        assert!(kinds.contains(&"replace"));
    }

    #[test]
    fn remove_change_for_node() {
        let node = n("n1", 0.0, 0.0);
        let c = node_to_remove_change(&node);
        assert!(matches!(c, NodeChange::Remove { ref id } if id == "n1"));
    }

    #[test]
    fn remove_change_for_edge() {
        let edge = e("e1", "a", "b");
        let c = edge_to_remove_change(&edge);
        assert!(matches!(c, EdgeChange::Remove { ref id } if id == "e1"));
    }
}
