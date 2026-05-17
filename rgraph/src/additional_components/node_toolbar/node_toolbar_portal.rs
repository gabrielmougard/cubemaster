//! Port of `xyflow-react/src/additional-components/NodeToolbar/NodeToolbarPortal.tsx`.
//!
//! Status: Phase 8 — implemented.
//!
//! Dioxus 0.7 has no `createPortal`, so the children render inline. The
//! visual result is equivalent because the toolbar uses
//! `position: absolute` relative to the nearest positioned ancestor
//! (the `<RGraph>` container itself).

use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct NodeToolbarPortalProps {
    pub children: Element,
}

#[component]
pub fn NodeToolbarPortal(props: NodeToolbarPortalProps) -> Element {
    rsx! { {props.children} }
}
