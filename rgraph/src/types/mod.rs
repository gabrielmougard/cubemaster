//! Port of `xyflow-react/src/types/index.ts`.
//!
//! Status: Phase 1 — re-exports active.
//!
//! Each submodule mirrors a single TS file:
//!
//! * `nodes.rs`          ← `types/nodes.ts`
//! * `edges.rs`          ← `types/edges.ts`
//! * `general.rs`        ← `types/general.ts`
//! * `instance.rs`       ← `types/instance.ts`
//! * `store.rs`          ← `types/store.ts`
//! * `component_props.rs`← `types/component-props.ts`

pub mod component_props;
pub mod edges;
pub mod general;
pub mod instance;
pub mod nodes;
pub mod store;

// Mirror the TS `export * from './…'` pattern.
pub use component_props::*;
pub use edges::*;
pub use general::*;
pub use instance::*;
pub use nodes::*;
pub use store::*;
