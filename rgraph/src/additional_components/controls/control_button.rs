//! Port of `xyflow-react/src/additional-components/Controls/ControlButton.tsx`.
//!
//! Status: Phase 8 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;
use dioxus::events::MouseEvent;

#[derive(Props, Clone)]
pub struct ControlButtonProps {
    #[props(default)]
    pub class_name: Option<String>,
    #[props(default)]
    pub title: Option<String>,
    #[props(default)]
    pub aria_label: Option<String>,
    #[props(default)]
    pub disabled: bool,
    #[props(default)]
    pub on_click: Option<EventHandler<MouseEvent>>,
    pub children: Element,
}

impl PartialEq for ControlButtonProps {
    fn eq(&self, other: &Self) -> bool {
        self.class_name == other.class_name
            && self.title == other.title
            && self.aria_label == other.aria_label
            && self.disabled == other.disabled
    }
}

/// `<ControlButton>`. Adds the `react-flow__controls-button` class to
/// the supplied [`Self::class_name`].
#[component]
pub fn ControlButton(props: ControlButtonProps) -> Element {
    let mut class = String::from("react-flow__controls-button");
    if let Some(extra) = &props.class_name {
        class.push(' ');
        class.push_str(extra);
    }
    let title = props.title.clone().unwrap_or_default();
    let aria_label = props.aria_label.clone().unwrap_or_default();
    let on_click = props.on_click;

    rsx! {
        button {
            r#type: "button",
            class: "{class}",
            title: "{title}",
            "aria-label": "{aria_label}",
            disabled: props.disabled,
            onclick: move |e| {
                if let Some(h) = &on_click {
                    h.call(e);
                }
            },
            {props.children}
        }
    }
}
