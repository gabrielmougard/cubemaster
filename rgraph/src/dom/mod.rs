//! `dom` — bridges to the Dioxus desktop webview for DOM-only APIs.
//!
//! Status: Phase 4 — implemented (resize-observer is partial pending
//! Phase 5 stream-based dispatch).
//!
//! These modules are the only places the crate is allowed to call
//! `dioxus::document::eval` (or the equivalent webview JS bridge).
//! Every other module deals in pure Rust state.
//!
//! Submodules:
//!   * [`eval`]            — tiny shared helpers around `eval`.
//!   * [`pointer`]         — converts Dioxus pointer events into
//!                           the [`rgraph_zoom::PointerInput`] /
//!                           `rgraph_drag` formats.
//!   * [`wheel`]           — converts Dioxus wheel events into the
//!                           [`rgraph_zoom::WheelInput`] format.
//!   * [`resize_observer`] — JS-bridged `ResizeObserver` shim used by
//!                           [`crate::hooks::use_resize_handler`] and
//!                           per-node measurement (Phase 5).

pub mod eval;
pub mod pointer;
pub mod resize_observer;
pub mod wheel;

pub use wheel::PaneBounds;
