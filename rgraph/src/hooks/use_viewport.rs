//! Port of `xyflow-react/src/hooks/useViewport.ts`.
//!
//! Status: Phase 3 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::ReadableExt;

use rgraph_core::types::viewport::Viewport;

use crate::context::use_rgraph_store;

/// Returns the current viewport (`{ x, y, zoom }`) read off the store
/// `transform`.
///
/// Mirrors the TS `useViewport`. Components that call this hook
/// re-render whenever the viewport (i.e. the `transform` signal)
/// changes.
#[must_use]
pub fn use_viewport<N, E>() -> Viewport
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();
    let t = *store.transform.read();
    Viewport {
        x: t.tx(),
        y: t.ty(),
        zoom: t.scale(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn use_viewport_returns_identity_by_default() {
        thread_local! { static SEEN: Cell<(f64, f64, f64)> = const { Cell::new((-1.0, -1.0, -1.0)) }; }

        #[component]
        fn Probe() -> Element {
            let v = use_viewport::<(), ()>();
            SEEN.with(|c| c.set((v.x, v.y, v.zoom)));
            rsx! { div {} }
        }
        fn Root() -> Element {
            rsx! { RGraphProvider::<(), ()> { Probe {} } }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert_eq!(SEEN.with(|c| c.get()), (0.0, 0.0, 1.0));
    }
}
