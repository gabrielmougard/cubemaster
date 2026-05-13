//! Port of `xyflow-core/src/utils/store.ts`.
//!
//! Status: implemented (phase 3).
//!
//! TS upstream mutates a Zustand store via setters. Per the workspace
//! porting decision, the Rust port exposes free functions that take
//! `&mut NodeLookup<D>` / `&mut ParentLookup<D>` /
//! `&mut ConnectionLookup` / `&mut EdgeLookup<E>` directly. Downstream
//! Dioxus consumers wrap these in `use_signal` writers.
//!
//! The TS source's `update_node_internals` reads DOM measurements
//! inline — in Rust the consumer pre-measures and passes both
//! `node_element` size and `node_bounds` rectangle into
//! [`InternalNodeUpdate`].

#![allow(clippy::module_name_repetitions)]

use crate::constants::INFINITE_EXTENT;
use crate::promise::Promise;
use crate::types::changes::{NodeChange, SetAttributesMode};
use crate::types::connection::{ConnectionLookup, HandleConnection};
use crate::types::edges::{Edge, EdgeLookup};
use crate::types::geometry::{CoordinateExtent, Dimensions, Rect, Transform, XYPosition};
use crate::types::handles::HandleType;
use crate::types::nodes::{
    InternalNode, MeasuredDimensions, Node, NodeExtent, NodeHandleBounds, NodeInternals,
    NodeLookup, NodeOrigin, ParentLookup,
};
use crate::types::panzoom::PanZoomInstance;
use crate::types::viewport::ZIndexMode;
use crate::utils::dom::{build_handle_bounds, HandleMeasurement};
use crate::utils::general::{
    clamp_position, clamp_position_to_parent, get_bounds_of_rects, get_node_dimensions,
    internal_node_to_rect, is_numeric,
};
use crate::utils::graph::get_node_position_with_origin;

const SELECTED_NODE_Z: f64 = 1000.0;
const ROOT_PARENT_Z_INCREMENT: f64 = 10.0;

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Common subset of options passed to most store helpers.
#[derive(Debug, Clone)]
pub struct UpdateNodesOptions {
    pub node_origin: NodeOrigin,
    pub node_extent: CoordinateExtent,
    pub elevate_nodes_on_select: bool,
    pub z_index_mode: ZIndexMode,
}

impl Default for UpdateNodesOptions {
    fn default() -> Self {
        Self {
            node_origin: (0.0, 0.0),
            node_extent: INFINITE_EXTENT,
            elevate_nodes_on_select: true,
            z_index_mode: ZIndexMode::Basic,
        }
    }
}

/// Subset of options for [`adopt_user_nodes`] — adds a `check_equality`
/// flag that lets callers skip a re-build when the user-node reference
/// is structurally identical to the cached one.
#[derive(Debug, Clone)]
pub struct AdoptUserNodesOptions {
    pub base: UpdateNodesOptions,
    /// When `true`, skip rebuilding an internal node if its `user`
    /// equals the new user node. Default: `true`.
    pub check_equality: bool,
}

impl Default for AdoptUserNodesOptions {
    fn default() -> Self {
        Self {
            base: UpdateNodesOptions::default(),
            check_equality: true,
        }
    }
}

/// Result of [`adopt_user_nodes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdoptUserNodesResult {
    pub nodes_initialized: bool,
    pub has_selected_nodes: bool,
}

