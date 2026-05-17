//! Port of `xyflow-react/src/hooks/useOnSelectionChange.ts`.
//!
//! Status: Phase 3 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::{use_drop, use_hook, WritableExt};

use crate::context::use_rgraph_store;
use crate::types::general::OnSelectionChangeFunc;

/// Options for [`use_on_selection_change`]. Mirrors the TS
/// `UseOnSelectionChangeOptions`.
pub struct UseOnSelectionChangeOptions<N: Clone, E: Clone> {
    /// The handler to register.
    pub on_change: OnSelectionChangeFunc<N, E>,
}

/// Subscribe to selection changes (nodes + edges).
///
/// Mirrors the TS hook. The supplied callback is appended to
/// `store.on_selection_change_handlers` on mount and removed on
/// unmount. As with TS, callers should memoise the callback so it
/// has stable identity across renders.
pub fn use_on_selection_change<N, E>(options: UseOnSelectionChangeOptions<N, E>)
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();
    let on_change = options.on_change;

    use_hook(move || {
        let mut current = store.on_selection_change_handlers.peek().clone();
        current.push(on_change);
        store.on_selection_change_handlers.clone().set(current);
    });

    // Remove on unmount. `Callback: PartialEq` lets us filter by
    // identity (per `Callback`'s pointer-equality semantics).
    use_drop(move || {
        let next: Vec<_> = store
            .on_selection_change_handlers
            .peek()
            .iter()
            .filter(|cb| **cb != on_change)
            .copied()
            .collect();
        store.on_selection_change_handlers.clone().set(next);
    });
}

// `peek` lives on `ReadableExt`; bring it into scope.
#[allow(unused_imports)]
use dioxus::prelude::ReadableExt;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn handler_registered_on_mount() {
        thread_local! { static SIZE: Cell<usize> = const { Cell::new(0) }; }

        #[component]
        fn Probe() -> Element {
            let cb: OnSelectionChangeFunc<(), ()> =
                use_callback(|_args: crate::types::general::OnSelectionChangeParams<(), ()>| {});
            use_on_selection_change::<(), ()>(UseOnSelectionChangeOptions { on_change: cb });
            let store = use_rgraph_store::<(), ()>();
            SIZE.with(|c| c.set(store.on_selection_change_handlers.read().len()));
            rsx! { div {} }
        }
        fn Root() -> Element {
            rsx! { RGraphProvider::<(), ()> { Probe {} } }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(SIZE.with(|c| c.get()) >= 1);
    }
}
