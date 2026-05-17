//! Port of `xyflow-react/src/container/ReactFlow/Wrapper.tsx`.
//!
//! Status: Phase 0 — stub.
//!
//! Auto-wraps the children in `RGraphProvider` *only* if there isn't one
//! already in context (so users can choose to mount `<RGraphProvider>`
//! themselves to share a store across sibling `<RGraph>`s).
//!
//! TODO(rgraph/phase7): port the conditional-provider logic.
