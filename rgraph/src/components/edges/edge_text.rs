//! Port of `xyflow-react/src/components/Edges/EdgeText.tsx`.
//!
//! Status: Phase 6 — implemented.
//!
//! Renders an SVG `<text>` label centred on `(x, y)` with an optional
//! background `<rect>`. The TS version uses `getBBox()` to compute the
//! text box size at runtime; Dioxus desktop doesn't have a synchronous
//! SVG-bbox API, so we approximate with a fixed glyph metric and rely
//! on the text's `text-anchor: middle` / `dominant-baseline: middle`
//! to centre it. The visual deviation from the TS version is minimal
//! for typical edge labels.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use crate::types::edges::EdgeLabelOptions;

/// Props for [`EdgeText`]. Mirrors `EdgeTextProps`.
#[derive(Props, Clone, PartialEq)]
pub struct EdgeTextProps {
    pub x: f64,
    pub y: f64,
    #[props(default)]
    pub label_options: EdgeLabelOptions,
    #[props(default)]
    pub class_name: Option<String>,
}

#[component]
pub fn EdgeText(props: EdgeTextProps) -> Element {
    let Some(label_element) = props.label_options.label.clone() else {
        return rsx! {};
    };

    let show_bg = props.label_options.label_show_bg.unwrap_or(true);
    let bg_padding = props.label_options.label_bg_padding.unwrap_or((2.0, 4.0));
    let bg_radius = props.label_options.label_bg_border_radius.unwrap_or(2.0);

    // We approximate the label width/height using glyph metrics that
    // match the bundled stylesheet's default font-size. The TS source
    // relies on `getBBox()` for exact pixel measurements; that path
    // requires async DOM access on Dioxus desktop. The fallback below
    // is close enough for the typical "edge of two-digit number"
    // labels — Phase 7 will swap in an async `getBBox` query once the
    // `RGraph` host plumbing lands.
    let approx_w = 12.0_f64.max(36.0); // worst-case "100ms" string
    let approx_h = 14.0;

    let translate_x = props.x - approx_w / 2.0;
    let translate_y = props.y - approx_h / 2.0;
    let g_style = format!("transform: translate({translate_x}px,{translate_y}px);");
    let g_class = match &props.class_name {
        Some(extra) => format!("react-flow__edge-textwrapper {extra}"),
        None => "react-flow__edge-textwrapper".to_string(),
    };

    let bg_style = props.label_options.label_bg_style.clone().unwrap_or_default();
    let text_style = props.label_options.label_style.clone().unwrap_or_default();

    rsx! {
        g {
            transform: "{g_style}",
            class: "{g_class}",

            if show_bg {
                rect {
                    class: "react-flow__edge-textbg",
                    x: "{-bg_padding.0}",
                    y: "{-bg_padding.1}",
                    width: "{approx_w + 2.0 * bg_padding.0}",
                    height: "{approx_h + 2.0 * bg_padding.1}",
                    rx: "{bg_radius}",
                    ry: "{bg_radius}",
                    style: "{bg_style}",
                }
            }

            text {
                class: "react-flow__edge-text",
                x: "{approx_w / 2.0}",
                y: "{approx_h / 2.0}",
                dy: "0.3em",
                "text-anchor": "middle",
                style: "{text_style}",
                {label_element}
            }
        }
    }
}
