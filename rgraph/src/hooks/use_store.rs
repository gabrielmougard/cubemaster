//! Port of `xyflow-react/src/hooks/useStore.ts`.
//!
//! Status: Phase 3 — implemented.
//!
//! TS source provides two hooks:
//!
//! * `useStore(selector, equalityFn?)` — generic selector hook over the
//!   Zustand store; subscribes the calling component to the slice
//!   returned by `selector` and short-circuits re-renders via
//!   `equalityFn` (typically `shallow`).
//! * `useStoreApi()` — returns `{ getState, setState, subscribe }` so
//!   callers can read/write outside React's reactive system.
//!
//! In the Dioxus port the two collapse into a single primitive:
//! [`use_store`] returns the live [`crate::store::RGraphStore`] handle.
//! Each store field is its own `Signal<T>`; subscribing to a slice is
//! achieved by **reading the relevant signal** in the calling
//! component (`store.nodes.read()` etc.). That's the per-field
//! equivalent of `useStore(s => s.nodes, shallow)` in TS — granular,
//! compile-time-checked, no selector function needed.
//!
//! For parity with the TS API surface we also expose [`use_store_api`]
//! as an alias of `use_store`. It returns the same handle and exists
//! so downstream code reading the docs has a 1:1 mapping.

#![allow(clippy::module_name_repetitions)]

use crate::context::use_rgraph_store;
use crate::store::RGraphStore;

/// Returns the [`RGraphStore`] handle injected by the nearest
/// `<RGraphProvider>`. Panics if no provider is mounted above the
/// caller (mirrors the TS `useStore` `null`-throws behaviour).
///
/// Subscribing to a particular slice is done by reading the relevant
/// signal:
///
/// ```ignore
/// use rgraph::prelude::*;
///
/// fn MyComponent() -> Element {
///     let store: RGraphStore<(), ()> = use_store();
///     let nodes_count = store.nodes.read().len();
///     rsx! { div { "nodes: {nodes_count}" } }
/// }
/// ```
///
/// In the TS source this requires `useStore(s => s.nodes.length,
/// shallow)`; in Rust the per-field signal does the equality
/// short-circuit automatically — only changes to `store.nodes` re-run
/// the calling component.
#[must_use]
pub fn use_store<N, E>() -> RGraphStore<N, E>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    use_rgraph_store::<N, E>()
}

/// Alias of [`use_store`] for parity with TS `useStoreApi`. The TS
/// version exposes `{ getState, setState, subscribe }`; in the Rust
/// port the `RGraphStore` handle already plays both roles — `peek()`
/// is `getState`, the `set_*` action methods are `setState`, and any
/// signal read inside a component is the equivalent of `subscribe`.
#[must_use]
pub fn use_store_api<N, E>() -> RGraphStore<N, E>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    use_rgraph_store::<N, E>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use dioxus::prelude::*;
    use std::cell::Cell;

    /// Mounting `<RGraphProvider>` and calling `use_store` from a child
    /// returns a usable handle whose signals can be read.
    #[test]
    fn use_store_returns_provider_store() {
        use dioxus_signals::ReadableExt;
        thread_local! { static OK: Cell<bool> = const { Cell::new(false) }; }

        #[component]
        fn Probe() -> Element {
            let store: RGraphStore<(), ()> = super::use_store();
            // Read a couple of signals to confirm the handle is live.
            let _w = *store.width.peek();
            let _h = *store.height.peek();
            OK.with(|c| c.set(true));
            rsx! { div {} }
        }

        fn Root() -> Element {
            rsx! {
                RGraphProvider::<(), ()> {
                    Probe {}
                }
            }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(OK.with(|c| c.get()));
    }

    /// `use_store_api` is just an alias.
    #[test]
    fn use_store_api_returns_same_handle_as_use_store() {
        thread_local! { static OK: Cell<bool> = const { Cell::new(false) }; }

        #[component]
        fn Probe() -> Element {
            let a: RGraphStore<(), ()> = super::use_store();
            let b: RGraphStore<(), ()> = super::use_store_api();
            OK.with(|c| c.set(a == b));
            rsx! { div {} }
        }

        fn Root() -> Element {
            rsx! { RGraphProvider::<(), ()> { Probe {} } }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(OK.with(|c| c.get()));
    }
}
