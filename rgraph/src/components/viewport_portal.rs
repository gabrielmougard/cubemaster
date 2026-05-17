//! Port of `xyflow-react/src/components/ViewportPortal/index.tsx`.
//!
//! Status: Phase 6 — implemented (inline; see
//! [`crate::components::edge_label_renderer`] for the same
//! caveat about portal semantics in Dioxus 0.7).

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ViewportPortalProps {
    pub children: Element,
}

/// `<ViewportPortal>` — children render at the viewport's coordinate
/// system. In Phase 6 we render them in-place wrapped in a div with
/// the expected `.react-flow__viewport-portal` class.
#[component]
pub fn ViewportPortal(props: ViewportPortalProps) -> Element {
    rsx! {
        div {
            class: "react-flow__viewport-portal",
            {props.children}
        }
    }
}
