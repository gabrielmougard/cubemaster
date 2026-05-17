//! Port of `xyflow-react/src/components/ConnectionLine/index.tsx`.
//!
//! Status: Phase 6 — implemented.
//!
//! Renders the in-flight connection line during a drag-to-connect
//! gesture. Reads the live connection state from the store via
//! [`crate::hooks::use_connection`] and draws an SVG path of the
//! requested [`ConnectionLineType`].

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::types::connection::ConnectionState;
use rgraph_core::types::edges::ConnectionLineType;
use rgraph_core::utils::connections::get_connection_status;
use rgraph_core::utils::edges::bezier::{get_bezier_path, GetBezierPathParams};
use rgraph_core::utils::edges::smoothstep::{get_smooth_step_path, GetSmoothStepPathParams};
use rgraph_core::utils::edges::straight::{get_straight_path, GetStraightPathParams};

use crate::components::edges::simple_bezier_edge::{get_simple_bezier_path, GetSimpleBezierPathParams};
use crate::context::use_rgraph_store;
use crate::hooks::use_connection::use_connection;
use crate::store::RGraphStore;
use crate::types::nodes::BuiltInNodeData;

#[derive(Props, Clone, PartialEq)]
pub struct ConnectionLineWrapperProps<
    N: Clone + PartialEq + 'static = BuiltInNodeData,
    E: Clone + PartialEq + 'static = (),
> {
    #[props(default = ConnectionLineType::Bezier)]
    pub type_: ConnectionLineType,
    #[props(default)]
    pub container_style: Option<String>,
    #[props(default)]
    pub style: Option<String>,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn ConnectionLineWrapper<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: ConnectionLineWrapperProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let nodes_connectable = *store.nodes_connectable.read();
    let width = *store.width.read();
    let height = *store.height.read();

    let connection = store.connection.read().clone();
    let (in_progress, is_valid) = match &connection {
        ConnectionState::InProgress(p) => (true, p.is_valid),
        ConnectionState::NoConnection => (false, None),
    };
    let render = width > 0.0 && nodes_connectable && in_progress;
    if !render {
        return rsx! {};
    }

    let g_class = match get_connection_status(is_valid) {
        Some(status) => format!("react-flow__connection {status}"),
        None => "react-flow__connection".to_string(),
    };
    let container_style = props.container_style.clone().unwrap_or_default();

    rsx! {
        svg {
            style: "{container_style}",
            width: "{width}",
            height: "{height}",
            class: "react-flow__connectionline react-flow__container",
            g {
                class: "{g_class}",
                ConnectionLine::<N, E> {
                    type_: props.type_,
                    style: props.style.clone(),
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct ConnectionLineProps<
    N: Clone + PartialEq + 'static = BuiltInNodeData,
    E: Clone + PartialEq + 'static = (),
> {
    pub type_: ConnectionLineType,
    #[props(default)]
    pub style: Option<String>,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn ConnectionLine<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: ConnectionLineProps<N, E>,
) -> Element {
    let connection = use_connection::<N, E>();
    let ConnectionState::InProgress(p) = connection else {
        return rsx! {};
    };

    let path = match props.type_ {
        ConnectionLineType::Bezier => {
            let (path, _, _, _, _) = get_bezier_path(GetBezierPathParams {
                source_x: p.from.x,
                source_y: p.from.y,
                source_position: p.from_position,
                target_x: p.to.x,
                target_y: p.to.y,
                target_position: p.to_position,
                curvature: 0.25,
            });
            path
        }
        ConnectionLineType::SimpleBezier => {
            let (path, _, _, _, _) = get_simple_bezier_path(GetSimpleBezierPathParams {
                source_x: p.from.x,
                source_y: p.from.y,
                source_position: p.from_position,
                target_x: p.to.x,
                target_y: p.to.y,
                target_position: p.to_position,
            });
            path
        }
        ConnectionLineType::Step => {
            let (path, _, _, _, _) = get_smooth_step_path(GetSmoothStepPathParams {
                source_x: p.from.x,
                source_y: p.from.y,
                source_position: p.from_position,
                target_x: p.to.x,
                target_y: p.to.y,
                target_position: p.to_position,
                border_radius: 0.0,
                center_x: None,
                center_y: None,
                offset: 20.0,
                step_position: 0.5,
            });
            path
        }
        ConnectionLineType::SmoothStep => {
            let (path, _, _, _, _) = get_smooth_step_path(GetSmoothStepPathParams {
                source_x: p.from.x,
                source_y: p.from.y,
                source_position: p.from_position,
                target_x: p.to.x,
                target_y: p.to.y,
                target_position: p.to_position,
                border_radius: 5.0,
                center_x: None,
                center_y: None,
                offset: 20.0,
                step_position: 0.5,
            });
            path
        }
        ConnectionLineType::Straight => {
            let (path, _, _, _, _) = get_straight_path(GetStraightPathParams {
                source_x: p.from.x,
                source_y: p.from.y,
                target_x: p.to.x,
                target_y: p.to.y,
            });
            path
        }
    };

    let style_str = props.style.clone().unwrap_or_default();
    rsx! {
        path {
            d: "{path}",
            fill: "none",
            class: "react-flow__connection-path",
            style: "{style_str}",
        }
    }
}
