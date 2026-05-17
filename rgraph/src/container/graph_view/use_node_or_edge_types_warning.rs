//! Port of `xyflow-react/src/container/GraphView/useNodeOrEdgeTypesWarning.ts`.
//!
//! Status: Phase 7 — implemented.
//!
//! Dev-only warning when the registered `node_types` or `edge_types`
//! map changes between renders. Frequent re-creation of the map causes
//! every node/edge to remount and is almost always a bug.
//!
//! The TS source uses `process.env.NODE_ENV === 'development'` to gate
//! the warning; in Rust we gate on `cfg!(debug_assertions)`, which
//! matches `cargo run`/`cargo test` but stays silent in release builds.

#![allow(clippy::module_name_repetitions)]

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::{use_effect, use_hook};

use crate::context::use_rgraph_store;
use crate::types::component_props::OnErrorArgs;

/// Identifier marker for which registry is being watched. The TS hook
/// has two overloads (nodeTypes / edgeTypes); we use one type-erased
/// helper plus a label so error messages can differentiate.
#[derive(Debug, Clone, Copy)]
pub enum TypesKind {
    Node,
    Edge,
}

/// Warn (via `on_error`) when the *identity* of the supplied registry
/// changes between renders. Identity is approximated by the key-set:
/// if any key was added or removed since the previous render, the
/// warning fires.
///
/// `keys` is the deduplicated `Vec<String>` of registry keys captured
/// at call time. Passing a snapshot (rather than the live registry
/// type-erased) keeps the hook trait-free.
pub fn use_node_or_edge_types_warning<N, E>(kind: TypesKind, keys: Vec<String>)
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    if !cfg!(debug_assertions) {
        return;
    }

    let store = use_rgraph_store::<N, E>();
    let prev: Rc<RefCell<Vec<String>>> = use_hook(|| Rc::new(RefCell::new(Vec::new())));

    use_effect(move || {
        use dioxus::prelude::ReadableExt;
        let mut prev_guard = prev.borrow_mut();
        if *prev_guard != keys {
            // Mirrors TS `errorMessages['error002']()` — re-created
            // typesMap on every render is the canonical mistake.
            if let Some(handler) = *store.on_error.peek() {
                let label = match kind {
                    TypesKind::Node => "nodeTypes",
                    TypesKind::Edge => "edgeTypes",
                };
                handler.call(OnErrorArgs {
                    id: "002".to_string(),
                    message: format!(
                        "[rgraph]: It looks like you've created a new {label} or edgeTypes \
                         object. If this wasn't on purpose please define the {label}/edgeTypes \
                         outside of the component or memoize them."
                    ),
                });
            }
            *prev_guard = keys.clone();
        }
    });
}
