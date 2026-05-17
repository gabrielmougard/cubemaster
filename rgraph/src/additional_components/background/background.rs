//! Port of `xyflow-react/src/additional-components/Background/Background.tsx`.
//!
//! Status: Phase 8 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use crate::context::use_rgraph_store;
use crate::store::RGraphStore;

use super::patterns::{DotPattern, LinePattern};
use super::types::{BackgroundGap, BackgroundOffset, BackgroundVariant};

#[derive(Props, Clone, PartialEq)]
pub struct BackgroundProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    /// Unique id when multiple `<Background>` instances are present
    /// on the same page. Appended to `pattern-{rfId}`.
    #[props(default)]
    pub id: Option<String>,
    #[props(default = BackgroundVariant::Dots)]
    pub variant: BackgroundVariant,
    #[props(default = BackgroundGap::Uniform(20.0))]
    pub gap: BackgroundGap,
    /// Pattern dot/cross size. `None` → default (1 for dots/lines, 6
    /// for cross).
    #[props(default)]
    pub size: Option<f64>,
    #[props(default = 1.0)]
    pub line_width: f64,
    #[props(default = BackgroundOffset::Uniform(0.0))]
    pub offset: BackgroundOffset,
    #[props(default)]
    pub color: Option<String>,
    #[props(default)]
    pub bg_color: Option<String>,
    #[props(default)]
    pub style: Option<String>,
    #[props(default)]
    pub class_name: Option<String>,
    #[props(default)]
    pub pattern_class_name: Option<String>,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn Background<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: BackgroundProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let transform = *store.transform.read();
    let rf_id = store.rf_id.read().clone();

    let default_size = match props.variant {
        BackgroundVariant::Dots | BackgroundVariant::Lines => 1.0,
        BackgroundVariant::Cross => 6.0,
    };
    let pattern_size = props.size.unwrap_or(default_size);
    let is_dots = matches!(props.variant, BackgroundVariant::Dots);
    let is_cross = matches!(props.variant, BackgroundVariant::Cross);

    let (gap_x, gap_y) = props.gap.as_tuple();
    let zoom = transform.scale();
    let scaled_gap = (
        if gap_x * zoom > 0.0 { gap_x * zoom } else { 1.0 },
        if gap_y * zoom > 0.0 { gap_y * zoom } else { 1.0 },
    );
    let scaled_size = pattern_size * zoom;
    let (offset_x, offset_y) = props.offset.as_tuple();

    let pattern_dimensions = if is_cross {
        (scaled_size, scaled_size)
    } else {
        scaled_gap
    };
    let scaled_offset = (
        if offset_x * zoom > 0.0 { offset_x * zoom } else { 1.0 + pattern_dimensions.0 / 2.0 },
        if offset_y * zoom > 0.0 { offset_y * zoom } else { 1.0 + pattern_dimensions.1 / 2.0 },
    );

    let pattern_id = match &props.id {
        Some(s) if !s.is_empty() => format!("pattern-{rf_id}{s}"),
        _ => format!("pattern-{rf_id}"),
    };

    // CSS custom properties + container-style + user style.
    let mut style_str = String::from("position:absolute;width:100%;height:100%;top:0;left:0;");
    if let Some(c) = &props.bg_color {
        style_str.push_str(&format!("--xy-background-color-props:{c};"));
    }
    if let Some(c) = &props.color {
        style_str.push_str(&format!("--xy-background-pattern-color-props:{c};"));
    }
    if let Some(extra) = &props.style {
        style_str.push_str(extra);
    }

    let class_str = match &props.class_name {
        Some(extra) => format!("react-flow__background {extra}"),
        None => "react-flow__background".to_string(),
    };

    let pattern_x = transform.tx() % scaled_gap.0;
    let pattern_y = transform.ty() % scaled_gap.1;
    let pattern_transform = format!(
        "translate(-{},-{})",
        scaled_offset.0, scaled_offset.1
    );
    let fill = format!("url(#{pattern_id})");

    rsx! {
        svg {
            class: "{class_str}",
            style: "{style_str}",
            "data-testid": "rf__background",
            pattern {
                id: "{pattern_id}",
                x: "{pattern_x}",
                y: "{pattern_y}",
                width: "{scaled_gap.0}",
                height: "{scaled_gap.1}",
                "pattern-units": "userSpaceOnUse",
                "pattern-transform": "{pattern_transform}",
                if is_dots {
                    DotPattern {
                        radius: scaled_size / 2.0,
                        class_name: props.pattern_class_name.clone(),
                    }
                } else {
                    LinePattern {
                        dimensions: pattern_dimensions,
                        line_width: props.line_width,
                        variant: props.variant,
                        class_name: props.pattern_class_name.clone(),
                    }
                }
            }
            rect {
                x: "0",
                y: "0",
                width: "100%",
                height: "100%",
                fill: "{fill}",
            }
        }
    }
}
