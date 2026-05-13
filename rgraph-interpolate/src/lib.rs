//! `rgraph-interpolate` — Rust port of [d3-interpolate](https://github.com/d3/d3-interpolate).
//!
//! Provides a family of interpolators that take two endpoints and return
//! a `Fn(t: f64) -> T` producing values along the way. All interpolators
//! are exact at `t = 0` and `t = 1` (matching d3's documented contract)
//! and well-defined for `t ∉ [0, 1]` via natural extension of the
//! underlying formula.
//!
//! # Modules
//!
//! * [`number`] — `interpolate_number`, `interpolate_round`,
//!   `interpolate_date_ms` / `interpolate_date_i128` (ports of `number.js`,
//!   `round.js`, `date.js`).
//! * [`array`] — `interpolate_number_array`, `interpolate_array_with`,
//!   `interpolate_number_matrix`, `interpolate_number_object`,
//!   `interpolate_object_with` (ports of `numberArray.js`, `array.js`,
//!   `object.js`).
//! * [`discrete`] — `interpolate_discrete`, `quantize`, `piecewise`
//!   (ports of `discrete.js`, `quantize.js`, `piecewise.js`).
//! * [`basis`] — `interpolate_basis`, `interpolate_basis_closed`, plus
//!   the `basis_step` low-level kernel (ports of `basis.js`,
//!   `basisClosed.js`).
//! * [`string_interp`] — `interpolate_string` (port of `string.js`),
//!   numeric-aware string interpolator preserving `b`'s static segments.
//! * [`color_helpers`] — internal `nogamma` / `gamma` / `hue` helpers
//!   shared by all color-space interpolators (port of `color.js`).
//! * [`rgb`], [`hsl`], [`lab`], [`hcl`], [`cubehelix`], [`hue`] — the
//!   color-space interpolators built on top of `rgraph_color`.
//! * [`zoom`] — smooth zoom-and-pan interpolator with `.duration`
//!   (port of `zoom.js`).
//!
//! # Deliberately not ported
//!
//! * `interpolate(a, b)` (the polymorphic dispatcher) — relies on JS's
//!   runtime `typeof`. Rust callers know which type they have at compile
//!   time and call the typed interpolator directly.
//! * `interpolateTransformCss` / `interpolateTransformSvg` — built on
//!   `DOMMatrix` / SVG's `transform.baseVal`, both browser-only. In a
//!   Dioxus app you typically interpolate `(x, y, rotate, scale)` as
//!   numeric tweens directly.
//! * `interpolateDate` — d3 returns a JS `Date` object. The Rust
//!   equivalent is [`number::interpolate_date_ms`] /
//!   [`number::interpolate_date_i128`] over plain epoch integers, which
//!   round-trip cleanly to any Rust date type (`std::time::SystemTime`,
//!   `chrono::DateTime`, etc.) without forcing a date dependency on
//!   downstream crates.

pub mod array;
pub mod basis;
pub mod color_helpers;
pub mod cubehelix;
pub mod discrete;
pub mod hcl;
pub mod hsl;
pub mod hue;
pub mod lab;
pub mod number;
pub mod rgb;
pub mod string_interp;
pub mod zoom;

/// Boxed string interpolator returned by every color-space interpolator
/// and by [`string_interp::interpolate_string`]. Aliased so the various
/// public signatures stay readable.
pub type StringInterp = Box<dyn Fn(f64) -> String>;

// Convenience re-exports — the most-used items.
pub use array::{
    interpolate_array_with, interpolate_number_array, interpolate_number_matrix,
    interpolate_number_object, interpolate_object_with,
};
pub use basis::{basis_step, interpolate_basis, interpolate_basis_closed};
pub use cubehelix::{
    interpolate_cubehelix, interpolate_cubehelix_gamma, interpolate_cubehelix_gamma_str,
    interpolate_cubehelix_long, interpolate_cubehelix_long_gamma,
    interpolate_cubehelix_long_str, interpolate_cubehelix_str,
};
pub use discrete::{interpolate_discrete, piecewise, quantize};
pub use hcl::{interpolate_hcl, interpolate_hcl_long, interpolate_hcl_long_str, interpolate_hcl_str};
pub use hsl::{interpolate_hsl, interpolate_hsl_long, interpolate_hsl_long_str, interpolate_hsl_str};
pub use hue::interpolate_hue;
pub use lab::{interpolate_lab, interpolate_lab_str};
pub use number::{interpolate_date_i128, interpolate_date_ms, interpolate_number, interpolate_round};
pub use rgb::{
    interpolate_rgb, interpolate_rgb_basis, interpolate_rgb_basis_closed,
    interpolate_rgb_gamma, interpolate_rgb_gamma_str, interpolate_rgb_str,
    interpolate_rgb_typed, rgb_spline,
};
pub use string_interp::interpolate_string;
pub use zoom::{ZoomInterpolator, interpolate_zoom, interpolate_zoom_rho};
