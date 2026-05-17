//! Port of `xyflow-react/src/styles/`.
//!
//! Exposes the two bundled CSS files as `&'static str` constants so the
//! companion app can `include` them into a Dioxus `<style>` block.
//!
//! The CSS itself is a verbatim copy of the original under
//! `../assets/{base,style}.css`.

pub mod utils;

/// Verbatim copy of `xyflow-react/src/styles/base.css`.
///
/// Minimal layout-only stylesheet (`.react-flow__*` class names).
pub const BASE_CSS: &str = include_str!("../../assets/base.css");

/// Verbatim copy of `xyflow-react/src/styles/style.css`.
///
/// Full default theme.
pub const STYLE_CSS: &str = include_str!("../../assets/style.css");
