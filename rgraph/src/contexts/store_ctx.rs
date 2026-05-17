//! Port of `xyflow-react/src/contexts/StoreContext.ts`.
//!
//! Status: Phase 2 — implemented as a re-export.
//!
//! The TS source defines `StoreContext` (a React Context) and exports
//! its `Provider`. In the Rust port the equivalent machinery lives in
//! [`crate::context`] (Dioxus' `use_context_provider` / `use_context`
//! pair). This module keeps the TS file-name parity by re-exporting
//! the same symbols.

pub use crate::context::*;
