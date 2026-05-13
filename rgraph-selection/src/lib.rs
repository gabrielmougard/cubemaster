//! `rgraph-selection` — Rust port of d3-selection's data-join and naming
//! primitives, adapted for declarative VDOMs (Dioxus).
//!
//! # Why this port omits the DOM half of d3-selection
//!
//! d3-selection is fundamentally a **live-DOM mutation library**: most of
//! its surface (`attr`, `style`, `text`, `html`, `append`, `insert`,
//! `remove`, `classed`, `property`, `on`, `dispatch`, `.node()`, `.raise()`,
//! `.lower()`) calls directly into `Element.setAttribute`, `Element.style`,
//! `Element.appendChild`, etc. Those operations have no idiomatic
//! counterpart in Dioxus, which uses a declarative VDOM: you describe the
//! desired tree via `rsx!{}` and the framework reconciles the DOM
//! automatically.
//!
//! This crate ports the parts of d3-selection that are **algorithmically
//! useful regardless of how rendering happens**:
//!
//! * [`namespace`] — XML namespace prefixes (`svg:`, `xlink:`, `xml:`,
//!   `xmlns:`, `xhtml:`) and the qualified-name parser. Useful when an app
//!   constructs SVG markup at runtime from user-typed attribute names.
//! * [`typenames`] — the dot-namespaced event-name parser
//!   (`"click.zoom mouseover.tooltip"`).
//! * [`local`] — `Local<K, T>`: a generic per-node data store with the same
//!   "walk up the parent chain" lookup semantics as d3's `local()`. Key is
//!   any `Hash + Eq + Clone` (typically a Dioxus element id).
//! * [`data_join`] — the algorithmic heart of d3-selection. Two layers:
//!   - [`data_join::bind_index`] / [`data_join::bind_key`] — faithful ports
//!     of `bindIndex`/`bindKey` returning a `JoinResult<D, N>` with
//!     `enter`, `update`, `exit`, plus the `next_update` insertion-order
//!     link.
//!   - [`data_join::KeyedDiff`] — a stateful Dioxus-friendly reconciler
//!     that takes new data each render and produces a plan
//!     (enter/update/exit/order) the caller drives via stable `key={}`
//!     attributes in `rsx!{}`.
//!
//! See module-level docs for usage details.

pub mod data_join;
pub mod local;
pub mod namespace;
pub mod typenames;

// Convenience re-exports — the most-used items.
pub use data_join::{DiffEntry, DiffPlan, EnterEntry, JoinResult, KeyedDiff, bind_index, bind_key};
pub use local::{Local, LocalId};
pub use namespace::{Name, SVG, XHTML, XLINK, XML, XMLNS, defaults as namespace_defaults};
pub use typenames::{Typename, parse_one as parse_typename, parse_typenames};
