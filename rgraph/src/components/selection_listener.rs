//! Port of `xyflow-react/src/components/SelectionListener/index.tsx`.
//!
//! Status: Phase 5 — implemented.
//!
//! Drives the registered `on_selection_change` callbacks each time the
//! selection (nodes + edges combined) changes. The TS source uses
//! Zustand's shallow-equality on id arrays to detect changes; we do
//! the same with manual id collection + `==` on `Vec<String>`.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use crate::context::use_rgraph_store;
use crate::store::RGraphStore;
use crate::types::general::{OnSelectionChangeFunc, OnSelectionChangeParams};

#[derive(Props, Clone, PartialEq)]
pub struct SelectionListenerProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    #[props(default)]
    pub on_selection_change: Option<OnSelectionChangeFunc<N, E>>,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn SelectionListener<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: SelectionListenerProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let handler_present = !store.on_selection_change_handlers.read().is_empty();
    let prop_handler = props.on_selection_change;

    // Track previous selection ids so we only fire when something
    // actually changed.
    let mut prev_node_ids: Signal<Vec<String>> = use_signal(Vec::new);
    let mut prev_edge_ids: Signal<Vec<String>> = use_signal(Vec::new);

    use_effect(move || {
        if !handler_present && prop_handler.is_none() {
            return;
        }

        let nodes_snapshot: Vec<_> = store
            .node_lookup
            .read()
            .values()
            .filter(|n| n.user.selected.unwrap_or(false))
            .map(|n| n.user.clone())
            .collect();
        let edges_snapshot: Vec<_> = store
            .edge_lookup
            .read()
            .values()
            .filter(|e| e.selected.unwrap_or(false))
            .cloned()
            .collect();

        let new_node_ids: Vec<String> = nodes_snapshot.iter().map(|n| n.id.clone()).collect();
        let new_edge_ids: Vec<String> = edges_snapshot.iter().map(|e| e.id.clone()).collect();

        if *prev_node_ids.peek() == new_node_ids && *prev_edge_ids.peek() == new_edge_ids {
            return;
        }
        prev_node_ids.set(new_node_ids);
        prev_edge_ids.set(new_edge_ids);

        let params = OnSelectionChangeParams::<N, E> {
            nodes: nodes_snapshot,
            edges: edges_snapshot,
        };
        if let Some(cb) = prop_handler {
            cb.call(params.clone());
        }
        for cb in store.on_selection_change_handlers.read().iter() {
            cb.call(params.clone());
        }
    });

    rsx! {}
}
