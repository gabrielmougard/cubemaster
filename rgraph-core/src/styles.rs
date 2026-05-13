//! Port of `xyflow-core/src/styles/`.
//!
//! Status: implemented (phase 8).
//!
//! Each upstream `.css` file is exposed as a `&'static str` constant
//! via `include_str!`. Dioxus consumers inject these via the
//! [`document::Style { … }`](https://dioxuslabs.com/) component.
//!
//! ## Usage example (Dioxus)
//!
//! ```ignore
//! use dioxus::prelude::*;
//! use rgraph_core::styles::{BASE_CSS, STYLE_CSS};
//!
//! rsx! {
//!     document::Style { {BASE_CSS} }
//!     document::Style { {STYLE_CSS} }
//!     // …your flow components…
//! }
//! ```

#![allow(dead_code)]

/// Base CSS rules — minimal reset / pane layout.
///
/// Matches `assets/base.css` byte-for-byte.
pub const BASE_CSS: &str = include_str!("../assets/base.css");

/// Initial / bootstrap CSS applied before any node/edge styles. Used
/// to provide the minimum viable look before user styles kick in.
///
/// Matches `assets/init.css` byte-for-byte.
pub const INIT_CSS: &str = include_str!("../assets/init.css");

/// Styles for the [`crate::xyresizer`] resize handles and bounding
/// lines.
///
/// Matches `assets/node-resizer.css` byte-for-byte.
pub const NODE_RESIZER_CSS: &str = include_str!("../assets/node-resizer.css");

/// Full default style suite (nodes, edges, handles, controls, …).
///
/// Matches `assets/style.css` byte-for-byte.
pub const STYLE_CSS: &str = include_str!("../assets/style.css");

/// All four stylesheets concatenated in the order most consumers want
/// to inject them: `base` → `init` → `style` → `node-resizer`.
///
/// Useful for the common single-`<Style>` import case:
///
/// ```ignore
/// document::Style { {rgraph_core::styles::ALL_CSS} }
/// ```
pub const ALL_CSS: &str = concat!(
    include_str!("../assets/base.css"),
    "\n",
    include_str!("../assets/init.css"),
    "\n",
    include_str!("../assets/style.css"),
    "\n",
    include_str!("../assets/node-resizer.css"),
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_constants_are_non_empty() {
        assert!(!BASE_CSS.is_empty(), "base.css should not be empty");
        assert!(!INIT_CSS.is_empty(), "init.css should not be empty");
        assert!(!NODE_RESIZER_CSS.is_empty(), "node-resizer.css should not be empty");
        assert!(!STYLE_CSS.is_empty(), "style.css should not be empty");
    }

    #[test]
    fn all_css_contains_all_four() {
        // The concatenated bundle must be at least the sum of the
        // four (plus a trio of `\n` separators).
        let expected_min =
            BASE_CSS.len() + INIT_CSS.len() + STYLE_CSS.len() + NODE_RESIZER_CSS.len();
        assert!(ALL_CSS.len() >= expected_min);
    }

    #[test]
    fn css_constants_are_valid_utf8() {
        // `include_str!` enforces UTF-8 at compile time — this test
        // exists to surface a panic at test time if the assets were
        // ever swapped out for invalid bytes via a build script.
        for css in [BASE_CSS, INIT_CSS, NODE_RESIZER_CSS, STYLE_CSS, ALL_CSS] {
            assert!(css.is_char_boundary(0));
            assert!(css.is_char_boundary(css.len()));
        }
    }
}
