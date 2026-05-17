//! Port of `xyflow-react/src/container/NodeRenderer/useResizeObserver.ts`.
//!
//! Status: Phase 5 — implemented.
//!
//! Owns the *shared* `ResizeObserver` shim installation at the
//! [`crate::container::node_renderer`] level. Per-node observation is
//! driven by [`crate::components::node_wrapper::use_node_observer`],
//! which polls `__rgraph_sizes` after each mount.
//!
//! The TS source allocates one `ResizeObserver` per `<NodeRenderer>`
//! and feeds entries into `updateNodeInternals`. Our Phase 5 port
//! defers to the per-node observer because Dioxus desktop lacks a
//! JS→Rust push channel as of 0.7.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::use_hook;

use crate::dom::resize_observer;

/// Install the resize-observer shim once for the lifetime of the
/// enclosing `<NodeRenderer>`. Returns nothing — the shim's API is
/// keyed by node id via [`crate::dom::resize_observer::observe`].
pub fn use_resize_observer() {
    use_hook(|| {
        resize_observer::install_once();
    });
}
