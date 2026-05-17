//! Port of `xyflow-react/src/additional-components/NodeResizer/NodeResizer.tsx`.
//!
//! Status: Phase 8 — fully implemented.
//!
//! Renders four line controls + four handle controls around the
//! containing node. Each control is an [`NodeResizeControl`] which
//! runs an [`XYResizer`] state machine and dispatches the resulting
//! `NodeChange`s through [`crate::store::actions::trigger_node_changes`].

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;
use rgraph_core::xyresizer::types::{
    ResizeControlVariant, XY_RESIZER_HANDLE_POSITIONS, XY_RESIZER_LINE_POSITIONS,
};

use crate::types::nodes::BuiltInNodeData;

use super::node_resize_control::NodeResizeControl;
use super::types::NodeResizerCommon;

#[derive(Props, Clone, PartialEq)]
pub struct NodeResizerProps<
    N: Clone + PartialEq + 'static = BuiltInNodeData,
    E: Clone + PartialEq + 'static = (),
> {
    #[props(default)]
    pub node_id: Option<String>,
    #[props(default)]
    pub color: Option<String>,
    #[props(default)]
    pub handle_class_name: Option<String>,
    #[props(default)]
    pub handle_style: Option<String>,
    #[props(default)]
    pub line_class_name: Option<String>,
    #[props(default)]
    pub line_style: Option<String>,
    #[props(default = true)]
    pub is_visible: bool,
    #[props(default = 10.0)]
    pub min_width: f64,
    #[props(default = 10.0)]
    pub min_height: f64,
    #[props(default = f64::MAX)]
    pub max_width: f64,
    #[props(default = f64::MAX)]
    pub max_height: f64,
    #[props(default = false)]
    pub keep_aspect_ratio: bool,
    #[props(default = true)]
    pub auto_scale: bool,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn NodeResizer<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: NodeResizerProps<N, E>,
) -> Element {
    if !props.is_visible {
        return rsx! {};
    }
    let common = NodeResizerCommon {
        node_id: props.node_id.clone(),
        color: props.color.clone(),
        min_width: props.min_width,
        min_height: props.min_height,
        max_width: props.max_width,
        max_height: props.max_height,
        keep_aspect_ratio: props.keep_aspect_ratio,
        auto_scale: props.auto_scale,
        should_resize: None,
        on_resize_start: None,
        on_resize: None,
        on_resize_end: None,
    };

    let lines = XY_RESIZER_LINE_POSITIONS.iter().map(|p| {
        let key = format!("line-{:?}", p);
        rsx! {
            NodeResizeControl::<N, E> {
                key: "{key}",
                node_id: props.node_id.clone(),
                position: Some(*p),
                variant: ResizeControlVariant::Line,
                class_name: props.line_class_name.clone(),
                style: props.line_style.clone(),
                color: props.color.clone(),
                common: common.clone(),
            }
        }
    });
    let handles = XY_RESIZER_HANDLE_POSITIONS.iter().map(|p| {
        let key = format!("handle-{:?}", p);
        rsx! {
            NodeResizeControl::<N, E> {
                key: "{key}",
                node_id: props.node_id.clone(),
                position: Some(*p),
                variant: ResizeControlVariant::Handle,
                class_name: props.handle_class_name.clone(),
                style: props.handle_style.clone(),
                color: props.color.clone(),
                common: common.clone(),
            }
        }
    });

    rsx! {
        for el in lines { {el} }
        for el in handles { {el} }
    }
}
