//! Port of `xyflow-react/src/components/EdgeLabelRenderer/index.tsx`.
//!
//! Status: Phase 6 — implemented (inline; the TS source uses
//! `createPortal` to escape into a dedicated div near `<RGraph>`'s
//! root — Dioxus 0.7 has no equivalent so we render the children
//! in-place inside a `.react-flow__edgelabel-renderer` wrapper).
//!
//! The visual result is the same in the common case: the wrapper is
//! `position: absolute; pointer-events: none;` per the bundled
//! stylesheet, and child labels stack above the SVG paths.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct EdgeLabelRendererProps {
    pub children: Element,
}

#[component]
pub fn EdgeLabelRenderer(props: EdgeLabelRendererProps) -> Element {
    rsx! {
        div {
            class: "react-flow__edgelabel-renderer",
            style: "position:absolute;width:100%;height:100%;top:0;left:0;pointer-events:none;",
            {props.children}
        }
    }
}
