//! Port of `xyflow-react/src/hooks/useGlobalKeyHandler.ts`.
//!
//! Status: Phase 3 — implemented (algorithm + signal wiring; the
//! actual global listener installation is Phase 4 territory).
//!
//! TS reference: maps the `delete` and `multi-selection` key chords to
//! store actions. When `delete` is pressed, all currently selected
//! nodes / edges are passed to `delete_elements` and the
//! nodes-selection rectangle is cleared. When the multi-selection
//! chord is held, `multi_selection_active` is set on the store.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::{ReadableExt, Signal, WritableExt};

use rgraph_core::types::viewport::KeyCode;

use crate::context::use_rgraph_store;
use crate::hooks::use_key_press::{use_key_press, KeyPressApi, UseKeyPressOptions};
use crate::store::RGraphStore;
use crate::types::edges::Edge;
use crate::types::nodes::Node;

/// Bundle returned by [`use_global_key_handler`] — wraps the two
/// [`KeyPressApi`]s the host needs to attach (one for `delete`, one
/// for the multi-selection chord). Hosts wire these to keyboard
/// events on the `<RGraph>` wrapper / window during Phase 4.
#[derive(Clone, Copy)]
pub struct GlobalKeyHandler {
    pub delete: KeyPressApi,
    pub multi_selection: KeyPressApi,
}

/// Effects that fire when the wrapped `KeyPressApi`s flip their
/// pressed signals. Hosts should call [`Self::run`] inside a
/// `use_effect` keyed on `(delete.pressed, multi_selection.pressed)`.
pub struct GlobalKeyHandlerEffects<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> {
    /// The store handle.
    pub store: RGraphStore<N, E>,
}

// `RGraphStore<N, E>` is `Copy` (all its fields are `Signal` /
// `Copy`), so the effects bundle can be too — `use_effect` callers
// often want to capture it into two independent closures.
impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Copy
    for GlobalKeyHandlerEffects<N, E>
{
}
impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Clone
    for GlobalKeyHandlerEffects<N, E>
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> GlobalKeyHandlerEffects<N, E> {
    /// Apply the delete-on-press effect: filter selected items and
    /// dispatch them through `trigger_*_changes`. Mirrors TS lines
    /// 31–38.
    ///
    /// We fan out to the existing change-machinery rather than calling
    /// the TS `deleteElements` directly: deletion goes through
    /// `Vec<NodeChange::Remove>` / `Vec<EdgeChange::Remove>` which
    /// also fires `on_nodes_change` / `on_edges_change` for free.
    pub fn run_delete(&self) {
        use crate::utils::changes::{edge_to_remove_change, node_to_remove_change};

        let nodes = self.store.nodes.peek().clone();
        let edges = self.store.edges.peek().clone();
        let node_changes: Vec<_> = nodes
            .iter()
            .filter(|n| n.selected.unwrap_or(false))
            .map(node_to_remove_change)
            .collect();
        let edge_changes: Vec<_> = edges
            .iter()
            .filter(|e| e.selected.unwrap_or(false))
            .map(edge_to_remove_change)
            .collect();

        // Mirror TS `triggerEdgeChanges(...) ; triggerNodeChanges(...);`
        // — edges first so removed nodes can pull in connected edges
        // through `applyEdgeChanges` middleware.
        if !edge_changes.is_empty() {
            if let Some(handler) = *self.store.on_edges_delete.peek() {
                let removed_edges: Vec<Edge<E>> = edges
                    .iter()
                    .filter(|e| e.selected.unwrap_or(false))
                    .cloned()
                    .collect();
                handler.call(removed_edges);
            }
            self.store.trigger_edge_changes(edge_changes);
        }
        if !node_changes.is_empty() {
            if let Some(handler) = *self.store.on_nodes_delete.peek() {
                let removed_nodes: Vec<Node<N>> = nodes
                    .iter()
                    .filter(|n| n.selected.unwrap_or(false))
                    .cloned()
                    .collect();
                handler.call(removed_nodes);
            }
            self.store.trigger_node_changes(node_changes);
        }

        // TS line 35: `nodesSelectionActive: false`.
        self.store.nodes_selection_active.clone().set(false);
    }

    /// Apply the multi-selection-on-press effect: simply mirror the
    /// pressed flag onto `store.multi_selection_active`. Mirrors TS
    /// lines 39–41.
    pub fn run_multi_selection(&self, pressed: bool) {
        self.store.multi_selection_active.clone().set(pressed);
    }
}

/// `use_global_key_handler({ delete_key_code, multi_selection_key_code })`.
///
/// Mirrors the TS hook. Returns a [`GlobalKeyHandler`] holding the two
/// underlying [`KeyPressApi`]s plus an [`GlobalKeyHandlerEffects`] the
/// host can call to react to chord changes.
///
/// Once Phase 4 lands, this hook will additionally install the global
/// `keydown`/`keyup` listeners through `dom::eval` so consumers don't
/// have to wire the events manually.
pub fn use_global_key_handler<N, E>(
    delete_key_code: Option<KeyCode>,
    multi_selection_key_code: Option<KeyCode>,
) -> (GlobalKeyHandler, GlobalKeyHandlerEffects<N, E>)
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let delete = use_key_press(
        delete_key_code,
        UseKeyPressOptions {
            act_inside_input_with_modifier: false,
            ..UseKeyPressOptions::default()
        },
    );
    let multi_selection = use_key_press(multi_selection_key_code, UseKeyPressOptions::default());

    let store = use_rgraph_store::<N, E>();

    (
        GlobalKeyHandler {
            delete,
            multi_selection,
        },
        GlobalKeyHandlerEffects { store },
    )
}

/// Convenience: returns the two pressed-state signals as a tuple, for
/// use in `use_effect` dependency lists.
#[must_use]
pub fn pressed_signals(handler: &GlobalKeyHandler) -> (Signal<bool>, Signal<bool>) {
    (handler.delete.pressed, handler.multi_selection.pressed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use crate::types::nodes::Node;
    use dioxus::prelude::*;
    use std::cell::Cell;

    /// `run_delete` removes only selected nodes/edges and clears
    /// `nodes_selection_active`.
    #[test]
    fn run_delete_filters_selected_only() {
        thread_local! {
            static NODE_COUNT: Cell<usize> = const { Cell::new(usize::MAX) };
            static EDGE_COUNT: Cell<usize> = const { Cell::new(usize::MAX) };
        }

        #[component]
        fn Probe() -> Element {
            use dioxus_signals::ReadableExt;
            let store = use_rgraph_store::<(), ()>();
            // Mark "a" as selected, "b" as not.
            let mut a = Node::<()>::minimal("a", 0.0, 0.0);
            a.selected = Some(true);
            let b = Node::<()>::minimal("b", 1.0, 1.0);
            // We need `has_default_nodes` so trigger_node_changes
            // applies the changes locally instead of relying on a
            // controlling parent.
            store.has_default_nodes.clone().set(true);
            store.set_nodes(vec![a, b]);

            // Drive the effect.
            let effects: GlobalKeyHandlerEffects<(), ()> =
                GlobalKeyHandlerEffects { store };
            effects.run_delete();

            NODE_COUNT.with(|c| c.set(store.nodes.peek().len()));
            EDGE_COUNT.with(|c| c.set(store.edges.peek().len()));
            rsx! { div {} }
        }

        fn Root() -> Element {
            rsx! { RGraphProvider::<(), ()> { Probe {} } }
        }

        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert_eq!(NODE_COUNT.with(|c| c.get()), 1); // "b" remains
        assert_eq!(EDGE_COUNT.with(|c| c.get()), 0);
    }
}
