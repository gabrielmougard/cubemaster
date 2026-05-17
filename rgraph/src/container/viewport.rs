//! Port of `xyflow-react/src/container/Viewport/index.tsx`.
//!
//! Status: Phase 4 — implemented.
//!
//! The `<Viewport>` component is a single `<div>` that applies the
//! current `transform` from the store as a CSS
//! `transform: translate(...) scale(...)`. Children render in
//! flow-coordinate space — they are siblings of the actual nodes.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use crate::context::use_rgraph_store;
use crate::store::RGraphStore;

/// Props for [`Viewport`]. The component is generic over the data
/// types so it pulls the store from context using the right shape.
#[derive(Props, Clone, PartialEq)]
pub struct ViewportProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    /// Children rendered inside the transformed `<div>`.
    pub children: Element,
    /// Phantom marker so callers can spell `Viewport::<MyNode, MyEdge>`
    /// even though the component itself doesn't carry a payload.
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

/// `<Viewport>` renders a transformed `<div>` whose `transform`
/// attribute mirrors the current `(tx, ty, zoom)` of the store.
#[component]
pub fn Viewport<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: ViewportProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let t = *store.transform.read();
    let style = format!(
        "transform: translate({tx}px,{ty}px) scale({z});",
        tx = t.tx(),
        ty = t.ty(),
        z = t.scale()
    );

    rsx! {
        div {
            class: "react-flow__viewport xyflow__viewport react-flow__container",
            style: "{style}",
            {props.children}
        }
    }
}
