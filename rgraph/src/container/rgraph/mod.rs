//! Port of `xyflow-react/src/container/ReactFlow/index.tsx`.
//!
//! Status: Phase 0 — stub.
//!
//! The top-level `RGraph` Dioxus component — public entry point of the
//! crate. Mirrors `<ReactFlow {...props}>`. Internally:
//!   * Wraps children in `RGraphProvider` (via [`wrapper`]).
//!   * Renders `StoreUpdater`, `GraphView`, `SelectionListener`,
//!     `Attribution`, `A11yDescriptions`.
//!
//! TODO(rgraph/phase7): port the component.

pub mod init_values;
pub mod wrapper;
