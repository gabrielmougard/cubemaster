//! Port of `xyflow-react/src/hooks/useOnViewportChange.ts`.
//!
//! Status: Phase 3 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::{use_drop, use_hook, WritableExt};

use crate::context::use_rgraph_store;
use crate::types::component_props::OnViewportChange;

/// Options accepted by [`use_on_viewport_change`].
///
/// Mirrors TS `UseOnViewportChangeOptions`.
#[derive(Default)]
pub struct UseOnViewportChangeOptions {
    pub on_start: Option<OnViewportChange>,
    pub on_change: Option<OnViewportChange>,
    pub on_end: Option<OnViewportChange>,
}

/// Subscribe to the three viewport-change phases.
///
/// Mirrors the TS hook. The TS source uses three independent
/// `useEffect`s keyed on each callback; we replicate the semantics
/// with a single mount-time installation plus an unmount-time
/// teardown that nulls the slots back out. Updating the supplied
/// callbacks at runtime requires a re-mount (Phase 7 will lift this
/// limitation by tracking callback identity in a memo).
pub fn use_on_viewport_change<N, E>(options: UseOnViewportChangeOptions)
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();

    use_hook(move || {
        if let Some(cb) = options.on_start {
            store.on_viewport_change_start.clone().set(Some(cb));
        }
        if let Some(cb) = options.on_change {
            store.on_viewport_change.clone().set(Some(cb));
        }
        if let Some(cb) = options.on_end {
            store.on_viewport_change_end.clone().set(Some(cb));
        }
    });

    use_drop(move || {
        store.on_viewport_change_start.clone().set(None);
        store.on_viewport_change.clone().set(None);
        store.on_viewport_change_end.clone().set(None);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn callbacks_installed_on_mount() {
        thread_local! { static OK: Cell<bool> = const { Cell::new(false) }; }

        #[component]
        fn Probe() -> Element {
            use dioxus_signals::ReadableExt;
            let cb: OnViewportChange =
                use_callback(|_v: rgraph_core::types::viewport::Viewport| {});
            use_on_viewport_change::<(), ()>(UseOnViewportChangeOptions {
                on_change: Some(cb),
                ..Default::default()
            });
            let store = use_rgraph_store::<(), ()>();
            OK.with(|c| c.set(store.on_viewport_change.peek().is_some()));
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
