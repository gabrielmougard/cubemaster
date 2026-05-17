//! Port of `xyflow-react/src/additional-components/Controls/Icons/Minus.tsx`.
//!
//! Status: Phase 8 — implemented.

use dioxus::prelude::*;

#[component]
pub fn MinusIcon() -> Element {
    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            "viewBox": "0 0 32 5",
            path { d: "M0 0h32v4.2H0z" }
        }
    }
}