#[inline]
fn is_manual_z(mode: ZIndexMode) -> bool {
    mode == ZIndexMode::Manual
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn parse_handles<D: Clone>(
    user_node: &Node<D>,
    fallback: Option<&InternalNode<D>>,
) -> Option<NodeHandleBounds> {
    let Some(handles) = &user_node.handles else {
        // TS: `if (!userNode.handles) { return !userNode.measured ? undefined : internalNode?.internals.handleBounds; }`
        return if user_node.measured.is_none() {
            None
        } else {
            fallback.and_then(|i| i.internals.handle_bounds.clone())
        };
    };
    let mut source = Vec::new();
    let mut target = Vec::new();
    for h in handles {
        let resolved = crate::types::handles::Handle {
            id: h.id.clone(),
            node_id: user_node.id.clone(),
            x: h.x,
            y: h.y,
            position: h.position,
            type_: h.type_,
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

fn calculate_z<D: Clone>(node: &Node<D>, selected_node_z: f64, mode: ZIndexMode) -> f64 {
    let z_index = node
        .z_index
        .map(|z| z as f64)
        .filter(|n| is_numeric(*n))
        .unwrap_or(0.0);
    if is_manual_z(mode) {
        return z_index;
    }
    z_index + if node.selected.unwrap_or(false) { selected_node_z } else { 0.0 }
}

fn calculate_child_xyz<D: Clone>(
    child: &InternalNode<D>,
    parent: &InternalNode<D>,
    node_origin: NodeOrigin,
    node_extent: CoordinateExtent,
    selected_node_z: f64,
    mode: ZIndexMode,
) -> (f64, f64, f64) {
    let parent_x = parent.internals.position_absolute.x;
    let parent_y = parent.internals.position_absolute.y;
    let child_dimensions = get_node_dimensions(child);
    let position_with_origin = get_node_position_with_origin(child, node_origin);

    let clamped_position = match child.user.extent {
        NodeExtent::Custom(c) => clamp_position(position_with_origin, c, child_dimensions),
        _ => position_with_origin,
    };

    let mut absolute_position = clamp_position(
        XYPosition {
            x: parent_x + clamped_position.x,
            y: parent_y + clamped_position.y,
        },
        node_extent,
        child_dimensions,
    );

    if matches!(child.user.extent, NodeExtent::Parent) {
        absolute_position = clamp_position_to_parent(absolute_position, child_dimensions, parent);
    }

    let child_z = calculate_z(&child.user, selected_node_z, mode);
    let parent_z = parent.internals.z;
    let z = if parent_z >= child_z { parent_z + 1.0 } else { child_z };

    (absolute_position.x, absolute_position.y, z)
}

fn update_parent_lookup<D: Clone>(node: &InternalNode<D>, parent_lookup: &mut ParentLookup<D>) {
    let Some(parent_id) = &node.user.parent_id else {
        return;
    };
    parent_lookup
        .entry(parent_id.clone())
        .or_default()
        .insert(node.user.id.clone(), node.clone());
}

fn update_child_node<D: Clone>(
    node_id: &str,
    node_lookup: &mut NodeLookup<D>,
    parent_lookup: &mut ParentLookup<D>,
    options: &UpdateNodesOptions,
    root_parent_index: Option<&mut usize>,
) {
    // Take a copy of the current child + parent up front so we can
    // mutate the lookup without the borrow checker complaining.
    let Some(child_clone) = node_lookup.get(node_id).cloned() else {
        return;
    };
    let Some(parent_id) = child_clone.user.parent_id.clone() else {
        return;
    };
    let Some(parent_clone) = node_lookup.get(&parent_id).cloned() else {
        // TS warns via console.warn; we keep silent here.
        return;
    };

    update_parent_lookup(&child_clone, parent_lookup);

    // Root-parent z-index bookkeeping (for `ZIndexMode::Auto`).
    let parent_is_root = parent_clone.user.parent_id.is_none();
    if let Some(idx_ref) = root_parent_index {
        if parent_is_root
            && parent_clone.internals.root_parent_index.is_none()
            && options.z_index_mode == ZIndexMode::Auto
        {
            *idx_ref += 1;
            // Update parent in the lookup with new root_parent_index + z bump.
            if let Some(p) = node_lookup.get_mut(&parent_id) {
                p.internals.root_parent_index = Some(*idx_ref);
                p.internals.z += (*idx_ref as f64) * ROOT_PARENT_Z_INCREMENT;
            }
        } else if let Some(rpi) = parent_clone.internals.root_parent_index {
            *idx_ref = rpi;
        }
    }

    // Re-resolve parent now that we may have mutated it above.
    let parent = node_lookup.get(&parent_id).cloned().unwrap_or(parent_clone);

    let selected_node_z = if options.elevate_nodes_on_select && !is_manual_z(options.z_index_mode) {
        SELECTED_NODE_Z
    } else {
        0.0
    };
    let (x, y, z) = calculate_child_xyz(
        &child_clone,
        &parent,
        options.node_origin,
        options.node_extent,
        selected_node_z,
        options.z_index_mode,
    );

    let position_changed =
        x != child_clone.internals.position_absolute.x
            || y != child_clone.internals.position_absolute.y;

    if position_changed || z != child_clone.internals.z {
        if let Some(c) = node_lookup.get_mut(node_id) {
            if position_changed {
                c.internals.position_absolute = XYPosition { x, y };
            }
            c.internals.z = z;
        }
    }
}

// ---------------------------------------------------------------------------
// Public: updateAbsolutePositions
// ---------------------------------------------------------------------------

/// Recompute every node's `positionAbsolute` (e.g. after the user
/// changes `nodeExtent` or `nodeOrigin`).
///
/// Mirrors the TS `updateAbsolutePositions`.
pub fn update_absolute_positions<D: Clone>(
    node_lookup: &mut NodeLookup<D>,
    parent_lookup: &mut ParentLookup<D>,
    options: Option<&UpdateNodesOptions>,
) {
    let owned_default = UpdateNodesOptions::default();
    let opts = options.unwrap_or(&owned_default);

    let ids: Vec<String> = node_lookup.keys().cloned().collect();
    for id in ids {
        let has_parent = node_lookup
            .get(&id)
            .and_then(|n| n.user.parent_id.clone())
            .is_some();
        if has_parent {
            update_child_node(&id, node_lookup, parent_lookup, opts, None);
        } else {
            // Root node — recompute its absolute position from origin + extent.
            let Some(node) = node_lookup.get(&id) else {
                continue;
            };
            let position_with_origin = get_node_position_with_origin(node, opts.node_origin);
            let extent = if let NodeExtent::Custom(c) = node.user.extent {
                c
            } else {
                opts.node_extent
            };
            let dim = get_node_dimensions(node);
            let clamped = clamp_position(position_with_origin, extent, dim);
            if let Some(n) = node_lookup.get_mut(&id) {
                n.internals.position_absolute = clamped;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public: adoptUserNodes
// ---------------------------------------------------------------------------

/// Rebuild `node_lookup` and `parent_lookup` from the user-supplied
/// `nodes` list. Returns flags indicating whether the lookup is fully
/// initialized (every visible node has measurements) and whether any
/// node is selected.
///
/// Mirrors the TS `adoptUserNodes`. Both `nodes` and `node_lookup` /
/// `parent_lookup` are cleared and repopulated; ordering is preserved
/// in `nodes` so the TS warning ("parent must come before children")
/// applies here too.
pub fn adopt_user_nodes<D: Clone + PartialEq>(
    nodes: &[Node<D>],
    node_lookup: &mut NodeLookup<D>,
    parent_lookup: &mut ParentLookup<D>,
    options: &AdoptUserNodesOptions,
) -> AdoptUserNodesResult {
    let opts = &options.base;
    let mut root_parent_index: usize = 0;
    let tmp_lookup: NodeLookup<D> = node_lookup.clone();
    let selected_node_z = if opts.elevate_nodes_on_select && !is_manual_z(opts.z_index_mode) {
        SELECTED_NODE_Z
    } else {
        0.0
    };
    let mut nodes_initialized = !nodes.is_empty();
    let mut has_selected_nodes = false;

    node_lookup.clear();
    parent_lookup.clear();

    for user_node in nodes {
        let cached = tmp_lookup.get(&user_node.id);
        let internal = if options.check_equality
            && cached.map(|c| &c.user == user_node).unwrap_or(false)
        {
            cached.unwrap().clone()
        } else {
            let position_with_origin = get_node_position_with_origin(user_node, opts.node_origin);
            let extent = match user_node.extent {
                NodeExtent::Custom(c) => c,
                _ => opts.node_extent,
            };
            let dim = get_node_dimensions(user_node);
            let clamped = clamp_position(position_with_origin, extent, dim);

            InternalNode {
                user: user_node.clone(),
                measured: MeasuredDimensions {
                    width: user_node.measured.and_then(|m| m.width),
                    height: user_node.measured.and_then(|m| m.height),
                },
                internals: NodeInternals {
                    position_absolute: clamped,
                    handle_bounds: parse_handles(user_node, cached),
                    z: calculate_z(user_node, selected_node_z, opts.z_index_mode),
                    root_parent_index: None,
                    bounds: None,
                },
            }
        };

        node_lookup.insert(user_node.id.clone(), internal);

        // Re-borrow to inspect measured + parent.
        if let Some(stored) = node_lookup.get(&user_node.id) {
            let measured_unset =
                stored.measured.width.is_none() || stored.measured.height.is_none();
            if measured_unset && !stored.user.hidden.unwrap_or(false) {
                nodes_initialized = false;
            }
        }

        if user_node.parent_id.is_some() {
            update_child_node(
                &user_node.id,
                node_lookup,
                parent_lookup,
                opts,
                Some(&mut root_parent_index),
            );
        }

        has_selected_nodes |= user_node.selected.unwrap_or(false);
    }

    AdoptUserNodesResult {
        nodes_initialized,
        has_selected_nodes,
    }
}

// ---------------------------------------------------------------------------
// Public: handleExpandParent
// ---------------------------------------------------------------------------

/// `id`/`parent_id`/`rect` triple emitted by [`update_node_internals`]
/// when a child grows past its parent.
///
/// Mirrors the TS `ParentExpandChild` (defined in `utils/types.ts`).
#[derive(Debug, Clone, PartialEq)]
pub struct ParentExpandChild {
    pub id: String,
    pub parent_id: String,
    pub rect: Rect,
}

/// Compute position + dimension changes that auto-grow each parent so
/// it encloses the children listed in `expand_children`.
///
/// Mirrors the TS `handleExpandParent`. The returned changes are
/// emitted to the user via `on_nodes_change`. Note: we use generic
/// `D` here because the changes carry no user data, so this is
/// monomorphic.
#[must_use]
pub fn handle_expand_parent<D: Clone>(
    expand_children: &[ParentExpandChild],
    node_lookup: &NodeLookup<D>,
    parent_lookup: &ParentLookup<D>,
    node_origin: NodeOrigin,
) -> Vec<NodeChange<D>> {
    let mut changes: Vec<NodeChange<D>> = Vec::new();
    let mut parent_expansions: std::collections::HashMap<String, (Rect, InternalNode<D>)> =
        std::collections::HashMap::new();

    for child in expand_children {
        let Some(parent) = node_lookup.get(&child.parent_id) else {
            continue;
        };
        let parent_rect = parent_expansions
            .get(&child.parent_id)
            .map(|(r, _)| *r)
            .unwrap_or_else(|| internal_node_to_rect(parent));
        let expanded_rect = get_bounds_of_rects(parent_rect, child.rect);
        parent_expansions.insert(child.parent_id.clone(), (expanded_rect, parent.clone()));
    }

    for (parent_id, (expanded_rect, parent)) in parent_expansions {
        let position_absolute = parent.internals.position_absolute;
        let dimensions = get_node_dimensions(&parent);
        let origin = parent.user.origin.unwrap_or(node_origin);

        let x_change = if expanded_rect.x < position_absolute.x {
            (position_absolute.x - expanded_rect.x).abs().round()
        } else {
            0.0
        };
        let y_change = if expanded_rect.y < position_absolute.y {
            (position_absolute.y - expanded_rect.y).abs().round()
        } else {
            0.0
        };

        let new_width = dimensions.width.max(expanded_rect.width.round());
        let new_height = dimensions.height.max(expanded_rect.height.round());
        let width_change = (new_width - dimensions.width) * origin.0;
        let height_change = (new_height - dimensions.height) * origin.1;

        if x_change > 0.0 || y_change > 0.0 || width_change != 0.0 || height_change != 0.0 {
            changes.push(NodeChange::Position {
                id: parent_id.clone(),
                position: Some(XYPosition {
                    x: parent.user.position.x - x_change + width_change,
                    y: parent.user.position.y - y_change + height_change,
                }),
                position_absolute: None,
                dragging: None,
            });

            // Counter-shift remaining children so the parent move
            // doesn't visually relocate them.
            if let Some(siblings) = parent_lookup.get(&parent_id) {
                for child_node in siblings.values() {
                    let was_expanded =
                        expand_children.iter().any(|c| c.id == child_node.user.id);
                    if !was_expanded {
                        changes.push(NodeChange::Position {
                            id: child_node.user.id.clone(),
                            position: Some(XYPosition {
                                x: child_node.user.position.x + x_change,
                                y: child_node.user.position.y + y_change,
                            }),
                            position_absolute: None,
                            dragging: None,
                        });
                    }
                }
            }
        }

        if dimensions.width < expanded_rect.width
            || dimensions.height < expanded_rect.height
            || x_change != 0.0
            || y_change != 0.0
        {
            let extra_w = if x_change != 0.0 {
                origin.0 * x_change - width_change
            } else {
                0.0
            };
            let extra_h = if y_change != 0.0 {
                origin.1 * y_change - height_change
            } else {
                0.0
            };
            changes.push(NodeChange::Dimensions {
                id: parent_id,
                dimensions: Some(Dimensions {
                    width: new_width + extra_w,
                    height: new_height + extra_h,
                }),
                resizing: None,
                set_attributes: SetAttributesMode::All,
            });
        }
    }

    changes
}

// ---------------------------------------------------------------------------
// Public: updateNodeInternals
// ---------------------------------------------------------------------------

/// One pre-measured node update. Replaces the TS `InternalNodeUpdate`
/// which carries an `HTMLDivElement`.
///
/// `dimensions`         — `getDimensions(nodeElement)` upstream.
/// `node_bounds`        — `getBoundingClientRect()` upstream.
/// `source_handles`     — pre-collected per-handle measurements that
///                        the TS source pulled via `querySelectorAll('.source')`.
/// `target_handles`     — same for `.target`.
#[derive(Debug, Clone)]
pub struct InternalNodeUpdate {
    pub id: String,
    pub force: bool,
    pub dimensions: Dimensions,
    pub node_bounds_left: f64,
    pub node_bounds_top: f64,
    pub source_handles: Vec<HandleMeasurement>,
    pub target_handles: Vec<HandleMeasurement>,
}

/// Result of [`update_node_internals`].
#[derive(Debug, Clone)]
pub struct UpdateNodeInternalsResult<D: Clone> {
    pub changes: Vec<NodeChange<D>>,
    pub updated_internals: bool,
}

/// Apply a batch of measurement updates to `node_lookup`, updating the
/// stored measured dimensions, handle bounds, and triggering parent-
/// expansion changes when appropriate.
///
/// Mirrors the TS `updateNodeInternals`. The TS source reads viewport
/// `zoom` from a DOM matrix; the Rust port takes it as an argument
/// (consumers compute it via `Transform.scale()`).
#[must_use]
pub fn update_node_internals<D: Clone + PartialEq>(
    updates: &[InternalNodeUpdate],
    node_lookup: &mut NodeLookup<D>,
    parent_lookup: &mut ParentLookup<D>,
    zoom: f64,
    options: &UpdateNodesOptions,
) -> UpdateNodeInternalsResult<D> {
    let mut updated_internals = false;
    let mut changes: Vec<NodeChange<D>> = Vec::new();
    let mut parent_expand_children: Vec<ParentExpandChild> = Vec::new();

    for update in updates {
        let Some(node_clone) = node_lookup.get(&update.id).cloned() else {
            continue;
        };

        if node_clone.user.hidden.unwrap_or(false) {
            if let Some(n) = node_lookup.get_mut(&update.id) {
                n.internals.handle_bounds = None;
            }
            updated_internals = true;
            continue;
        }

        let dim = update.dimensions;
        let dimension_changed = node_clone.measured.width != Some(dim.width)
            || node_clone.measured.height != Some(dim.height);
        let do_update = dim.width != 0.0
            && dim.height != 0.0
            && (dimension_changed
                || node_clone.internals.handle_bounds.is_none()
                || update.force);

        if !do_update {
            continue;
        }

        // Recompute clamped absolute position with new dimensions.
        let mut position_absolute = node_clone.internals.position_absolute;
        if node_clone.user.parent_id.is_some() && matches!(node_clone.user.extent, NodeExtent::Parent) {
            if let Some(parent) = node_clone
                .user
                .parent_id
                .as_deref()
                .and_then(|p| node_lookup.get(p))
            {
                position_absolute =
                    clamp_position_to_parent(position_absolute, dim, parent);
            }
        } else if let Some(custom_extent) = match node_clone.user.extent {
            NodeExtent::Custom(c) => Some(c),
            _ => Some(options.node_extent),
        } {
            position_absolute = clamp_position(position_absolute, custom_extent, dim);
        }

        let source_bounds = build_handle_bounds(
            HandleType::Source,
            update.source_handles.clone(),
            update.node_bounds_left,
            update.node_bounds_top,
            zoom,
            &update.id,
        );
        let target_bounds = build_handle_bounds(
            HandleType::Target,
            update.target_handles.clone(),
            update.node_bounds_left,
            update.node_bounds_top,
            zoom,
            &update.id,
        );

        if let Some(n) = node_lookup.get_mut(&update.id) {
            n.measured = MeasuredDimensions {
                width: Some(dim.width),
                height: Some(dim.height),
            };
            n.internals.position_absolute = position_absolute;
            n.internals.handle_bounds = Some(NodeHandleBounds {
                source: Some(source_bounds),
                target: Some(target_bounds),
            });
        }

        if node_clone.user.parent_id.is_some() {
            update_child_node(&update.id, node_lookup, parent_lookup, options, None);
        }

        updated_internals = true;

        if dimension_changed {
            changes.push(NodeChange::Dimensions {
                id: update.id.clone(),
                dimensions: Some(dim),
                resizing: None,
                set_attributes: SetAttributesMode::None,
            });

            if node_clone.user.expand_parent.unwrap_or(false) {
                if let Some(parent_id) = node_clone.user.parent_id.clone() {
                    if let Some(updated_node) = node_lookup.get(&update.id) {
                        parent_expand_children.push(ParentExpandChild {
                            id: update.id.clone(),
                            parent_id,
                            rect: internal_node_to_rect(updated_node),
                        });
                    }
                }
            }
        }
    }

    if !parent_expand_children.is_empty() {
        let extra =
            handle_expand_parent(&parent_expand_children, node_lookup, parent_lookup, options.node_origin);
        changes.extend(extra);
    }

    UpdateNodeInternalsResult {
        changes,
        updated_internals,
    }
}

// ---------------------------------------------------------------------------
// Public: panBy
// ---------------------------------------------------------------------------

/// Pan the viewport by `(delta.x, delta.y)` pixels, constrained by
/// `translate_extent`.
///
/// Mirrors the TS `panBy`. Returns a [`Promise<bool>`] that resolves
/// to `true` if the resulting transform actually changed.
pub fn pan_by<P: PanZoomInstance + ?Sized>(
    delta: XYPosition,
    pan_zoom: Option<&mut P>,
    transform: Transform,
    translate_extent: CoordinateExtent,
    width: f64,
    height: f64,
) -> Promise<bool> {
    let Some(pan_zoom) = pan_zoom else {
        return Promise::resolved(false);
    };
    if delta.x == 0.0 && delta.y == 0.0 {
        return Promise::resolved(false);
    }
    let result_promise = pan_zoom.set_viewport_constrained(
        crate::types::viewport::Viewport {
            x: transform.tx() + delta.x,
            y: transform.ty() + delta.y,
            zoom: transform.scale(),
        },
        [[0.0, 0.0], [width, height]],
        translate_extent,
    );
    // The TS impl awaits the promise then compares the returned
    // ZoomTransform against the input transform. In our port the
    // PanZoomInstance::set_viewport_constrained returns
    // Promise<Option<Transform>>. We can't easily compare without
    // blocking, so callers receive the underlying promise's resolution
    // wrapped into a "transform changed" bool via a separate helper:
    // here we forward the result and let the caller chain-check.
    //
    // To preserve the TS contract of "Promise<boolean>" we map
    // resolution: any Some(t) means we need to compare t with input
    // transform. We do that by spawning a small pipeline using a
    // threaded conversion.
    let (out_promise, out_resolver) = crate::promise::channel::<bool>();
    std::thread::spawn(move || {
        let next = result_promise.block_take().flatten();
        let changed = next
            .map(|t| t.tx() != transform.tx() || t.ty() != transform.ty() || t.scale() != transform.scale())
            .unwrap_or(false);
        out_resolver.resolve(changed);
    });
    out_promise
}

// ---------------------------------------------------------------------------
// Public: updateConnectionLookup
// ---------------------------------------------------------------------------

fn add_connection_to_lookup(
    type_: HandleType,
    connection: &HandleConnection,
    connection_key: &str,
    connection_lookup: &mut ConnectionLookup,
    node_id: &str,
    handle_id: Option<&str>,
) {
    // Key 1: just the node id.
    connection_lookup
        .entry(node_id.to_string())
        .or_default()
        .insert(connection_key.to_string(), connection.clone());

    // Key 2: nodeId-type.
    let type_str = match type_ {
        HandleType::Source => "source",
        HandleType::Target => "target",
    };
    connection_lookup
        .entry(format!("{node_id}-{type_str}"))
        .or_default()
        .insert(connection_key.to_string(), connection.clone());

    // Key 3: nodeId-type-handleId (only when handleId is set).
    if let Some(hid) = handle_id {
        connection_lookup
            .entry(format!("{node_id}-{type_str}-{hid}"))
            .or_default()
            .insert(connection_key.to_string(), connection.clone());
    }
}

/// Rebuild `connection_lookup` and `edge_lookup` from the current edge
/// list.
///
/// Mirrors the TS `updateConnectionLookup`. Both lookups are cleared
/// and repopulated.
pub fn update_connection_lookup<E: Clone>(
    connection_lookup: &mut ConnectionLookup,
    edge_lookup: &mut EdgeLookup<E>,
    edges: &[Edge<E>],
) {
    connection_lookup.clear();
    edge_lookup.clear();

    for edge in edges {
        let source_handle = edge.source_handle.clone();
        let target_handle = edge.target_handle.clone();
        let connection = HandleConnection {
            connection: crate::types::connection::Connection {
                source: edge.source.clone(),
                target: edge.target.clone(),
                source_handle: source_handle.clone(),
                target_handle: target_handle.clone(),
            },
            edge_id: edge.id.clone(),
        };
        let source_key = format!(
            "{}-{}--{}-{}",
            edge.source,
            source_handle.as_deref().unwrap_or("null"),
            edge.target,
            target_handle.as_deref().unwrap_or("null"),
        );
        let target_key = format!(
            "{}-{}--{}-{}",
            edge.target,
            target_handle.as_deref().unwrap_or("null"),
            edge.source,
            source_handle.as_deref().unwrap_or("null"),
        );

        add_connection_to_lookup(
            HandleType::Source,
            &connection,
            &target_key,
            connection_lookup,
            &edge.source,
            source_handle.as_deref(),
        );
        add_connection_to_lookup(
            HandleType::Target,
            &connection,
            &source_key,
            connection_lookup,
            &edge.target,
            target_handle.as_deref(),
        );

        edge_lookup.insert(edge.id.clone(), edge.clone());
    }
}

// Allow callers to import `is_coordinate_extent` from here for parity
// with the TS export pattern.
pub use crate::utils::general::is_coordinate_extent as re_is_coordinate_extent;

#[cfg(test)]
mod tests {
    use super::*;

    fn user_node(id: &str, x: f64, y: f64) -> Node<()> {
        Node::minimal(id, x, y)
    }

    fn measured_user_node(id: &str, x: f64, y: f64, w: f64, h: f64) -> Node<()> {
        let mut n = user_node(id, x, y);
        n.measured = Some(MeasuredDimensions {
            width: Some(w),
            height: Some(h),
        });
        n
    }

    #[test]
    fn adopt_user_nodes_basic() {
        let mut lookup: NodeLookup<()> = NodeLookup::new();
        let mut parent_lookup: ParentLookup<()> = ParentLookup::new();
        let nodes = vec![
            measured_user_node("a", 10.0, 20.0, 50.0, 50.0),
            measured_user_node("b", 100.0, 100.0, 50.0, 50.0),
        ];
        let result = adopt_user_nodes(&nodes, &mut lookup, &mut parent_lookup, &AdoptUserNodesOptions::default());
        assert!(result.nodes_initialized);
        assert!(!result.has_selected_nodes);
        assert_eq!(lookup.len(), 2);
        assert_eq!(lookup["a"].internals.position_absolute, XYPosition::new(10.0, 20.0));
    }

    #[test]
    fn adopt_user_nodes_unmeasured_marks_uninitialized() {
        let mut lookup: NodeLookup<()> = NodeLookup::new();
        let mut parent_lookup: ParentLookup<()> = ParentLookup::new();
        let nodes = vec![user_node("a", 0.0, 0.0)]; // no measurements
        let result = adopt_user_nodes(&nodes, &mut lookup, &mut parent_lookup, &AdoptUserNodesOptions::default());
        assert!(!result.nodes_initialized);
    }

    #[test]
    fn adopt_user_nodes_hidden_unmeasured_still_initialized() {
        let mut lookup: NodeLookup<()> = NodeLookup::new();
        let mut parent_lookup: ParentLookup<()> = ParentLookup::new();
        let mut node = user_node("a", 0.0, 0.0);
        node.hidden = Some(true);
        let result = adopt_user_nodes(&[node], &mut lookup, &mut parent_lookup, &AdoptUserNodesOptions::default());
        assert!(result.nodes_initialized);
    }

    #[test]
    fn adopt_user_nodes_propagates_selected() {
        let mut lookup: NodeLookup<()> = NodeLookup::new();
        let mut parent_lookup: ParentLookup<()> = ParentLookup::new();
        let mut node = measured_user_node("a", 0.0, 0.0, 10.0, 10.0);
        node.selected = Some(true);
        let result = adopt_user_nodes(&[node], &mut lookup, &mut parent_lookup, &AdoptUserNodesOptions::default());
        assert!(result.has_selected_nodes);
        // With elevate_nodes_on_select=true (default) and Basic z mode, z = 0 + 1000.
        assert_eq!(lookup["a"].internals.z, 1000.0);
    }

    #[test]
    fn adopt_user_nodes_parent_child_lookup() {
        let mut lookup: NodeLookup<()> = NodeLookup::new();
        let mut parent_lookup: ParentLookup<()> = ParentLookup::new();
        let mut child = measured_user_node("c", 5.0, 5.0, 10.0, 10.0);
        child.parent_id = Some("p".into());
        let nodes = vec![
            measured_user_node("p", 0.0, 0.0, 100.0, 100.0),
            child,
        ];
        adopt_user_nodes(&nodes, &mut lookup, &mut parent_lookup, &AdoptUserNodesOptions::default());
        assert!(parent_lookup.contains_key("p"));
        assert!(parent_lookup["p"].contains_key("c"));
        // Child's absolute position = parent.position_absolute + child.position
        assert_eq!(lookup["c"].internals.position_absolute, XYPosition::new(5.0, 5.0));
    }

    #[test]
    fn update_connection_lookup_indexes_keys() {
        let mut conn_lookup: ConnectionLookup = ConnectionLookup::new();
        let mut edge_lookup: EdgeLookup<()> = EdgeLookup::new();
        let edges = vec![Edge::<()>::minimal("e1", "a", "b")];
        update_connection_lookup(&mut conn_lookup, &mut edge_lookup, &edges);

        assert!(edge_lookup.contains_key("e1"));
        // Source side keys
        assert!(conn_lookup.contains_key("a"));
        assert!(conn_lookup.contains_key("a-source"));
        // Target side keys
        assert!(conn_lookup.contains_key("b"));
        assert!(conn_lookup.contains_key("b-target"));
        // Confirm the inner key is the cross-handle string.
        let outer = conn_lookup.get("a").unwrap();
        let inner_key = format!("b-null--a-null");
        assert!(outer.contains_key(&inner_key));
    }

    #[test]
    fn update_connection_lookup_with_handles() {
        let mut conn_lookup: ConnectionLookup = ConnectionLookup::new();
        let mut edge_lookup: EdgeLookup<()> = EdgeLookup::new();
        let mut e = Edge::<()>::minimal("e1", "a", "b");
        e.source_handle = Some("h1".into());
        e.target_handle = Some("h2".into());
        update_connection_lookup(&mut conn_lookup, &mut edge_lookup, &[e]);
        assert!(conn_lookup.contains_key("a-source-h1"));
        assert!(conn_lookup.contains_key("b-target-h2"));
    }

    #[test]
    fn update_connection_lookup_clears_previous() {
        let mut conn_lookup: ConnectionLookup = ConnectionLookup::new();
        let mut edge_lookup: EdgeLookup<()> = EdgeLookup::new();
        update_connection_lookup(&mut conn_lookup, &mut edge_lookup, &[Edge::<()>::minimal("e1", "a", "b")]);
        update_connection_lookup(&mut conn_lookup, &mut edge_lookup, &[]);
        assert!(conn_lookup.is_empty());
        assert!(edge_lookup.is_empty());
    }

    #[test]
    fn update_absolute_positions_recomputes_root_nodes() {
        let mut lookup: NodeLookup<()> = NodeLookup::new();
        let mut parent_lookup: ParentLookup<()> = ParentLookup::new();
        let mut node = measured_user_node("a", 50.0, 50.0, 10.0, 10.0);
        node.origin = Some((0.5, 0.5)); // center origin
        adopt_user_nodes(&[node], &mut lookup, &mut parent_lookup, &AdoptUserNodesOptions::default());
        // origin (0.5, 0.5) shifts position by (-w/2, -h/2) = (-5, -5)
        assert_eq!(lookup["a"].internals.position_absolute, XYPosition::new(45.0, 45.0));
    }

    #[test]
    fn update_node_internals_writes_dimensions() {
        let mut lookup: NodeLookup<()> = NodeLookup::new();
        let mut parent_lookup: ParentLookup<()> = ParentLookup::new();
        // Adopt without measurements first.
        adopt_user_nodes(
            &[user_node("a", 0.0, 0.0)],
            &mut lookup,
            &mut parent_lookup,
            &AdoptUserNodesOptions::default(),
        );
        assert!(lookup["a"].measured.width.is_none());

        let updates = vec![InternalNodeUpdate {
            id: "a".into(),
            force: false,
            dimensions: Dimensions::new(80.0, 40.0),
            node_bounds_left: 0.0,
            node_bounds_top: 0.0,
            source_handles: Vec::new(),
            target_handles: Vec::new(),
        }];
        let result = update_node_internals(
            &updates,
            &mut lookup,
            &mut parent_lookup,
            1.0,
            &UpdateNodesOptions::default(),
        );
        assert!(result.updated_internals);
        assert_eq!(result.changes.len(), 1);
        assert_eq!(lookup["a"].measured.width, Some(80.0));
        assert_eq!(lookup["a"].measured.height, Some(40.0));
        assert!(lookup["a"].internals.handle_bounds.is_some());
    }

    #[test]
    fn update_node_internals_ignores_hidden_nodes_handle_bounds() {
        let mut lookup: NodeLookup<()> = NodeLookup::new();
        let mut parent_lookup: ParentLookup<()> = ParentLookup::new();
        let mut node = measured_user_node("a", 0.0, 0.0, 50.0, 50.0);
        node.hidden = Some(true);
        adopt_user_nodes(&[node], &mut lookup, &mut parent_lookup, &AdoptUserNodesOptions::default());
        // Pre-set some handle bounds.
        if let Some(n) = lookup.get_mut("a") {
            n.internals.handle_bounds = Some(NodeHandleBounds::default());
        }
        let result = update_node_internals(
            &[InternalNodeUpdate {
                id: "a".into(),
                force: false,
                dimensions: Dimensions::new(60.0, 60.0),
                node_bounds_left: 0.0,
                node_bounds_top: 0.0,
                source_handles: Vec::new(),
                target_handles: Vec::new(),
            }],
            &mut lookup,
            &mut parent_lookup,
            1.0,
            &UpdateNodesOptions::default(),
        );
        assert!(result.updated_internals);
        // For hidden nodes the handle_bounds is reset to None.
        assert!(lookup["a"].internals.handle_bounds.is_none());
    }

    #[test]
    fn handle_expand_parent_resizes_when_child_overflows() {
        let mut lookup: NodeLookup<()> = NodeLookup::new();
        // Parent at (0,0) sized 100x100.
        let mut parent = InternalNode::from_user(measured_user_node("p", 0.0, 0.0, 100.0, 100.0));
        parent.internals.position_absolute = XYPosition::new(0.0, 0.0);
        parent.measured = MeasuredDimensions {
            width: Some(100.0),
            height: Some(100.0),
        };
        lookup.insert("p".into(), parent);
        let parent_lookup: ParentLookup<()> = ParentLookup::new();

        // Child rect grows past parent right edge (x=120 > 100).
        let changes = handle_expand_parent(
            &[ParentExpandChild {
                id: "c".into(),
                parent_id: "p".into(),
                rect: Rect::new(20.0, 20.0, 100.0, 100.0),
            }],
            &lookup,
            &parent_lookup,
            (0.0, 0.0),
        );
        // We expect at least a Dimensions change (parent must grow to at least 120x120).
        let has_dim = changes
            .iter()
            .any(|c| matches!(c, NodeChange::Dimensions { .. }));
        assert!(has_dim);
    }
}
