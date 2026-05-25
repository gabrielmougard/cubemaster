//! Port of `xyflow-react/src/components/NodeWrapper/useNodeObserver.ts`.
//!
//! Status: Phase 5 — implemented (one-shot measurement on mount; the
//! continuous-observer pipeline lives in
//! [`crate::container::node_renderer::use_resize_observer`]).
//!
//! Returns an `onmounted` callback the wrapper should attach to its
//! `<div>`. The callback:
//!
//! 1. Fetches the node's bounding-client-rect via
//!    [`dioxus::events::MountedData::get_client_rect`].
//! 2. Schedules an `update_node_internals` dispatch with the measured
//!    dimensions so the store's `node_lookup` learns the correct size
//!    on first paint.
//!
//! Continuous re-measurement happens through the shared
//! `ResizeObserver` shim from [`crate::dom::resize_observer`] —
//! [`use_node_observer`] doesn't subscribe to it directly. The node id
//! is recorded under the same `data-id` attribute the TS source uses,
//! so the shim can resolve elements by selector if needed.

#![allow(clippy::module_name_repetitions)]

use dioxus::events::MountedData;
use dioxus::prelude::*;

use rgraph_core::types::geometry::Dimensions;
use rgraph_core::utils::store::InternalNodeUpdate as CoreInternalNodeUpdate;

use crate::context::use_rgraph_store;
use crate::store::RGraphStore;

/// Returned by [`use_node_observer`]. Carries the `onmounted` callback
/// the host should attach to the wrapper `<div>`.
#[derive(Clone, Copy)]
pub struct NodeObserverApi {
    pub on_mounted: Callback<Event<MountedData>>,
}

/// Install a per-node measurement effect for the wrapper element.
///
/// `node_id` must match the `data-id` attribute the wrapper sets so
/// the shared `ResizeObserver` can resolve the element later via
/// selector queries.
pub fn use_node_observer<N, E>(node_id: String) -> NodeObserverApi
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let id = node_id;

    let on_mounted = use_callback(move |evt: Event<MountedData>| {
        let store = store;
        let id = id.clone();
        spawn(async move {
            let Ok(rect) = evt.get_client_rect().await else { return };
            let dim = Dimensions {
                width: rect.size.width,
                height: rect.size.height,
            };
            if dim.width <= 0.0 || dim.height <= 0.0 {
                return;
            }

            // Bypass `RGraphStore::update_node_internals` (whose action
            // callback hasn't been wired yet) and write straight into
            // the lookup. Mirrors the TS `updateNodeInternals(map)`
            // path with a synthetic single-element batch.
            apply_dimension_update(store, &id, dim);
        });
    });

    NodeObserverApi { on_mounted }
}

/// Apply a single dimension measurement to `store.node_lookup`. Used
/// by [`use_node_observer`] and the shared
/// [`crate::container::node_renderer::use_resize_observer`] hook.
pub(crate) fn apply_dimension_update<N, E>(
    store: RGraphStore<N, E>,
    node_id: &str,
    dim: Dimensions,
) where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    use rgraph_core::types::geometry::Position;
    use rgraph_core::types::nodes::MeasuredDimensions;
    use rgraph_core::utils::dom::HandleMeasurement;
    use rgraph_core::utils::store::{update_node_internals, UpdateNodesOptions};

    // Synthesize handle measurements for the built-in node types so
    // edges have something to attach to. The real `useNodeObserver`
    // upstream does a `querySelectorAll('.source')` / `'.target'` on
    // the node's DOM subtree; the Dioxus port doesn't yet bridge that,
    // so we fall back to the canonical positions emitted by
    // `<DefaultNode>` / `<InputNode>` / `<OutputNode>`.
    let node_type = {
        use dioxus::prelude::ReadableExt;
        let lookup = store.node_lookup.peek();
        lookup
            .get(node_id)
            .and_then(|n| n.user.type_.clone())
            .unwrap_or_else(|| "default".to_string())
    };
    // We synthesise *zero-sized* handle bounds anchored exactly on the
    // node's edge. The visible handle dot (sized via CSS) sits on top
    // of this point thanks to `.react-flow__handle-{top,bottom,…}` and
    // its `transform: translate(...)` centering. Using 0×0 here makes
    // `get_handle_position` return exactly the boundary point instead
    // of "boundary + half-handle", so the edge meets the handle dot
    // visually instead of leaving a gap.
    let bottom = HandleMeasurement {
        id: None,
        position: Position::Bottom,
        bounds_left: dim.width / 2.0,
        bounds_top: dim.height,
        width: 0.0,
        height: 0.0,
    };
    let top = HandleMeasurement {
        id: None,
        position: Position::Top,
        bounds_left: dim.width / 2.0,
        bounds_top: 0.0,
        width: 0.0,
        height: 0.0,
    };
    let (source_handles, target_handles) = match node_type.as_str() {
        "input" => (vec![bottom.clone()], Vec::new()),
        "output" => (Vec::new(), vec![top.clone()]),
        "group" => (Vec::new(), Vec::new()),
        _ => (vec![bottom.clone()], vec![top.clone()]),
    };

    let updates = vec![CoreInternalNodeUpdate {
        id: node_id.to_string(),
        force: true,
        dimensions: dim,
        node_bounds_left: 0.0,
        node_bounds_top: 0.0,
        source_handles,
        target_handles,
    }];

    let zoom = store.transform.peek().scale();
    let options = UpdateNodesOptions {
        node_origin: *store.node_origin.peek(),
        node_extent: *store.node_extent.peek(),
        elevate_nodes_on_select: *store.elevate_nodes_on_select.peek(),
        z_index_mode: *store.z_index_mode.peek(),
    };

    let result = {
        use dioxus::prelude::ReadableExt;
        let mut node_lookup = store.node_lookup.clone().write_unchecked();
        let mut parent_lookup = store.parent_lookup.clone().write_unchecked();
        update_node_internals(&updates, &mut node_lookup, &mut parent_lookup, zoom, &options)
    };

    if !result.updated_internals {
        return;
    }

    // The lookup mutation above doesn't fire signal subscribers — we
    // must explicitly poke the lookup signal so visible-node-ids
    // hooks pick up the new measurement.
    {
        use dioxus::prelude::WritableExt;
        let lookup = store.node_lookup.peek().clone();
        store.node_lookup.clone().set(lookup);
    }

    if !result.changes.is_empty() {
        store.trigger_node_changes(result.changes);
    }

    // Stamp `measured` directly onto the cloned user-facing node list
    // so derived hooks like `use_nodes` see the dimension. This is a
    // best-effort write — the canonical state lives in `node_lookup`.
    {
        use dioxus::prelude::{ReadableExt, WritableExt};
        let mut nodes = store.nodes.peek().clone();
        let mut changed = false;
        for n in nodes.iter_mut() {
            if n.id == node_id {
                let prev = n.measured;
                let next = MeasuredDimensions {
                    width: Some(dim.width),
                    height: Some(dim.height),
                };
                if prev != Some(next) {
                    n.measured = Some(next);
                    changed = true;
                }
                break;
            }
        }
        if changed {
            store.nodes.clone().set(nodes);
        }
    }
}
