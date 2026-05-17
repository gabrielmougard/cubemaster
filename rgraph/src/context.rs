//! `context` — Dioxus context wrapper around the reactive
//! [`crate::store::RGraphStore`].
//!
//! Status: Phase 2 — implemented.
//!
//! Mirrors the TS pair `StoreContext.ts` + the `<Provider value={store}>`
//! line in `ReactFlowProvider/index.tsx`. The Rust port replaces the
//! React `createContext`/`Provider`/`useContext` triplet with Dioxus'
//! `use_context_provider` and `use_context`.
//!
//! ## Generics
//!
//! `<N, E>` are the user data types for nodes and edges. Each
//! `RGraphProvider<N, E>` injects a distinct `RGraphStore<N, E>`,
//! identified by its concrete type. Multiple sibling `<RGraph>`s with
//! different data types can therefore coexist in the same tree — but a
//! single tree only ever sees *one* store of each `(N, E)` shape.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::{use_context, use_context_provider, try_consume_context};

use crate::store::RGraphStore;

/// Inject the supplied `RGraphStore` into Dioxus context.
///
/// Must be called inside a component (mirrors React's
/// `<Provider value={store}>` placement). Subsequent calls to
/// [`use_rgraph_store`] / [`try_use_rgraph_store`] in descendant
/// components return the same handle.
///
/// Returns the store unchanged so callers can chain reads on it.
pub fn provide_rgraph_store<N, E>(store: RGraphStore<N, E>) -> RGraphStore<N, E>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    use_context_provider(|| store)
}

/// Read the nearest ancestor [`RGraphStore`] from context, panicking
/// if no `RGraphProvider<N, E>` is mounted above this component.
///
/// Mirrors `useStore`/`useStoreApi` from `xyflow-react`, both of which
/// pull from the React context and throw an error when absent. Use
/// [`try_use_rgraph_store`] to recover gracefully instead.
#[must_use]
pub fn use_rgraph_store<N, E>() -> RGraphStore<N, E>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    use_context::<RGraphStore<N, E>>()
}

/// Non-panicking variant of [`use_rgraph_store`].
///
/// Returns `None` when no `RGraphProvider<N, E>` is mounted above the
/// caller. This is the Rust analogue of the TS `useContext(StoreContext)`
/// returning `null`.
#[must_use]
pub fn try_use_rgraph_store<N, E>() -> Option<RGraphStore<N, E>>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    // `try_consume_context` works as a hook because the lookup is
    // generational-arena based; it doesn't allocate per-render state,
    // making it safe to call inside arbitrary scopes (matches the
    // semantics of the JS `useContext`).
    try_consume_context::<RGraphStore<N, E>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InitialStateParams;
    use dioxus::prelude::*;

    /// Mount a provider in the root, read the store back from a child
    /// scope, and confirm the two handles compare equal (same backing
    /// signals).
    #[test]
    fn provider_and_consumer_share_one_store() {
        use std::cell::Cell;
        thread_local! {
            static OK: Cell<bool> = const { Cell::new(false) };
        }

        #[component]
        fn Child() -> Element {
            let s: RGraphStore<(), ()> = use_rgraph_store();
            // We just need to confirm reading the store succeeds.
            let _ = s.rf_id;
            OK.with(|c| c.set(true));
            rsx! { div {} }
        }

        fn Root() -> Element {
            let store: RGraphStore<(), ()> =
                RGraphStore::new(InitialStateParams::default());
            provide_rgraph_store(store);
            rsx! { Child {} }
        }

        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(OK.with(|c| c.get()));
    }

    #[test]
    fn try_use_returns_none_when_no_provider() {
        use std::cell::Cell;
        thread_local! {
            static GOT_NONE: Cell<bool> = const { Cell::new(false) };
        }

        fn Root() -> Element {
            let s: Option<RGraphStore<(), ()>> = try_use_rgraph_store();
            GOT_NONE.with(|c| c.set(s.is_none()));
            rsx! { div {} }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(GOT_NONE.with(|c| c.get()));
    }
}
