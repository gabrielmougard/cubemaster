//! Port of `xyflow-react/src/components/Panel/index.tsx`.
//!
//! Status: Phase 4 — implemented.
//!
//! The `<Panel>` component is a position-aware floating div. The TS
//! version splits the `position` enum on `-` and adds each fragment
//! as its own class (`top-left` → `top` + `left`). We mirror that
//! exactly.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::types::viewport::PanelPosition;

/// Props for [`Panel`]. Mirrors TS `PanelProps`.
#[derive(Props, Clone, PartialEq)]
pub struct PanelProps {
    /// The position of the panel. Defaults to `top-left`.
    #[props(default)]
    pub position: Option<PanelPosition>,
    /// Optional class names appended after the framework classes.
    #[props(default)]
    pub class_name: Option<String>,
    /// Optional `style="…"` snippet applied to the panel.
    #[props(default)]
    pub style: Option<String>,
    /// Optional explicit `data-message` attribute (used by
    /// `<Attribution>`). Mirrors the TS spread of arbitrary HTML
    /// attributes.
    #[props(default)]
    pub data_message: Option<String>,
    pub children: Element,
}

/// Convert a [`PanelPosition`] into the two CSS classes used by the
/// stylesheet (e.g. `(PanelPosition::TopLeft, ["top", "left"])`).
fn position_classes(p: PanelPosition) -> (&'static str, &'static str) {
    match p {
        PanelPosition::TopLeft => ("top", "left"),
        PanelPosition::TopCenter => ("top", "center"),
        PanelPosition::TopRight => ("top", "right"),
        PanelPosition::BottomLeft => ("bottom", "left"),
        PanelPosition::BottomCenter => ("bottom", "center"),
        PanelPosition::BottomRight => ("bottom", "right"),
        PanelPosition::CenterLeft => ("center", "left"),
        PanelPosition::CenterRight => ("center", "right"),
    }
}

/// The `<Panel />` component helps you position content above the
/// viewport. It is used internally by `<MiniMap />` and `<Controls />`
/// (Phase 8).
///
/// ```ignore
/// rsx! {
///     Panel { position: PanelPosition::TopLeft, "Hello" }
/// }
/// ```
#[component]
pub fn Panel(props: PanelProps) -> Element {
    let position = props.position.unwrap_or(PanelPosition::TopLeft);
    let (vertical, horizontal) = position_classes(position);

    let mut classes = String::from("react-flow__panel ");
    classes.push_str(vertical);
    classes.push(' ');
    classes.push_str(horizontal);
    if let Some(extra) = &props.class_name {
        classes.push(' ');
        classes.push_str(extra);
    }

    let style = props.style.clone().unwrap_or_default();

    if let Some(msg) = props.data_message {
        rsx! {
            div {
                class: "{classes}",
                style: "{style}",
                "data-message": "{msg}",
                {props.children}
            }
        }
    } else {
        rsx! {
            div {
                class: "{classes}",
                style: "{style}",
                {props.children}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_classes_split_correctly() {
        assert_eq!(position_classes(PanelPosition::TopLeft), ("top", "left"));
        assert_eq!(position_classes(PanelPosition::BottomCenter), ("bottom", "center"));
        assert_eq!(position_classes(PanelPosition::CenterRight), ("center", "right"));
    }
}
