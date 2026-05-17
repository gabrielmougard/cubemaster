//! Port of `xyflow-react/src/components/Edges/EdgeAnchor.tsx`.
//!
//! Status: Phase 6 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::events::MouseData;
use dioxus::prelude::*;

use rgraph_core::types::geometry::Position;

fn shift_x(x: f64, shift: f64, position: Position) -> f64 {
    match position {
        Position::Left => x - shift,
        Position::Right => x + shift,
        _ => x,
    }
}

fn shift_y(y: f64, shift: f64, position: Position) -> f64 {
    match position {
        Position::Top => y - shift,
        Position::Bottom => y + shift,
        _ => y,
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct EdgeAnchorProps {
    pub position: Position,
    pub center_x: f64,
    pub center_y: f64,
    #[props(default = 10.0)]
    pub radius: f64,
    /// `"source"` or `"target"` — used to compose the CSS class.
    pub type_: String,
    #[props(default)]
    pub on_mouse_down: Option<Callback<Event<MouseData>>>,
    #[props(default)]
    pub on_mouse_enter: Option<Callback<Event<MouseData>>>,
    #[props(default)]
    pub on_mouse_out: Option<Callback<Event<MouseData>>>,
}

/// Edge-update anchor: a transparent circle drawn at one end of an
/// edge. Pointer events on the circle drive the reconnect gesture.
#[component]
pub fn EdgeAnchor(props: EdgeAnchorProps) -> Element {
    let cx = shift_x(props.center_x, props.radius, props.position);
    let cy = shift_y(props.center_y, props.radius, props.position);
    let class_name = format!("react-flow__edgeupdater react-flow__edgeupdater-{}", props.type_);

    let on_mouse_down = move |evt: Event<MouseData>| {
        if let Some(cb) = props.on_mouse_down {
            cb.call(evt);
        }
    };
    let on_mouse_enter = move |evt: Event<MouseData>| {
        if let Some(cb) = props.on_mouse_enter {
            cb.call(evt);
        }
    };
    let on_mouse_out = move |evt: Event<MouseData>| {
        if let Some(cb) = props.on_mouse_out {
            cb.call(evt);
        }
    };

    rsx! {
        circle {
            class: "{class_name}",
            cx: "{cx}",
            cy: "{cy}",
            r: "{props.radius}",
            stroke: "transparent",
            fill: "transparent",
            onmousedown: on_mouse_down,
            onmouseenter: on_mouse_enter,
            onmouseout: on_mouse_out,
        }
    }
}
