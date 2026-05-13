//! `rgraph-core` — Rust port of [`@xyflow/system`](https://github.com/xyflow/xyflow/tree/main/packages/system).
//!
//! Framework-agnostic core for node-graph UIs, intended to be consumed by
//! a Dioxus components crate. The DOM-listener-attaching half of the JS
//! original is intentionally omitted; downstream Dioxus components feed
//! raw events into the pure state machines exposed here.
//!
//! See the crate-level `README.md` for the full porting roadmap and
//! per-module status.
//!
//! # Module overview
//!
//! * [`constants`]      — error message catalogue and shared defaults.
//! * [`types`]          — pure data types (geometry, nodes, edges, …).
//! * [`utils`]          — pure functions: math, graph, edge paths, store.
//! * [`xypanzoom`]      — viewport pan/zoom (wraps [`rgraph_zoom`]).
//! * [`xydrag`]         — node/selection drag (wraps [`rgraph_drag`]).
//! * [`xyhandle`]       — connection / handle state machine.
//! * [`xyresizer`]      — node resize (wraps [`rgraph_drag`]).
//! * [`xyminimap`]      — minimap viewport interaction (wraps [`rgraph_zoom`]).
//! * [`styles`]         — `&'static str` constants for the four bundled
//!                        stylesheets.
//! * [`promise`]        — tiny std-only `Promise<T>` used in place of JS
//!                        `Promise<T>` for completion signalling on
//!                        animated viewport changes.

#![allow(dead_code)]

pub mod constants;
pub mod promise;
pub mod styles;
pub mod types;
pub mod utils;

pub mod xydrag;
pub mod xyhandle;
pub mod xyminimap;
pub mod xypanzoom;
pub mod xyresizer;

// Convenience re-exports — the top-level API surface should mirror
// `@xyflow/system`'s `export * from './…';` pattern in `src/index.ts`.
//
// NOTE: `types::edges` and `utils::edges` are both submodules. Once the
// glob re-exports below carry real items, we may need to disambiguate
// (e.g. `pub use types::edges as edge_types;`). For now the modules are
// empty so no clash occurs at use sites.
//
// TODO(rgraph-core/phase0): expand these re-exports as types are filled in.
pub use constants::*;
pub use promise::Promise;
#[allow(ambiguous_glob_reexports)]
pub use types::*;
#[allow(ambiguous_glob_reexports)]
pub use utils::*;
