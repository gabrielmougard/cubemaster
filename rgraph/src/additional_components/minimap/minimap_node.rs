//! Port of `xyflow-react/src/additional-components/MiniMap/MiniMapNode.tsx`.
//!
//! Status: Phase 8 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;
use dioxus::events::MouseEvent;

#[derive(Props, Clone)]
pub struct MiniMapNodeProps {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub border_radius: f64,
    pub class_name: String,
    #[props(default)]
    pub color: Option<String>,
    pub shape_rendering: String,
    #[props(default)]
    pub stroke_color: Option<String>,
    #[props(default)]
    pub stroke_width: Option<f64>,
    #[props(default)]
    pub style: Option<String>,
    pub selected: bool,
    #[props(default)]
    pub on_click: Option<EventHandler<(MouseEvent, String)>>,
}

impl PartialEq for MiniMapNodeProps {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.x == other.x
            && self.y == other.y
            && self.width == other.width
            && self.height == other.height
            && self.border_radius == other.border_radius
            && self.class_name == other.class_name
            && self.color == other.color
            && self.shape_rendering == other.shape_rendering
            && self.stroke_color == other.stroke_color
            && self.stroke_width == other.stroke_width
            && self.style == other.style
            && self.selected == other.selected
    }
}

#[component]
pub fn MiniMapNode(props: MiniMapNodeProps) -> Element {
    let fill = props.color.clone().unwrap_or_default();
    let stroke = props.stroke_color.clone().unwrap_or_default();
    let stroke_width = props.stroke_width.unwrap_or(0.0);
    let mut class = String::from("react-flow__minimap-node");
    if props.selected {
        class.push_str(" selected");
    }
    if !props.class_name.is_empty() {
        class.push(' ');
        class.push_str(&props.class_name);
    }
    let style = format!("fill:{fill};stroke:{stroke};stroke-width:{stroke_width};");
    let id = props.id.clone();
    let on_click = props.on_click;

    rsx! {
        rect {
            class: "{class}",
            x: "{props.x}",
            y: "{props.y}",
            rx: "{props.border_radius}",
            ry: "{props.border_radius}",
            width: "{props.width}",
            height: "{props.height}",
            style: "{style}",
            "shape-rendering": "{props.shape_rendering}",
            onclick: move |e| {
                if let Some(h) = &on_click {
                    h.call((e, id.clone()));
                }
            },
        }
    }
}
