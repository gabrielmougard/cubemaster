//! Port of `xyflow-react/src/additional-components/Background/Patterns.tsx`.
//!
//! Status: Phase 8 — implemented.
//!
//! Two sub-components are emitted as children of the `<pattern>` block
//! in [`super::background::Background`]: a single dot or a line cross.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use super::types::BackgroundVariant;

#[derive(Props, Clone, PartialEq)]
pub struct DotPatternProps {
    pub radius: f64,
    #[props(default)]
    pub class_name: Option<String>,
}

#[component]
pub fn DotPattern(props: DotPatternProps) -> Element {
    let extra = props.class_name.clone().unwrap_or_default();
    let class = format!("react-flow__background-pattern dots {extra}");
    rsx! {
        circle {
            cx: "{props.radius}",
            cy: "{props.radius}",
            r: "{props.radius}",
            class: "{class}",
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct LinePatternProps {
    pub dimensions: (f64, f64),
    pub variant: BackgroundVariant,
    #[props(default = 1.0)]
    pub line_width: f64,
    #[props(default)]
    pub class_name: Option<String>,
}

#[component]
pub fn LinePattern(props: LinePatternProps) -> Element {
    let (w, h) = props.dimensions;
    let d = format!("M{} 0 V{} M0 {} H{}", w / 2.0, h, h / 2.0, w);
    let extra = props.class_name.clone().unwrap_or_default();
    let class = format!(
        "react-flow__background-pattern {} {}",
        props.variant.as_str(),
        extra
    );
    rsx! {
        path {
            "stroke-width": "{props.line_width}",
            d: "{d}",
            class: "{class}",
        }
    }
}
